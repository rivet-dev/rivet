use std::time::Instant;

use futures_util::StreamExt;
use futures_util::{FutureExt, TryStreamExt};
use gas::prelude::*;
use rivet_metrics::KeyValue;
use rivet_runner_protocol::protocol;
use udb_util::{FormalKey, SERIALIZABLE, SNAPSHOT, TxnExt};
use universaldb::{
	self as udb,
	options::{ConflictRangeType, StreamingMode},
};

use crate::{
	keys, metrics,
	workflows::runner::{AllocatePendingActorsInput, RUNNER_ELIGIBLE_THRESHOLD_MS},
};

use super::{
	ACTOR_START_THRESHOLD_MS, Allocate, BASE_RETRY_TIMEOUT_MS, Destroy, Input, PendingAllocation,
	RETRY_RESET_DURATION_MS, State, destroy,
};

#[derive(Deserialize, Serialize)]
pub struct LifecycleState {
	pub generation: u32,

	// TODO: Make these optional? These might not match the properties in the workflow state but it shouldn't
	// matter for the functionality of the lifecycle loop
	pub runner_id: Id,
	pub runner_workflow_id: Id,

	pub sleeping: bool,
	pub alarm_ts: Option<i64>,
	pub gc_timeout_ts: Option<i64>,

	pub reschedule_state: RescheduleState,
}

impl LifecycleState {
	pub fn new(runner_id: Id, runner_workflow_id: Id) -> Self {
		LifecycleState {
			generation: 0,
			runner_id,
			runner_workflow_id,
			sleeping: false,
			alarm_ts: None,
			gc_timeout_ts: Some(util::timestamp::now() + ACTOR_START_THRESHOLD_MS),
			reschedule_state: RescheduleState::default(),
		}
	}
}

#[derive(Serialize, Deserialize)]
pub struct LifecycleRes {
	pub generation: u32,
	pub kill: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct RescheduleState {
	last_retry_ts: i64,
	retry_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateRunnerInput {
	actor_id: Id,
	runner_id: Id,
	runner_workflow_id: Id,
}

// This is called when allocated by an outside source while the actor was pending.
#[activity(UpdateRunner)]
async fn update_runner(ctx: &ActivityCtx, input: &UpdateRunnerInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	state.sleep_ts = None;
	state.pending_allocation_ts = None;
	state.runner_id = Some(input.runner_id);
	state.runner_workflow_id = Some(input.runner_workflow_id);

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct AllocateActorInput {
	actor_id: Id,
	generation: u32,
	runner_name_selector: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllocateActorOutput {
	pub runner_id: Id,
	pub runner_workflow_id: Id,
}

// If no availability, returns the timestamp of the actor's queue key
#[activity(AllocateActor)]
async fn allocate_actor(
	ctx: &ActivityCtx,
	input: &AllocateActorInput,
) -> Result<std::result::Result<AllocateActorOutput, i64>> {
	let start_instant = Instant::now();
	let mut state = ctx.state::<State>()?;
	let namespace_id = state.namespace_id;

	// NOTE: This txn should closely resemble the one found in the allocate_pending_actors activity of the
	// client wf
	let res = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let ping_threshold_ts = util::timestamp::now() - RUNNER_ELIGIBLE_THRESHOLD_MS;
			let txs = tx.subspace(keys::subspace());

			// Check if a queue exists
			let pending_actor_subspace = txs.subspace(
				&keys::datacenter::PendingActorByRunnerNameSelectorKey::subspace(
					namespace_id,
					input.runner_name_selector.clone(),
				),
			);
			let queue_exists = txs
				.get_ranges_keyvalues(
					udb::RangeOption {
						mode: StreamingMode::Exact,
						limit: Some(1),
						..(&pending_actor_subspace).into()
					},
					// NOTE: This is not SERIALIZABLE because we don't want to conflict with other
					// inserts/clears to this range
					SNAPSHOT,
				)
				.next()
				.await
				.is_some();

			if !queue_exists {
				let runner_alloc_subspace =
					txs.subspace(&keys::datacenter::RunnerAllocIdxKey::subspace(
						namespace_id,
						input.runner_name_selector.clone(),
					));

				let mut stream = txs.get_ranges_keyvalues(
					udb::RangeOption {
						mode: StreamingMode::Iterator,
						..(&runner_alloc_subspace).into()
					},
					// NOTE: This is not SERIALIZABLE because we don't want to conflict with all of the
					// keys, just the one we choose
					SNAPSHOT,
				);

				let mut highest_version = None;

				loop {
					let Some(entry) = stream.try_next().await? else {
						break;
					};

					let (old_runner_alloc_key, old_runner_alloc_key_data) =
						txs.read_entry::<keys::datacenter::RunnerAllocIdxKey>(&entry)?;

					if let Some(highest_version) = highest_version {
						// We have passed all of the runners with the highest version. This is reachable if
						// the ping of the highest version workers makes them ineligible
						if old_runner_alloc_key.version < highest_version {
							break;
						}
					} else {
						highest_version = Some(old_runner_alloc_key.version);
					}

					// An empty runner means we have reached the end of the runners with the highest version
					if old_runner_alloc_key.remaining_millislots == 0 {
						break;
					}

					// Scan by last ping
					if old_runner_alloc_key.last_ping_ts < ping_threshold_ts {
						continue;
					}

					// Add read conflict only for this key
					txs.add_conflict_key(&old_runner_alloc_key, ConflictRangeType::Read)?;

					// Clear old entry
					txs.delete(&old_runner_alloc_key);

					let new_remaining_slots =
						old_runner_alloc_key_data.remaining_slots.saturating_sub(1);
					let new_remaining_millislots =
						(new_remaining_slots * 1000) / old_runner_alloc_key_data.total_slots;

					// Write new allocation key with 1 less slot
					txs.write(
						&keys::datacenter::RunnerAllocIdxKey::new(
							namespace_id,
							input.runner_name_selector.clone(),
							old_runner_alloc_key.version,
							new_remaining_millislots,
							old_runner_alloc_key.last_ping_ts,
							old_runner_alloc_key.runner_id,
						),
						rivet_key_data::converted::RunnerAllocIdxKeyData {
							workflow_id: old_runner_alloc_key_data.workflow_id,
							remaining_slots: new_remaining_slots,
							total_slots: old_runner_alloc_key_data.total_slots,
						},
					)?;

					// Update runner record
					txs.write(
						&keys::runner::RemainingSlotsKey::new(old_runner_alloc_key.runner_id),
						new_remaining_slots,
					)?;

					// Set runner id of actor
					txs.write(
						&keys::actor::RunnerIdKey::new(input.actor_id),
						old_runner_alloc_key.runner_id,
					)?;

					// Insert actor index key
					txs.write(
						&keys::runner::ActorKey::new(
							old_runner_alloc_key.runner_id,
							input.actor_id,
						),
						input.generation,
					)?;

					// Set actor as not sleeping
					txs.delete(&keys::actor::SleepTsKey::new(input.actor_id));

					return Ok(Ok(AllocateActorOutput {
						runner_id: old_runner_alloc_key.runner_id,
						runner_workflow_id: old_runner_alloc_key_data.workflow_id,
					}));
				}
			}

			// At this point in the txn there is no availability. Write the actor to the alloc queue to wait.

			let pending_ts = util::timestamp::now();

			// NOTE: This will conflict with serializable reads to the alloc queue, which is the behavior we
			// want. If a runner reads from the queue while this is being inserted, one of the two txns will
			// retry and we ensure the actor does not end up in queue limbo.
			txs.write(
				&keys::datacenter::PendingActorByRunnerNameSelectorKey::new(
					namespace_id,
					input.runner_name_selector.clone(),
					pending_ts,
					input.actor_id,
				),
				input.generation,
			)?;

			return Ok(Err(pending_ts));
		})
		.custom_instrument(tracing::info_span!("actor_allocate_tx"))
		.await?;

	let dt = start_instant.elapsed().as_secs_f64();
	metrics::ACTOR_ALLOCATE_DURATION
		.record(dt, &[KeyValue::new("did_reserve", res.is_ok().to_string())]);

	if let Ok(res) = &res {
		state.sleep_ts = None;
		state.pending_allocation_ts = None;
		state.runner_id = Some(res.runner_id);
		state.runner_workflow_id = Some(res.runner_workflow_id);
	}

	Ok(res)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetNotConnectableInput {
	pub actor_id: Id,
}

#[activity(SetNotConnectable)]
pub async fn set_not_connectable(ctx: &ActivityCtx, input: &SetNotConnectableInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	ctx.udb()?
		.run(|tx, _mc| async move {
			let connectable_key = keys::actor::ConnectableKey::new(input.actor_id);
			tx.clear(&keys::subspace().pack(&connectable_key));

			Ok(())
		})
		.custom_instrument(tracing::info_span!("actor_deallocate_tx"))
		.await?;

	state.connectable_ts = None;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct DeallocateInput {
	pub actor_id: Id,
}

#[activity(Deallocate)]
pub async fn deallocate(ctx: &ActivityCtx, input: &DeallocateInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;
	let runner_name_selector = &state.runner_name_selector;
	let namespace_id = state.namespace_id;
	let runner_id = state.runner_id;

	ctx.udb()?
		.run(|tx, _mc| async move {
			let connectable_key = keys::actor::ConnectableKey::new(input.actor_id);
			tx.clear(&keys::subspace().pack(&connectable_key));

			if let Some(runner_id) = runner_id {
				destroy::clear_slot(
					input.actor_id,
					namespace_id,
					runner_name_selector,
					runner_id,
					&tx,
				)
				.await?;
			}

			Ok(())
		})
		.custom_instrument(tracing::info_span!("actor_deallocate_tx"))
		.await?;

	state.connectable_ts = None;
	state.runner_id = None;
	state.runner_workflow_id = None;

	Ok(())
}

/// Returns None if a destroy signal was received while pending for allocation.
pub async fn spawn_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	generation: u32,
) -> Result<Option<AllocateActorOutput>> {
	// Attempt allocation
	let allocate_res = ctx
		.activity(AllocateActorInput {
			actor_id: input.actor_id,
			runner_name_selector: input.runner_name_selector.clone(),
			generation,
		})
		.await?;

	let allocate_res = match allocate_res {
		Ok(x) => x,
		Err(pending_allocation_ts) => {
			tracing::warn!(
				actor_id=?input.actor_id,
				"failed to allocate (no availability), waiting for allocation",
			);

			ctx.activity(SetPendingAllocationInput {
				pending_allocation_ts,
			})
			.await?;

			// If allocation fails, the allocate txn already inserted this actor into the queue. Now we wait for
			// an `Allocate` signal
			match ctx.listen::<PendingAllocation>().await? {
				PendingAllocation::Allocate(sig) => {
					ctx.activity(UpdateRunnerInput {
						actor_id: input.actor_id,
						runner_id: sig.runner_id,
						runner_workflow_id: sig.runner_workflow_id,
					})
					.await?;

					AllocateActorOutput {
						runner_id: sig.runner_id,
						runner_workflow_id: sig.runner_workflow_id,
					}
				}
				PendingAllocation::Destroy(_) => {
					tracing::debug!(actor_id=?input.actor_id, "destroying before actor allocated");

					let cleared = ctx
						.activity(ClearPendingAllocationInput {
							actor_id: input.actor_id,
							namespace_id: input.namespace_id,
							runner_name_selector: input.runner_name_selector.clone(),
							pending_allocation_ts,
						})
						.await?;

					// If this actor was no longer present in the queue it means it was allocated. We must now
					// wait for the allocated signal to prevent a race condition.
					if !cleared {
						let sig = ctx.listen::<Allocate>().await?;

						ctx.activity(UpdateRunnerInput {
							actor_id: input.actor_id,
							runner_id: sig.runner_id,
							runner_workflow_id: sig.runner_workflow_id,
						})
						.await?;
					}

					return Ok(None);
				}
			}
		}
	};

	ctx.signal(protocol::Command::StartActor {
		actor_id: input.actor_id,
		generation,
		config: Box::new(protocol::ActorConfig {
			name: input.name.clone(),
			key: input.key.clone(),
			// HACK: We should not use dynamic timestamp here, but we don't validate if signal data
			// changes (like activity inputs) so this is fine for now.
			create_ts: util::timestamp::now(),
			input: input.input.clone(),
		}),
	})
	.to_workflow_id(allocate_res.runner_workflow_id)
	.send()
	.await?;

	Ok(Some(allocate_res))
}

/// Returns true if the actor should be destroyed.
pub async fn reschedule_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	state: &mut LifecycleState,
	sleeping: bool,
) -> Result<bool> {
	tracing::debug!(actor_id=?input.actor_id, "rescheduling actor");

	// There shouldn't be an allocation if the actor is sleeping
	if !sleeping {
		ctx.activity(DeallocateInput {
			actor_id: input.actor_id,
		})
		.await?;

		// Allocate other pending actors from queue
		let res = ctx
			.activity(AllocatePendingActorsInput {
				namespace_id: input.namespace_id,
				name: input.runner_name_selector.clone(),
			})
			.await?;

		// Dispatch pending allocs
		for alloc in res.allocations {
			ctx.signal(alloc.signal)
				.to_workflow::<super::Workflow>()
				.tag("actor_id", alloc.actor_id)
				.send()
				.await?;
		}
	}

	let next_generation = state.generation + 1;

	// Waits for the actor to be ready (or destroyed) and automatically retries if failed to allocate.
	let res = ctx
		.loope(state.reschedule_state.clone(), |ctx, resched_state| {
			let input = input.clone();

			async move {
				// Determine next backoff sleep duration
				let mut backoff = util::backoff::Backoff::new_at(
					8,
					None,
					BASE_RETRY_TIMEOUT_MS,
					500,
					resched_state.retry_count,
				);

				let (now, reset) = ctx
					.v(2)
					.activity(CompareRetryInput {
						last_retry_ts: resched_state.last_retry_ts,
					})
					.await?;

				resched_state.retry_count = if reset {
					0
				} else {
					resched_state.retry_count + 1
				};
				resched_state.last_retry_ts = now;

				// Don't sleep for first retry
				if resched_state.retry_count > 0 {
					let next = backoff.step().expect("should not have max retry");

					// Sleep for backoff or destroy early
					if let Some(_sig) = ctx
						.listen_with_timeout::<Destroy>(Instant::from(next) - Instant::now())
						.await?
					{
						tracing::debug!("destroying before actor start");

						return Ok(Loop::Break(None));
					}
				}

				if let Some(res) = spawn_actor(ctx, &input, next_generation).await? {
					Ok(Loop::Break(Some((resched_state.clone(), res))))
				} else {
					// Destroyed early
					Ok(Loop::Break(None))
				}
			}
			.boxed()
		})
		.await?;

	// Update loop state
	if let Some((reschedule_state, res)) = res {
		state.generation = next_generation;
		state.runner_id = res.runner_id;
		state.runner_workflow_id = res.runner_workflow_id;

		// Save reschedule state in global state
		state.reschedule_state = reschedule_state;

		// Reset gc timeout once allocated
		state.gc_timeout_ts = Some(util::timestamp::now() + ACTOR_START_THRESHOLD_MS);

		Ok(false)
	} else {
		Ok(true)
	}
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetPendingAllocationInput {
	pending_allocation_ts: i64,
}

#[activity(SetPendingAllocation)]
pub async fn set_pending_allocation(
	ctx: &ActivityCtx,
	input: &SetPendingAllocationInput,
) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	state.pending_allocation_ts = Some(input.pending_allocation_ts);

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct ClearPendingAllocationInput {
	actor_id: Id,
	namespace_id: Id,
	runner_name_selector: String,
	pending_allocation_ts: i64,
}

#[activity(ClearPendingAllocation)]
pub async fn clear_pending_allocation(
	ctx: &ActivityCtx,
	input: &ClearPendingAllocationInput,
) -> Result<bool> {
	// Clear self from alloc queue
	let cleared = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let pending_alloc_key =
				keys::subspace().pack(&keys::datacenter::PendingActorByRunnerNameSelectorKey::new(
					input.namespace_id,
					input.runner_name_selector.clone(),
					input.pending_allocation_ts,
					input.actor_id,
				));

			let exists = tx.get(&pending_alloc_key, SERIALIZABLE).await?.is_some();

			tx.clear(&pending_alloc_key);

			Ok(exists)
		})
		.await?;

	Ok(cleared)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CompareRetryInput {
	last_retry_ts: i64,
}

#[activity(CompareRetry)]
async fn compare_retry(ctx: &ActivityCtx, input: &CompareRetryInput) -> Result<(i64, bool)> {
	let now = util::timestamp::now();

	// If the last retry ts is more than RETRY_RESET_DURATION_MS, reset retry count
	Ok((now, input.last_retry_ts < now - RETRY_RESET_DURATION_MS))
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetStartedInput {
	pub actor_id: Id,
}

#[activity(SetStarted)]
pub async fn set_started(ctx: &ActivityCtx, input: &SetStartedInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	state.start_ts = Some(util::timestamp::now());
	state.connectable_ts = Some(util::timestamp::now());

	ctx.udb()?
		.run(|tx, _mc| async move {
			let connectable_key = keys::actor::ConnectableKey::new(input.actor_id);
			tx.set(
				&keys::subspace().pack(&connectable_key),
				&connectable_key
					.serialize(())
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
			);

			Ok(())
		})
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetSleepingInput {
	pub actor_id: Id,
}

#[activity(SetSleeping)]
pub async fn set_sleeping(ctx: &ActivityCtx, input: &SetSleepingInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;
	let sleep_ts = util::timestamp::now();

	state.sleep_ts = Some(sleep_ts);
	state.connectable_ts = None;

	ctx.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());

			// Make not connectable
			txs.delete(&keys::actor::ConnectableKey::new(input.actor_id));

			txs.write(&keys::actor::SleepTsKey::new(input.actor_id), sleep_ts)?;

			Ok(())
		})
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetCompleteInput {}

#[activity(SetComplete)]
pub async fn set_complete(ctx: &ActivityCtx, input: &SetCompleteInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	state.complete_ts = Some(util::timestamp::now());

	Ok(())
}
