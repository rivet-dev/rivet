use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use rivet_envoy_client::{
	config::{
		self, BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
		WebSocketSender,
	},
	envoy::start_envoy_sync,
	handle::EnvoyHandle,
	protocol,
};
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::{
	accept_hdr_async,
	tungstenite::{
		Message,
		handshake::server::{Request, Response},
		protocol::{CloseFrame, frame::coding::CloseCode},
	},
};
use vbare::Versioned;

#[derive(Default)]
struct Callbacks {
	started: tokio::sync::Notify,
	stopped: tokio::sync::Notify,
}

impl EnvoyCallbacks for Callbacks {
	fn on_actor_start(
		&self,
		_: EnvoyHandle,
		_: String,
		_: u32,
		_: protocol::ActorConfig,
		_: Option<protocol::PreloadedKv>,
	) -> BoxFuture<anyhow::Result<()>> {
		self.started.notify_one();
		Box::pin(async { Ok(()) })
	}
	fn on_actor_stop(
		&self,
		_: EnvoyHandle,
		_: String,
		_: u32,
		_: protocol::StopActorReason,
	) -> BoxFuture<anyhow::Result<()>> {
		self.stopped.notify_one();
		Box::pin(async { Ok(()) })
	}
	fn on_shutdown(&self) {}
	fn fetch(
		&self,
		_: EnvoyHandle,
		_: String,
		_: protocol::GatewayId,
		_: protocol::RequestId,
		_: HttpRequest,
	) -> BoxFuture<anyhow::Result<HttpResponse>> {
		panic!("no requests should arrive in this test")
	}
	fn websocket(
		&self,
		_: EnvoyHandle,
		_: String,
		_: protocol::GatewayId,
		_: protocol::RequestId,
		_: HttpRequest,
		_: String,
		_: HashMap<String, String>,
		_: bool,
		_: bool,
		_: WebSocketSender,
	) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
		panic!("no requests should arrive in this test")
	}
	fn can_hibernate(
		&self,
		_: &str,
		_: &protocol::GatewayId,
		_: &protocol::RequestId,
		_: &HttpRequest,
	) -> BoxFuture<anyhow::Result<bool>> {
		panic!("no requests should arrive in this test")
	}
}

fn config(endpoint: String, callbacks: Arc<Callbacks>) -> EnvoyConfig {
	EnvoyConfig {
		version: 1,
		endpoint,
		token: None,
		namespace: "default".into(),
		pool_name: "missing".into(),
		prepopulate_actor_names: HashMap::new(),
		metadata: None,
		not_global: true,
		debug_latency_ms: None,
		max_queued_requests_per_actor: config::DEFAULT_MAX_QUEUED_REQUESTS_PER_ACTOR,
		stopped_generation_floor_ttl: config::DEFAULT_STOPPED_GENERATION_FLOOR_TTL,
		stopped_generation_floor_capacity: config::DEFAULT_STOPPED_GENERATION_FLOOR_CAPACITY,
		callbacks,
		connector: None,
	}
}

async fn accept(
	stream: tokio::net::TcpStream,
) -> tokio_tungstenite::WebSocketStream<tokio::net::TcpStream> {
	accept_hdr_async(stream, |_: &Request, mut response: Response| {
		response
			.headers_mut()
			.insert("Sec-WebSocket-Protocol", "rivet".parse().unwrap());
		Ok(response)
	})
	.await
	.unwrap()
}

async fn close_after_metadata(listener: &TcpListener, code: CloseCode, reason: &str) {
	let (stream, _) = listener.accept().await.unwrap();
	let mut ws = accept(stream).await;
	assert!(matches!(ws.next().await, Some(Ok(Message::Binary(_)))));
	ws.send(Message::Close(Some(CloseFrame {
		code,
		reason: reason.into(),
	})))
	.await
	.unwrap();
}

#[tokio::test]
async fn missing_pool_stops_instead_of_reconnecting_forever() {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let endpoint = format!("http://{}", listener.local_addr().unwrap());
	let server = tokio::spawn(async move {
		close_after_metadata(&listener, CloseCode::Policy, "pool.not_found#ray").await;
	});
	let handle = start_envoy_sync(config(endpoint, Arc::new(Callbacks::default())));
	let stopped = timeout(Duration::from_secs(3), handle.wait_stopped()).await;
	if stopped.is_err() {
		handle.shutdown_and_wait(true).await;
	}
	assert!(
		stopped.is_ok(),
		"missing pool must stop the envoy rather than retry forever"
	);
	let error = handle.started().await.unwrap_err().to_string();
	assert!(
		error.contains("pool.not_found") && error.contains("missing") && error.contains("default")
	);
	server.await.unwrap();
}

#[tokio::test]
async fn transient_connect_failure_is_retried() {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let endpoint = format!("http://{}", listener.local_addr().unwrap());
	let server = tokio::spawn(async move {
		close_after_metadata(
			&listener,
			CloseCode::Again,
			"envoy.connect_unavailable;retry_after_ms=100#ray",
		)
		.await;
		let (stream, _) = listener.accept().await.unwrap();
		let mut ws = accept(stream).await;
		assert!(matches!(ws.next().await, Some(Ok(Message::Binary(_)))));
		let init = protocol::ToEnvoy::ToEnvoyInit(protocol::ToEnvoyInit {
			metadata: protocol::ProtocolMetadata {
				envoy_lost_threshold: 10_000,
				actor_stop_threshold: 30_000,
				max_response_payload_size: 32 * 1024 * 1024,
			},
		});
		let bytes =
			protocol::versioned::ToEnvoy::serialize(&init, protocol::PROTOCOL_VERSION).unwrap();
		ws.send(Message::Binary(bytes.into())).await.unwrap();
		let _ = ws.next().await;
	});
	let handle = start_envoy_sync(config(endpoint, Arc::new(Callbacks::default())));
	let started = timeout(Duration::from_secs(5), handle.started()).await;
	handle.shutdown_and_wait(true).await;
	assert!(
		matches!(started, Ok(Ok(()))),
		"transient rejection should reconnect and initialize"
	);
	server.await.unwrap();
}

#[tokio::test]
async fn missing_pool_after_initialization_stops_active_generations() {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let endpoint = format!("http://{}", listener.local_addr().unwrap());
	let callbacks = Arc::new(Callbacks::default());
	let server_callbacks = callbacks.clone();
	let server = tokio::spawn(async move {
		let (stream, _) = listener.accept().await.unwrap();
		let mut ws = accept(stream).await;
		assert!(matches!(ws.next().await, Some(Ok(Message::Binary(_)))));
		let init = protocol::ToEnvoy::ToEnvoyInit(protocol::ToEnvoyInit {
			metadata: protocol::ProtocolMetadata {
				envoy_lost_threshold: 10_000,
				actor_stop_threshold: 30_000,
				max_response_payload_size: 32 * 1024 * 1024,
			},
		});
		let bytes =
			protocol::versioned::ToEnvoy::serialize(&init, protocol::PROTOCOL_VERSION).unwrap();
		ws.send(Message::Binary(bytes.into())).await.unwrap();
		let start = protocol::ToEnvoy::ToEnvoyCommands(vec![protocol::CommandWrapper {
			checkpoint: protocol::ActorCheckpoint {
				actor_id: "actor".into(),
				generation: 1,
				index: 1,
			},
			inner: protocol::Command::CommandStartActor(protocol::CommandStartActor {
				config: protocol::ActorConfig {
					name: "work".into(),
					key: None,
					create_ts: 0,
					input: None,
				},
				hibernating_requests: Vec::new(),
				preloaded_kv: None,
			}),
		}]);
		let bytes =
			protocol::versioned::ToEnvoy::serialize(&start, protocol::PROTOCOL_VERSION).unwrap();
		ws.send(Message::Binary(bytes.into())).await.unwrap();
		server_callbacks.started.notified().await;
		drop(ws);
		close_after_metadata(&listener, CloseCode::Policy, "pool.not_found#ray").await;
	});
	let handle = start_envoy_sync(config(endpoint, callbacks.clone()));
	timeout(Duration::from_secs(5), handle.started())
		.await
		.unwrap()
		.unwrap();
	let stopped = timeout(Duration::from_secs(8), handle.wait_stopped()).await;
	if stopped.is_err() {
		handle.shutdown_and_wait(true).await;
	}
	assert!(
		stopped.is_ok(),
		"a lost pool must terminate an initialized envoy"
	);
	timeout(Duration::from_secs(1), callbacks.stopped.notified())
		.await
		.expect("active actor was not stopped");
	server.await.unwrap();
}
