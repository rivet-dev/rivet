use std::{
	collections::HashMap,
	path::Path,
	result::Result::{Err, Ok},
	sync::Arc,
	thread::JoinHandle,
	time::Duration,
};

use anyhow::*;
use deno_core::{v8_set_flags, JsRuntime};
use deno_runtime::worker::MainWorkerTerminateHandle;
use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use pegboard_actor_kv::ActorKv;
use pegboard_config::{isolate_runner::Config, runner_protocol};
use tokio::{
	fs,
	net::TcpStream,
	sync::{mpsc, watch, RwLock},
};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use utils::FdbPool;
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
// 7 day logs retention
const LOGS_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

fn main() -> Result<()> {
	rivet_runtime::run(main_inner()).transpose()?;
	Ok(())
}

async fn main_inner() -> Result<()> {
	// Initialize with a default CryptoProvider for rustls
	let provider = rustls::crypto::ring::default_provider();
	provider
		.install_default()
		.expect("Failed to install crypto provider");

	let working_path = std::env::args()
		.skip(1)
		.next()
		.context("`working_path` arg required")?;
	let working_path = Path::new(&working_path);

	rivet_logs::Logs::new(working_path.join("logs"), LOGS_RETENTION)
		.start()
		.await?;

	let config_data = fs::read_to_string(working_path.join("config.json")).await?;
	let config = serde_json::from_str::<Config>(&config_data)?;

	let fdb_pool = utils::setup_fdb_pool(&config).await?;

	tracing::info!(pid=%std::process::id(), "starting");

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
		res = retry_connection(&config, &fdb_pool, actors, fatal_tx) => res,
		// If any fatal error occurs in the isolate threads, kill the entire program
		_ = fatal_rx.changed() => Err(anyhow!("Fatal error")),
	};

	// Write exit code
	if let Err(err) = &res {
		tracing::error!(?err);

		fs::write(working_path.join("exit-code"), 1.to_string().as_bytes()).await?;
	}

	res
}

async fn retry_connection(
	config: &Config,
	fdb_pool: &FdbPool,
	actors: Arc<RwLock<HashMap<(Uuid, u32), mpsc::Sender<(i32, bool)>>>>,
	fatal_tx: watch::Sender<()>,
) -> Result<()> {
	loop {
		use std::result::Result::{Err, Ok};
		match tokio_tungstenite::connect_async(format!("ws://{}", config.manager_ws_addr)).await {
			Ok((socket, _)) => {
				handle_connection(config, fdb_pool, actors.clone(), fatal_tx.clone(), socket)
					.await?
			}
			Err(err) => tracing::error!("Failed to connect: {err}"),
		}

		tracing::info!("Retrying connection");
		std::thread::sleep(Duration::from_secs(1));
	}
}

async fn handle_connection(
	config: &Config,
	fdb_pool: &FdbPool,
	actors: Arc<RwLock<HashMap<(Uuid, u32), mpsc::Sender<(i32, bool)>>>>,
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
			runner_protocol::ToRunner::Start {
				actor_id,
				generation,
			} => {
				let mut guard = actors.write().await;

				if guard.contains_key(&(actor_id, generation)) {
					tracing::error!(
						"Actor {actor_id}-{generation} already exists, ignoring new start packet"
					);
				} else {
					// For receiving the terminate handle from the isolate thread
					let (terminate_tx, terminate_rx) =
						mpsc::channel::<MainWorkerTerminateHandle>(1);
					let (signal_tx, signal_rx) = mpsc::channel(1);

					// Store actor signal sender
					guard.insert((actor_id, generation), signal_tx);
					drop(guard);

					// Spawn a new thread for the isolate
					let config2 = config.clone();
					let fdb_pool2 = fdb_pool.clone();
					let handle = std::thread::Builder::new()
						.name(format!("{actor_id}-{generation}"))
						.spawn(move || {
							isolate::run(config2, fdb_pool2, actor_id, generation, terminate_tx)
						})?;

					tokio::task::spawn(watch_thread(
						fdb_pool.clone(),
						actors.clone(),
						fatal_tx.clone(),
						actor_id,
						generation,
						terminate_rx,
						signal_rx,
						handle,
					));
				}
			}
			runner_protocol::ToRunner::Signal {
				actor_id,
				generation,
				signal,
				persist_storage,
			} => {
				if let Some(signal_tx) = actors.read().await.get(&(actor_id, generation)) {
					// Tell actor thread to stop. Removing the actor is handled in the tokio task above.
					signal_tx
						.try_send((signal, persist_storage))
						.context("failed to send stop signal to actor thread watcher")?;
				} else {
					tracing::warn!("Actor {actor_id}-{generation} not found for stopping");
				}
			}
			runner_protocol::ToRunner::Terminate => bail!("Received terminate"),
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
	fdb_pool: FdbPool,
	actors: Arc<RwLock<HashMap<(Uuid, u32), mpsc::Sender<(i32, bool)>>>>,
	fatal_tx: watch::Sender<()>,
	actor_id: Uuid,
	generation: u32,
	mut terminate_rx: mpsc::Receiver<MainWorkerTerminateHandle>,
	mut signal_rx: mpsc::Receiver<(i32, bool)>,
	handle: JoinHandle<Result<()>>,
) {
	// Await terminate handle. If the transmitting end of the terminate handle was dropped (`recv` returned
	// `None`), either the worker failed to create or the thread stopped. The latter is handled later
	let terminate_handle = terminate_rx.recv().await;
	drop(terminate_rx);

	// Wait for either the thread to stop or a signal to be received
	let persist_storage = tokio::select! {
		biased;
		_ = poll_thread(&handle) => true,
		res = signal_rx.recv() => {
			let Some((_signal, persist_storage)) = res else {
				tracing::error!(?actor_id, ?generation, "failed to receive signal");
				fatal_tx.send(()).expect("receiver cannot be dropped");
				return;
			};

			if let Some(terminate_handle) = terminate_handle {
				// Currently, we terminate regardless of what the signal is
				terminate_handle.terminate();
			}

			persist_storage
		}
	};

	// Remove actor
	{
		actors.write().await.remove(&(actor_id, generation));
	}

	// Remove state
	if !persist_storage {
		if let Err(err) = ActorKv::new((&*fdb_pool).clone(), actor_id).destroy().await {
			tracing::error!(?err, ?actor_id, "failed to destroy actor kv");
			fatal_tx.send(()).expect("receiver cannot be dropped");
			return;
		};
	}

	// Cleanup thread
	poll_thread(&handle).await;
	cleanup_thread(actor_id, generation, handle, &fatal_tx);
}

async fn poll_thread(handle: &JoinHandle<Result<()>>) {
	loop {
		if handle.is_finished() {
			return;
		}

		tokio::time::sleep(THREAD_STATUS_POLL_INTERVAL).await;
	}
}

fn cleanup_thread(
	actor_id: Uuid,
	generation: u32,
	handle: JoinHandle<Result<()>>,
	fatal_tx: &watch::Sender<()>,
) {
	let res = handle.join();

	match res {
		Ok(Err(err)) => {
			tracing::error!(?actor_id, ?generation, "Isolate thread failed:\n{err:?}");
			fatal_tx.send(()).expect("receiver cannot be dropped")
		}
		Err(_) => fatal_tx.send(()).expect("receiver cannot be dropped"),
		_ => {}
	}
}
