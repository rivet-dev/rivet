//! Backfills the `RunnerNameSelectorKey` for actors created before the
//! `InitStateAndDb` activity started writing this key.

use std::time::{Duration, Instant};

use futures_util::TryStreamExt;
use gas::prelude::*;
use universaldb::KeySelector;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;
use universaldb::utils::end_of_key_range;

use crate::keys;

pub const BACKFILL_NAME: &str = "actor_runner_name_selector";

/// Timeout to stop processing early to avoid transaction timeout.
const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

const BATCH_SIZE: usize = 1024;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {}

#[workflow]
pub async fn pegboard_actor_runner_name_selector_backfill(
	ctx: &mut WorkflowCtx,
	_input: &Input,
) -> Result<()> {
	#[derive(Serialize, Deserialize)]
	struct State {
		/// The last actor_id processed, used for pagination.
		after_actor_id: Option<Id>,
		/// Total actors backfilled so far.
		total_backfilled: u64,
	}

	ctx.loope(
		State {
			after_actor_id: None,
			total_backfilled: 0,
		},
		|ctx, state| {
			Box::pin(async move {
				let output = ctx
					.activity(BackfillBatchInput {
						after_actor_id: state.after_actor_id,
						batch_size: BATCH_SIZE,
					})
					.await?;

				state.total_backfilled += output.backfilled_count as u64;

				if let Some(last_actor_id) = output.last_actor_id {
					state.after_actor_id = Some(last_actor_id);
					Ok(Loop::Continue)
				} else {
					tracing::info!(
						total_backfilled = state.total_backfilled,
						"completed runner_name_selector backfill"
					);
					Ok(Loop::Break(()))
				}
			})
		},
	)
	.await?;

	ctx.activity(MarkCompleteInput {
		name: BACKFILL_NAME.to_string(),
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct MarkCompleteInput {
	pub name: String,
}

#[activity(MarkComplete)]
pub async fn mark_complete(ctx: &ActivityCtx, input: &MarkCompleteInput) -> Result<()> {
	ctx.udb()?
		.run(|tx| {
			let name = input.name.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				tx.write(
					&keys::backfill::CompleteKey::new(&name),
					rivet_util::timestamp::now(),
				)?;
				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("mark_backfill_complete_tx"))
		.await?;

	tracing::info!(name = %input.name, "marked backfill as complete");

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct BackfillBatchInput {
	pub after_actor_id: Option<Id>,
	pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillBatchOutput {
	/// The last actor_id processed in this batch, used for pagination.
	/// If None, there are no more actors to process.
	pub last_actor_id: Option<Id>,
	/// Number of actors backfilled in this batch.
	pub backfilled_count: usize,
}

#[activity(BackfillBatch)]
pub async fn backfill_batch(
	ctx: &ActivityCtx,
	input: &BackfillBatchInput,
) -> Result<BackfillBatchOutput> {
	// Find actors missing runner_name_selector key
	let actors_to_backfill: Vec<(Id, Id)> = ctx
		.udb()?
		.run(|tx| {
			let after_actor_id = input.after_actor_id;
			let batch_size = input.batch_size;
			async move {
				let start = Instant::now();
				let tx = tx.with_subspace(keys::subspace());

				// Get actors that have a workflow_id key, starting after the last processed actor
				let actor_data_subspace = keys::subspace().subspace(&keys::actor::DataSubspaceKey);

				// Build range start key based on pagination cursor
				let begin_key = if let Some(after_actor_id) = after_actor_id {
					// Start after the last processed actor's WorkflowIdKey
					let after_key = keys::actor::WorkflowIdKey::new(after_actor_id);
					let packed = keys::subspace().pack(&after_key);
					KeySelector::first_greater_than(packed)
				} else {
					KeySelector::first_greater_or_equal(actor_data_subspace.bytes().to_vec())
				};

				let range_option = universaldb::RangeOption {
					mode: StreamingMode::Iterator,
					begin: begin_key,
					end: KeySelector::first_greater_or_equal(end_of_key_range(
						actor_data_subspace.bytes(),
					)),
					..Default::default()
				};

				let mut actors_missing_runner_name = Vec::new();
				let mut stream = tx.get_ranges_keyvalues(range_option, Snapshot);

				while let Some(entry) = stream.try_next().await? {
					if start.elapsed() > EARLY_TXN_TIMEOUT {
						tracing::warn!("timed out finding actors to backfill");
						break;
					}

					let (key, workflow_id) = tx.read_entry::<keys::actor::WorkflowIdKey>(&entry)?;
					let actor_id = key.actor_id();

					// Check if runner_name_selector key exists
					let runner_name_selector_key =
						keys::actor::RunnerNameSelectorKey::new(actor_id);
					let exists = tx.exists(&runner_name_selector_key, Snapshot).await?;

					if !exists {
						actors_missing_runner_name.push((actor_id, workflow_id));

						if actors_missing_runner_name.len() >= batch_size {
							break;
						}
					}
				}

				Ok(actors_missing_runner_name)
			}
		})
		.custom_instrument(tracing::info_span!("find_actors_tx"))
		.await?;

	if actors_to_backfill.is_empty() {
		return Ok(BackfillBatchOutput {
			last_actor_id: None,
			backfilled_count: 0,
		});
	}

	let last_actor_id = actors_to_backfill.last().map(|(actor_id, _)| *actor_id);

	tracing::debug!(
		count = actors_to_backfill.len(),
		?last_actor_id,
		"backfilling batch of actors"
	);

	// Get workflow data for actors that need backfill
	let workflow_ids: Vec<Id> = actors_to_backfill.iter().map(|(_, wf_id)| *wf_id).collect();
	let workflows = ctx.get_workflows(workflow_ids).await?;

	// Build backfill data
	let mut backfill_data = Vec::new();
	for (actor_id, workflow_id) in &actors_to_backfill {
		if let Some(workflow) = workflows.iter().find(|w| w.workflow_id == *workflow_id) {
			let input: crate::workflows::actor::Input =
				workflow.parse_input::<crate::workflows::actor::Workflow>()?;
			backfill_data.push((*actor_id, input.runner_name_selector));
		} else {
			tracing::warn!(?actor_id, ?workflow_id, "workflow not found for actor");
		}
	}

	// Write runner_name_selector keys
	if !backfill_data.is_empty() {
		ctx.udb()?
			.run(|tx| {
				let backfill_data = backfill_data.clone();
				async move {
					let tx = tx.with_subspace(keys::subspace());

					for (actor_id, runner_name_selector) in backfill_data {
						tx.write(
							&keys::actor::RunnerNameSelectorKey::new(actor_id),
							runner_name_selector,
						)?;
					}

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("backfill_runner_name_selector_tx"))
			.await?;
	}

	Ok(BackfillBatchOutput {
		last_actor_id,
		backfilled_count: backfill_data.len(),
	})
}
