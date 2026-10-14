use super::{signing, webhook_keys};
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
use rivet_api_peer::auth::webhook_signing::{self as peer, DEADLINE_HEADER};
use rivet_api_types::auth::webhooks::SignResponse;
use rivet_auth::ResourceKind;

#[utoipa::path(post, operation_id = "auth_webhooks_sign", path = "/auth/webhooks/sign",
    request_body(content = Vec<u8>, content_type = "application/octet-stream"),
    responses((status = 200, body = SignResponse)), security(("bearer_auth" = [])))]
#[tracing::instrument(skip_all)]
pub async fn sign(Extension(ctx): Extension<ApiCtx>, request: Request) -> Response {
	signing::no_store(match sign_inner(&ctx, request).await {
		Ok(body) => Json(body).into_response(),
		Err(error) => ApiError::from(error).into_response(),
	})
}

async fn sign_inner(ctx: &ApiCtx, request: Request) -> Result<SignResponse> {
	let (namespace_id, _) = signing::credential(ctx, ResourceKind::WebhookSigning).await?;
	// Provisions lazily and repairs workflow dispatch after an interrupted ensure.
	webhook_keys::mutate(
		ctx,
		namespace_id,
		rivet_auth_jwt::ops::webhook::Change::Ensure,
	)
	.await?;
	let (body, permit) = peer::read_body(request).await?;
	let check_started = rivet_util::timestamp::now();
	let (current_namespace, expires_ts) =
		signing::credential(ctx, ResourceKind::WebhookSigning).await?;
	if current_namespace != namespace_id {
		return Err(rivet_auth::errors::Auth::InsufficientPermissions.build());
	}
	let deadline = expires_ts.min(check_started.saturating_add(5_000));
	if ctx.config().is_leader() {
		return peer::sign_body(ctx, namespace_id, deadline, body, permit).await;
	}
	let _permit = permit;
	let url = ctx
		.config()
		.leader_dc()?
		.peer_url
		.join(&format!("/auth/webhooks/{namespace_id}/sign"))?;
	let response = rivet_pools::reqwest::client()
		.await?
		.post(url)
		.header("content-type", "application/octet-stream")
		.header(DEADLINE_HEADER, deadline.to_string())
		.timeout(std::time::Duration::from_secs(5))
		.body(body)
		.send()
		.await
		.map_err(|_| Signing::Unavailable.build())?;
	let result: SignResponse = rivet_api_util::parse_response(response).await?;
	if rivet_util::timestamp::now() >= deadline {
		return Err(Signing::Unavailable.build());
	}
	Ok(result)
}
