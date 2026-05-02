use anyhow::Result;
use epoxy_protocol::protocol::CachingBehavior;
use futures_util::StreamExt;
use gas::prelude::*;

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	/// In order of ascending ping to current dc.
	pub dc_labels: Vec<u16>,
}

// This lists datacenters where a runner config exists for the given runner name.
// Runners auto-create runner configs, so moving a runner to a different datacenter can leave the
// old datacenter config behind. That stale config must be removed manually or it will still be
// returned here.
#[operation]
pub async fn list_runner_config_enabled_dcs(ctx: &OperationCtx, input: &Input) -> Result<Output> {
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

	Ok(Output { dc_labels })
}

async fn list_runner_config_enabled_dcs_inner(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<u16>> {
	let (dcs_by_ping_res, enabled_dcs) = tokio::join!(
		ctx.op(datacenter::ops::list_by_ping::Input {}),
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
					.await;

				match res {
					Ok(res) => res.value.map(|_| dc.datacenter_label),
					Err(err) => {
						tracing::warn!(
							?err,
							namespace_id=?input.namespace_id,
							runner_name=%input.runner_name,
							dc_label=dc.datacenter_label,
							"failed to read runner config from dc"
						);
						None
					}
				}
			})
			.buffer_unordered(512)
			.filter_map(std::future::ready)
			.collect::<Vec<_>>()
	);

	// Use the filtered dcs list to make the enabled dcs list filtered
	Ok(dcs_by_ping_res?
		.datacenters
		.into_iter()
		.filter_map(|dc| enabled_dcs.iter().find(|edc| &&dc.dc_label == edc).copied())
		.collect())
}
