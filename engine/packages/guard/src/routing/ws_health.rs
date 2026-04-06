use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use hyper_tungstenite::tungstenite::Message;
use rivet_guard_core::custom_serve::CustomServeTrait;
use rivet_guard_core::request_context::RequestContext;
use rivet_guard_core::{ResponseBody, RoutingOutput, WebSocketHandle};

pub fn matches_path(path: &str) -> bool {
	path == "/health/ws" || path == "/health/ws/"
}

pub fn route_request() -> RoutingOutput {
	RoutingOutput::CustomServe(Arc::new(WebSocketHealthService))
}

struct WebSocketHealthService;

#[async_trait]
impl CustomServeTrait for WebSocketHealthService {
	async fn handle_request(
		&self,
		_req: Request<Full<Bytes>>,
		_req_ctx: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		Ok(Response::builder()
			.status(StatusCode::UPGRADE_REQUIRED)
			.body(ResponseBody::Full(Full::new(Bytes::from_static(
				b"WebSocket-only endpoint",
			))))?)
	}

	async fn handle_websocket(
		&self,
		_req_ctx: &mut RequestContext,
		websocket: WebSocketHandle,
		_after_hibernation: bool,
	) -> Result<Option<tokio_tungstenite::tungstenite::protocol::frame::CloseFrame>> {
		let ws_rx = websocket.recv();

		while let Some(message) = ws_rx.lock().await.next().await {
			match message? {
				Message::Text(text) if text == "ping" => {
					websocket.send(Message::Text("pong".into())).await?;
				}
				Message::Ping(payload) => {
					websocket.send(Message::Pong(payload)).await?;
				}
				Message::Close(_) => break,
				_ => {}
			}
		}

		Ok(None)
	}
}
