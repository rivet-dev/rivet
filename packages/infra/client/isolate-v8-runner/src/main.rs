use std::{
	collections::HashMap,
	os::fd::AsRawFd,
	path::{Path, PathBuf},
	result::Result::{Err, Ok},
	sync::Arc,
	thread::JoinHandle,
	time::Duration,
};

use anyhow::*;
use deno_runtime::deno_core::{v8_set_flags, JsRuntime};
use deno_runtime::worker::MainWorkerTerminateHandle;
use foundationdb as fdb;
use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use tokio::{
	fs,
	net::TcpStream,
	sync::{mpsc, RwLock},
};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tracing_subscriber::prelude::*;
use utils::var;
use uuid::Uuid;

mod config;
mod ext;
mod isolate;
mod log_shipper;
mod throttle;
mod utils;

enum Packet {
	Msg(runner_protocol::ToRunner),
	Pong,
	None,
}

/// Manager port to connect to.
const THREAD_STATUS_POLL_INTERVAL: Duration = Duration::from_millis(500);
const PING_INTERVAL: Duration = Duration::from_secs(1);

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();

	let working_path = std::env::args()
		.skip(1)
		.next()
		.context("`working_path` arg required")?;
	let working_path = Path::new(&working_path);

	redirect_logs(working_path.join("log")).await?;

	// Start FDB network thread
	let _network = unsafe { fdb::boot() };
	tokio::spawn(utils::fdb_health_check());

	// Write PID to file
	fs::write(
		working_path.join("pid"),
		std::process::id().to_string().as_bytes(),
	)
	.await?;

	let actors_path = var("ACTORS_PATH")?;
	let runner_addr = var("RUNNER_ADDR")?;
	let actors_path = Path::new(&actors_path);

	// Set v8 flags (https://chromium.googlesource.com/v8/v8/+/refs/heads/main/src/flags/flag-definitions.h)
	let invalid = v8_set_flags(vec![
		// Binary name
		"UNUSED_BUT_NECESSARY_ARG0".into(),
		// Disable eval
		"--disallow-code-generation-from-strings".into(),
	]);
	assert!(
		invalid.len() == 1,
		"v8 did not understand these flags: {:?}",
		invalid.into_iter().skip(1).collect::<Vec<_>>(),
	);

	// Explicitly start runtime on current thread
	JsRuntime::init_platform(None, false);

	let actors = Arc::new(RwLock::new(HashMap::new()));
	// TODO: Should be a `watch::channel` but the tokio version used by deno is old and doesn't implement
	// `Clone` for `watch::Sender`
	let (fatal_tx, mut fatal_rx) = tokio::sync::mpsc::channel::<()>(1);

	let res = tokio::select! {
		res = retry_connection(actors_path, actors, fatal_tx) => res,
		// If any fatal error occurs in the isolate threads, kill the entire program
		_ = fatal_rx.recv() => Err(anyhow!("Fatal error")),
	};

	// Write exit code
	if res.is_err() {
		fs::write(working_path.join("exit-code"), 1.to_string().as_bytes()).await?;
	}

	res
}

async fn retry_connection(
	actors_path: &Path,
	actors: Arc<RwLock<HashMap<Uuid, mpsc::Sender<i32>>>>,
	fatal_tx: mpsc::Sender<()>,
) -> Result<()> {
	loop {
		use std::result::Result::{Err, Ok};
		match tokio_tungstenite::connect_async(format!("ws://{}", config.runner_addr)).await {
			Ok((socket, _)) => {
				handle_connection(actors_path, actors.clone(), fatal_tx.clone(), socket).await?
			}
			Err(err) => tracing::error!("Failed to connect: {err}"),
		}

		tracing::info!("Retrying connection");
		std::thread::sleep(Duration::from_secs(1));
	}
}

async fn handle_connection(
	actors_path: &Path,
	actors: Arc<RwLock<HashMap<Uuid, mpsc::Sender<i32>>>>,
	fatal_tx: mpsc::Sender<()>,
	socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Result<()> {
	tracing::info!("Connected");

	let (mut tx, mut rx) = socket.split();

	// NOTE: Currently, the error from the ping thread is not caught but we assume error handling elsewhere
	// will catch any connection issues.
	// Start ping thread
	let _: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
		loop {
			tokio::time::sleep(PING_INTERVAL).await;
			tx.send(Message::Ping(Vec::new())).await?;
		}
	});

	loop {
		let packet = match read_packet(&mut rx).await? {
			Packet::Msg(packet) => packet,
			Packet::Pong => continue,
			Packet::None => return Ok(()),
		};

		match packet {
			runner_protocol::ToRunner::Start { actor_id } => {
				let mut guard = actors.write().await;

				if guard.contains_key(&actor_id) {
					tracing::warn!("Actor {actor_id} already exists, ignoring new start packet");
				} else {
					// For receiving the terminate handle
					let (terminate_tx, mut terminate_rx) =
						tokio::sync::mpsc::channel::<MainWorkerTerminateHandle>(1);
					let (signal_tx, mut signal_rx) = tokio::sync::mpsc::channel(1);

					// Store actor stop sender
					guard.insert(actor_id, signal_tx);
					drop(guard);

					// Spawn a new thread for the isolate
					let actors_path = actors_path.to_path_buf();
					let handle = std::thread::Builder::new()
						.name(actor_id.to_string())
						.spawn(move || isolate::run(actors_path, actor_id, terminate_tx))?;

					// Alerts the main thread if any child threads had a fatal error
					let fatal_tx = fatal_tx.clone();

					// This task polls the isolate thread we just spawned to see if it errored. Should handle
					// all errors gracefully.
					let actors = actors.clone();
					tokio::task::spawn(async move {
						let Some(terminate_handle) = terminate_rx.recv().await else {
							// If the transmitting end of the terminate handle was dropped (`recv` returned
							// `None`), it must be that the thread stopped
							tracing::error!("failed to receive terminate handle");
							cleanup_thread(actor_id, handle, fatal_tx);
							return;
						};

						drop(terminate_rx);

						tokio::select! {
							biased;
							_ = poll_thread(&handle) => cleanup_thread(actor_id, handle, fatal_tx),
							res = signal_rx.recv() => {
								let Some(_signal) = res else {
									tracing::error!("failed to receive signal");
									fatal_tx.try_send(()).expect("receiver cannot be dropped");
									return;
								};

								terminate_handle.terminate();
							}
						}

						// Remove actor
						actors.write().await.remove(&actor_id);
					});
				}
			}
			runner_protocol::ToRunner::Signal { actor_id, signal } => {
				if let Some(signal_tx) = actors.read().await.get(&actor_id) {
					// Tell actor thread to stop. Removing the actor is handled in the tokio task above.
					signal_tx.try_send(signal).context("failed to send stop signal to actor poll task")?;
				} else {
					tracing::warn!("Actor {actor_id} not found for stopping");
				}
			}
		}
	}
}

async fn read_packet(
	socket: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Result<Packet> {
	let buf = match socket.next().await {
		Some(Ok(Message::Binary(buf))) => buf,
		Some(Ok(Message::Close(_))) => {
			tracing::error!("Connection closed");
			return Ok(Packet::None);
		}
		Some(Ok(Message::Pong(_))) => {
			tracing::debug!("received pong");
			return Ok(Packet::Pong);
		}
		Some(Ok(msg)) => bail!("unexpected message: {msg:?}"),
		Some(Err(err)) => {
			tracing::error!("Connection failed: {err}");
			return Ok(Packet::None);
		}
		None => {
			tracing::error!("Stream closed");
			return Ok(Packet::None);
		}
	};

	let packet = serde_json::from_slice(&buf)?;

	Ok(Packet::Msg(packet))
}

async fn poll_thread(handle: &JoinHandle<Result<()>>) {
	loop {
		if handle.is_finished() {
			return;
		}

		tokio::time::sleep(THREAD_STATUS_POLL_INTERVAL).await;
	}
}

fn cleanup_thread(actor_id: Uuid, handle: JoinHandle<Result<()>>, fatal_tx: mpsc::Sender<()>) {
	let res = handle.join();

	match res {
		Ok(Err(err)) => {
			tracing::error!(?actor_id, "Isolate thread failed:\n{err:?}");
			fatal_tx.try_send(()).expect("receiver cannot be dropped")
		}
		Err(_) => fatal_tx.try_send(()).expect("receiver cannot be dropped"),
		_ => {}
	}
}

async fn redirect_logs(log_file_path: PathBuf) -> Result<()> {
	tracing::info!("Redirecting all logs to {}", log_file_path.display());
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

fn init_tracing() {
	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();
}
