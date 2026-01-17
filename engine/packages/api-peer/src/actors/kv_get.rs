use anyhow::*;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use gas::prelude::*;
use pegboard_actor_kv as actor_kv;
use rivet_api_builder::ApiCtx;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KvGetPath {
	pub actor_id: Id,
	pub key: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KvGetQuery {
	pub namespace: String,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsKvGetResponse)]
pub struct KvGetResponse {
	/// Value encoded in base 64.
	pub value: String,
	pub update_ts: i64,
}

#[utoipa::path(
	get,
	operation_id = "actors_kv_get",
	path = "/actors/{actor_id}/kv/keys/{key}",
	params(
		("actor_id" = Id, Path),
		("key" = String, Path),
	),
	responses(
		(status = 200, body = KvGetResponse),
	),
)]
#[tracing::instrument(skip_all)]
pub async fn kv_get(ctx: ApiCtx, path: KvGetPath, query: KvGetQuery) -> Result<KvGetResponse> {
	// Get the actor first to verify it exists
	let actors_res = ctx
		.op(pegboard::ops::actor::get::Input {
			actor_ids: vec![path.actor_id],
			fetch_error: false,
		})
		.await?;

	let actor = actors_res
		.actors
		.into_iter()
		.next()
		.ok_or_else(|| pegboard::errors::Actor::NotFound.build())?;

	// Verify the actor belongs to the specified namespace
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace,
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	if actor.namespace_id != namespace.namespace_id {
		return Err(pegboard::errors::Actor::NotFound.build());
	}

	// Decode base64 key
	let key_bytes = BASE64_STANDARD
		.decode(&path.key)
		.context("failed to decode base64 key")?;

	// Get the KV value
	let udb = ctx.pools().udb()?;
	let (keys, values, metadata) =
		actor_kv::get(&*udb, path.actor_id, vec![key_bytes.clone()]).await?;

	// Check if key was found
	if keys.is_empty() {
		return Err(pegboard::errors::Actor::KvKeyNotFound.build());
	}

	// Encode value as base64
	let value_base64 = BASE64_STANDARD.encode(&values[0]);

	Ok(KvGetResponse {
		value: value_base64,
		update_ts: metadata[0].update_ts,
	})
}
