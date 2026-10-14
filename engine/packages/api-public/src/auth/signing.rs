use crate::{ctx::ApiCtx, errors::Signing};
use anyhow::Result;
use rivet_auth::{AccessNamespaceScope, OperationKind, ResourceKind, TargetScope};
use rivet_util::Id;

/// Signing is available only to namespace-scoped service JWTs with an explicit grant.
pub async fn credential(ctx: &ApiCtx, resource: ResourceKind) -> Result<(Id, i64)> {
	if !ctx.config().auth_required()?.jwt.issuance_enabled() {
		ctx.skip_auth();
		return Err(Signing::Unavailable.build());
	}
	let authenticated = ctx.authenticate().await?;
	let jwt = authenticated.require_jwt()?;
	let namespace_id = jwt.verified().token.namespace_id;
	let expires_ts = i64::try_from(jwt.verified().token.claims.exp)
		.ok()
		.and_then(|seconds| seconds.checked_mul(1000))
		.ok_or_else(|| Signing::InvalidInput.build())?;
	if expires_ts <= rivet_util::timestamp::now() {
		return Err(rivet_auth::errors::Auth::TokenExpired.build());
	}
	ctx.auth(
		AccessNamespaceScope::Id(namespace_id),
		resource,
		TargetScope::Any,
		OperationKind::Create,
	)
	.await?;
	namespace_exists(ctx, namespace_id).await?;
	Ok((namespace_id, expires_ts))
}

pub async fn namespace_exists(ctx: &ApiCtx, namespace_id: Id) -> Result<()> {
	let exists = if ctx.config().is_leader() {
		ctx.op(namespace::ops::exists_authoritative::Input { namespace_id })
			.await?
	} else {
		rivet_api_util::request_remote_datacenter::<bool>(
			ctx.config(),
			ctx.config().leader_dc()?.datacenter_label,
			&format!("/namespaces/{namespace_id}/exists-authoritative"),
			rivet_api_util::Method::GET,
			Option::<&()>::None,
			Option::<&()>::None,
		)
		.await?
	};
	if !exists {
		return Err(rivet_auth::errors::Auth::InsufficientPermissions.build());
	}
	Ok(())
}

pub fn no_store(mut response: axum::response::Response) -> axum::response::Response {
	response.headers_mut().insert(
		axum::http::header::CACHE_CONTROL,
		axum::http::HeaderValue::from_static("no-store"),
	);
	response
}

pub fn expiration(now: i64, seconds: u64, parent: i64) -> Result<i64> {
	let exp = now
		.checked_add(
			i64::try_from(seconds)
				.ok()
				.and_then(|s| s.checked_mul(1000))
				.ok_or_else(|| Signing::InvalidInput.build())?,
		)
		.ok_or_else(|| Signing::InvalidInput.build())?
		.min(parent);
	if exp / 1000 <= now / 1000 {
		return Err(rivet_auth::errors::Auth::TokenExpired.build());
	}
	Ok(exp)
}
