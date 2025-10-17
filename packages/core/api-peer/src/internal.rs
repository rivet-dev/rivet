use anyhow::Result;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use serde::{Deserialize, Serialize};
use universalpubsub::PublishOpts;

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

#[derive(Serialize, Deserialize)]
pub struct SetTracingConfigRequest {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub filter: Option<Option<String>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub sampler_ratio: Option<Option<f64>>,
}

#[derive(Serialize)]
pub struct SetTracingConfigResponse {}

#[tracing::instrument(skip_all)]
pub async fn set_tracing_config(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	body: SetTracingConfigRequest,
) -> Result<SetTracingConfigResponse> {
	// Broadcast message to all services via UPS
	let subject = "rivet.debug.tracing.config";
	let message = serde_json::to_vec(&body)?;

	ctx.ups()?
		.publish(subject, &message, PublishOpts::broadcast())
		.await?;

	tracing::info!(
		filter = ?body.filter,
		sampler_ratio = ?body.sampler_ratio,
		"broadcasted tracing config update"
	);

	Ok(SetTracingConfigResponse {})
}
