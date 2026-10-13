use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rivet_envoy_client::config::{
	BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
	WebSocketSender,
};
use rivet_envoy_client::envoy::start_envoy_sync;
use rivet_envoy_client::handle::EnvoyHandle;
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

#[tokio::test]
async fn stopped_global_envoy_releases_callback_context() {
	let dropped = Arc::new(Notify::new());
	let callbacks: Arc<dyn EnvoyCallbacks> = Arc::new(DropCallbacks {
		dropped: dropped.clone(),
	});
	let handle = start_envoy_sync(EnvoyConfig {
		version: 1,
		endpoint: "http://127.0.0.1:1".to_owned(),
		token: None,
		namespace: "test".to_owned(),
		pool_name: "test".to_owned(),
		prepopulate_actor_names: HashMap::new(),
		metadata: None,
		not_global: false,
		debug_latency_ms: None,
		callbacks,
	});

	handle.shutdown(true);
	handle.wait_stopped().await;
	drop(handle);

	tokio::time::timeout(Duration::from_secs(5), dropped.notified())
		.await
		.expect("stopped global envoy retained its callback context");
}
