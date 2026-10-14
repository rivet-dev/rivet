use super::signing;
use crate::{ctx::ApiCtx, errors::Signing};
use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path},
};
use rivet_api_peer::auth::webhooks::MutationRequest;
use rivet_api_types::auth::webhooks::{KeyStatus, Metadata, RevokeRequest, RotateRequest};
use rivet_auth::{AccessNamespaceScope, OperationKind, ResourceKind, TargetScope};
use rivet_auth_jwt::ops::webhook::Change;
use rivet_util::Id;

#[utoipa::path(post, operation_id = "auth_webhooks_ensure", path = "/auth/webhooks/ensure",
    responses((status = 200, body = Metadata)), security(("bearer_auth" = [])))]
#[tracing::instrument(skip_all)]
pub async fn ensure(Extension(ctx): Extension<ApiCtx>) -> Response {
	let result: Result<Metadata> = async {
		let (namespace_id, _) = signing::credential(&ctx, ResourceKind::WebhookSigning).await?;
		mutate(&ctx, namespace_id, Change::Ensure).await?;
		super::webhook_discovery::get(&ctx, namespace_id)
			.await?
			.metadata
			.ok_or_else(|| Signing::Unavailable.build())
	}
	.await;
	signing::no_store(match result {
		Ok(body) => Json(body).into_response(),
		Err(error) => ApiError::from(error).into_response(),
	})
}

#[utoipa::path(get, operation_id = "auth_webhook_keys_get", path = "/namespaces/{namespace}/auth/webhook-keys",
    params(("namespace" = String, Path, description = "Namespace name")),
    responses((status = 200, body = KeyStatus)), security(("bearer_auth" = [])))]
#[tracing::instrument(skip_all)]
pub async fn get(Extension(ctx): Extension<ApiCtx>, Path((name,)): Path<(String,)>) -> Response {
	respond(
		async {
			let id = authorize(&ctx, name, OperationKind::Read).await?;
			if ctx.config().is_leader() {
				rivet_api_peer::auth::webhooks::get(ctx.into(), (id,), ()).await
			} else {
				rivet_api_util::request_remote_datacenter(
					ctx.config(),
					ctx.config().leader_dc()?.datacenter_label,
					&format!("/auth/webhooks/{id}/status"),
					rivet_api_util::Method::GET,
					Option::<&()>::None,
					Option::<&()>::None,
				)
				.await
			}
		}
		.await,
	)
}

#[utoipa::path(post, operation_id = "auth_webhook_keys_rotate", path = "/namespaces/{namespace}/auth/webhook-keys/rotate",
    params(("namespace" = String, Path, description = "Namespace name")), request_body = RotateRequest,
    responses((status = 200, body = KeyStatus)), security(("bearer_auth" = [])))]
#[tracing::instrument(skip_all)]
pub async fn rotate(
	Extension(ctx): Extension<ApiCtx>,
	Path((name,)): Path<(String,)>,
	Json(body): Json<RotateRequest>,
) -> Response {
	respond(
		async {
			let id = authorize(&ctx, name, OperationKind::Update).await?;
			mutate(
				&ctx,
				id,
				Change::Rotate {
					expected_generation: body.expected_generation,
				},
			)
			.await
		}
		.await,
	)
}

#[utoipa::path(post, operation_id = "auth_webhook_keys_revoke", path = "/namespaces/{namespace}/auth/webhook-keys/revoke",
    params(("namespace" = String, Path, description = "Namespace name")), request_body = RevokeRequest,
    responses((status = 200, body = KeyStatus)), security(("bearer_auth" = [])))]
#[tracing::instrument(skip_all)]
pub async fn revoke(
	Extension(ctx): Extension<ApiCtx>,
	Path((name,)): Path<(String,)>,
	Json(body): Json<RevokeRequest>,
) -> Response {
	respond(
		async {
			let id = authorize(&ctx, name, OperationKind::Delete).await?;
			let request_id = gas::prelude::Uuid::parse_str(&body.request_id)
				.map_err(|_| Signing::InvalidInput.build())?
				.into_bytes();
			mutate(
				&ctx,
				id,
				Change::Revoke {
					request_id,
					expected_generation: body.expected_generation,
					kids: body.kids,
				},
			)
			.await
		}
		.await,
	)
}

async fn authorize(ctx: &ApiCtx, name: String, operation: OperationKind) -> Result<Id> {
	let ns = ctx
		.op(namespace::ops::resolve_for_name_global::Input { name })
		.await?
		.ok_or_else(|| Signing::NotFound.build())?;
	ctx.auth(
		AccessNamespaceScope::Id(ns.namespace_id),
		ResourceKind::WebhookSigning,
		TargetScope::Any,
		operation,
	)
	.await?;
	signing::namespace_exists(ctx, ns.namespace_id).await?;
	Ok(ns.namespace_id)
}

pub(crate) async fn mutate(ctx: &ApiCtx, namespace_id: Id, change: Change) -> Result<KeyStatus> {
	let request = MutationRequest {
		namespace_id,
		change,
	};
	if ctx.config().is_leader() {
		rivet_api_peer::auth::webhooks::mutate(ctx.clone().into(), (), (), request).await
	} else {
		rivet_api_util::request_remote_datacenter(
			ctx.config(),
			ctx.config().leader_dc()?.datacenter_label,
			"/auth/webhooks/mutate",
			rivet_api_util::Method::POST,
			Option::<&()>::None,
			Some(&request),
		)
		.await
	}
}

fn respond(result: Result<KeyStatus>) -> Response {
	signing::no_store(match result {
		Ok(body) => Json(body).into_response(),
		Err(error) => ApiError::from(error).into_response(),
	})
}
