use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rivet_envoy_client::config::{
	BoxFuture, EnvoyCallbacks, HttpRequest, HttpResponse, WebSocketHandler,
};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;

/// Default test callbacks that handle HTTP ping and WebSocket echo.
#[derive(Default)]
pub struct DefaultTestCallbacks {
	pub is_shutdown: Arc<AtomicBool>,
}

impl EnvoyCallbacks for DefaultTestCallbacks {
	fn on_actor_start(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		generation: u32,
		_config: protocol::ActorConfig,
		_preloaded_kv: Option<protocol::PreloadedKv>,
		_sqlite_startup_data: Option<protocol::SqliteStartupData>,
	) -> BoxFuture<anyhow::Result<()>> {
		Box::pin(async move {
			tracing::info!(%actor_id, generation, "actor started");
			Ok(())
		})
	}

	fn on_actor_stop(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		generation: u32,
		reason: protocol::StopActorReason,
	) -> BoxFuture<anyhow::Result<()>> {
		Box::pin(async move {
			tracing::info!(%actor_id, generation, ?reason, "actor stopped");
			Ok(())
		})
	}

	fn on_shutdown(&self) {
		tracing::info!("envoy shutdown");

		self.is_shutdown.store(true, Ordering::SeqCst);
	}

	fn fetch(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		request: HttpRequest,
	) -> BoxFuture<anyhow::Result<HttpResponse>> {
		Box::pin(async move {
			tracing::debug!(%actor_id, method = %request.method, path = %request.path, "handling fetch");

			match request.path.as_str() {
				"/ping" => {
					let body = serde_json::to_vec(&serde_json::json!({
						"actorId": actor_id,
						"status": "ok",
						"timestamp": std::time::SystemTime::now()
							.duration_since(std::time::UNIX_EPOCH)
							.unwrap_or_default()
							.as_millis() as i64,
					}))?;

					let mut headers = HashMap::new();
					headers.insert("content-type".to_string(), "application/json".to_string());
					headers.insert("content-length".to_string(), body.len().to_string());

					Ok(HttpResponse {
						status: 200,
						headers,
						body: Some(body),
						body_stream: None,
					})
				}
				_ => {
					let body = b"not found".to_vec();
					let mut headers = HashMap::new();
					headers.insert("content-length".to_string(), body.len().to_string());

					Ok(HttpResponse {
						status: 404,
						headers,
						body: Some(body),
						body_stream: None,
					})
				}
			}
		})
	}

	fn websocket(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
		_path: String,
		_headers: HashMap<String, String>,
		_is_hibernatable: bool,
		_is_restoring_hibernatable: bool,
		_sender: rivet_envoy_client::config::WebSocketSender,
	) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
		Box::pin(async move {
			tracing::debug!(%actor_id, "handling websocket");
			Ok(WebSocketHandler {
				on_message: Box::new(move |msg| {
					let text = format!("Echo: {}", String::from_utf8_lossy(&msg.data));
					tracing::debug!(echo = %text, "echoing websocket message");
					msg.sender.send_text(&text);
					Box::pin(async {})
				}),
				on_close: Box::new(|code, reason| {
					Box::pin(async move {
						tracing::debug!(code, %reason, "websocket closed");
					})
				}),
				on_open: None,
			})
		})
	}

	fn can_hibernate(
		&self,
		_actor_id: &str,
		_gateway_id: &protocol::GatewayId,
		_request_id: &protocol::RequestId,
		_request: &HttpRequest,
	) -> rivet_envoy_client::config::BoxFuture<anyhow::Result<bool>> {
		Box::pin(async { Ok(false) })
	}
}
