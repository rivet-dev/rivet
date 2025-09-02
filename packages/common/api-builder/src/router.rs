use anyhow::Result;
use axum::{
	Router,
	extract::{Request, State},
	http::StatusCode,
	middleware::{self, Next},
	response::{IntoResponse, Json, Response},
	routing::get as axum_get,
};
use serde_json::json;
use tower_http::cors::CorsLayer;

use crate::{
	ApiError, RequestIds, context::ApiCtx, create_trace_layer, errors::ApiNotFound,
	global_context::GlobalApiCtx, middleware::http_logging_middleware,
};

pub type ApiRouter = Router<GlobalApiCtx>;

/// Middleware to build ApiCtx and expose it as an extension
async fn api_ctx_middleware(
	State(global_ctx): State<GlobalApiCtx>,
	mut req: Request,
	next: Next,
) -> Result<Response, StatusCode> {
	let dc_label = global_ctx.config.dc_label();

	// Get request IDs from request extensions (set by http_logging_middleware)
	let request_ids = req
		.extensions()
		.get::<RequestIds>()
		.copied()
		.unwrap_or_else(|| RequestIds::new(dc_label));

	let ctx = ApiCtx::new(global_ctx, request_ids.ray_id, request_ids.req_id)
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

	// Insert the ApiCtx into request extensions
	req.extensions_mut().insert(ctx);

	Ok(next.run(req).await)
}

pub async fn create_router(
	name: &'static str,
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	builder: fn(ApiRouter) -> ApiRouter,
) -> Result<Router> {
	let ctx = GlobalApiCtx::new(config.clone(), pools, name).await?;

	let user_router = builder(Router::new());

	// Add standard middleware
	let router = Router::new()
		.fallback(not_found_handler)
		.layer(CorsLayer::permissive())
		.layer(create_trace_layer())
		.route("/health", axum_get(health_check))
		.merge(user_router)
		.layer(middleware::from_fn_with_state(
			config,
			http_logging_middleware,
		))
		.route_layer(middleware::from_fn_with_state(
			ctx.clone(),
			api_ctx_middleware,
		))
		// We need to remove the state from the router so it can be routable
		//
		// See https://docs.rs/axum/latest/axum/struct.Router.html#method.with_state
		.with_state::<()>(ctx);

	Ok(router)
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
	Json(json!({
		"status": "ok",
		"timestamp": chrono::Utc::now().timestamp_millis()
	}))
}

/// 404 handler
pub async fn not_found_handler() -> impl IntoResponse {
	ApiError::from(ApiNotFound.build()).into_response()
}
