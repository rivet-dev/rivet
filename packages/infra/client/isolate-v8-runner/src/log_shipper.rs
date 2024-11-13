use std::{
	io::{BufRead, Write},
	net::{SocketAddr, TcpStream},
	sync::mpsc,
	thread::JoinHandle,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::*;
use serde::Serialize;
use serde_json;
use uuid::Uuid;

use crate::{config::ActorOwner, throttle};

/// Maximum length of a single log line
pub const MAX_LINE_BYTES: usize = 1024;

/// Maximum number of bytes to buffer before dropping logs
pub const MAX_BUFFER_BYTES: usize = 1024 * 1024;

/// Maximum number of lines to print to stdout for debugging. This helps
/// identify the reasons for program crashes from the isolate's output.
const MAX_PREVIEW_LINES: usize = 128;

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum StreamType {
	StdOut = 0,
	StdErr = 1,
}

pub struct ReceivedMessage {
	pub stream_type: StreamType,
	pub ts: u64,
	pub message: String,
}

/// Sends logs from the container to the Vector agent on the machine.
///
/// This will run until the `msg_rx` sender is dropped before shutting down.
///
/// If attempting to reconnect while the runner is shut down, this will exit immediately, dropping
/// all logs in the process. This is to ensure that if Vector becomes unreachable, we don't end up
/// with a lot of lingering runners that refuse to exit.
pub struct LogShipper {
	pub actor_id: Uuid,

	/// Notifies of process shutdown.
	pub shutdown_rx: mpsc::Receiver<()>,

	/// Receiver for messages to be shipped. This holds a buffer of messages waiting to be send.
	///
	/// If the socket closes or creates back pressure, logs will be dropped on the main thread when
	/// trying to send to this channel.
	pub msg_rx: mpsc::Receiver<ReceivedMessage>,

	pub vector_socket_addr: SocketAddr,

	pub owner: ActorOwner,
}

impl LogShipper {
	pub fn spawn(self) -> JoinHandle<()> {
		std::thread::spawn(move || self.run())
	}

	fn run(self) {
		// Retry loop
		loop {
			match self.run_inner() {
				Result::Ok(()) => {
					println!("{}: Exiting log shipper", self.actor_id);
					break;
				}
				Err(err) => {
					eprintln!("{}: Log shipper error: {err:?}", self.actor_id);

					// Wait before attempting to reconnect. Wait for disconnect in this time
					// period.
					match self
						.shutdown_rx
						.recv_timeout(std::time::Duration::from_secs(15))
					{
						Result::Ok(_) => {
							println!("{}: Log shipper received shutdown", self.actor_id);
							break;
						}
						Err(mpsc::RecvTimeoutError::Disconnected) => {
							eprintln!(
								"{}: Log shipper shutdown unexpectedly disconnected",
								self.actor_id
							);
							break;
						}
						Err(mpsc::RecvTimeoutError::Timeout) => {
							// Not shut down, attempt reconnect
						}
					}
				}
			}
		}
	}

	fn run_inner(&self) -> Result<()> {
		println!(
			"{}: Connecting log shipper to Vector at {}",
			self.actor_id, self.vector_socket_addr
		);

		let mut stream = TcpStream::connect(self.vector_socket_addr)?;

		println!("{}: Log shipper connected", self.actor_id);

		while let Result::Ok(message) = self.msg_rx.recv() {
			let vector_message = match &self.owner {
				ActorOwner::DynamicServer { server_id } => VectorMessage::DynamicServers {
					server_id: server_id.as_str(),
					task: "main", // Backwards compatibility with logs
					stream_type: message.stream_type as u8,
					ts: message.ts,
					message: message.message.as_str(),
				},
			};

			serde_json::to_writer(&mut stream, &vector_message)?;
			stream.write_all(b"\n")?;
		}

		println!("{}: Log shipper msg_rx disconnected", self.actor_id);

		Ok(())
	}
}

/// Vector-compatible message format
#[derive(Serialize)]
#[serde(tag = "source")]
enum VectorMessage<'a> {
	#[serde(rename = "dynamic_servers")]
	DynamicServers {
		server_id: &'a str,
		task: &'a str,
		stream_type: u8,
		ts: u64,
		message: &'a str,
	},
}

/// Spawn a thread to ship logs from a stream to LogShipper
pub fn ship_logs(
	actor_id: Uuid,
	msg_tx: Option<mpsc::SyncSender<ReceivedMessage>>,
	stream_type: StreamType,
	stream: impl BufRead + Send + 'static,
) -> JoinHandle<()> {
	std::thread::spawn(move || {
		// Reduces logging spikes. This logging is in place in order to ensure that a single
		// spike of logs does not exhaust the long rate limit.
		//
		// 64 logs/s
		let mut throttle_short = throttle::Throttle::new(960, Duration::from_secs(15));

		// Reduces logs from noisy games. Set reasonable caps on how
		// much can be logged per minute. This is here to prevent games
		// that log as fast as possible (i.e. positions of objects every
		// tick) from exhausting the system while still allowing sane
		// amounts of logging. This happens very frequently.
		//
		// 4 logs/s * 1024 bytes/log = 4096 bytes/lobby/s = 14.7 MB/lobby/hr = 353.8 MB/lobby/day  = 10.6 GB/lobby/month
		let mut throttle_long = throttle::Throttle::new(1200, Duration::from_secs(300));

		// Throttles error logs
		let mut throttle_error = throttle::Throttle::new(1, Duration::from_secs(60));

		// How many lines have been logged as a preview, see `MAX_PREVIEW_LINES`
		let mut preview_iine_count = 0;

		for line in stream.lines() {
			// Throttle
			if let Err(err) = throttle_short.tick() {
				if err.first_throttle_in_window
					&& send_message(
						actor_id,
						&msg_tx,
						Some(&mut throttle_error),
						stream_type,
						format_rate_limit(err.time_remaining),
					) {
					break;
				}
				continue;
			} else if let Err(err) = throttle_long.tick() {
				if err.first_throttle_in_window {
					if send_message(
						actor_id,
						&msg_tx,
						Some(&mut throttle_error),
						stream_type,
						format_rate_limit(err.time_remaining),
					) {
						break;
					}
				}
				continue;
			}

			// Read message
			let mut message = line.expect("failed to read line");

			// Truncate message to MAX_LINE_BYTES. This safely truncates to ensure we don't split a
			// string on a character boundary.
			if let Some((byte_idx, _)) = message
				.char_indices()
				.find(|&(byte_idx, _)| byte_idx > MAX_LINE_BYTES)
			{
				message.truncate(byte_idx);
				message.push_str(" (truncated)")
			}

			// Log preview of lines from the program for easy debugging from Pegboard
			if preview_iine_count < MAX_PREVIEW_LINES {
				preview_iine_count += 1;
				println!(
					"{actor_id}: {stream_type:?}: {message}",
					stream_type = stream_type,
					message = message,
				);

				if preview_iine_count == MAX_PREVIEW_LINES {
					println!(
						"{actor_id}: {stream_type:?}: ...not logging any more lines...",
						stream_type = stream_type,
					);
				}
			}

			if send_message(
				actor_id,
				&msg_tx,
				Some(&mut throttle_error),
				stream_type,
				message,
			) {
				break;
			}
		}

		println!("{actor_id}: Ship {stream_type:?} logs thread exiting");
	})
}

/// Sends a message to the log shipper
///
/// Returns true if receiver is disconnected
pub fn send_message(
	actor_id: Uuid,
	msg_tx: &Option<mpsc::SyncSender<ReceivedMessage>>,
	throttle_error: Option<&mut throttle::Throttle>,
	stream_type: StreamType,
	message: String,
) -> bool {
	let Some(msg_tx) = msg_tx.as_ref() else {
		return false;
	};

	// Timestamp is formatted in nanoseconds since that's the way it's formatted in
	// ClickHouse
	let ts = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("time went backwards")
		.as_nanos() as u64;

	// Attempt to send message. This will fail if the channel is full, relieving back
	// pressure if Vector is not running.
	match msg_tx.try_send(ReceivedMessage {
		stream_type,
		ts,
		message,
	}) {
		Result::Ok(_) => {}
		Err(mpsc::TrySendError::Full(_)) => {
			if throttle_error.map_or(true, |x| x.tick().is_ok()) {
				eprintln!("{actor_id}: log shipper buffer full, logs are being dropped");
			}
		}
		Err(mpsc::TrySendError::Disconnected(_)) => {
			eprintln!("{actor_id}: log shipper unexpectedly disconnected, exiting");
			return true;
		}
	}

	false
}

fn format_rate_limit(duration: Duration) -> String {
	format!("...logs rate limited for {} seconds, see rivet.gg/docs/dynamic-servers/concepts/logging...", duration.as_secs())
}
