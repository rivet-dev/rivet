use axum::{
	Json, Router,
	extract::{Request, State},
	middleware::{self, Next},
	response::{IntoResponse, Response},
	routing::{delete, get, post, put},
};
use hyper::{
	Method, StatusCode,
	header::{HeaderName, RETRY_AFTER},
};
use rivet_engine_context::EngineContext;
use rivet_guard_core::{X_RIVET_RAY_ID, auth, errors::X_RIVET_ERROR};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

mod routes;
mod tokens;

/// The process's waits for namespace forks. Without UPS no wait can receive a notice, and every
/// fork create reads the namespace at its deadline.
fn namespace_start_waits(
	ctx: &EngineContext,
) -> std::sync::Arc<pegboard::namespace_waits::StartWaits> {
	let ups = match ctx.ups() {
		Ok(ups) => Some(ups),
		Err(error) => {
			tracing::warn!(
				?error,
				"namespace fork waits have no UPS; fork creates read the namespace at their deadline"
			);
			None
		}
	};
	pegboard::namespace_waits::StartWaits::new(
		ups,
		ctx.config().pegboard().gateway_start_waits_capacity(),
	)
}

/// `start_waits` are the gateway's actor start waits, which an actor fork shares so its wait ids
/// never collide with a gateway request's.
pub fn router(
	ctx: &EngineContext,
	start_waits: &std::sync::Arc<pegboard_gateway::start_waits::StartWaits>,
) -> Router {
	router_with_retry_budget(
		ctx,
		start_waits,
		std::sync::Arc::new(pegboard_gateway::RetryBudget::from_config(ctx)),
	)
}

/// Guard shares one retry budget between actor routing and public API requests.
pub fn router_with_retry_budget(
	ctx: &EngineContext,
	start_waits: &std::sync::Arc<pegboard_gateway::start_waits::StartWaits>,
	budget: std::sync::Arc<pegboard_gateway::RetryBudget>,
) -> Router {
	// Clients can read metadata before authenticating. Other routes enforce scoped grants.
	let protected = Router::new()
		.route("/datacenters", get(routes::list_datacenters))
		.route("/pools", get(routes::list_pools))
		.route("/pools/{pool}/nodes", get(routes::list_pool_nodes))
		.route("/namespaces", get(routes::list_namespaces))
		.route("/namespaces", post(routes::create_namespace))
		.route(
			"/namespaces/{name}/archive",
			post(routes::archive_namespace),
		)
		.route("/actors", get(routes::list_actors))
		.route("/actors", post(routes::create_actor))
		.route("/actors", put(routes::get_or_create_actor))
		.route("/actors/names", get(routes::list_actor_names))
		.route("/actors/{actor_id}", get(routes::get_actor))
		.route("/actors/{actor_id}", delete(routes::destroy_actor))
		.route("/actors/{actor_id}/sleep", post(routes::sleep_actor))
		.route("/actors/{actor_id}/fork", post(routes::fork_actor))
		.route(
			"/actors/{actor_id}/reschedule",
			post(routes::reschedule_actor),
		)
		.route(
			"/pools/{pool}",
			get(routes::get_pool)
				.put(routes::upsert_pool)
				.delete(routes::delete_pool),
		)
		.route_layer(middleware::from_fn_with_state(
			ctx.clone(),
			require_credential,
		));
	Router::new()
		.route("/metadata", get(routes::metadata))
		.route("/auth/tokens", post(tokens::create))
		.route("/auth/tokens/inspect", get(tokens::inspect))
		.merge(protected)
		.layer(middleware::from_fn(classify_read_refusal))
		.layer(axum::Extension(budget))
		// Outermost, so preflights are answered before authentication. RivetKit clients send
		// every request with credentials, which rejects wildcards, so the request's own values
		// are mirrored and the readable headers are listed.
		.layer(
			CorsLayer::new()
				.allow_origin(AllowOrigin::mirror_request())
				.allow_methods(AllowMethods::mirror_request())
				.allow_headers(AllowHeaders::mirror_request())
				.allow_credentials(true)
				.expose_headers([
					HeaderName::from_static(X_RIVET_RAY_ID),
					HeaderName::from_static(X_RIVET_ERROR),
					RETRY_AFTER,
				])
				.max_age(std::time::Duration::from_secs(86_400)),
		)
		.layer(axum::Extension(namespace_start_waits(ctx)))
		.layer(axum::Extension(start_waits.clone()))
		.with_state(ctx.clone())
}

/// Refusals that happen before any work and answer 503 at a read-only HTTP boundary. Every
/// other method keeps the 400 they answer with, which released clients already handle.
const READ_RETRYABLE_REFUSALS: [&str; 2] = ["depot.overloaded", "actor.fork_unavailable"];

/// A storage or admission refusal is retryable at a read-only HTTP boundary. A mutation can
/// already have committed before a subsequent read is refused, so keep its existing status.
async fn classify_read_refusal(request: Request, next: Next) -> Response {
	let read_only = matches!(*request.method(), Method::GET | Method::HEAD);
	let mut response = next.run(request).await;
	let retryable_refusal = response
		.headers()
		.get(X_RIVET_ERROR)
		.is_some_and(|value| READ_RETRYABLE_REFUSALS.iter().any(|code| value == *code));
	if read_only && response.status() == StatusCode::BAD_REQUEST && retryable_refusal {
		*response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
	}
	response
}

/// Authenticate and enforce route grants before running its handler.
async fn require_credential(
	State(ctx): State<EngineContext>,
	request: Request,
	next: Next,
) -> Response {
	let token = auth::transport_token(request.headers()).map(str::to_owned);
	let method = request.method().to_string();
	let uri = request.uri().clone();
	let result = async {
		let credential = auth::authenticate(&ctx, token.as_deref()).await?;
		tokens::authorize(&ctx, &credential, &method, &uri).await?;
		anyhow::Ok(())
	}
	.await;
	if let Err(error) = result {
		return auth::http_rejection(&error)
			.map(axum::body::Body::from)
			.into_response();
	}
	next.run(request).await
}

/// A request's failure as an HTTP response. A structured error keeps its group, code and
/// message, with the status the public actor API uses. Anything else is an internal failure
/// whose details stay in the logs.
pub(crate) struct ApiError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for ApiError {
	fn from(error: E) -> Self {
		Self(error.into())
	}
}

/// `error` as `depot.overloaded` when it is an unstructured Depot refusal that ran nothing and
/// carries a retry hint, so the caller's edge retries it after the hint instead of reading an
/// internal error. Its `reason` names the answer, such as `apply_backlog` or `not_maintainer`,
/// not only load. Any other error is returned as it is.
///
/// It applies to every route: Depot refuses some opens for now without a structured error, such
/// as an open of a volume whose lazy fork is still running.
fn unstructured_refusal(error: anyhow::Error) -> anyhow::Error {
	let structured = error
		.chain()
		.any(|cause| cause.downcast_ref::<rivet_error::RivetError>().is_some());
	if structured {
		return error;
	}
	let Some(retry_after_ms) = depot_client::retry_after_ms(&error) else {
		return error;
	};
	tracing::debug!(?error, retry_after_ms, "depot refused a request for now");
	depot_client::Overloaded {
		reason: depot_client::refusal_reason(&error).to_owned(),
		retry_after_ms,
	}
	.build()
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		let error = unstructured_refusal(self.0);
		let structured = error
			.chain()
			.find_map(|cause| cause.downcast_ref::<rivet_error::RivetError>());
		let Some(structured) = structured else {
			tracing::warn!(?error, "actor API request failed");
			let body = serde_json::json!({
				"group": "core",
				"code": "internal_error",
				"message": "An internal error occurred",
			});
			return (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response();
		};
		let status = match (structured.group(), structured.code()) {
			("actor" | "namespace" | "pool", "not_found") => StatusCode::NOT_FOUND,
			(
				"namespace",
				"name_not_unique" | "pending" | "archived" | "failed" | "has_forks" | "has_nodes"
				| "backfill_pending" | "backfill_skipped" | "migrating_from_v2",
			)
			| ("pool", "nodes_connected")
			| ("actor", "duplicate_key" | "fork_failed" | "fork_too_deep") => StatusCode::CONFLICT,
			("auth", "invalid_token" | "token_expired") => StatusCode::UNAUTHORIZED,
			("auth", "insufficient_permissions" | "issuance_disabled")
			| ("namespace", "protected") => StatusCode::FORBIDDEN,
			("auth", "verification_unavailable" | "issuance_unavailable")
			| ("actor", "migrating_from_v2") => StatusCode::SERVICE_UNAVAILABLE,
			// A grant of an operation an older engine worker cannot read yet, refused before
			// anything was written or signed. No release answered it.
			("auth", "operation_unavailable") => StatusCode::SERVICE_UNAVAILABLE,
			// Refused at admission, before any work, for every method. No release answered it,
			// and released clients retry a 5xx but treat this refusal as a 400 as fatal.
			("namespace", "creating") => StatusCode::SERVICE_UNAVAILABLE,
			// Existing SDKs retry all 5xx responses; this create may already have taken effect.
			("actor", "create_pending") => StatusCode::CONFLICT,
			// The fork's deadline passed while its copy goes on.
			("actor", "fork_pending") => StatusCode::GATEWAY_TIMEOUT,
			// Storage, messaging and admission refusals keep the 400 that released clients
			// read; `classify_read_refusal` answers 503 for the ones a read may retry.
			_ => StatusCode::BAD_REQUEST,
		};
		let mut body = serde_json::json!({
			"group": structured.group(),
			"code": structured.code(),
			"message": structured.message(),
		});
		// Metadata carries what a caller acts on, such as the actor a pending fork created, and
		// a retryable refusal's hint.
		let metadata = structured.metadata();
		if let Some(metadata) = &metadata {
			body["metadata"] = metadata.clone();
		}
		let mut response = (status, Json(body)).into_response();
		let error_code = format!("{}.{}", structured.group(), structured.code());
		if let Ok(value) = hyper::header::HeaderValue::from_str(&error_code) {
			response.headers_mut().insert("x-rivet-error", value);
		}
		// A retryable refusal tells the client when to come back. Retry-After counts whole
		// seconds, so the hint rounds up and never reads as now.
		let retry_after_ms = metadata
			.as_ref()
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
