use anyhow::{Result, bail};
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response};
use pegboard::tunnel::id::RequestId;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;

use crate::WebSocketHandle;
use crate::proxy_service::ResponseBody;
use crate::request_context::RequestContext;

pub enum HibernationResult {
	Continue,
	Close,
}

/// Trait for custom request serving logic that can handle both HTTP and WebSocket requests
#[async_trait]
pub trait CustomServeTrait: Send + Sync {
	/// Handle a regular HTTP request
	async fn handle_request(
		&self,
		req: Request<Full<Bytes>>,
		request_context: &mut RequestContext,
		request_id: RequestId,
	) -> Result<Response<ResponseBody>>;

	/// Handle a WebSocket connection after upgrade. Supports connection retries.
	async fn handle_websocket(
		&self,
		_websocket: WebSocketHandle,
		_headers: &hyper::HeaderMap,
		_path: &str,
		_request_context: &mut RequestContext,
		// Identifies the websocket across retries.
		_unique_request_id: RequestId,
		// True if this websocket is reconnecting after hibernation.
		_after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		bail!("service does not support websockets");
	}

	/// Returns true if the websocket should close.
	async fn handle_websocket_hibernation(
		&self,
		_websocket: WebSocketHandle,
		_unique_request_id: RequestId,
	) -> Result<HibernationResult> {
		bail!("service does not support websocket hibernation");
	}
}
