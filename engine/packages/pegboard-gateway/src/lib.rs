use axum::{
	Router,
	response::{IntoResponse, Response},
};
use rivet_engine_context::EngineContext;

mod acquire;
mod http_tunnel;
mod raw_rest_path;
mod retry_budget;
mod route;
mod routes;
mod start_wait;
pub mod start_waits;
mod websocket_tunnel;

pub use retry_budget::{Retries, RetryBudget};

/// The process's start waits, shared by every gateway request. Building them starts nothing:
/// each wait subscribes for its notices when a request creates it, and the waiting requests
/// receive them. Without UPS no wait can receive a notice, and every request waits on its
/// fallback poll alone.
pub fn start_waits(ctx: &EngineContext) -> std::sync::Arc<start_waits::StartWaits> {
	let ups = match ctx.ups() {
		Ok(ups) => Some(ups),
		Err(error) => {
			tracing::warn!(
				?error,
				"gateway start waits have no UPS; requests wait on their fallback polls alone"
			);
			None
		}
	};
	start_waits::StartWaits::new(
		ctx.env().clone(),
		ups,
		ctx.config().pegboard().gateway_start_waits_capacity(),
	)
}

pub fn router(ctx: &EngineContext, waits: &std::sync::Arc<start_waits::StartWaits>) -> Router {
	Router::new()
		// Nesting to allow us to extract the raw user path
		.nest(
			"/gateway/{selector}",
			Router::new().fallback(axum::routing::any(routes::gateway)),
		)
		// The nest does not match a bare trailing slash
		.route("/gateway/{selector}/", axum::routing::any(routes::gateway))
		// A request on no route may still name its actor by header or subprotocol; the
		// fallback also answers everything else the engine does not serve.
		.fallback(axum::routing::any(routes::direct))
		.layer(axum::Extension(waits.clone()))
		// Retries of refused requests across this process share one budget.
		.layer(axum::Extension(std::sync::Arc::new(
			retry_budget::RetryBudget::from_config(ctx),
		)))
		.with_state(ctx.clone())
}

#[derive(Debug, rivet_error::RivetError)]
#[error("actor", "not_found", "The actor does not exist or was destroyed.")]
pub(crate) struct ActorNotFound;

/// The gateway's query parameters are malformed or ask for something it does not support.
#[derive(Debug, rivet_error::RivetError, serde::Serialize)]
#[error(
	"guard",
	"query_invalid_params",
	"invalid query gateway params",
	"invalid query gateway params: {detail}"
)]
pub(crate) struct QueryInvalidParams {
	pub detail: String,
}

/// A gateway request's failure as an HTTP response, in the canonical format every edge error
/// uses (see [`rivet_guard_core::errors`]). A refusal the client may retry also carries its
/// `Retry-After` hint.
pub(crate) struct GatewayError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for GatewayError {
	fn from(error: E) -> Self {
		Self(error.into())
	}
}

impl IntoResponse for GatewayError {
	fn into_response(self) -> Response {
		let error = self.0;
		if let Some(refusal) = retry_budget::refusal(&error) {
			tracing::debug!(?error, "gateway request refused; the client may retry");
			return retry_budget::refusal_response(refusal);
		}
		log_failure(&error);
		let mut response =
			rivet_guard_core::errors::http_response(&error).map(axum::body::Body::from);
		// A retryable error that carries a hint also sends it as `Retry-After`.
		let retry_after_ms = rivet_error::RivetError::extract(&error)
			.metadata()
			.and_then(|metadata| metadata.get("retry_after_ms")?.as_u64());
		if let Some(retry_after_ms) = retry_after_ms {
			let retry_after_secs = retry_after_ms.div_ceil(1_000).max(1);
			response.headers_mut().insert(
				hyper::header::RETRY_AFTER,
				hyper::header::HeaderValue::from(retry_after_secs),
			);
		}
		response
	}
}

/// Logs a failed gateway request. The client sees only the error's group and code, so the
/// whole chain is logged.
pub(crate) fn log_failure(error: &anyhow::Error) {
	let structured = error
		.chain()
		.any(|cause| cause.downcast_ref::<rivet_error::RivetError>().is_some());
	if structured {
		tracing::debug!(?error, "gateway request failed");
	} else {
		tracing::warn!(?error, "gateway request failed with an internal error");
	}
}


