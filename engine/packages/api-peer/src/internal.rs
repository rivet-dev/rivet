use anyhow::*;
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
#[serde(deny_unknown_fields)]
pub struct CachePurgeResponse {}

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

#[derive(Serialize, Deserialize)]
pub struct BumpServerlessAutoscalerRequest {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub struct BumpServerlessAutoscalerResponse {}

pub async fn bump_serverless_autoscaler(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	body: BumpServerlessAutoscalerRequest,
) -> Result<BumpServerlessAutoscalerResponse> {
	ctx.signal(pegboard::workflows::serverless::pool::BumpConfig {})
		.to_workflow::<pegboard::workflows::serverless::pool::Workflow>()
		.tag("runner_name", body.runner_name)
		.tag("namespace_id", body.namespace_id)
		.send()
		.await?;

	Ok(BumpServerlessAutoscalerResponse {})
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetTracingConfigRequest {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub filter: Option<Option<String>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub sampler_ratio: Option<Option<f64>>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplicaReconfigureRequest {}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReplicaReconfigureResponse {}

/// Triggers the epoxy coordinator to reconfigure all replicas.
///
/// Useful when a replica's configuration is outdated for any reason and needs to be re-notified of
/// changes.
///
/// This should never need to be called manually if everything is operating correctly.
pub async fn epoxy_replica_reconfigure(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	_body: ReplicaReconfigureRequest,
) -> Result<ReplicaReconfigureResponse> {
	if ctx.config().is_leader() {
		ctx.signal(epoxy::workflows::coordinator::ReplicaReconfigure {})
			.to_workflow::<epoxy::workflows::coordinator::Workflow>()
			.tag("replica", ctx.config().epoxy_replica_id())
			.send()
			.await?;
	}

	Ok(ReplicaReconfigureResponse {})
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub struct GetEpoxyStateResponse {
	pub config: epoxy::types::ClusterConfig,
}

/// Returns the current epoxy coordinator cluster configuration.
///
/// Useful for inspecting the current state of the epoxy cluster, including all replicas and their statuses.
pub async fn get_epoxy_state(ctx: ApiCtx, _path: (), _query: ()) -> Result<GetEpoxyStateResponse> {
	let workflow_id = ctx
		.find_workflow::<epoxy::workflows::coordinator::Workflow>((
			"replica",
			ctx.config().epoxy_replica_id(),
		))
		.await?
		.ok_or_else(|| anyhow!("epoxy coordinator workflow not found"))?;

	let wfs = ctx.get_workflows(vec![workflow_id]).await?;
	let wf = wfs.first().ok_or_else(|| anyhow!("workflow not found"))?;

	let state: epoxy::workflows::coordinator::State =
		wf.parse_state().context("failed to parse workflow state")?;

	Ok(GetEpoxyStateResponse {
		config: state.config,
	})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetEpoxyStateRequest {
	pub config: epoxy::types::ClusterConfig,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub struct SetEpoxyStateResponse {}

/// Overrides the epoxy coordinator cluster configuration and triggers reconfiguration.
///
/// Useful for manually adjusting the cluster state in case the replica status drifts from the
/// state in the coordinator. This will automatically trigger a reconfigure when called.
///
/// This should never need to be called manually if everything is operating correctly.
pub async fn set_epoxy_state(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	body: SetEpoxyStateRequest,
) -> Result<SetEpoxyStateResponse> {
	ensure!(
		body.config.coordinator_replica_id == ctx.config().epoxy_replica_id(),
		"config coordinator_replica_id ({}) does not match current replica id ({})",
		body.config.coordinator_replica_id,
		ctx.config().epoxy_replica_id()
	);

	if ctx.config().is_leader() {
		ctx.signal(epoxy::workflows::coordinator::OverrideState {
			config: body.config,
		})
		.to_workflow::<epoxy::workflows::coordinator::Workflow>()
		.tag("replica", ctx.config().epoxy_replica_id())
		.send()
		.await?;
	}

	Ok(SetEpoxyStateResponse {})
}
