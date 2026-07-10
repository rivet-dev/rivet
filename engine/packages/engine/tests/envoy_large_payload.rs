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
