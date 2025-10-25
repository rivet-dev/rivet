use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_metrics::KeyValue;
use rivet_runner_protocol as protocol;
use rivet_types::{
	actors::CrashPolicy, keys::namespace::runner_config::RunnerConfigVariant,
	runner_configs::RunnerConfigKind,
};
use std::time::Instant;
use universaldb::options::{ConflictRangeType, MutationType, StreamingMode};
use universaldb::utils::{FormalKey, IsolationLevel::*};

use crate::{keys, metrics, workflows::runner::RUNNER_ELIGIBLE_THRESHOLD_MS};

use super::{
	ACTOR_START_THRESHOLD_MS, Allocate, BASE_RETRY_TIMEOUT_MS, Destroy, Input, PendingAllocation,
	RETRY_RESET_DURATION_MS, State, destroy,
};

#[derive(Deserialize, Serialize)]
pub struct LifecycleState {
	pub generation: u32,

	// Set when currently running (not rescheduling or sleeping)
	pub runner_id: Option<Id>,
	pub runner_workflow_id: Option<Id>,

	pub sleeping: bool,
	/// If a wake was received in between an actor's intent to sleep and actor stop.
	#[serde(default)]
	pub will_wake: bool,
	/// Whether or not the last wake was triggered by an alarm.
	#[serde(default)]
	pub wake_for_alarm: bool,
	pub alarm_ts: Option<i64>,
	pub gc_timeout_ts: Option<i64>,

	pub reschedule_state: RescheduleState,
}

impl LifecycleState {
	pub fn new(runner_id: Id, runner_workflow_id: Id) -> Self {
		LifecycleState {
			generation: 0,
			runner_id: Some(runner_id),
			runner_workflow_id: Some(runner_workflow_id),
			sleeping: false,
			will_wake: false,
			wake_for_alarm: false,
			alarm_ts: None,
			gc_timeout_ts: Some(util::timestamp::now() + ACTOR_START_THRESHOLD_MS),
			reschedule_state: RescheduleState::default(),
		}
	}

	pub fn new_sleeping() -> Self {
		LifecycleState {
			generation: 0,
			runner_id: None,
			runner_workflow_id: None,
			sleeping: true,
			will_wake: false,
			wake_for_alarm: false,
			alarm_ts: None,
			gc_timeout_ts: None,
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
	force_allocate: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllocateActorOutput {
	Allocated {
		runner_id: Id,
		runner_workflow_id: Id,
	},
	Pending {
		pending_allocation_ts: i64,
	},
	Sleep,
}

// If no availability, returns the timestamp of the actor's queue key
#[activity(AllocateActor)]
async fn allocate_actor(
	ctx: &ActivityCtx,
	input: &AllocateActorInput,
) -> Result<AllocateActorOutput> {
	let start_instant = Instant::now();

	let mut state = ctx.state::<State>()?;
	let namespace_id = state.namespace_id;
	let crash_policy = state.crash_policy;
	let runner_name_selector = &state.runner_name_selector;

	// Check if valid serverless config exists for the current ns + runner name
	let runner_config_res = ctx
		.op(namespace::ops::runner_config::get::Input {
			runners: vec![(namespace_id, runner_name_selector.clone())],
			bypass_cache: false,
		})
		.await?;
	let has_valid_serverless = runner_config_res
		.first()
		.map(|runner| match &runner.config.kind {
			RunnerConfigKind::Serverless { max_runners, .. } => *max_runners != 0,
			_ => false,
		})
		.unwrap_or_default();

	// NOTE: This txn should closely resemble the one found in the allocate_pending_actors activity of the
	// client wf
	let (for_serverless, res) = ctx
		.udb()?
		.run(|tx| async move {
			let ping_threshold_ts = util::timestamp::now() - RUNNER_ELIGIBLE_THRESHOLD_MS;

			// Check if runner is an serverless runner
			let for_serverless = tx
				.with_subspace(namespace::keys::subspace())
				.exists(
					&namespace::keys::runner_config::ByVariantKey::new(
						namespace_id,
						RunnerConfigVariant::Serverless,
						runner_name_selector.clone(),
					),
					Serializable,
				)
				.await?;

			let tx = tx.with_subspace(keys::subspace());

			if for_serverless {
				tx.atomic_op(
					&rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey::new(
						namespace_id,
						runner_name_selector.clone(),
					),
					&1i64.to_le_bytes(),
					MutationType::Add,
				);
			}

			// Check if a queue exists
			let pending_actor_subspace = keys::subspace().subspace(
				&keys::ns::PendingActorByRunnerNameSelectorKey::subspace(
					namespace_id,
					runner_name_selector.clone(),
				),
			);
			let queue_exists = tx
				.get_ranges_keyvalues(
					universaldb::RangeOption {
						mode: StreamingMode::Exact,
						limit: Some(1),
						..(&pending_actor_subspace).into()
					},
					// NOTE: This is not Serializable because we don't want to conflict with other
					// inserts/clears to this range
					Snapshot,
				)
				.next()
				.await
				.is_some();

			if !queue_exists {
				let runner_alloc_subspace =
					keys::subspace().subspace(&keys::ns::RunnerAllocIdxKey::subspace(
						namespace_id,
						runner_name_selector.clone(),
					));

				let mut stream = tx.get_ranges_keyvalues(
					universaldb::RangeOption {
						mode: StreamingMode::Iterator,
						..(&runner_alloc_subspace).into()
					},
					// NOTE: This is not Serializable because we don't want to conflict with all of the
					// keys, just the one we choose
					Snapshot,
				);

				let mut highest_version = None;

				loop {
					let Some(entry) = stream.try_next().await? else {
						break;
					};

					let (old_runner_alloc_key, old_runner_alloc_key_data) =
						tx.read_entry::<keys::ns::RunnerAllocIdxKey>(&entry)?;

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
					tx.add_conflict_key(&old_runner_alloc_key, ConflictRangeType::Read)?;

					// Clear old entry
					tx.delete(&old_runner_alloc_key);

					let new_remaining_slots =
						old_runner_alloc_key_data.remaining_slots.saturating_sub(1);
					let new_remaining_millislots =
						(new_remaining_slots * 1000) / old_runner_alloc_key_data.total_slots;

					// Write new allocation key with 1 less slot
					tx.write(
						&keys::ns::RunnerAllocIdxKey::new(
							namespace_id,
							runner_name_selector.clone(),
							old_runner_alloc_key.version,
							new_remaining_millislots,
							old_runner_alloc_key.last_ping_ts,
							old_runner_alloc_key.runner_id,
						),
						rivet_data::converted::RunnerAllocIdxKeyData {
							workflow_id: old_runner_alloc_key_data.workflow_id,
							remaining_slots: new_remaining_slots,
							total_slots: old_runner_alloc_key_data.total_slots,
						},
					)?;

					// Update runner record
					tx.write(
						&keys::runner::RemainingSlotsKey::new(old_runner_alloc_key.runner_id),
						new_remaining_slots,
					)?;

					// Set runner id of actor
					tx.write(
						&keys::actor::RunnerIdKey::new(input.actor_id),
						old_runner_alloc_key.runner_id,
					)?;

					// Insert actor index key
					tx.write(
						&keys::runner::ActorKey::new(
							old_runner_alloc_key.runner_id,
							input.actor_id,
						),
						input.generation,
					)?;

					// Set actor as not sleeping
					tx.delete(&keys::actor::SleepTsKey::new(input.actor_id));

					return Ok((
						for_serverless,
						AllocateActorOutput::Allocated {
							runner_id: old_runner_alloc_key.runner_id,
							runner_workflow_id: old_runner_alloc_key_data.workflow_id,
						},
					));
				}
			}

			// At this point in the txn there is no availability

			match (crash_policy, input.force_allocate, has_valid_serverless) {
				(CrashPolicy::Sleep, false, false) => {
					Ok((for_serverless, AllocateActorOutput::Sleep))
				}
				// Write the actor to the alloc queue to wait
				_ => {
					let pending_allocation_ts = util::timestamp::now();

					// NOTE: This will conflict with serializable reads to the alloc queue, which is the behavior we
					// want. If a runner reads from the queue while this is being inserted, one of the two txns will
					// retry and we ensure the actor does not end up in queue limbo.
					tx.write(
						&keys::ns::PendingActorByRunnerNameSelectorKey::new(
							namespace_id,
							runner_name_selector.clone(),
							pending_allocation_ts,
							input.actor_id,
						),
						input.generation,
					)?;

					Ok((
						for_serverless,
						AllocateActorOutput::Pending {
							pending_allocation_ts,
						},
					))
				}
			}
		})
		.custom_instrument(tracing::info_span!("actor_allocate_tx"))
		.await?;

	let dt = start_instant.elapsed().as_secs_f64();
	metrics::ACTOR_ALLOCATE_DURATION.record(
		dt,
		&[KeyValue::new(
			"did_reserve",
			matches!(res, AllocateActorOutput::Allocated { .. }).to_string(),
		)],
	);

	state.for_serverless = for_serverless;
	state.allocated_serverless_slot = for_serverless;

	match &res {
		AllocateActorOutput::Allocated {
			runner_id,
			runner_workflow_id,
		} => {
			state.sleep_ts = None;
			state.pending_allocation_ts = None;
			state.runner_id = Some(*runner_id);
			state.runner_workflow_id = Some(*runner_workflow_id);
		}
		AllocateActorOutput::Pending {
			pending_allocation_ts,
		} => {
			tracing::warn!(
				actor_id=?input.actor_id,
				"failed to allocate (no availability), waiting for allocation",
			);

			state.pending_allocation_ts = Some(*pending_allocation_ts);
		}
		AllocateActorOutput::Sleep => {}
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
		.run(|tx| async move {
			let connectable_key = keys::actor::ConnectableKey::new(input.actor_id);
			tx.clear(&keys::subspace().pack(&connectable_key));

			Ok(())
		})
		.custom_instrument(tracing::info_span!("actor_set_not_connectable_tx"))
		.await?;

	state.connectable_ts = None;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct DeallocateInput {
	pub actor_id: Id,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeallocateOutput {
	pub for_serverless: bool,
}

#[activity(Deallocate)]
pub async fn deallocate(ctx: &ActivityCtx, input: &DeallocateInput) -> Result<DeallocateOutput> {
	let mut state = ctx.state::<State>()?;
	let runner_name_selector = &state.runner_name_selector;
	let namespace_id = state.namespace_id;
	let runner_id = state.runner_id;
	let for_serverless = state.for_serverless;

	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			tx.delete(&keys::actor::ConnectableKey::new(input.actor_id));

			// Only clear slot if we have a runner id
			if let Some(runner_id) = runner_id {
				destroy::clear_slot(
					input.actor_id,
					namespace_id,
					runner_name_selector,
					runner_id,
					for_serverless,
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
	// Slot was cleared by the above txn
	state.allocated_serverless_slot = false;

	Ok(DeallocateOutput {
		for_serverless: state.for_serverless,
	})
}

#[derive(Debug)]
pub enum SpawnActorOutput {
	Allocated {
		runner_id: Id,
		runner_workflow_id: Id,
	},
	Sleep,
	Destroy,
}

/// Wrapper around `allocate_actor` that handles pending state.
pub async fn spawn_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	generation: u32,
	force_allocate: bool,
) -> Result<SpawnActorOutput> {
	// Attempt allocation
	let allocate_res = ctx
		.activity(AllocateActorInput {
			actor_id: input.actor_id,
			generation,
			force_allocate,
		})
		.await?;

	match allocate_res {
		AllocateActorOutput::Allocated {
			runner_id,
			runner_workflow_id,
		} => {
			// Bump the autoscaler so it can scale up
			ctx.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
				.send()
				.await?;

			ctx.signal(crate::workflows::runner::Command {
				inner: protocol::Command::CommandStartActor(protocol::CommandStartActor {
					actor_id: input.actor_id.to_string(),
					generation,
					config: protocol::ActorConfig {
						name: input.name.clone(),
						key: input.key.clone(),
						// HACK: We should not use dynamic timestamp here, but we don't validate if signal data
						// changes (like activity inputs) so this is fine for now.
						create_ts: util::timestamp::now(),
						input: input
							.input
							.as_ref()
							.map(|x| BASE64_STANDARD.decode(x))
							.transpose()?,
					},
				}),
			})
			.to_workflow_id(runner_workflow_id)
			.send()
			.await?;

			Ok(SpawnActorOutput::Allocated {
				runner_id,
				runner_workflow_id,
			})
		}
		AllocateActorOutput::Pending {
			pending_allocation_ts,
		} => {
			// Bump the autoscaler so it can scale up
			ctx.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
				.send()
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

					ctx.signal(crate::workflows::runner::Command {
						inner: protocol::Command::CommandStartActor(protocol::CommandStartActor {
							actor_id: input.actor_id.to_string(),
							generation,
							config: protocol::ActorConfig {
								name: input.name.clone(),
								key: input.key.clone(),
								// HACK: We should not use dynamic timestamp here, but we don't validate if signal data
								// changes (like activity inputs) so this is fine for now.
								create_ts: util::timestamp::now(),
								input: input
									.input
									.as_ref()
									.map(|x| BASE64_STANDARD.decode(x))
									.transpose()?,
							},
						}),
					})
					.to_workflow_id(sig.runner_workflow_id)
					.send()
					.await?;

					Ok(SpawnActorOutput::Allocated {
						runner_id: sig.runner_id,
						runner_workflow_id: sig.runner_workflow_id,
					})
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

					Ok(SpawnActorOutput::Destroy)
				}
			}
		}
		AllocateActorOutput::Sleep => Ok(SpawnActorOutput::Sleep),
	}
}

/// Wrapper around `spawn_actor` that handles rescheduling retries. Returns true if the actor should be
/// destroyed.
pub async fn reschedule_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	state: &mut LifecycleState,
	force_reschedule: bool,
) -> Result<SpawnActorOutput> {
	tracing::debug!(actor_id=?input.actor_id, "rescheduling actor");

	// Determine next backoff sleep duration
	let mut backoff = util::backoff::Backoff::new_at(
		8,
		None,
		BASE_RETRY_TIMEOUT_MS,
		500,
		state.reschedule_state.retry_count,
	);

	let (now, reset) = ctx
		.v(2)
		.activity(CompareRetryInput {
			last_retry_ts: state.reschedule_state.last_retry_ts,
		})
		.await?;

	state.reschedule_state.retry_count = if reset {
		0
	} else {
		state.reschedule_state.retry_count + 1
	};
	state.reschedule_state.last_retry_ts = now;

	// // Don't sleep for first retry
	// if state.reschedule_state.retry_count > 0 {
	// 	let next = backoff.step().expect("should not have max retry");

	// 	// Sleep for backoff or destroy early
	// 	if let Some(_sig) = ctx
	// 		.listen_with_timeout::<Destroy>(Instant::from(next) - Instant::now())
	// 		.await?
	// 	{
	// 		tracing::debug!("destroying before actor start");

	// 		return Ok(SpawnActorOutput::Destroy);
	// 	}
	// }

	let next_generation = state.generation + 1;
	let spawn_res = spawn_actor(
		ctx,
		&input,
		next_generation,
		force_reschedule || state.wake_for_alarm,
	)
	.await?;

	if let SpawnActorOutput::Allocated {
		runner_id,
		runner_workflow_id,
	} = &spawn_res
	{
		state.generation = next_generation;
		state.runner_id = Some(*runner_id);
		state.runner_workflow_id = Some(*runner_workflow_id);

		// Reset gc timeout once allocated
		state.gc_timeout_ts = Some(util::timestamp::now() + ACTOR_START_THRESHOLD_MS);
	}

	Ok(spawn_res)
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
		.run(|tx| async move {
			let pending_alloc_key =
				keys::subspace().pack(&keys::ns::PendingActorByRunnerNameSelectorKey::new(
					input.namespace_id,
					input.runner_name_selector.clone(),
					input.pending_allocation_ts,
					input.actor_id,
				));

			let exists = tx.get(&pending_alloc_key, Serializable).await?.is_some();

			tx.clear(&pending_alloc_key);

			Ok(exists)
		})
		.custom_instrument(tracing::info_span!("actor_clear_pending_alloc_tx"))
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

	// If the last retry ts is more than RETRY_RESET_DURATION_MS ago, reset retry count
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
		.run(|tx| async move {
			let connectable_key = keys::actor::ConnectableKey::new(input.actor_id);
			tx.set(
				&keys::subspace().pack(&connectable_key),
				&connectable_key.serialize(())?,
			);

			Ok(())
		})
		.custom_instrument(tracing::info_span!("actor_set_started_tx"))
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
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			// Make not connectable
			tx.delete(&keys::actor::ConnectableKey::new(input.actor_id));

			tx.write(&keys::actor::SleepTsKey::new(input.actor_id), sleep_ts)?;

			Ok(())
		})
		.custom_instrument(tracing::info_span!("actor_set_sleeping_tx"))
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
