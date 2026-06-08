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

use super::{Phase, phase_timeout};
use crate::{errors, metrics};

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
	let router = phase_timeout(
		Phase::new("route_api_public", &metrics::ROUTE_API_PUBLIC_DURATION),
		ctx.config().guard().route_api_public_timeout(),
		rivet_api_public::router(ctx.config().clone(), ctx.pools().clone()),
		|elapsed, timeout| {
			errors::RouteApiPublicTimeout {
				elapsed_ms: elapsed.as_millis() as u64,
				timeout_ms: timeout.as_millis() as u64,
			}
			.build()
		},
	)
	.await?;

	let service = Arc::new(ApiPublicService { router });

	return Ok(Some(RoutingOutput::CustomServe(service)));
}
