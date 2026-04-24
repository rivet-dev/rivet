use std::{
	collections::HashMap,
	io::Cursor,
	net::SocketAddr,
	sync::{
		Arc, Mutex,
		atomic::{AtomicBool, Ordering},
	},
};

use anyhow::{Result, bail};
use axum::{
	Json, Router,
	body::Bytes,
	extract::{Path, State},
	http::{HeaderMap, StatusCode, header},
	response::IntoResponse,
	routing::{post, put},
};
use rivet_envoy_client::{
	config::{
		BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
		WebSocketSender,
	},
	context::{SharedContext, WsTxMessage},
	handle::EnvoyHandle,
	protocol,
};
use rivetkit::{Actor, Ctx, action, client::GetOrCreateOptions};
use rivetkit_client_protocol as wire;
use rivetkit_core::ActorContext;
use serde::Serialize;
use serde_json::{Value as JsonValue, json};
use tokio::{net::TcpListener, sync::mpsc};
use vbare::OwnedVersionedData;

struct CallerActor;

impl Actor for CallerActor {
	type Input = ();
	type ConnParams = ();
	type ConnState = ();
	type Action = action::Raw;
}

#[derive(Clone)]
struct TestState {
	saw_sibling_action: Arc<AtomicBool>,
}

#[tokio::test]
async fn actor_ctx_client_calls_sibling_action() {
	let state = TestState {
		saw_sibling_action: Arc::new(AtomicBool::new(false)),
	};
	let app = Router::new()
		.route("/actors", put(get_or_create_actor))
		.route("/gateway/{actor_id}/action/{action}", post(sibling_action))
		.with_state(state.clone());
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let server = tokio::spawn(async move {
		axum::serve(listener, app).await.unwrap();
	});

	let core_ctx = ActorContext::new("caller-1", "caller", Vec::new(), "local");
	core_ctx.configure_envoy(test_envoy_handle(endpoint(addr)), Some(1));
	let ctx = Ctx::<CallerActor>::new(core_ctx);

	let output = call_sibling(ctx).await.unwrap();

	assert_eq!(output, json!({ "reply": "pong" }));
	assert!(state.saw_sibling_action.load(Ordering::SeqCst));

	server.abort();
}

async fn call_sibling(ctx: Ctx<CallerActor>) -> Result<JsonValue> {
	let sibling = ctx.client()?.get_or_create(
		"sibling",
		vec!["sibling-key".to_string()],
		GetOrCreateOptions::default(),
	)?;

	sibling.action("ping", vec![json!("from-caller")]).await
}

async fn get_or_create_actor(Json(body): Json<JsonValue>) -> impl IntoResponse {
	assert_eq!(body.get("name"), Some(&json!("sibling")));

	Json(json!({
		"actor": {
			"actor_id": "sibling-1",
			"name": "sibling",
			"key": body.get("key").and_then(JsonValue::as_str).unwrap_or("[]"),
		},
		"created": false,
	}))
}

async fn sibling_action(
	State(state): State<TestState>,
	Path((actor_id, action)): Path<(String, String)>,
	headers: HeaderMap,
	body: Bytes,
) -> impl IntoResponse {
	assert_eq!(actor_id, "sibling-1@secret");
	assert_eq!(action, "ping");
	assert_eq!(
		headers
			.get("x-rivet-token")
			.and_then(|value| value.to_str().ok()),
		Some("secret")
	);
	state.saw_sibling_action.store(true, Ordering::SeqCst);

	let request =
		<wire::versioned::HttpActionRequest as OwnedVersionedData>::deserialize_with_embedded_version(
			&body,
		)
		.unwrap();
	let args: Vec<JsonValue> =
		ciborium::from_reader(Cursor::new(request.args)).expect("decode action args");
	assert_eq!(args, vec![json!("from-caller")]);

	let payload = wire::versioned::HttpActionResponse::wrap_latest(wire::HttpActionResponse {
		output: cbor(&json!({ "reply": "pong" })),
	})
	.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
	.unwrap();

	(
		StatusCode::OK,
		[(header::CONTENT_TYPE, "application/octet-stream")],
		payload,
	)
}

fn test_envoy_handle(endpoint: String) -> EnvoyHandle {
	let (envoy_tx, _envoy_rx) = mpsc::unbounded_channel();
	let shared = Arc::new(SharedContext {
		config: EnvoyConfig {
			version: 1,
			endpoint,
			token: Some("secret".to_string()),
			namespace: "test-ns".to_string(),
			pool_name: "test-pool".to_string(),
			prepopulate_actor_names: HashMap::new(),
			metadata: None,
			not_global: true,
			debug_latency_ms: None,
			callbacks: Arc::new(IdleEnvoyCallbacks),
		},
		envoy_key: "test-envoy".to_string(),
		envoy_tx,
		actors: Arc::new(Mutex::new(HashMap::new())),
		live_tunnel_requests: Arc::new(Mutex::new(HashMap::new())),
		pending_hibernation_restores: Arc::new(Mutex::new(HashMap::new())),
		ws_tx: Arc::new(tokio::sync::Mutex::new(
			None::<mpsc::UnboundedSender<WsTxMessage>>,
		)),
		protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
		shutting_down: AtomicBool::new(false),
		stopped_tx: tokio::sync::watch::channel(true).0,
	});

	EnvoyHandle::from_shared(shared)
}

struct IdleEnvoyCallbacks;

impl EnvoyCallbacks for IdleEnvoyCallbacks {
	fn on_actor_start(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_generation: u32,
		_config: protocol::ActorConfig,
		_preloaded_kv: Option<protocol::PreloadedKv>,
		_sqlite_schema_version: u32,
		_sqlite_startup_data: Option<protocol::SqliteStartupData>,
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
		Box::pin(async { bail!("fetch should not run in c.client test") })
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
		Box::pin(async { bail!("websocket should not run in c.client test") })
	}

	fn can_hibernate(
		&self,
		_actor_id: &str,
		_gateway_id: &protocol::GatewayId,
		_request_id: &protocol::RequestId,
		_request: &HttpRequest,
	) -> bool {
		false
	}
}

fn endpoint(addr: SocketAddr) -> String {
	format!("http://{addr}")
}

fn cbor<T: Serialize>(value: &T) -> Vec<u8> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded).expect("encode test cbor");
	encoded
}
