use std::collections::HashMap;
use std::sync::Arc;

use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use rivet_envoy_client::config::{
	BoxFuture, EnvoyCallbacks, HttpRequest, HttpResponse, WebSocketHandler, WebSocketMessage,
	WebSocketSender,
};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;
use tokio::sync::{Mutex, oneshot};

use crate::types;

/// Type alias for the threadsafe event callback function.
pub type EventCallback = napi::threadsafe_function::ThreadsafeFunction<
	serde_json::Value,
	napi::threadsafe_function::ErrorStrategy::Fatal,
>;

/// Map of pending callback response channels, keyed by response ID.
pub type ResponseMap = Arc<Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>>;

/// Map of open WebSocket senders, keyed by concatenated gateway_id + request_id (8 bytes).
pub type WsSenderMap = Arc<Mutex<HashMap<[u8; 8], WebSocketSender>>>;

fn make_ws_key(gateway_id: &protocol::GatewayId, request_id: &protocol::RequestId) -> [u8; 8] {
	let mut key = [0u8; 8];
	key[..4].copy_from_slice(gateway_id);
	key[4..].copy_from_slice(request_id);
	key
}

/// Callbacks implementation that bridges envoy events to JavaScript via N-API.
pub struct BridgeCallbacks {
	event_cb: EventCallback,
	response_map: ResponseMap,
	ws_sender_map: WsSenderMap,
}

impl BridgeCallbacks {
	pub fn new(
		event_cb: EventCallback,
		response_map: ResponseMap,
		ws_sender_map: WsSenderMap,
	) -> Self {
		Self {
			event_cb,
			response_map,
			ws_sender_map,
		}
	}

	fn send_event(&self, envelope: serde_json::Value) {
		self.event_cb
			.call(envelope, ThreadsafeFunctionCallMode::NonBlocking);
	}
}

impl EnvoyCallbacks for BridgeCallbacks {
	fn on_actor_start(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		generation: u32,
		config: protocol::ActorConfig,
		_preloaded_kv: Option<protocol::PreloadedKv>,
	) -> BoxFuture<anyhow::Result<()>> {
		let response_map = self.response_map.clone();
		let event_cb = self.event_cb.clone();

		Box::pin(async move {
			let response_id = uuid::Uuid::new_v4().to_string();
			let envelope = serde_json::json!({
				"kind": "actor_start",
				"actorId": actor_id,
				"generation": generation,
				"name": config.name,
				"key": config.key,
				"createTs": config.create_ts,
				"input": config.input.map(|v| base64_encode(&v)),
				"responseId": response_id,
			});

			let (tx, rx) = oneshot::channel();
			{
				let mut map = response_map.lock().await;
				map.insert(response_id, tx);
			}

			tracing::info!(%actor_id, "calling JS actor_start callback via TSFN");
			let status = event_cb.call(envelope, ThreadsafeFunctionCallMode::NonBlocking);
			tracing::info!(%actor_id, ?status, "TSFN call returned");

			let _response = rx
				.await
				.map_err(|_| anyhow::anyhow!("callback response channel closed"))?;

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
		let response_map = self.response_map.clone();
		let event_cb = self.event_cb.clone();

		Box::pin(async move {
			let response_id = uuid::Uuid::new_v4().to_string();
			let envelope = serde_json::json!({
				"kind": "actor_stop",
				"actorId": actor_id,
				"generation": generation,
				"reason": format!("{reason:?}"),
				"responseId": response_id,
			});

			let (tx, rx) = oneshot::channel();
			{
				let mut map = response_map.lock().await;
				map.insert(response_id, tx);
			}

			event_cb.call(envelope, ThreadsafeFunctionCallMode::NonBlocking);

			let _response = rx
				.await
				.map_err(|_| anyhow::anyhow!("callback response channel closed"))?;

			Ok(())
		})
	}

	fn on_shutdown(&self) {
		let envelope = serde_json::json!({
			"kind": "shutdown",
			"reason": "envoy shutdown",
		});
		self.send_event(envelope);
	}

	fn fetch(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		request: HttpRequest,
	) -> BoxFuture<anyhow::Result<HttpResponse>> {
		let response_map = self.response_map.clone();
		let event_cb = self.event_cb.clone();

		Box::pin(async move {
			let msg_id = protocol::MessageId {
				gateway_id,
				request_id,
				message_index: 0,
			};
			let response_id = uuid::Uuid::new_v4().to_string();
			let envelope = serde_json::json!({
				"kind": "http_request",
				"actorId": actor_id,
				"messageId": types::encode_message_id(&msg_id),
				"method": request.method,
				"path": request.path,
				"headers": request.headers,
				"body": request.body.map(|b| base64_encode(&b)),
				"stream": false,
				"responseId": response_id,
			});

			let (tx, rx) = oneshot::channel();
			{
				let mut map = response_map.lock().await;
				map.insert(response_id, tx);
			}

			event_cb.call(envelope, ThreadsafeFunctionCallMode::NonBlocking);

			let response = rx
				.await
				.map_err(|_| anyhow::anyhow!("callback response channel closed"))?;

			let status = response
				.get("status")
				.and_then(|v| v.as_u64())
				.unwrap_or(200) as u16;
			let headers: HashMap<String, String> = response
				.get("headers")
				.and_then(|v| serde_json::from_value(v.clone()).ok())
				.unwrap_or_default();
			let body = response
				.get("body")
				.and_then(|v| v.as_str())
				.and_then(|s| base64_decode(s));

			Ok(HttpResponse {
				status,
				headers,
				body,
				body_stream: None,
			})
		})
	}

	fn websocket(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		_request: HttpRequest,
		path: String,
		headers: HashMap<String, String>,
		_is_hibernatable: bool,
		_is_restoring_hibernatable: bool,
		_sender: WebSocketSender,
	) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
		let event_cb = self.event_cb.clone();
		let ws_sender_map = self.ws_sender_map.clone();

		Box::pin(async move {
			let msg_id = protocol::MessageId {
				gateway_id,
				request_id,
				message_index: 0,
			};
			let msg_id_bytes = types::encode_message_id(&msg_id);

			let ws_key = make_ws_key(&gateway_id, &request_id);

			let event_cb_open = event_cb.clone();
			let event_cb_msg = event_cb.clone();
			let event_cb_close = event_cb.clone();
			let actor_id_open = actor_id.clone();
			let actor_id_msg = actor_id.clone();
			let actor_id_close = actor_id;
			let msg_id_bytes_close = msg_id_bytes.clone();
			let ws_sender_map_open = ws_sender_map.clone();
			let ws_sender_map_close = ws_sender_map.clone();

			Ok(WebSocketHandler {
				on_message: Box::new(move |msg: WebSocketMessage| {
					let msg_id = protocol::MessageId {
						gateway_id: msg.gateway_id,
						request_id: msg.request_id,
						message_index: msg.message_index,
					};
					let envelope = serde_json::json!({
						"kind": "websocket_message",
						"actorId": actor_id_msg,
						"messageId": types::encode_message_id(&msg_id),
						"data": base64_encode(&msg.data),
						"binary": msg.binary,
					});
					event_cb_msg.call(envelope, ThreadsafeFunctionCallMode::NonBlocking);
					Box::pin(async {})
				}),
				on_close: Box::new(move |code, reason| {
					let envelope = serde_json::json!({
						"kind": "websocket_close",
						"actorId": actor_id_close,
						"messageId": msg_id_bytes_close,
						"code": code,
						"reason": reason,
					});
					event_cb_close.call(envelope, ThreadsafeFunctionCallMode::NonBlocking);

					let ws_sender_map_close = ws_sender_map_close.clone();
					Box::pin(async move {
						let mut senders = ws_sender_map_close.lock().await;
						senders.remove(&ws_key);
					})
				}),
				// on_open fires the websocket_open event only after the sender is stored,
				// guaranteeing that ws.send() works as soon as JS receives the event.
				on_open: Some(Box::new(move |sender: WebSocketSender| {
					let envelope = serde_json::json!({
						"kind": "websocket_open",
						"actorId": actor_id_open,
						"messageId": msg_id_bytes,
						"path": path,
						"headers": headers,
					});
					event_cb_open.call(envelope, ThreadsafeFunctionCallMode::NonBlocking);

					Box::pin(async move {
						let mut senders = ws_sender_map_open.lock().await;
						senders.insert(ws_key, sender);
					})
				})),
			})
		})
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

fn base64_encode(data: &[u8]) -> String {
	use base64::Engine;
	base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(data: &str) -> Option<Vec<u8>> {
	use base64::Engine;
	base64::engine::general_purpose::STANDARD.decode(data).ok()
}
