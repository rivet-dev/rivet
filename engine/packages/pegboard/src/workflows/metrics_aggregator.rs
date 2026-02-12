use std::time::{Duration, Instant};

use anyhow::Result;
use futures_util::{FutureExt, TryStreamExt};
use gas::prelude::*;
use universaldb::{options::StreamingMode, utils::IsolationLevel::*};

use crate::{keys, metrics};

const TICK_RATE: Duration = Duration::from_secs(15);
const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {}

#[workflow]
pub async fn pegboard_metrics_aggregator(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.repeat(|ctx| {
		async move {
			// Run before sleeping so the initial export is immediate
			ctx.join((
				activity(AggregatePendingActorsInput {}),
				// activity(AggregateActiveActorsInput { }),
				activity(AggregateServerlessDesiredSlotsInput {}),
			))
			.await?;

			ctx.sleep(TICK_RATE).await?;

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct AggregatePendingActorsInput {}

/// Scans pending actors subspace and aggregates metrics.
#[activity(AggregatePendingActors)]
async fn aggregate_pending_actors(
	ctx: &ActivityCtx,
	_input: &AggregatePendingActorsInput,
) -> Result<()> {
	metrics::ACTOR_PENDING_ALLOCATION.reset();

	let mut last_key = Vec::new();
	loop {
		last_key = ctx
			.udb()?
			.run(|tx| {
				let last_key = &last_key;
				async move {
					let start = Instant::now();
					let tx = tx.with_subspace(keys::subspace());
					let mut new_last_key = Vec::new();

					let actor_pending_subspace = keys::subspace().subspace(
						&keys::ns::PendingActorByRunnerNameSelectorKey::entire_subspace(),
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
							tx.read_entry::<keys::ns::PendingActorByRunnerNameSelectorKey>(&entry)?;

						metrics::ACTOR_PENDING_ALLOCATION
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

// #[derive(Debug, Clone, Serialize, Deserialize, Hash)]
// struct AggregateActiveActorsInput {}

// /// Scans runner alloc idx and aggregates metrics.
// #[activity(AggregateActiveActors)]
// async fn aggregate_active_actors(
// 	ctx: &ActivityCtx,
// 	_input: &AggregateActiveActorsInput,
// ) -> Result<()> {
// 	metrics::ACTOR_ACTIVE.reset();

// 	let mut last_key = Vec::new();
// 	loop {
// 		last_key = ctx
// 			.udb()?
// 			.run(|tx| {
// 				let last_key = &last_key;
// 				async move {
// 					let start = Instant::now();
// 					let tx = tx.with_subspace(keys::subspace());
// 					let mut new_last_key = Vec::new();

// 					let runner_alloc_subspace =
// 						keys::subspace().subspace(&keys::ns::RunnerAllocIdxKey::entire_subspace());
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
// 							tx.read_entry::<keys::ns::RunnerAllocIdxKey>(&entry)?;

// 						let active_actors = alloc_data
// 							.total_slots
// 							.saturating_sub(alloc_data.remaining_slots)
// 							as i64;

// 						if active_actors != 0 {
// 							metrics::ACTOR_ACTIVE
// 								.with_label_values(&[
// 									&runner_alloc_key.namespace_id.to_string(),
// 									&runner_alloc_key.name,
// 								])
// 								.add(active_actors);
// 						}

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

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct AggregateServerlessDesiredSlotsInput {}

/// Scans serverless desired slots and aggregates metrics.
#[activity(AggregateServerlessDesiredSlots)]
async fn aggregate_serverless_desired_slots(
	ctx: &ActivityCtx,
	_input: &AggregateServerlessDesiredSlotsInput,
) -> Result<()> {
	metrics::SERVERLESS_DESIRED_SLOTS.reset();

	let mut last_key = Vec::new();
	loop {
		last_key = ctx
			.udb()?
			.run(|tx| {
				let last_key = &last_key;
				async move {
					let start = Instant::now();
					let tx = tx.with_subspace(keys::subspace());
					let mut new_last_key = Vec::new();

					let serverless_desired_slots_subspace = keys::subspace().subspace(
						&rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey::entire_subspace(),
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

						if desired_slots != 0 {
							metrics::SERVERLESS_DESIRED_SLOTS
								.with_label_values(&[
									&serverless_desired_slots_key.namespace_id.to_string(),
									&serverless_desired_slots_key.runner_name,
								])
								.add(desired_slots);
						}

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
