use std::time::{Duration, Instant};

use anyhow::Result;
use futures_util::TryStreamExt;
use gas::prelude::*;
use universaldb::{options::StreamingMode, utils::IsolationLevel::*};

const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::new(config.clone(), pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"metrics_aggregator",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	let mut interval = tokio::time::interval(Duration::from_secs(15));
	interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

	loop {
		interval.tick().await;
		tick(&ctx).await?;
	}
}

async fn tick(ctx: &StandaloneCtx) -> Result<()> {
	tokio::try_join!(
		aggregate_pending_actors(ctx),
		// aggregate_actor_active(ctx),
		aggregate_serverless_desired_slots(ctx),
	)?;

	Ok(())
}

/// Scans pending actors subspace and aggregates metrics.
async fn aggregate_pending_actors(ctx: &StandaloneCtx) -> Result<()> {
	pegboard::metrics::ACTOR_PENDING_ALLOCATION.reset();

	let mut last_key = Vec::new();
	loop {
		last_key = ctx
			.udb()?
			.run(|tx| {
				let last_key = &last_key;
				async move {
					let start = Instant::now();
					let tx = tx.with_subspace(pegboard::keys::subspace());
					let mut new_last_key = Vec::new();

					let actor_pending_subspace = pegboard::keys::subspace().subspace(
						&pegboard::keys::ns::PendingActorByRunnerNameSelectorKey::entire_subspace(),
					);
					let range = actor_pending_subspace.range();

					let range_start = if last_key.is_empty() {
						&range.0
					} else {
						&last_key
					};
					let range_end = &actor_pending_subspace.range().1;

					let mut stream = tx.get_ranges_keyvalues(
						universaldb::RangeOption {
							mode: StreamingMode::WantAll,
							..(range_start.as_slice(), range_end.as_slice()).into()
						},
						Snapshot,
					);

					loop {
						if start.elapsed() > EARLY_TXN_TIMEOUT {
							tracing::warn!("timed out processing pending actors metrics");
							break;
						}

						let Some(entry) = stream.try_next().await? else {
							new_last_key = Vec::new();
							break;
						};

						let (pending_actor_key, _) =
						tx.read_entry::<pegboard::keys::ns::PendingActorByRunnerNameSelectorKey>(
							&entry,
						)?;

						pegboard::metrics::ACTOR_PENDING_ALLOCATION
							.with_label_values(&[
								&pending_actor_key.namespace_id.to_string(),
								&pending_actor_key.runner_name_selector,
							])
							.inc();

						new_last_key = [entry.key(), &[0xff]].concat();
					}

					Ok(new_last_key)
				}
			})
			.await?;

		if last_key.is_empty() {
			break;
		}
	}

	Ok(())
}

// /// Scans runner alloc idx and aggregates metrics.
// async fn aggregate_actor_active(ctx: &StandaloneCtx) -> Result<()> {
// 	pegboard::metrics::ACTOR_ACTIVE.reset();

// 	let mut last_key = Vec::new();
// 	loop {
// 		last_key = ctx
// 			.udb()?
// 			.run(|tx| {
// 				let last_key = &last_key;
// 				async move {
// 					let start = Instant::now();
// 					let tx = tx.with_subspace(pegboard::keys::subspace());
// 					let mut new_last_key = Vec::new();

// 					let runner_alloc_subspace = pegboard::keys::subspace()
// 						.subspace(&pegboard::keys::ns::RunnerAllocIdxKey::entire_subspace());
// 					let range = runner_alloc_subspace.range();

// 					let range_start = if last_key.is_empty() {
// 						&range.0
// 					} else {
// 						&last_key
// 					};
// 					let range_end = &runner_alloc_subspace.range().1;

// 					let mut stream = tx.get_ranges_keyvalues(
// 						universaldb::RangeOption {
// 							mode: StreamingMode::WantAll,
// 							..(range_start.as_slice(), range_end.as_slice()).into()
// 						},
// 						Snapshot,
// 					);

// 					loop {
// 						if start.elapsed() > EARLY_TXN_TIMEOUT {
// 							tracing::warn!("timed out processing active actor metrics");
// 							break;
// 						}

// 						let Some(entry) = stream.try_next().await? else {
// 							new_last_key = Vec::new();
// 							break;
// 						};

// 						let (runner_alloc_key, alloc_data) =
// 							tx.read_entry::<pegboard::keys::ns::RunnerAllocIdxKey>(&entry)?;

// 						pegboard::metrics::ACTOR_ACTIVE
// 							.with_label_values(&[
// 								&runner_alloc_key.namespace_id.to_string(),
// 								&runner_alloc_key.name,
// 							])
// 							.add(
// 								alloc_data
// 									.total_slots
// 									.saturating_sub(alloc_data.remaining_slots) as i64,
// 							);

// 						new_last_key = [entry.key(), &[0xff]].concat();
// 					}

// 					Ok(new_last_key)
// 				}
// 			})
// 			.await?;

// 		if last_key.is_empty() {
// 			break;
// 		}
// 	}

// 	Ok(())
// }

/// Scans serverless desired slots and aggregates metrics.
async fn aggregate_serverless_desired_slots(ctx: &StandaloneCtx) -> Result<()> {
	pegboard::metrics::SERVERLESS_DESIRED_SLOTS.reset();

	let mut last_key = Vec::new();
	loop {
		last_key = ctx
			.udb()?
			.run(|tx| {
				let last_key = &last_key;
				async move {
					let start = Instant::now();
					let tx = tx.with_subspace(pegboard::keys::subspace());
					let mut new_last_key = Vec::new();

					let serverless_desired_slots_subspace = pegboard::keys::subspace().subspace(
							&rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey::entire_subspace(
							),
						);
					let range = serverless_desired_slots_subspace.range();

					let range_start = if last_key.is_empty() {
						&range.0
					} else {
						&last_key
					};
					let range_end = &serverless_desired_slots_subspace.range().1;

					let mut stream = tx.get_ranges_keyvalues(
						universaldb::RangeOption {
							mode: StreamingMode::WantAll,
							..(range_start.as_slice(), range_end.as_slice()).into()
						},
						Snapshot,
					);

					loop {
						if start.elapsed() > EARLY_TXN_TIMEOUT {
							tracing::warn!("timed out processing serverless desired slot metrics");
							break;
						}

						let Some(entry) = stream.try_next().await? else {
							new_last_key = Vec::new();
							break;
						};

						let (serverless_desired_slots_key, desired_slots) =
							tx.read_entry::<rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey>(&entry)?;

						pegboard::metrics::SERVERLESS_DESIRED_SLOTS
							.with_label_values(&[
								&serverless_desired_slots_key.namespace_id.to_string(),
								&serverless_desired_slots_key.runner_name,
							])
							.add(desired_slots);

						new_last_key = [entry.key(), &[0xff]].concat();
					}

					Ok(new_last_key)
				}
			})
			.await?;

		if last_key.is_empty() {
			break;
		}
	}

	Ok(())
}
