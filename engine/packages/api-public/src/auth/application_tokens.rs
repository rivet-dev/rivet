use super::signing;
use crate::{ctx::ApiCtx, errors::Signing};
use anyhow::Result;
use axum::{
	extract::Request,
	response::{IntoResponse, Response},
};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json},
};
use rivet_api_types::auth::application_tokens::{CreateRequest, CreateResponse};
use rivet_auth::ResourceKind;
use rivet_auth_jwt::{TokenError, application};

#[utoipa::path(
    post, operation_id = "auth_application_tokens_create", path = "/auth/application-tokens",
    request_body(content = CreateRequest, content_type = "application/json"),
    responses((status = 200, body = CreateResponse)), security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn create(Extension(ctx): Extension<ApiCtx>, request: Request) -> Response {
	signing::no_store(match create_inner(&ctx, request).await {
		Ok(response) => Json(response).into_response(),
		Err(error) => ApiError::from(error).into_response(),
	})
}

async fn create_inner(ctx: &ApiCtx, request: Request) -> Result<CreateResponse> {
	let (namespace_id, _) = signing::credential(ctx, ResourceKind::ApplicationToken).await?;
	if request
		.headers()
		.get(axum::http::header::CONTENT_TYPE)
		.and_then(|h| h.to_str().ok())
		.and_then(|v| v.split(';').next())
		.map(str::trim)
		!= Some("application/json")
		|| request
			.headers()
			.contains_key(axum::http::header::CONTENT_ENCODING)
	{
		return Err(Signing::InvalidInput.build());
	}
	let bytes = tokio::time::timeout(
		std::time::Duration::from_secs(30),
		axum::body::to_bytes(request.into_body(), 16 * 1024),
	)
	.await
	.map_err(|_| Signing::Unavailable.build())?
	.map_err(|_| Signing::PayloadTooLarge.build())?;
	let body: CreateRequest = super::strict_json::parse(&bytes)?;
	let jwt = &ctx.config().auth_required()?.jwt;
	application::validate_application_data(
		&body.audience,
		&body.purpose,
		body.subject.as_deref(),
		&body.claims,
		jwt.audience(),
	)
	.map_err(|_| Signing::InvalidInput.build())?;
	let duration = body.duration.unwrap_or(jwt.default_duration().as_secs());
	if duration == 0 || duration > jwt.max_duration().as_secs() {
		return Err(Signing::InvalidInput.build());
	}
	let (current_namespace, deadline) =
		signing::credential(ctx, ResourceKind::ApplicationToken).await?;
	if current_namespace != namespace_id {
		return Err(rivet_auth::errors::Auth::InsufficientPermissions.build());
	}
	let expires_no_later_than_ts =
		signing::expiration(rivet_util::timestamp::now(), duration, deadline)?;
	let issued = ctx
		.jwt_key_ring_cache()?
		.issue_application(
			&application::service::IssueRequest {
				namespace_id,
				audience: body.audience,
				purpose: body.purpose,
				subject: body.subject,
				app: body.claims,
				expires_no_later_than_ts,
			},
			jwt.max_duration().as_secs(),
		)
		.await
		.map_err(|err| {
			if err.downcast_ref::<TokenError>() == Some(&TokenError::TokenTooLarge) {
				Signing::PayloadTooLarge.build()
			} else {
				Signing::Unavailable.build()
			}
		})?;
	Ok(CreateResponse {
		token: issued.token,
		issued_ts: issued.issued_ts,
		expires_ts: issued.expires_ts,
	})
}
