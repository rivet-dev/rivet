use std::{collections::BTreeSet, time::Duration};

use anyhow::Result;
use futures_util::{StreamExt, stream::FuturesUnordered};
use gas::prelude::*;
use rivet_api_types::runner_configs::list as runner_configs_list;
use rivet_api_util::{Method, request_remote_datacenter};

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
	let namespace = ctx
		.op(namespace::ops::get_global::Input {
			namespace_ids: vec![input.namespace_id],
		})
		.await?
		.into_iter()
		.next()
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let mut dc_labels = BTreeSet::new();

	if local_dc_exists(ctx, input).await? {
		dc_labels.insert(ctx.config().dc_label());
	}

	let remote_dc_labels =
		request_remote_dcs_with_runner_config(ctx, &namespace.name, input.runner_name.clone())
			.await?;
	dc_labels.extend(remote_dc_labels);

	Ok(dc_labels.into_iter().collect())
}

async fn local_dc_exists(ctx: &OperationCtx, input: &Input) -> Result<bool> {
	let runner_configs = ctx
		.op(crate::ops::runner_config::get::Input {
			runners: vec![(input.namespace_id, input.runner_name.clone())],
			bypass_cache: false,
		})
		.await?;

	Ok(!runner_configs.is_empty())
}

async fn request_remote_dcs_with_runner_config(
	ctx: &OperationCtx,
	namespace_name: &str,
	runner_name: String,
) -> Result<Vec<u16>> {
	let datacenters = ctx.config().topology().datacenters.clone();
	let namespace_name = namespace_name.to_owned();

	let mut requests = datacenters
		.iter()
		.filter(|dc| dc.datacenter_label != ctx.config().dc_label())
		.map(|dc| {
			let namespace_name = namespace_name.clone();
			let runner_name = runner_name.clone();
			let dc_label = dc.datacenter_label;

			async move {
				let start = std::time::Instant::now();
				let result = tokio::time::timeout(
					REQUEST_TIMEOUT,
					query_remote_dc_has_runner_config(ctx, dc_label, namespace_name, runner_name),
				)
				.await;

				match result {
					Ok(Ok(has_config)) => {
						tracing::debug!(
							?dc_label,
							?has_config,
							duration_ms = %start.elapsed().as_millis(),
							"remote dc runner config query completed"
						);
						if has_config { Some(dc_label) } else { None }
					}
					Ok(Err(err)) => {
						tracing::warn!(
							?err,
							?dc_label,
							duration_ms = %start.elapsed().as_millis(),
							"failed to query enabled dc"
						);
						None
					}
					Err(err) => {
						tracing::warn!(
							?err,
							?dc_label,
							duration_ms = %start.elapsed().as_millis(),
							"timed out querying enabled dc"
						);
						None
					}
				}
			}
		})
		.collect::<FuturesUnordered<_>>();

	let mut dc_labels = BTreeSet::new();
	while let Some(dc_label) = requests.next().await {
		dc_labels.extend(dc_label);
	}

	Ok(dc_labels.into_iter().collect())
}

async fn query_remote_dc_has_runner_config(
	ctx: &OperationCtx,
	dc_label: u16,
	namespace_name: String,
	runner_name: String,
) -> Result<bool> {
	let runner_configs_query = runner_configs_list::ListQuery {
		namespace: namespace_name,
		variant: None,
		runner_names: None,
		runner_name: vec![runner_name],
		limit: None,
		cursor: None,
	};

	let res = request_remote_datacenter::<runner_configs_list::ListResponse>(
		ctx.config(),
		dc_label,
		"/runner-configs",
		Method::GET,
		Some(&runner_configs_query),
		Option::<&()>::None,
	)
	.await;

	// NOTE: Errors are treated as the DC not having a runner config. If a remote DC is
	// temporarily unreachable, it will be excluded from the enabled set. This is intentional
	// for availability since the cache will be invalidated when the config changes.
	match res {
		Ok(res) => Ok(!res.runner_configs.is_empty()),
		Err(err) => {
			tracing::warn!(?err, ?dc_label, "failed to query remote runner configs");
			Ok(false)
		}
	}
}

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
