use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use universaldb::utils::IsolationLevel::*;

use crate::{errors, keys};

#[derive(Debug)]
pub struct Input {
	pub runners: Vec<(Id, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunnerConfig {
	pub namespace_id: Id,
	pub name: String,
	pub config: rivet_types::namespaces::RunnerConfig,
}

#[operation]
pub async fn namespace_runner_config_get_local(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<RunnerConfig>> {
	if !ctx.config().is_leader() {
		return Err(errors::Namespace::NotLeader.build());
	}

	let runner_configs = ctx
		.udb()?
		.run(|tx| async move {
			futures_util::stream::iter(input.runners.clone())
				.map(|(namespace_id, runner_name)| {
					let tx = tx.clone();

					async move {
						let tx = tx.with_subspace(keys::subspace());

						let runner_config_key =
							keys::RunnerConfigKey::new(namespace_id, runner_name.clone());

						// Runner config not found
						let Some(runner_config) =
							tx.read_opt(&runner_config_key, Serializable).await?
						else {
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
		})
		.custom_instrument(tracing::info_span!("runner_config_get_local_tx"))
		.await?;

	Ok(runner_configs)
}
