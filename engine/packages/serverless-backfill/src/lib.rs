use anyhow::Result;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::from_pools(pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"serverless_backfill",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	let serverless_data = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());

			let serverless_desired_subspace = pegboard::keys::subspace().subspace(
				&rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey::entire_subspace(),
			);

			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&serverless_desired_subspace).into()
				},
				// NOTE: This is a snapshot to prevent conflict with updates to this subspace
				Snapshot,
			)
			.map(|res| {
				tx.unpack::<rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey>(res?.key())
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("read_serverless_tx"))
		.await?;

	if serverless_data.is_empty() {
		return Ok(());
	}

	tracing::info!("backfilling serverless");

	let runner_configs = ctx
		.op(pegboard::ops::runner_config::get::Input {
			runners: serverless_data
				.iter()
				.map(|key| (key.namespace_id, key.runner_name.clone()))
				.collect(),
			bypass_cache: true,
		})
		.await?;

	for key in &serverless_data {
		if !runner_configs
			.iter()
			.any(|rc| rc.namespace_id == key.namespace_id)
		{
			tracing::debug!(
				namespace_id=?key.namespace_id,
				runner_name=?key.runner_name,
				"runner config not found, likely deleted"
			);
			continue;
		};

		ctx.workflow(pegboard::workflows::runner_pool::Input {
			namespace_id: key.namespace_id,
			runner_name: key.runner_name.clone(),
		})
		.tag("namespace_id", key.namespace_id)
		.tag("runner_name", key.runner_name.clone())
		.unique()
		.dispatch()
		.await?;
	}

	Ok(())
}
