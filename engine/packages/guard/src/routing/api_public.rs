use std::sync::Arc;

use anyhow::*;
use async_trait::async_trait;
use bytes::Bytes;
use gas::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response};
use rivet_guard_core::WebSocketHandle;
use rivet_guard_core::proxy_service::{ResponseBody, RoutingOutput};
use rivet_guard_core::{CustomServeTrait, request_context::RequestContext};
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tower::Service;

struct ApiPublicService {
	router: axum::Router,
}

#[async_trait]
impl CustomServeTrait for ApiPublicService {
	async fn handle_request(
		&self,
		req: Request<Full<Bytes>>,
		_request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Clone the router to get a mutable service
		let mut service = self.router.clone();

		// Call the service
		let response = service
			.call(req)
			.await
			.map_err(|e| anyhow::anyhow!("Failed to call api-public service: {}", e))?;

		// Collect the body and convert to ResponseBody
		let (parts, body) = response.into_parts();
		let collected = body
			.collect()
			.await
			.map_err(|e| anyhow::anyhow!("Failed to collect response body: {}", e))?;
		let bytes = collected.to_bytes();
		let response_body = ResponseBody::Full(Full::new(bytes));
		let response = Response::from_parts(parts, response_body);

		Ok(response)
	}

	async fn handle_websocket(
		&self,
		_client_ws: WebSocketHandle,
		_headers: &hyper::HeaderMap,
		_path: &str,
		_request_context: &mut RequestContext,
		_unique_request_id: Uuid,
	) -> Result<Option<CloseFrame>> {
		bail!("api-public does not support WebSocket connections")
	}
}

/// Route requests to the api-public service
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	target: &str,
	_host: &str,
	_path: &str,
) -> Result<Option<RoutingOutput>> {
	// Check target
	if target != "api-public" {
		return Ok(None);
	}

	// Create the router once
	let router =
		rivet_api_public::create_router("api-public", ctx.config().clone(), ctx.pools().clone())
			.await?;

	let service = Arc::new(ApiPublicService { router });

	return Ok(Some(RoutingOutput::CustomServe(service)));
}
