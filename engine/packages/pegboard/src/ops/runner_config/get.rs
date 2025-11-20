use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub runners: Vec<(Id, String)>,
	pub bypass_cache: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunnerConfig {
	pub namespace_id: Id,
	pub name: String,
	pub config: rivet_types::runner_configs::RunnerConfig,
}

#[operation]
pub async fn pegboard_runner_config_get(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<RunnerConfig>> {
	if input.bypass_cache {
		runner_config_get_inner(ctx, input.runners.clone()).await
	} else {
		ctx.cache()
			.clone()
			.request()
			// Short TTL for faster updates
			.ttl(5000)
			.fetch_all_json("namespace.runner_config.get", input.runners.clone(), {
				|mut cache, runners| async move {
					let runner_configs = runner_config_get_inner(ctx, runners).await?;

					for runner_config in runner_configs {
						cache.resolve(
							&(runner_config.namespace_id, runner_config.name.clone()),
							runner_config,
						);
					}

					Ok(cache)
				}
			})
			.await
	}
}

async fn runner_config_get_inner(
	ctx: &OperationCtx,
	runners: Vec<(Id, String)>,
) -> Result<Vec<RunnerConfig>> {
	ctx.udb()?
		.run(|tx| {
			let runners = runners.clone();
			async move {
				futures_util::stream::iter(runners)
					.map(|(namespace_id, runner_name)| {
						let tx = tx.clone();

						async move {
							let tx = tx.with_subspace(namespace::keys::subspace());

							let runner_config_key = keys::runner_config::DataKey::new(
								namespace_id,
								runner_name.clone(),
							);

							let Some(runner_config) =
								tx.read_opt(&runner_config_key, Serializable).await?
							else {
								// Runner config not found
								return Ok(None);
							};

							Ok(Some(RunnerConfig {
								namespace_id,
								name: runner_name,
								config: runner_config,
							}))
						}
					})
					.buffer_unordered(1024)
					.try_filter_map(|x| std::future::ready(Ok(x)))
					.try_collect::<Vec<_>>()
					.await
			}
		})
		.custom_instrument(tracing::info_span!("runner_config_get_tx"))
		.await
}
