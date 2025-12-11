use anyhow::*;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use pegboard_actor_kv as actor_kv;
use rivet_api_builder::ApiCtx;
use rivet_runner_protocol::mk2 as rp;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KvListPath {
	pub actor_id: Id,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KvListQuery {
	/// Base64 encoded key prefix to filter by
	pub prefix: Option<String>,
	/// Number of results to return (default 100, max 1000)
	pub limit: Option<usize>,
	/// Whether to reverse the order
	pub reverse: Option<bool>,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsKvListResponse)]
pub struct KvListResponse {
	pub entries: Vec<KvEntry>,
}

#[derive(Serialize, ToSchema)]
pub struct KvEntry {
	/// Key encoded in base64
	pub key: String,
	/// Value encoded in base64
	pub value: String,
	pub update_ts: i64,
}

#[utoipa::path(
	get,
	operation_id = "actors_kv_list",
	path = "/actors/{actor_id}/kv/keys",
	params(
		("actor_id" = Id, Path),
		("prefix" = Option<String>, Query, description = "Base64 encoded key prefix to filter by"),
		("limit" = Option<usize>, Query, description = "Number of results to return (default 100, max 1000)"),
		("reverse" = Option<bool>, Query, description = "Whether to reverse the order"),
	),
	responses(
		(status = 200, body = KvListResponse),
	),
)]
#[tracing::instrument(skip_all)]
pub async fn kv_list(ctx: ApiCtx, path: KvListPath, query: KvListQuery) -> Result<KvListResponse> {
	// Parse query parameters
	let limit = query.limit.unwrap_or(100).min(1000);
	let reverse = query.reverse.unwrap_or(false);

	// Build list query
	let list_query = if let Some(prefix) = query.prefix {
		let prefix_bytes = BASE64_STANDARD
			.decode(&prefix)
			.context("failed to decode base64 prefix")?;
		rp::KvListQuery::KvListPrefixQuery(rp::KvListPrefixQuery {
			key: prefix_bytes,
		})
	} else {
		rp::KvListQuery::KvListAllQuery
	};

	// Get the KV entries
	let udb = ctx.pools().udb()?;
	let (keys, values, metadata) =
		actor_kv::list(&*udb, path.actor_id, list_query, reverse, Some(limit)).await?;

	// Build response
	let entries = keys
		.into_iter()
		.zip(values.into_iter())
		.zip(metadata.into_iter())
		.map(|((key, value), meta)| KvEntry {
			key: BASE64_STANDARD.encode(&key),
			value: BASE64_STANDARD.encode(&value),
			update_ts: meta.update_ts,
		})
		.collect();

	Ok(KvListResponse { entries })
}
