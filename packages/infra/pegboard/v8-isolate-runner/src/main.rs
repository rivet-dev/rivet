use std::{
	collections::HashMap,
	os::fd::AsRawFd,
	path::{Path, PathBuf},
	time::Duration,
};

use anyhow::*;
use deno_runtime::deno_core::JsRuntime;
use futures_util::{stream::SplitStream, StreamExt};
use tokio::{
	fs,
	net::TcpStream,
	sync::{mpsc, watch},
};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use utils::var;
use uuid::Uuid;

mod config;
mod isolate;
mod log_shipper;
mod throttle;
mod utils;

/// Manager port to connect to.
const RUNNER_PORT: u16 = 54321;
const THREAD_STATUS_POLL: Duration = Duration::from_millis(500);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
	let working_path = std::env::args()
		.skip(1)
		.next()
		.context("`working_path` arg required")?;
	let working_path = Path::new(&working_path);

	redirect_logs(working_path.join("log")).await?;

	// Write PID to file
	fs::write(
		working_path.join("pid"),
		std::process::id().to_string().as_bytes(),
	)
	.await?;

	let actors_path = var("ACTORS_PATH")?;
	let actors_path = Path::new(&actors_path);

	// Explicitly start runtime on current thread
	JsRuntime::init_platform(None, false);

	let mut actors = HashMap::new();
	// TODO: Should be a `watch::channel` but the tokio version used by deno is old and doesn't implement
	// `Clone` for `watch::Sender`
	let (fatal_tx, mut fatal_rx) = tokio::sync::mpsc::channel::<()>(1);

	let res = tokio::select! {
		res = retry_connection(actors_path, &mut actors, fatal_tx) => res,
		// If any fatal error occurs in the isolate threads, kill the entire program
		_ = fatal_rx.recv() => Err(anyhow!("Fatal error")),
	};

	if res.is_err() {
		fs::write(working_path.join("exit-code"), 1.to_string().as_bytes()).await?;
	}

	res
}

async fn retry_connection(
	actors_path: &Path,
	actors: &mut HashMap<Uuid, watch::Sender<()>>,
	fatal_tx: mpsc::Sender<()>,
) -> Result<()> {
	loop {
		use std::result::Result::{Err, Ok};
		match tokio_tungstenite::connect_async(format!("ws://0.0.0.0:{RUNNER_PORT}")).await {
			Ok((socket, _)) => {
				handle_connection(actors_path, actors, fatal_tx.clone(), socket).await?
			}
			Err(err) => eprintln!("Failed to connect: {err}"),
		}

		eprintln!("Retrying connection");
		std::thread::sleep(Duration::from_secs(1));
	}
}

async fn handle_connection(
	actors_path: &Path,
	actors: &mut HashMap<Uuid, watch::Sender<()>>,
	fatal_tx: mpsc::Sender<()>,
	socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Result<()> {
	println!("Connected");

	let (_tx, mut rx) = socket.split();

	loop {
		let Some(packet) = read_packet(&mut rx).await? else {
			return Ok(());
		};

		match packet {
			runner_protocol::ToRunner::Start { actor_id } => {
				if actors.contains_key(&actor_id) {
					eprintln!("Actor {actor_id} already exists, ignoring new start packet");
				} else {
					// For signalling an isolate to stop
					let (stop_tx, stop_rx) = tokio::sync::watch::channel(());

					// Spawn a new thread for the isolate
					let actors_path = actors_path.to_path_buf();
					let handle = std::thread::Builder::new()
						.name(actor_id.to_string())
						.spawn(move || isolate::run(actors_path, actor_id, stop_rx))?;

					// This task polls the isolate thread we just spawned to see if it errored
					let fatal_tx = fatal_tx.clone();
					tokio::task::spawn(async move {
						loop {
							if handle.is_finished() {
								let res = handle.join();

								use std::result::Result::{Err, Ok};
								match res {
									Ok(Err(err)) => {
										eprintln!("Isolate thread failed ({actor_id}):\n{err:?}");
										fatal_tx.try_send(()).expect("receiver cannot be dropped")
									}
									Err(_) => {
										fatal_tx.try_send(()).expect("receiver cannot be dropped")
									}
									_ => {}
								}

								break;
							}

							tokio::time::sleep(THREAD_STATUS_POLL).await
						}
					});

					// Store actor stop sender
					actors.insert(actor_id, stop_tx);
				}
			}
			runner_protocol::ToRunner::Signal { actor_id, .. } => {
				if let Some(stop_tx) = actors.get(&actor_id) {
					// Tell actor thread to stop (cleanup is handled above)
					stop_tx.send(())?;
				} else {
					eprintln!("Actor {actor_id} not found for stopping");
				}
			}
		}
	}
}

async fn read_packet(
	socket: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Result<Option<runner_protocol::ToRunner>> {
	use std::result::Result::{Err, Ok};
	let buf = match socket.next().await {
		Some(Ok(Message::Binary(buf))) => buf,
		Some(Ok(Message::Close(_))) => {
			println!("Connection closed");
			return Ok(None);
		}
		Some(Ok(msg)) => bail!("unexpected message: {msg:?}"),
		Some(Err(err)) => {
			eprintln!("Connection failed: {err}");
			return Ok(None);
		}
		None => {
			println!("Stream closed");
			return Ok(None);
		}
	};

	let packet = serde_json::from_slice(&buf)?;

	Ok(Some(packet))
}

async fn redirect_logs(log_file_path: PathBuf) -> Result<()> {
	println!("Redirecting all logs to {}", log_file_path.display());
	let log_file = fs::OpenOptions::new()
		.write(true)
		.create(true)
		.append(true)
		.open(log_file_path)
		.await?;
	let log_fd = log_file.as_raw_fd();

	nix::unistd::dup2(log_fd, nix::libc::STDOUT_FILENO)?;
	nix::unistd::dup2(log_fd, nix::libc::STDERR_FILENO)?;

	Ok(())
}
