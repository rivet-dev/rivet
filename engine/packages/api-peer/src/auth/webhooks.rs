//! Internal leader-local lifecycle API. Public handlers authenticate and authorize first.
use anyhow::Result;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use rivet_api_types::auth::webhooks::KeyStatus;
use rivet_auth_jwt::{
	ops::webhook::{self, Change},
	webhook::{Error, Ring},
};
use rivet_auth_policy::errors::Signing;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationRequest {
	pub namespace_id: Id,
	pub change: Change,
}

#[tracing::instrument(skip_all)]
pub async fn mutate(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	request: MutationRequest,
) -> Result<KeyStatus> {
	if matches!(request.change, Change::Ensure) {
		webhook::ensure_workflow(&ctx, request.namespace_id)
			.await
			.map_err(map_error)?;
	}
	let ring = ctx
		.op(webhook::mutate::Input {
			namespace_id: request.namespace_id,
			change: request.change,
		})
		.await
		.map_err(map_error)?;
	Ok(status(&ring))
}

#[tracing::instrument(skip_all)]
pub async fn get(ctx: ApiCtx, (namespace_id,): (Id,), _query: ()) -> Result<KeyStatus> {
	let snapshot = ctx
		.op(webhook::get::Input { namespace_id })
		.await
		.map_err(map_error)?
		.ok_or_else(|| Signing::NotFound.build())?;
	Ok(status(&snapshot.ring))
}

pub fn status(ring: &Ring) -> KeyStatus {
	KeyStatus {
		generation: ring.keys.generation,
		active_kid: rivet_auth_jwt::webhook::public_kid(&ring.keys.active.key.public_key),
		pending_kid: ring
			.keys
			.pending
			.as_ref()
			.map(|key| rivet_auth_jwt::webhook::public_kid(&key.key.public_key)),
		last_emergency_request_id: ring
			.last_emergency
			.as_ref()
			.map(|r| Uuid::from_bytes(r.request_id).to_string()),
		last_emergency_generation: ring.last_emergency.as_ref().map(|r| r.committed_generation),
	}
}

pub fn map_error(error: anyhow::Error) -> anyhow::Error {
	match error.downcast_ref::<Error>() {
		Some(Error::InvalidInput) => Signing::InvalidInput.build(),
		Some(Error::NotFound) => Signing::NotFound.build(),
		Some(Error::Conflict) => Signing::Conflict.build(),
		_ => Signing::Unavailable.build(),
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Discovery {
	pub jwks: rivet_api_types::auth::webhooks::Jwks,
	pub metadata: Option<rivet_api_types::auth::webhooks::Metadata>,
}

#[tracing::instrument(skip_all)]
pub async fn discovery(ctx: ApiCtx, (namespace_id,): (Id,), _query: ()) -> Result<Discovery> {
	if !ctx
		.op(namespace::ops::exists_authoritative::Input { namespace_id })
		.await?
	{
		return Err(Signing::NotFound.build());
	}
	let snapshot = ctx
		.op(webhook::get::Input { namespace_id })
		.await
		.map_err(map_error)?
		.ok_or_else(|| Signing::NotFound.build())?;
	let ring = &snapshot.ring;
	let now = rivet_util::timestamp::now();
	let current = now >= ring.keys.active.activated_ts
		&& now < ring.keys.active.sign_until_ts
		&& ring.keys.leader_datacenter_id == ctx.config().dc_label()
		&& ring.keys.leader_config_generation
			== ctx.config().auth_required()?.jwt.issuer_generation();
	let base = &ctx.config().leader_dc()?.public_url;
	let metadata = if current {
		Some(rivet_api_types::auth::webhooks::Metadata {
			namespace_id,
			alg: "ed25519".into(),
			kid: rivet_auth_jwt::webhook::public_kid(&ring.keys.active.key.public_key),
			jwks_url: base
				.join(&format!(
					"namespaces/{namespace_id}/auth/webhooks/jwks.json"
				))?
				.to_string(),
			generation: ring.keys.generation,
		})
	} else {
		None
	};
	let keys = ring
		.public_keys(now)
		.into_iter()
		.map(|key| rivet_api_types::auth::webhooks::PublicKey {
			kty: key.kty,
			crv: key.crv,
			kid: key.kid,
			public_key_use: key.public_key_use,
			alg: key.alg,
			x: key.x,
		})
		.collect();
	if snapshot.read_started.elapsed() >= rivet_auth_jwt::SIGNING_CACHE_LEASE {
		return Err(Signing::Unavailable.build());
	}
	Ok(Discovery {
		jwks: rivet_api_types::auth::webhooks::Jwks { keys },
		metadata,
	})
}
