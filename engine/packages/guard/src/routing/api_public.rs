use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use gas::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response};
use rivet_guard_core::request_context::RequestContext;
use rivet_guard_core::{CustomServeTrait, ResponseBody, RoutingOutput};
use tower::Service;

struct ApiPublicService {
	router: axum::Router,
}

#[async_trait]
impl CustomServeTrait for ApiPublicService {
	#[tracing::instrument(skip_all)]
	async fn handle_request(
		&self,
		req: Request<Full<Bytes>>,
		_req_ctx: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Clone the router to get a mutable service
		let mut service = self.router.clone();

		// Call the service
		let response = service
			.call(req)
			.await
			.context("failed to call api-public service")?;

		// Collect the body and convert to ResponseBody
		let (parts, body) = response.into_parts();
		let collected = body
			.collect()
			.await
			.context("failed to collect response body")?;
		let bytes = collected.to_bytes();
		let response_body = ResponseBody::Full(Full::new(bytes));
		let response = Response::from_parts(parts, response_body);

		Ok(response)
	}
}

/// Route requests to the api-public service
#[tracing::instrument(skip_all)]
pub async fn route_request(ctx: &StandaloneCtx, target: &str) -> Result<Option<RoutingOutput>> {
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
