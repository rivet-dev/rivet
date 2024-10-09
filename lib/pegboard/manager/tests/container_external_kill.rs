// NOTE: Requires installing skopeo and umoci on the machine running this test

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use nix::{
	sys::signal::{kill, Signal},
	unistd::Pid,
};
use pegboard::protocol;
use pegboard_manager::{utils, Ctx};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::tungstenite::protocol::Message;
use uuid::Uuid;

mod common;
use common::*;

/// Verifies client picks up container killed by external signal.
#[tokio::test(flavor = "multi_thread")]
async fn container_external_kill() {
	setup_tracing();

	tracing::info!("starting test");

	let _temp_dir = setup_dependencies().await;

	let ctx_wrapper: Arc<Mutex<Option<Arc<Ctx>>>> = Arc::new(Mutex::new(None));
	let (close_tx, close_rx) = tokio::sync::watch::channel(());

	let port = portpicker::pick_unused_port().expect("no free ports");
	start_server(ctx_wrapper.clone(), close_tx, port, handle_connection);

	// Init project directories
	let tmp_dir = tempfile::TempDir::new().unwrap();
	utils::init(tmp_dir.path()).await.unwrap();
	tracing::info!(path=%tmp_dir.path().display(), "client dir");

	start_client(tmp_dir.path(), ctx_wrapper, close_rx, port).await;
}

async fn handle_connection(
	ctx_wrapper: Arc<Mutex<Option<Arc<Ctx>>>>,
	close_tx: tokio::sync::watch::Sender<()>,
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

		let container_id = Uuid::new_v4();
		let container_port = portpicker::pick_unused_port().expect("no free ports");

		// Receive messages from socket
		while let Some(msg) = rx.next().await {
			match msg.unwrap() {
				Message::Binary(buf) => {
					let protocol_version = 1;
					let packet = protocol::ToServer::deserialize(protocol_version, &buf).unwrap();

					match packet {
						protocol::ToServer::Init { .. } => {
							send_init_packet(&mut tx).await;

							start_echo_container(&mut tx, container_id, container_port).await;
						}
						protocol::ToServer::Events(events) => {
							for event in events {
								tracing::info!(?event, "received event");

								let protocol::Event::ContainerStateUpdate { state, .. } =
									event.inner.deserialize().unwrap();

								match state {
									protocol::ContainerState::Running { pid, .. } => {
										// Kill container
										tracing::info!("killing container");
										kill(Pid::from_raw(pid as i32), Signal::SIGKILL).unwrap();
									}
									protocol::ContainerState::Exited { .. } => {
										tokio::time::sleep(Duration::from_millis(5)).await;

										// Verify client state
										let containers = ctx.containers().read().await;
										assert!(
											!containers.contains_key(&container_id),
											"container still in client memory"
										);

										// Test complete
										close_tx.send(()).unwrap();
									}
									protocol::ContainerState::Starting
									| protocol::ContainerState::Stopped => {}
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
