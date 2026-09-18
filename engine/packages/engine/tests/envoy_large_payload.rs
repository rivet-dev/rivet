#[path = "common/mod.rs"]
mod common;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::{
	Message, client::IntoClientRequest, protocol::WebSocketConfig,
};

const DEFAULT_TUNGSTENITE_MESSAGE_LIMIT: usize = 16 << 20;
const BELOW_DEFAULT_LIMIT: usize = DEFAULT_TUNGSTENITE_MESSAGE_LIMIT - 64 * 1024;
const ABOVE_DEFAULT_LIMIT: usize = DEFAULT_TUNGSTENITE_MESSAGE_LIMIT + 64 * 1024;

#[test]
fn envoy_websocket_accepts_payloads_across_default_tungstenite_limit() {
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
			let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
				builder.with_actor_behavior("test-actor", |_| {
					Box::new(common::test_envoy::EchoActor::new())
				})
			})
			.await;

			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"test-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Sleep,
			)
			.await;
			let actor_id = res.actor.actor_id.to_string();
			wait_for_envoy_actor(&envoy, &actor_id).await;

			let mut request = format!("ws://127.0.0.1:{}/ws", ctx.leader_dc().guard_port())
				.into_client_request()
				.expect("failed to create WebSocket request");
			request.headers_mut().insert(
				"Sec-WebSocket-Protocol",
				format!(
					"rivet, rivet_target.actor, rivet_actor.{}",
					urlencoding::encode(&actor_id)
				)
				.parse()
				.unwrap(),
			);

			let websocket_config = WebSocketConfig::default()
				.max_message_size(None)
				.max_frame_size(None);
			let (ws_stream, response) = tokio_tungstenite::connect_async_with_config(
				request,
				Some(websocket_config),
				false,
			)
			.await
			.expect("failed to connect WebSocket through guard");
			assert_eq!(response.status(), 101);
			let (mut write, mut read) = ws_stream.split();

			let mut disconnect = envoy.wait_for_next_connection_event(
				common::test_envoy::EnvoyConnectionEvent::Disconnected,
			);
			disconnect.assert_no_event();

			for payload_size in [BELOW_DEFAULT_LIMIT, ABOVE_DEFAULT_LIMIT] {
				tracing::info!(payload_size, "sending WebSocket payload through guard");
				write
					.send(Message::Binary(vec![b'x'; payload_size].into()))
					.await
					.expect("failed to send WebSocket payload through guard");

				let response =
					tokio::time::timeout(std::time::Duration::from_secs(20), read.next())
						.await
						.unwrap_or_else(|_| {
							panic!("timed out waiting for {payload_size}-byte WebSocket echo")
						})
						.unwrap_or_else(|| {
							panic!(
								"WebSocket stream ended before {payload_size}-byte payload was echoed"
							)
						})
						.expect("failed to receive WebSocket echo");

				let Message::Text(response) = response else {
					panic!("expected text echo, got {response:?}");
				};
				let response = response.as_bytes();
				assert_eq!(response.len(), "Echo: ".len() + payload_size);
				assert_eq!(&response[.."Echo: ".len()], b"Echo: ");
				assert!(response["Echo: ".len()..].iter().all(|byte| *byte == b'x'));
				disconnect.assert_no_event();
			}
		},
	);
}

#[test]
fn oversized_decoded_actor_message_does_not_disconnect_envoy() {
	common::run(
		common::TestOpts::new(1).with_timeout(90),
		|ctx| async move {
			let dc = ctx.leader_dc();
			// Engine binaries stamp their compiled wire versions during config loading. The test
			// harness constructs Config directly, so mirror that startup step before using pubsub.
			dc.config
				.set_protocols(rivet_build_meta::compiled_runtime_protocols());
			let (namespace, _) = common::setup_test_namespace(dc).await;
			let envoy = common::setup_envoy(dc, &namespace, |builder| {
				builder.with_actor_behavior("test-actor", |_| {
					Box::new(common::test_envoy::EchoActor::new())
				})
			})
			.await;

			let res = common::create_actor(
				dc.guard_port(),
				&namespace,
				"test-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Sleep,
			)
			.await;
			let actor_id = res.actor.actor_id.to_string();
			wait_for_envoy_actor(&envoy, &actor_id).await;

			let response_payload_limit = dc.config.pegboard().envoy_max_response_payload_size();
			assert!(
				response_payload_limit < dc.config.guard().websocket_max_message_size(),
				"test payload must pass Guard's transport limit before decoded validation"
			);

			let websocket_config = WebSocketConfig::default()
				.max_message_size(None)
				.max_frame_size(None);
			let (mut oversized_ws, response) = tokio_tungstenite::connect_async_with_config(
				actor_websocket_request(dc.guard_port(), &actor_id),
				Some(websocket_config),
				false,
			)
			.await
			.expect("failed to connect oversized actor WebSocket through guard");
			assert_eq!(response.status(), 101);

			let mut disconnect = envoy.wait_for_next_connection_event(
				common::test_envoy::EnvoyConnectionEvent::Disconnected,
			);
			disconnect.assert_no_event();

			// EchoActor prefixes the response with `Echo: `, making the actor-to-engine decoded
			// payload exceed the configured limit while the client-to-Guard message remains valid.
			oversized_ws
				.send(Message::Binary(vec![b'x'; response_payload_limit].into()))
				.await
				.expect("failed to send payload that produces an oversized actor response");

			let close =
				tokio::time::timeout(std::time::Duration::from_secs(20), oversized_ws.next())
					.await
					.expect("timed out waiting for oversized response rejection")
					.expect("application WebSocket ended without a close frame")
					.expect("failed to receive oversized response rejection");
			match close {
				Message::Close(Some(frame)) => {
					assert_eq!(u16::from(frame.code), 1009);
					assert!(frame.reason.contains("maximum size"));
				}
				other => panic!("expected message-too-big close frame, got {other:?}"),
			}

			let websocket_config = WebSocketConfig::default()
				.max_message_size(None)
				.max_frame_size(None);
			let (mut followup_ws, response) = tokio_tungstenite::connect_async_with_config(
				actor_websocket_request(dc.guard_port(), &actor_id),
				Some(websocket_config),
				false,
			)
			.await
			.expect("failed to connect follow-up actor WebSocket through guard");
			assert_eq!(response.status(), 101);

			followup_ws
				.send(Message::Text("still-alive".into()))
				.await
				.expect("failed to send follow-up message");
			let response =
				tokio::time::timeout(std::time::Duration::from_secs(20), followup_ws.next())
					.await
					.expect("timed out waiting for follow-up response")
					.expect("follow-up WebSocket ended before response")
					.expect("failed to receive follow-up response");
			assert_eq!(response, Message::Text("Echo: still-alive".into()));

			disconnect.assert_no_event();
		},
	);
}

fn actor_websocket_request(
	guard_port: u16,
	actor_id: &str,
) -> tokio_tungstenite::tungstenite::http::Request<()> {
	let mut request = format!("ws://127.0.0.1:{guard_port}/ws")
		.into_client_request()
		.expect("failed to create WebSocket request");
	request.headers_mut().insert(
		"Sec-WebSocket-Protocol",
		format!(
			"rivet, rivet_target.actor, rivet_actor.{}",
			urlencoding::encode(actor_id)
		)
		.parse()
		.unwrap(),
	);
	request
}

async fn wait_for_envoy_actor(envoy: &common::test_envoy::TestEnvoy, actor_id: &str) {
	tokio::time::timeout(std::time::Duration::from_secs(5), async {
		loop {
			if envoy.has_actor(actor_id).await {
				break;
			}
			tokio::time::sleep(std::time::Duration::from_millis(50)).await;
		}
	})
	.await
	.expect("envoy should receive actor");
}
