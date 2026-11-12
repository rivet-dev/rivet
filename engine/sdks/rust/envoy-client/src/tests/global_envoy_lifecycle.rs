use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::config::{
	BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
	WebSocketSender,
};
use crate::config::{
	DEFAULT_MAX_QUEUED_REQUESTS_PER_ACTOR, DEFAULT_STOPPED_GENERATION_FLOOR_CAPACITY,
	DEFAULT_STOPPED_GENERATION_FLOOR_TTL, EnvoyConnector, EnvoyStream,
};
use crate::envoy::{GLOBAL_ENVOY, ToEnvoyMessage, start_envoy_sync};
use crate::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;
use tokio::sync::Notify;

struct DropCallbacks {
	dropped: Arc<Notify>,
}

impl Drop for DropCallbacks {
	fn drop(&mut self) {
		self.dropped.notify_one();
	}
}

impl EnvoyCallbacks for DropCallbacks {
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
		Box::pin(async { anyhow::bail!("fetch should not be called in lifecycle tests") })
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
		Box::pin(async { anyhow::bail!("websocket should not be called in lifecycle tests") })
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

static GLOBAL_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct PendingConnector;

impl EnvoyConnector for PendingConnector {
	fn connect(&self, _url: &str) -> BoxFuture<anyhow::Result<EnvoyStream>> {
		Box::pin(std::future::pending())
	}
}

fn config(dropped: Arc<Notify>) -> EnvoyConfig {
	EnvoyConfig {
		version: 1,
		endpoint: "http://127.0.0.1:1".to_owned(),
		token: None,
		namespace: "test".to_owned(),
		pool_name: "test".to_owned(),
		prepopulate_actor_names: HashMap::new(),
		metadata: None,
		not_global: false,
		debug_latency_ms: None,
		stopped_generation_floor_ttl: DEFAULT_STOPPED_GENERATION_FLOOR_TTL,
		stopped_generation_floor_capacity: DEFAULT_STOPPED_GENERATION_FLOOR_CAPACITY,
		max_queued_requests_per_actor: DEFAULT_MAX_QUEUED_REQUESTS_PER_ACTOR,
		connector: Some(Arc::new(PendingConnector)),
		callbacks: Arc::new(DropCallbacks { dropped }),
	}
}

#[tokio::test]
async fn stopped_global_envoy_releases_callback_context() {
	let _guard = GLOBAL_TEST_LOCK.lock().await;
	let dropped = Arc::new(Notify::new());
	let handle = start_envoy_sync(config(dropped.clone()));
	handle.shutdown(true);
	tokio::time::timeout(Duration::from_secs(5), handle.wait_stopped())
		.await
		.unwrap();
	drop(handle);
	tokio::time::timeout(Duration::from_secs(5), dropped.notified())
		.await
		.expect("stopped global envoy retained its callback context");
}

#[tokio::test]
async fn rejected_global_envoy_stays_cached_after_shutdown() {
	let _guard = GLOBAL_TEST_LOCK.lock().await;
	let handle = start_envoy_sync(config(Arc::new(Notify::new())));
	// Drive the real rejection/cleanup path without opening a network connection.
	handle
		.shared
		.envoy_tx
		.send(ToEnvoyMessage::RegistrationRejected {
			session: 0,
			ray_id: None,
		})
		.unwrap();
	tokio::time::timeout(Duration::from_secs(5), handle.wait_stopped())
		.await
		.unwrap();
	assert!(handle.terminal_error().unwrap().contains("pool.not_found"));
	let reused = start_envoy_sync(config(Arc::new(Notify::new())));
	let same_instance = Arc::ptr_eq(&handle.shared, &reused.shared);
	if !same_instance {
		reused.shutdown(true);
		tokio::time::timeout(Duration::from_secs(5), reused.wait_stopped())
			.await
			.unwrap();
	}
	// Only this test owns the global slot; release the intentionally terminal cache
	// so the ordinary-stop case remains independent of test execution order.
	GLOBAL_ENVOY.get().unwrap().lock().take();
	assert!(
		same_instance,
		"cleanup discarded the terminal registration error"
	);
	assert_eq!(reused.terminal_error(), handle.terminal_error());
}
