use std::{
	collections::HashMap,
	os::fd::AsRawFd,
	path::{Path, PathBuf},
	result::Result::{Err, Ok},
	sync::Arc,
	thread::JoinHandle,
	time::Duration,
};

use actor_kv::ActorKv;
use anyhow::*;
use deno_core::{v8_set_flags, JsRuntime};
use deno_runtime::worker::MainWorkerTerminateHandle;
use foundationdb as fdb;
use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use pegboard::protocol;
use pegboard_config::{isolate_runner::Config, runner_protocol};
use tokio::{
	fs,
	net::TcpStream,
	sync::{mpsc, watch, RwLock},
};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tracing_subscriber::prelude::*;
use uuid::Uuid;

mod ext;
mod isolate;
mod log_shipper;
mod metadata;
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

	let config_data = fs::read_to_string(working_path.join("config.json")).await?;
	let config = serde_json::from_str::<Config>(&config_data)?;

	// Start FDB network thread
	let _network = unsafe { fdb::boot() };
	tokio::spawn(utils::fdb_health_check(config.clone()));

	// Write PID to file
	fs::write(
		working_path.join("pid"),
		std::process::id().to_string().as_bytes(),
	)
	.await?;

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
	let (fatal_tx, mut fatal_rx) = watch::channel(());

	let res = tokio::select! {
		res = retry_connection(&config, actors, fatal_tx) => res,
		// If any fatal error occurs in the isolate threads, kill the entire program
		_ = fatal_rx.changed() => Err(anyhow!("Fatal error")),
	};

	// Write exit code
	if res.is_err() {
		fs::write(working_path.join("exit-code"), 1.to_string().as_bytes()).await?;
	}

	res
}

async fn retry_connection(
	config: &Config,
	actors: Arc<RwLock<HashMap<Uuid, mpsc::Sender<(i32, bool)>>>>,
	fatal_tx: watch::Sender<()>,
) -> Result<()> {
	loop {
		use std::result::Result::{Err, Ok};
		match tokio_tungstenite::connect_async(format!("ws://{}", config.runner_addr)).await {
			Ok((socket, _)) => {
				handle_connection(config, actors.clone(), fatal_tx.clone(), socket).await?
			}
			Err(err) => tracing::error!("Failed to connect: {err}"),
		}

		tracing::info!("Retrying connection");
		std::thread::sleep(Duration::from_secs(1));
	}
}

async fn handle_connection(
	config: &Config,
	actors: Arc<RwLock<HashMap<Uuid, mpsc::Sender<(i32, bool)>>>>,
	fatal_tx: watch::Sender<()>,
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
					// For receiving the owner from the isolate thread
					let (owner_tx, owner_rx) = mpsc::channel::<protocol::ActorOwner>(1);
					// For receiving the terminate handle from the isolate thread
					let (terminate_tx, terminate_rx) =
						mpsc::channel::<MainWorkerTerminateHandle>(1);
					let (signal_tx, signal_rx) = mpsc::channel(1);

					// Store actor signal sender
					guard.insert(actor_id, signal_tx);
					drop(guard);

					// Spawn a new thread for the isolate
					let config2 = config.clone();
					let handle = std::thread::Builder::new()
						.name(actor_id.to_string())
						.spawn(move || isolate::run(config2, actor_id, owner_tx, terminate_tx))?;

					tokio::task::spawn(watch_thread(
						config.clone(),
						actors.clone(),
						fatal_tx.clone(),
						actor_id,
						owner_rx,
						terminate_rx,
						signal_rx,
						handle,
					));
				}
			}
			runner_protocol::ToRunner::Signal {
				actor_id,
				signal,
				persist_state,
			} => {
				if let Some(signal_tx) = actors.read().await.get(&actor_id) {
					// Tell actor thread to stop. Removing the actor is handled in the tokio task above.
					signal_tx
						.try_send((signal, persist_state))
						.context("failed to send stop signal to actor thread watcher")?;
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

/// Polls the isolate thread we just spawned to see if it errored. Should handle all errors gracefully.
async fn watch_thread(
	config: Config,
	actors: Arc<RwLock<HashMap<Uuid, mpsc::Sender<(i32, bool)>>>>,
	fatal_tx: watch::Sender<()>,
	actor_id: Uuid,
	mut owner_rx: mpsc::Receiver<protocol::ActorOwner>,
	mut terminate_rx: mpsc::Receiver<MainWorkerTerminateHandle>,
	mut signal_rx: mpsc::Receiver<(i32, bool)>,
	handle: JoinHandle<Result<()>>,
) {
	// Await actor owner
	let Some(actor_owner) = owner_rx.recv().await else {
		// If the transmitting end of the terminate handle was dropped (`recv` returned `None`), it must be
		// that the thread stopped
		tracing::error!(?actor_id, "failed to receive actor owner");
		fatal_tx.send(()).expect("receiver cannot be dropped");
		return;
	};

	drop(owner_rx);

	// Await terminate handle. If the transmitting end of the terminate handle was dropped (`recv` returned
	// `None`), either the worker failed to create or the thread stopped. The latter is handled later
	let terminate_handle = terminate_rx.recv().await;
	drop(terminate_rx);

	// Wait for either the thread to stop or a signal to be received
	let persist_state = tokio::select! {
		biased;
		_ = poll_thread(&handle) => true,
		res = signal_rx.recv() => {
			let Some((_signal, persist_state)) = res else {
				tracing::error!(?actor_id, "failed to receive signal");
				fatal_tx.send(()).expect("receiver cannot be dropped");
				return;
			};

			if let Some(terminate_handle) = terminate_handle {
				// Currently, we terminate regardless of what the signal is
				terminate_handle.terminate();
			}

			persist_state
		}
	};

	// Remove actor
	{
		actors.write().await.remove(&actor_id);
	}

	// Remove state
	if !persist_state {
		let db = match utils::fdb_handle(&config) {
			Ok(db) => db,
			Err(err) => {
				tracing::error!(?err, ?actor_id, "failed to create fdb handle");
				fatal_tx.send(()).expect("receiver cannot be dropped");
				return;
			}
		};

		if let Err(err) = ActorKv::new(db, actor_owner).destroy().await {
			tracing::error!(?err, ?actor_id, "failed to destroy actor kv");
			fatal_tx.send(()).expect("receiver cannot be dropped");
			return;
		};
	}

	// Cleanup thread
	poll_thread(&handle).await;
	cleanup_thread(actor_id, handle, &fatal_tx);
}

async fn poll_thread(handle: &JoinHandle<Result<()>>) {
	loop {
		if handle.is_finished() {
			return;
		}

		tokio::time::sleep(THREAD_STATUS_POLL_INTERVAL).await;
	}
}

fn cleanup_thread(actor_id: Uuid, handle: JoinHandle<Result<()>>, fatal_tx: &watch::Sender<()>) {
	let res = handle.join();

	match res {
		Ok(Err(err)) => {
			tracing::error!(?actor_id, "Isolate thread failed:\n{err:?}");
			fatal_tx.send(()).expect("receiver cannot be dropped")
		}
		Err(_) => fatal_tx.send(()).expect("receiver cannot be dropped"),
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
