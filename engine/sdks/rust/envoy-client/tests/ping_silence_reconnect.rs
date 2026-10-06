//! Drives a real envoy against a fake engine WebSocket server through a reconnect.
//!
//! The engine sends Init and any missed commands before its first ping on a new connection. A
//! ping from the previous connection must not stop an actor started by those commands.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rivet_envoy_client::config::{
	BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
	WebSocketSender,
};
use rivet_envoy_client::envoy::start_envoy_sync;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Instant;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use vbare::OwnedVersionedData;

/// The envoy subtracts min(3s, threshold / 2), so it stops actors 2s after the last ping.
const ENVOY_LOST_THRESHOLD_MS: i64 = 4_000;
const SILENCE_DEADLINE: Duration = Duration::from_millis(2_000);

struct IdleCallbacks;

impl EnvoyCallbacks for IdleCallbacks {
	fn on_actor_start(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_generation: u32,
		_config: protocol::ActorConfig,
		_preloaded_kv: Option<protocol::PreloadedKv>,
	) -> BoxFuture<anyhow::Result<()>> {
		Box::pin(async { Ok(()) })
	}

	fn on_shutdown(&self) {}

	fn fetch(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
	) -> BoxFuture<anyhow::Result<HttpResponse>> {
		Box::pin(async { anyhow::bail!("fetch should not be called") })
	}

	fn websocket(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
		_path: String,
		_headers: HashMap<String, String>,
		_is_hibernatable: bool,
		_is_restoring_hibernatable: bool,
		_sender: WebSocketSender,
	) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
		Box::pin(async { anyhow::bail!("websocket should not be called") })
	}

	fn can_hibernate(
		&self,
		_actor_id: &str,
		_gateway_id: &protocol::GatewayId,
		_request_id: &protocol::RequestId,
		_request: &HttpRequest,
	) -> BoxFuture<anyhow::Result<bool>> {
		Box::pin(async { Ok(false) })
	}
}

async fn accept(listener: &TcpListener) -> WebSocketStream<TcpStream> {
	let (stream, _) = listener.accept().await.expect("accept envoy connection");
	tokio_tungstenite::accept_hdr_async(stream, |_: &Request, mut response: Response| {
		response.headers_mut().insert(
			"Sec-WebSocket-Protocol",
			"rivet".parse().expect("header value"),
		);
		Ok(response)
	})
	.await
	.expect("upgrade envoy connection")
}

async fn send(ws: &mut WebSocketStream<TcpStream>, message: protocol::ToEnvoy) {
	let data = protocol::versioned::ToEnvoy::wrap_latest(message)
		.serialize(protocol::PROTOCOL_VERSION)
		.expect("encode ToEnvoy");
	ws.send(Message::Binary(data.into()))
		.await
		.expect("send to envoy");
}

async fn recv(ws: &mut WebSocketStream<TcpStream>) -> protocol::ToRivet {
	loop {
		let frame = ws
			.next()
			.await
			.expect("envoy connection closed")
			.expect("read from envoy");
		if let Message::Binary(data) = frame {
			return protocol::versioned::ToRivet::deserialize(&data, protocol::PROTOCOL_VERSION)
				.expect("decode ToRivet");
		}
	}
}

async fn recv_actor_state(ws: &mut WebSocketStream<TcpStream>) -> protocol::ActorState {
	loop {
		if let protocol::ToRivet::ToRivetEvents(events) = recv(ws).await {
			for event in events {
				if let protocol::Event::EventActorStateUpdate(update) = event.inner {
					return update.state;
				}
			}
		}
	}
}

fn init() -> protocol::ToEnvoy {
	protocol::ToEnvoy::ToEnvoyInit(protocol::ToEnvoyInit {
		metadata: protocol::ProtocolMetadata {
			envoy_lost_threshold: ENVOY_LOST_THRESHOLD_MS,
			actor_stop_threshold: 30_000,
			max_response_payload_size: 1024 * 1024,
		},
	})
}

fn start_actor() -> protocol::ToEnvoy {
	protocol::ToEnvoy::ToEnvoyCommands(vec![protocol::CommandWrapper {
		checkpoint: protocol::ActorCheckpoint {
			actor_id: "actor-a".to_string(),
			generation: 1,
			index: 0,
		},
		inner: protocol::Command::CommandStartActor(protocol::CommandStartActor {
			config: protocol::ActorConfig {
				name: "test".to_string(),
				key: None,
				create_ts: 0,
				input: None,
			},
			hibernating_requests: Vec::new(),
			preloaded_kv: None,
		}),
	}])
}

#[tokio::test]
async fn ping_from_previous_connection_does_not_stop_actor_started_after_reconnect() {
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind fake engine");
	let handle = start_envoy_sync(EnvoyConfig {
		version: 1,
		endpoint: format!("http://{}", listener.local_addr().expect("address")),
		token: None,
		namespace: "test".to_string(),
		pool_name: "test".to_string(),
		prepopulate_actor_names: HashMap::new(),
		metadata: None,
		not_global: true,
		debug_latency_ms: None,
		callbacks: Arc::new(IdleCallbacks),
	});

	// The first connection is pinged once, then the engine drops it.
	let mut first = accept(&listener).await;
	send(&mut first, init()).await;
	send(
		&mut first,
		protocol::ToEnvoy::ToEnvoyPing(protocol::ToEnvoyPing { ts: 0 }),
	)
	.await;
	while !matches!(recv(&mut first).await, protocol::ToRivet::ToRivetPong(_)) {}
	let old_ping_at = Instant::now();
	drop(first);

	// The envoy reconnects. Once the old ping is past the silence deadline, the engine sends
	// Init and a missed start command before its first ping on this connection.
	let mut second = accept(&listener).await;
	tokio::time::sleep_until(old_ping_at + SILENCE_DEADLINE + Duration::from_millis(200)).await;
	send(&mut second, init()).await;
	send(&mut second, start_actor()).await;
	assert!(matches!(
		recv_actor_state(&mut second).await,
		protocol::ActorState::ActorStateRunning
	));

	let early =
		tokio::time::timeout(Duration::from_millis(300), recv_actor_state(&mut second)).await;
	assert!(
		early.is_err(),
		"actor stopped because of a ping from the previous connection: {early:?}"
	);

	// After its first ping, the new connection goes silent, so the actor stops at that ping's
	// silence deadline.
	send(
		&mut second,
		protocol::ToEnvoy::ToEnvoyPing(protocol::ToEnvoyPing { ts: 0 }),
	)
	.await;
	let pinged_at = Instant::now();
	let state = tokio::time::timeout(Duration::from_secs(5), recv_actor_state(&mut second))
		.await
		.expect("actor should stop once the new connection goes silent");
	assert!(
		pinged_at.elapsed() >= SILENCE_DEADLINE - Duration::from_millis(100),
		"actor stopped {:?} after the ping, before the silence deadline",
		pinged_at.elapsed()
	);
	let protocol::ActorState::ActorStateStopped(stopped) = state else {
		panic!("expected a stopped actor, got {state:?}");
	};
	assert_eq!(stopped.code, protocol::StopCode::Error);

	handle.shutdown(true);
}
