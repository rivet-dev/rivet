use anyhow::Result;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CachePurgeRequest {
	pub base_key: String,
	pub keys: Vec<rivet_cache::RawCacheKey>,
}

#[derive(Serialize)]
pub struct CachePurgeResponse {}

#[tracing::instrument(skip_all)]
pub async fn cache_purge(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	body: CachePurgeRequest,
) -> Result<CachePurgeResponse> {
	ctx.cache()
		.clone()
		.request()
		.purge(&body.base_key, body.keys)
		.await?;

	Ok(CachePurgeResponse {})
}

#[derive(Serialize)]
pub struct BumpServerlessAutoscalerResponse {}

#[tracing::instrument(skip_all)]
pub async fn bump_serverless_autoscaler(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	_body: (),
) -> Result<BumpServerlessAutoscalerResponse> {
	ctx.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
		.send()
		.await?;

	Ok(BumpServerlessAutoscalerResponse {})
}
