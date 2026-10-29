use axum::{
	Json, Router,
	extract::{Request, State},
	middleware::{self, Next},
	response::{IntoResponse, Response},
	routing::{delete, get, post, put},
};
use hyper::{
	StatusCode,
	header::{HeaderName, RETRY_AFTER},
};
use rivet_engine_context::EngineContext;
use rivet_guard_core::{X_RIVET_RAY_ID, auth, errors::X_RIVET_ERROR};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

mod routes;
mod tokens;

pub fn router(ctx: &EngineContext) -> Router {
	// Clients can read metadata before authenticating. Other routes enforce scoped grants.
	let protected = Router::new()
		.route("/datacenters", get(routes::list_datacenters))
		.route("/pools", get(routes::list_pools))
		.route("/pools/{pool}/nodes", get(routes::list_pool_nodes))
		.route("/namespaces", get(routes::list_namespaces))
		.route("/namespaces", post(routes::create_namespace))
		.route("/actors", get(routes::list_actors))
		.route("/actors", post(routes::create_actor))
		.route("/actors", put(routes::get_or_create_actor))
		.route("/actors/names", get(routes::list_actor_names))
		.route("/actors/{actor_id}", get(routes::get_actor))
		.route("/actors/{actor_id}", delete(routes::destroy_actor))
		.route("/actors/{actor_id}/sleep", post(routes::sleep_actor))
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
		.with_state(ctx.clone())
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

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		let error = self.0;
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
			("namespace", "name_not_unique")
			| ("pool", "nodes_connected")
			| ("actor", "duplicate_key") => StatusCode::CONFLICT,
			("auth", "invalid_token" | "token_expired") => StatusCode::UNAUTHORIZED,
			("auth", "insufficient_permissions" | "issuance_disabled") => StatusCode::FORBIDDEN,
			("auth", "verification_unavailable" | "issuance_unavailable") => {
				StatusCode::SERVICE_UNAVAILABLE
			}
			_ => StatusCode::BAD_REQUEST,
		};
		let body = serde_json::json!({
			"group": structured.group(),
			"code": structured.code(),
			"message": structured.message(),
		});
		let mut response = (status, Json(body)).into_response();
		if let Ok(reason) = format!("{}.{}", structured.group(), structured.code()).parse() {
			response.headers_mut().insert("x-rivet-error", reason);
		}
		response
	}
}


