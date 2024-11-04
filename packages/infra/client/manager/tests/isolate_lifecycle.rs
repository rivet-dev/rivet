// NOTE: Requires installing skopeo and umoci on the machine running this test

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use nix::sys::signal::Signal;
use pegboard::protocol;
use pegboard_manager::Ctx;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::tungstenite::protocol::Message;
use uuid::Uuid;

mod common;
use common::*;

#[derive(Debug)]
enum State {
	None,
	Starting,
	Running,
	Stopped,
	Exited,
}

/// Goes through all actor states and commands.
#[tokio::test(flavor = "multi_thread")]
async fn isolate_lifecycle() {
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

	start_client(config, ctx_wrapper, close_rx.clone(), port).await;
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
		let mut actor_state = State::None;

		// Receive messages from socket
		while let Some(msg) = rx.next().await {
			match msg.unwrap() {
				Message::Binary(buf) => {
					let protocol_version = 1;
					let packet = protocol::ToServer::deserialize(protocol_version, &buf).unwrap();

					match packet {
						protocol::ToServer::Init { .. } => {
							send_init_packet(&mut tx).await;

							start_js_echo_actor(&mut tx, actor_id).await;
						}
						protocol::ToServer::Events(events) => {
							for event in events {
								tracing::info!(?event, "received event");

								let protocol::Event::ActorStateUpdate { state, .. } =
									event.inner.deserialize().unwrap();

								match state {
									protocol::ActorState::Starting => {
										if let State::None = actor_state {
											actor_state = State::Starting;
										} else {
											panic!(
												"invalid prior state: {actor_state:?} -> {state:?}"
											);
										}

										// Verify client state
										let actors = ctx.actors().read().await;
										assert!(
											actors.contains_key(&actor_id),
											"actor not in client memory"
										);
									}
									protocol::ActorState::Running { ref ports, .. } => {
										let port = ports.values().next().unwrap().source;

										if let State::Starting = actor_state {
											actor_state = State::Running;
										} else {
											panic!(
												"invalid prior state: {actor_state:?} -> {state:?}"
											);
										}

										// Verify client state
										let actors = ctx.actors().read().await;
										assert!(
											actors.contains_key(&actor_id),
											"actor not in client memory"
										);

										tokio::time::sleep(std::time::Duration::from_millis(250))
											.await;

										tracing::info!("sending echo");

										// Send echo test
										let req = b"hello world";
										let res = reqwest::Client::new()
											.post(format!("http://0.0.0.0:{port}"))
											.body(req.to_vec())
											.send()
											.await
											.unwrap()
											.error_for_status()
											.unwrap()
											.bytes()
											.await
											.unwrap();
										assert_eq!(req, &res[..], "echo failed");

										tracing::info!("echo success");

										// Stop actor
										send_command(
											&mut tx,
											protocol::Command::SignalActor {
												actor_id,
												signal: Signal::SIGKILL as i32,
											},
										)
										.await;
									}
									protocol::ActorState::Stopped => {
										if let State::Running = actor_state {
											actor_state = State::Stopped;
										} else {
											panic!(
												"invalid prior state: {actor_state:?} -> {state:?}"
											);
										}

										// Verify client state
										let actors = ctx.actors().read().await;
										assert!(
											actors.contains_key(&actor_id),
											"actor not in client memory"
										);
									}
									protocol::ActorState::Exited { .. } => {
										if let State::Stopped = actor_state {
											actor_state = State::Exited;
										} else {
											panic!(
												"invalid prior state: {actor_state:?} -> {state:?}"
											);
										}

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
