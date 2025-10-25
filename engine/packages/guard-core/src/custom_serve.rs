use anyhow::*;
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response};
use uuid::Uuid;

use crate::WebSocketHandle;
use crate::proxy_service::ResponseBody;
use crate::request_context::RequestContext;

/// Trait for custom request serving logic that can handle both HTTP and WebSocket requests
#[async_trait]
pub trait CustomServeTrait: Send + Sync {
	/// Handle a regular HTTP request
	async fn handle_request(
		&self,
		req: Request<Full<Bytes>>,
		request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>>;

	/// Handle a WebSocket connection after upgrade. Supports connection retries.
	async fn handle_websocket(
		&self,
		websocket: WebSocketHandle,
		headers: &hyper::HeaderMap,
		path: &str,
		request_context: &mut RequestContext,
		// Identifies the websocket across retries.
		unique_request_id: Uuid,
	) -> Result<()>;
}
