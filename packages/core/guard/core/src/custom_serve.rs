use anyhow::*;
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming as BodyIncoming;
use hyper::{Request, Response};
use hyper_tungstenite::HyperWebsocket;

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

	/// Handle a WebSocket connection after upgrade.
	///
	/// Contract for retries:
	/// - Return `Ok(())` after you have accepted (`await`ed) the client websocket and
	///   completed the streaming lifecycle. No further retries are possible.
	/// - Return `Err((client_ws, err))` if you have NOT accepted the websocket yet and
	///   want the proxy to optionally re-resolve and retry with a different handler.
	///   You must not `await` the websocket before returning this error.
	async fn handle_websocket(
		&self,
		client_ws: HyperWebsocket,
		headers: &hyper::HeaderMap,
		path: &str,
		request_context: &mut RequestContext,
	) -> std::result::Result<(), (HyperWebsocket, anyhow::Error)>;
}
