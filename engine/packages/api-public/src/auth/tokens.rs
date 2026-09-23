use anyhow::{Context, Result};
use axum::{
	http::{HeaderValue, header::CACHE_CONTROL},
	response::{IntoResponse, Response},
};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json},
};
use rivet_api_types::auth::tokens::{CreateRequest, CreateResponse, InspectResponse, TargetScope};
use rivet_auth::{OwnedGrant, Scope, errors::Auth};
use rivet_auth_jwt::{ValidatedGrantSet, issuer::IssueRequest};

use crate::ctx::ApiCtx;

#[utoipa::path(
	post,
	operation_id = "auth_tokens_create",
	path = "/auth/tokens",
	request_body(content = CreateRequest, content_type = "application/json"),
	responses((status = 200, body = CreateResponse)),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn create(
	Extension(ctx): Extension<ApiCtx>,
	Json(body): Json<CreateRequest>,
) -> Response {
	match create_inner(ctx, body).await {
		Ok(response) => {
			let mut response = Json(response).into_response();
			response
				.headers_mut()
				.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
			response
		}
		Err(error) => {
			rivet_auth_jwt::metrics::record_issuance("rejected");
			ApiError::from(error).into_response()
		}
	}
}

#[tracing::instrument(level = "debug", skip_all)]
async fn create_inner(ctx: ApiCtx, body: CreateRequest) -> Result<CreateResponse> {
	let Some(auth) = &ctx.config().auth else {
		ctx.skip_auth();
		return Err(Auth::IssuanceDisabled.build());
	};
	let jwt = &auth.jwt;
	if !jwt.issuance_enabled() {
		ctx.skip_auth();
		return Err(Auth::IssuanceDisabled.build());
	}

	ctx.authenticate().await?.require_admin_token()?;
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: body.namespace,
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;
	let namespace_id = namespace.namespace_id;
	let grants = ValidatedGrantSet::new(
		namespace_id,
		body.grants.into_iter().map(|grant| OwnedGrant {
			namespace: Scope::Id(namespace_id),
			resource: grant.resource.into(),
			target: match grant.target {
				TargetScope::Any => Scope::Any,
				TargetScope::Id(id) => Scope::Id(id),
			},
			operations: grant.operations,
		}),
	)
	.map_err(|_| invalid_input("invalid JWT grant set"))?;
	if body
		.subject
		.as_ref()
		.is_some_and(|subject| subject.len() > rivet_auth_jwt::MAX_SUBJECT_BYTES)
	{
		return Err(invalid_input(format!(
			"subject exceeds {} UTF-8 bytes",
			rivet_auth_jwt::MAX_SUBJECT_BYTES
		)));
	}
	let duration = body
		.duration
		.unwrap_or_else(|| jwt.default_duration().as_secs());
	if duration == 0 || duration > jwt.max_duration().as_secs() {
		return Err(invalid_input(format!(
			"duration must be between 1 and {} seconds",
			jwt.max_duration().as_secs()
		)));
	}
	let now = rivet_util::timestamp::now();
	let expires_no_later_than_ts = bound_expiration(now, duration)?;

	let request = IssueRequest {
		namespace_id,
		grants: grants.into_grants(),
		expires_no_later_than_ts,
		subject: body.subject,
		issuer_token_id: rivet_auth_jwt::Id::nil(),
	};
	let issued = ctx
		.op(rivet_auth_jwt::ops::issue::Input {
			request,
			key_ring_cache: ctx.jwt_key_ring_cache()?,
		})
		.await
		.map_err(|error| {
			tracing::warn!(?error, "JWT signing request failed");
			Auth::IssuanceUnavailable.build()
		})?;

	Ok(CreateResponse {
		token: issued.token,
		issued_ts: issued.issued_ts,
		expires_ts: issued.expires_ts,
	})
}

#[utoipa::path(
	get,
	operation_id = "auth_tokens_inspect",
	path = "/auth/tokens/inspect",
	responses((status = 200, body = InspectResponse)),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn inspect(Extension(ctx): Extension<ApiCtx>) -> Response {
	match inspect_inner(ctx).await {
		Ok(response) => {
			let mut response = Json(response).into_response();
			response
				.headers_mut()
				.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
			response
		}
		Err(error) => ApiError::from(error).into_response(),
	}
}

#[tracing::instrument(level = "debug", skip_all)]
async fn inspect_inner(ctx: ApiCtx) -> Result<InspectResponse> {
	let credential = ctx.authenticate().await?;
	inspect_response(credential.require_jwt()?.verified())
}

fn inspect_response(jwt: &rivet_auth_jwt::VerifiedJwt) -> Result<InspectResponse> {
	let issued_ts = seconds_to_milliseconds(jwt.token.claims.iat)?;
	let expires_ts = seconds_to_milliseconds(jwt.token.claims.exp)?;
	let grants = jwt
		.token
		.grants
		.iter()
		.map(|grant| {
			Ok(rivet_api_types::auth::tokens::Grant {
				resource: grant
					.resource
					.try_into()
					.map_err(|_| Auth::InvalidToken.build())?,
				target: match grant.target {
					Scope::Any => TargetScope::Any,
					Scope::Id(id) => TargetScope::Id(id),
				},
				operations: grant.operations.clone(),
			})
		})
		.collect::<Result<Vec<_>>>()?;

	Ok(InspectResponse {
		namespace_id: jwt.token.namespace_id,
		subject: jwt.token.claims.sub.clone(),
		grants,
		issued_ts,
		expires_ts,
	})
}

fn seconds_to_milliseconds(seconds: u64) -> Result<i64> {
	i64::try_from(seconds)
		.context("JWT timestamp exceeds the API timestamp range")?
		.checked_mul(1_000)
		.context("JWT timestamp exceeds the API timestamp range")
}

fn bound_expiration(now: i64, duration_seconds: u64) -> Result<i64> {
	let requested_expires_ts = now
		.checked_add(
			i64::try_from(duration_seconds)
				.context("duration does not fit in milliseconds")?
				.checked_mul(1_000)
				.context("duration overflow")?,
		)
		.context("expiration overflow")?;
	if requested_expires_ts.div_euclid(1_000) <= now.div_euclid(1_000) {
		return Err(Auth::TokenExpired.build());
	}
	Ok(requested_expires_ts)
}

fn invalid_input(message: impl Into<String>) -> anyhow::Error {
	crate::errors::Validation::InvalidInput {
		message: message.into(),
	}
	.build()
}

#[cfg(test)]
#[path = "../../tests/unit/auth_tokens.rs"]
mod tests;
