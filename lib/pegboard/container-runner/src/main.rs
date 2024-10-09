use std::{
	fs,
	io::{BufRead, BufReader},
	path::Path,
	process::{Command, Stdio},
	sync::mpsc,
	thread,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::*;
use signal_hook::{consts::signal::SIGTERM, iterator::Signals};
use utils::{var, Stakeholder};

pub mod log_shipper;
pub mod throttle;
pub mod utils;

/// Maximum length of a single log line
const MAX_LINE_BYTES: usize = 1024;

/// Maximum number of bytes to buffer before dropping logs
const MAX_BUFFER_BYTES: usize = 1024 * 1024;

/// Maximum number of lines to print to stdout for debugging. This helps
/// identify the reasons for program crashes from the container's output.
const MAX_PREVIEW_LINES: usize = 128;

fn main() -> anyhow::Result<()> {
	let pegboard_container_dir = std::env::args()
		.skip(1)
		.next()
		.context("`container_dir` arg required")?;
	let pegboard_container_dir = Path::new(&pegboard_container_dir);

	// Write PID to file
	fs::write(
		pegboard_container_dir.join("pid"),
		std::process::id().to_string().as_bytes(),
	)?;

	let root_user_enabled = var("PEGBOARD_META_root_user_enabled")? == "1";
	let stakeholder = match var("PEGBOARD_META_stakeholder").ok() {
		Some(x) if x == "dynamic_server" => Stakeholder::DynamicServer {
			server_id: var("PEGBOARD_META_server_id")?,
		},
		Some(x) => bail!("invalid container stakeholder: {x}"),
		None => bail!("no container stakeholder specified"),
	};

	let (shutdown_tx, shutdown_rx) = mpsc::sync_channel(1);

	// Start log shipper
	let (msg_tx, msg_rx) =
		mpsc::sync_channel::<log_shipper::ReceivedMessage>(MAX_BUFFER_BYTES / MAX_LINE_BYTES);
	let log_shipper = log_shipper::LogShipper {
		shutdown_rx,
		msg_rx,
		stakeholder,
	};
	let log_shipper_thread = log_shipper.spawn();

	// Run the container
	let exit_code = match run_container(msg_tx.clone(), &pegboard_container_dir, root_user_enabled)
	{
		Result::Ok(exit_code) => exit_code,
		Err(err) => {
			eprintln!("run container failed: {err:?}");
			send_message(
				&msg_tx,
				None,
				log_shipper::StreamType::StdErr,
				format!("Aborting"),
			);

			1
		}
	};

	// Shutdown all threads
	match shutdown_tx.send(()) {
		Result::Ok(_) => {
			println!("Sent shutdown signal");
		}
		Err(err) => {
			eprintln!("Failed to send shutdown signal: {err:?}");
		}
	}

	// Wait for log shipper to finish
	drop(msg_tx);
	match log_shipper_thread.join() {
		Result::Ok(_) => {}
		Err(err) => {
			eprintln!("log shipper failed: {err:?}")
		}
	}

	fs::write(
		pegboard_container_dir.join("exit-code"),
		exit_code.to_string().as_bytes(),
	)?;

	std::process::exit(exit_code)
}

/// Sets up & runs the container using runc.
///
/// Returns the exit code of the container that will be passed to the parent
fn run_container(
	msg_tx: mpsc::SyncSender<log_shipper::ReceivedMessage>,
	pegboard_container_dir: &Path,
	root_user_enabled: bool,
) -> anyhow::Result<i32> {
	// Extract container id from dir
	let container_id = pegboard_container_dir
		.iter()
		.last()
		.context("empty `pegboard_container_dir`")?
		.to_string_lossy()
		.to_string();
	let oci_bundle_path = pegboard_container_dir.join("oci-bundle");
	let oci_bundle_config_json = oci_bundle_path.join("config.json");

	// Validate OCI bundle
	let oci_bundle_str =
		fs::read_to_string(&oci_bundle_config_json).context("failed to read OCI bundle")?;
	let oci_bundle = serde_json::from_str::<serde_json::Value>(&oci_bundle_str)
		.context("failed to parse OCI bundle")?;
	let (Some(uid), Some(gid)) = (
		oci_bundle["process"]["user"]["uid"].as_i64(),
		oci_bundle["process"]["user"]["gid"].as_i64(),
	) else {
		bail!("missing uid or gid in OCI bundle");
	};
	if !root_user_enabled && (uid == 0 || gid == 0) {
		send_message(
			&msg_tx,
			None,
			log_shipper::StreamType::StdErr,
			format!("Server is attempting to run as root user or group (uid: {uid}, gid: {gid})"),
		);
		send_message(
			&msg_tx,
			None,
			log_shipper::StreamType::StdErr,
			format!("See https://rivet.gg/docs/dynamic-servers/concepts/docker-root-user"),
		);
		bail!("root user or group detected");
	}

	// Spawn runc container
	println!(
		"Starting container {} with OCI bundle {}",
		container_id,
		oci_bundle_path.display()
	);
	let mut runc_child = Command::new("runc")
		.arg("run")
		.arg(&container_id)
		.arg("-b")
		.arg(oci_bundle_path)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("failed to spawn runc");
	let runc_stdout = BufReader::new(runc_child.stdout.take().unwrap());
	let runc_stderr = BufReader::new(runc_child.stderr.take().unwrap());

	// Catch SIGTERM and forward to child
	//
	// This will wait for the child to exit and then exit itself so we have time to ship all of the
	// required logs
	let mut signals = Signals::new(&[SIGTERM])?;
	thread::spawn(move || {
		for _ in signals.forever() {
			println!("Received SIGTERM, forwarding to runc container {container_id}");
			let status = Command::new("runc")
				.arg("kill")
				.arg("--all")
				.arg(&container_id)
				.arg("SIGTERM")
				.status();
			println!("runc kill status: {:?}", status);
			break;
		}
	});

	// Ship stdout & stderr logs
	let stdout_handle = ship_logs(msg_tx.clone(), log_shipper::StreamType::StdOut, runc_stdout);
	let stderr_handle = ship_logs(msg_tx.clone(), log_shipper::StreamType::StdErr, runc_stderr);

	// Wait for threads to finish
	match stdout_handle.join() {
		Result::Ok(_) => {}
		Err(err) => {
			eprintln!("stdout thread panicked: {err:?}")
		}
	}
	match stderr_handle.join() {
		Result::Ok(_) => {}
		Err(err) => {
			eprintln!("stderr thread panicked: {err:?}")
		}
	}

	// Wait for runc to exit
	let runc_exit_code = match runc_child.wait() {
		Result::Ok(exit_code) => {
			if let Some(exit_code) = exit_code.code() {
				println!("runc exited with code {}", exit_code);
				exit_code
			} else {
				eprintln!("Unable to read exit code");
				1
			}
		}
		Err(err) => {
			eprintln!("Failed to wait for runc: {err:?}");
			1
		}
	};

	Ok(runc_exit_code)
}

/// Spawn a thread to ship logs from a stream to log_shipper::LogShipper
fn ship_logs(
	msg_tx: mpsc::SyncSender<log_shipper::ReceivedMessage>,
	stream_type: log_shipper::StreamType,
	stream: impl BufRead + Send + 'static,
) -> thread::JoinHandle<()> {
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
					"{stream_type:?}: {message}",
					stream_type = stream_type,
					message = message
				);

				if preview_iine_count == MAX_PREVIEW_LINES {
					println!(
						"{stream_type:?}: ...not logging any more lines...",
						stream_type = stream_type
					);
				}
			}

			if send_message(&msg_tx, Some(&mut throttle_error), stream_type, message) {
				break;
			}
		}

		println!("Ship {stream_type:?} logs thread exiting");
	})
}

/// Sends a message to the log shipper
///
/// Returns true if receiver is disconnected
fn send_message(
	msg_tx: &mpsc::SyncSender<log_shipper::ReceivedMessage>,
	throttle_error: Option<&mut throttle::Throttle>,
	stream_type: log_shipper::StreamType,
	message: String,
) -> bool {
	// Timestamp is formatted in nanoseconds since that's the way it's formatted in
	// ClickHouse
	let ts = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("time went backwards")
		.as_nanos() as u64;

	// Attempt to send message. This will fail if the channel is full, relieving back
	// pressure if Vector is not running.
	match msg_tx.try_send(log_shipper::ReceivedMessage {
		stream_type,
		ts,
		message,
	}) {
		Result::Ok(_) => {}
		Err(mpsc::TrySendError::Full(_)) => {
			if throttle_error.map_or(true, |x| x.tick().is_ok()) {
				eprintln!("log shipper buffer full, logs are being dropped");
			}
		}
		Err(mpsc::TrySendError::Disconnected(_)) => {
			eprintln!("log shipper unexpectedly disconnected, exiting");
			return true;
		}
	}

	false
}

fn format_rate_limit(duration: Duration) -> String {
	format!("...logs rate limited for {} seconds, see rivet.gg/docs/dynamic-servers/concepts/logging...", duration.as_secs())
}
