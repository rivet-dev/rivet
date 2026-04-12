use anyhow::Result;
use epoxy_protocol::generated::v2::CachingBehavior;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub dc_labels: Vec<u16>,
}

// This lists datacenters where a runner config exists for the given runner name.
// Runners auto-create runner configs, so moving a runner to a different datacenter can leave the
// old datacenter config behind. That stale config must be removed manually or it will still be
// returned here.
#[operation]
pub async fn list_runner_config_enabled_dcs(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let start = std::time::Instant::now();
	let dc_labels = ctx
		.cache()
		.clone()
		.request()
		.ttl(3_600_000)
		.fetch_one_json(
			"runner.list_runner_config_enabled_dcs",
			(input.namespace_id, input.runner_name.clone()),
			move |mut cache, key| async move {
				let inner_start = std::time::Instant::now();
				let dc_labels = list_runner_config_enabled_dcs_inner(ctx, input).await?;
				tracing::debug!(
					duration_ms = %inner_start.elapsed().as_millis(),
					?dc_labels,
					"list_runner_config_enabled_dcs cache miss"
				);
				cache.resolve(&key, dc_labels.clone());
				Ok(cache)
			},
		)
		.await?;

	let dc_labels = dc_labels.unwrap_or_default();
	tracing::debug!(
		duration_ms = %start.elapsed().as_millis(),
		?dc_labels,
		"list_runner_config_enabled_dcs completed"
	);

	Ok(Output { dc_labels })
}

async fn list_runner_config_enabled_dcs_inner(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<u16>> {
	futures_util::stream::iter(ctx.config().topology().datacenters.clone())
		.map(|dc| async move {
			let runner_config_key = keys::runner_config::GlobalDataKey::new(
				dc.datacenter_label,
				input.namespace_id,
				input.runner_name.clone(),
			);
			let res = ctx
				.op(epoxy::ops::kv::get_optimistic::Input {
					replica_id: ctx.config().epoxy_replica_id(),
					key: namespace::keys::subspace().pack(&runner_config_key),
					caching_behavior: CachingBehavior::Optimistic,
					target_replicas: None,
					save_empty: true,
				})
				.await?;

			Ok(res.value.map(|_| dc.datacenter_label))
		})
		.buffer_unordered(512)
		.try_filter_map(|x| std::future::ready(Ok(x)))
		.try_collect::<Vec<_>>()
		.await
}
