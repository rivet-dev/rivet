use anyhow::*;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
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
pub struct KvGetQuery {}

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
pub async fn kv_get(ctx: ApiCtx, path: KvGetPath, _query: KvGetQuery) -> Result<KvGetResponse> {
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
		update_ts: metadata[0].create_ts,
	})
}
