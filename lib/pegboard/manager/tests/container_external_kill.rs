// NOTE: Requires installing skopeo and umoci on the machine running this test

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use nix::{
	sys::signal::{kill, Signal},
	unistd::Pid,
};
use pegboard::protocol;
use pegboard_manager::Ctx;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::tungstenite::protocol::Message;
use uuid::Uuid;

mod common;
use common::*;

/// Verifies client picks up actor killed by external signal.
#[tokio::test(flavor = "multi_thread")]
async fn container_external_kill() {
	setup_tracing();

	tracing::info!("starting test");

	let (_gen_tmp_dir, gen_tmp_dir_path) = setup_dependencies().await;

	let ctx_wrapper: Arc<Mutex<Option<Arc<Ctx>>>> = Arc::new(Mutex::new(None));
	let (close_tx, close_rx) = tokio::sync::watch::channel(());
	let close_tx = Arc::new(close_tx);

	let port = portpicker::pick_unused_port().expect("no free ports");
	start_server(ctx_wrapper.clone(), close_tx, port, handle_connection);

	// Init project directories
	let tmp_dir = tempfile::TempDir::new().unwrap();
	let config = init_client(&gen_tmp_dir_path, tmp_dir.path()).await;
	tracing::info!(path=%tmp_dir.path().display(), "client dir");

	start_client(config, ctx_wrapper, close_rx, port).await;
}

async fn handle_connection(
	ctx_wrapper: Arc<Mutex<Option<Arc<Ctx>>>>,
	close_tx: Arc<tokio::sync::watch::Sender<()>>,
	raw_stream: TcpStream,
) {
	tokio::spawn(async move {
		let ws_stream = tokio_tungstenite::accept_async(raw_stream).await.unwrap();
		let (mut tx, mut rx) = ws_stream.split();

		tokio::time::sleep(std::time::Duration::from_millis(16)).await;

		// Read ctx from wrapper
		let ctx = {
			let guard = ctx_wrapper.lock().await;
			guard.clone().unwrap()
		};

		let actor_id = Uuid::new_v4();
		let actor_port = portpicker::pick_unused_port().expect("no free ports");

		// Receive messages from socket
		while let Some(msg) = rx.next().await {
			match msg.unwrap() {
				Message::Binary(buf) => {
					let protocol_version = 1;
					let packet = protocol::ToServer::deserialize(protocol_version, &buf).unwrap();

					match packet {
						protocol::ToServer::Init { .. } => {
							send_init_packet(&mut tx).await;

							start_echo_actor(&mut tx, actor_id, actor_port).await;
						}
						protocol::ToServer::Events(events) => {
							for event in events {
								tracing::info!(?event, "received event");

								let protocol::Event::ActorStateUpdate { state, .. } =
									event.inner.deserialize().unwrap();

								match state {
									protocol::ActorState::Running { pid, .. } => {
										// Kill actor
										tracing::info!("killing actor");
										kill(Pid::from_raw(pid as i32), Signal::SIGKILL).unwrap();
									}
									protocol::ActorState::Exited { .. } => {
										tokio::time::sleep(Duration::from_millis(5)).await;

										// Verify client state
										let actors = ctx.actors().read().await;
										assert!(
											!actors.contains_key(&actor_id),
											"actor still in client memory"
										);

										// Test complete
										close_tx.send(()).unwrap();
									}
									protocol::ActorState::Starting
									| protocol::ActorState::Stopped => {}
									state => panic!("unexpected state received: {state:?}"),
								}
							}
						}
						_ => {}
					}
				}
				Message::Close(_) => {
					panic!("socket closed");
				}
				_ => {}
			}
		}

		tracing::info!("client disconnected");
	});
}
