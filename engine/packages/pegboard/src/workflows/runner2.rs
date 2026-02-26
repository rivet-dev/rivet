use std::time::{Duration, Instant};

use futures_util::{FutureExt, StreamExt, TryStreamExt};
use gas::prelude::*;
use rand::prelude::SliceRandom;
use rivet_data::converted::RunnerByKeyKeyData;
use rivet_runner_protocol::{self as protocol, PROTOCOL_MK2_VERSION, versioned};
use universaldb::{
	options::{ConflictRangeType, StreamingMode},
	utils::IsolationLevel::*,
};
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

use crate::{keys, workflows::actor::Allocate};

const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
	pub runner_id: Id,
	pub namespace_id: Id,
	pub name: String,
	pub key: String,
	pub version: u32,
	pub total_slots: u32,
	pub protocol_version: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct State {
	namespace_id: Id,
	create_ts: i64,
}

impl State {
	fn new(namespace_id: Id, create_ts: i64) -> Self {
		State {
			namespace_id,
			create_ts,
		}
	}
}

/// Reason why the runner lifecycle loop exited.
#[derive(Debug, Serialize, Deserialize)]
enum RunnerStopReason {
	/// Runner was draining and completed.
	Draining,
	/// Runner connection expired (no recent ping).
	ConnectionLost,
}

#[workflow]
pub async fn pegboard_runner2(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.activity(InitInput {
		runner_id: input.runner_id,
		namespace_id: input.namespace_id,
		name: input.name.clone(),
		key: input.key.clone(),
		version: input.version,
		total_slots: input.total_slots,
		protocol_version: input.protocol_version,
		create_ts: ctx.create_ts(),
	})
	.await?;

	// Drain older runner versions if configured
	let drain_result = ctx
		.activity(DrainOlderVersionsInput {
			namespace_id: input.namespace_id,
			name: input.name.clone(),
			version: input.version,
		})
		.await?;
	for workflow_id in drain_result.older_runner_workflow_ids {
		ctx.signal(Stop {
			reset_actor_rescheduling: false,
		})
		.to_workflow_id(workflow_id)
		.send()
		.await?;
	}

	check_queue(ctx, input.namespace_id, &input.name).await?;

	let exit_reason = ctx
		.loope(LifecycleState::new(), |ctx, state| {
			let input = input.clone();

			async move {
				let runner_lost_threshold = ctx.config().pegboard().runner_lost_threshold();

				match ctx
					.listen_with_timeout::<Main>(runner_lost_threshold)
					.await?
				{
					Some(Main::Init(_)) => {
						if !state.draining {
							ctx.activity(MarkEligibleInput {
								runner_id: input.runner_id,
							})
							.await?;
						}

						check_queue(ctx, input.namespace_id, &input.name).await?;
					}
					Some(Main::CheckQueue(_)) => {
						check_queue(ctx, input.namespace_id, &input.name).await?;
					}
					Some(Main::Stop(sig)) => {
						handle_stopping(ctx, &input, state, sig.reset_actor_rescheduling).await?;
					}
					None => {
						let expired = ctx
							.activity(CheckExpiredInput {
								runner_id: input.runner_id,
								draining: state.draining,
							})
							.await?;

						if expired {
							return Ok(Loop::Break(RunnerStopReason::ConnectionLost));
						} else if state.draining {
							return Ok(Loop::Break(RunnerStopReason::Draining));
						} else {
							return Ok(Loop::Continue);
						}
					}
				}

				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await?;

	ctx.activity(ClearDbInput {
		runner_id: input.runner_id,
		name: input.name.clone(),
		key: input.key.clone(),
		update_state: RunnerState::Stopped,
	})
	.await?;

	let actors = ctx
		.activity(FetchRemainingActorsInput {
			runner_id: input.runner_id,
		})
		.await?;

	// Determine lost reason based on why the loop exited
	let lost_reason = match exit_reason {
		RunnerStopReason::ConnectionLost => {
			crate::workflows::actor::LostReason::RunnerConnectionLost
		}
		RunnerStopReason::Draining => crate::workflows::actor::LostReason::RunnerDrainingTimeout,
	};

	// Set all remaining actors as lost
	for (actor_id, generation) in actors {
		let res = ctx
			.signal(crate::workflows::actor::Lost {
				generation,
				force_reschedule: false,
				reset_rescheduling: false,
				reason: Some(lost_reason.clone()),
			})
			.to_workflow::<crate::workflows::actor::Workflow>()
			.tag("actor_id", actor_id)
			.graceful_not_found()
			.send()
			.await?;
		if res.is_none() {
			tracing::warn!(
				?actor_id,
				"actor workflow not found, likely already stopped"
			);
		}
	}

	// Close websocket connection (its unlikely to be open)
	ctx.activity(SendMessagesToRunnerInput {
		runner_id: input.runner_id,
		messages: vec![protocol::mk2::ToRunner::ToRunnerClose],
	})
	.await?;

	Ok(())
}

async fn check_queue(ctx: &mut WorkflowCtx, namespace_id: Id, name: &str) -> Result<()> {
	// Check for pending actors (which happen when there is not enough runner capacity)
	let res = ctx
		.activity(AllocatePendingActorsInput {
			namespace_id,
			name: name.to_string(),
		})
		.await?;

	// Dispatch pending allocs
	for alloc in res.allocations {
		ctx.signal(alloc.signal)
			.to_workflow::<crate::workflows::actor::Workflow>()
			.tag("actor_id", alloc.actor_id)
			.send()
			.await?;
	}

	Ok(())
}

async fn handle_stopping(
	ctx: &mut WorkflowCtx,
	input: &Input,
	state: &mut LifecycleState,
	reset_actor_rescheduling: bool,
) -> Result<()> {
	if !state.draining {
		// The workflow will enter a draining state where it can still process signals if
		// needed. After the runner lost threshold it will exit this loop and stop.
		state.draining = true;

		// Can't parallelize these two activities, requires reading from state
		ctx.activity(ClearDbInput {
			runner_id: input.runner_id,
			name: input.name.clone(),
			key: input.key.clone(),
			update_state: RunnerState::Draining,
		})
		.await?;

		let actors = ctx
			.activity(FetchRemainingActorsInput {
				runner_id: input.runner_id,
			})
			.await?;

		// Set all remaining actors as going away immediately
		if !actors.is_empty() {
			for (actor_id, generation) in &actors {
				let res = ctx
					.signal(crate::workflows::actor::GoingAway {
						generation: *generation,
						reset_rescheduling: reset_actor_rescheduling,
					})
					.to_workflow::<crate::workflows::actor::Workflow>()
					.tag("actor_id", actor_id)
					.graceful_not_found()
					.send()
					.await?;

				if res.is_none() {
					tracing::warn!(
						?actor_id,
						"actor workflow not found, likely already stopped"
					);
				}
			}
		}
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct LifecycleState {
	draining: bool,
}

impl LifecycleState {
	fn new() -> Self {
		LifecycleState { draining: false }
	}
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InitInput {
	runner_id: Id,
	namespace_id: Id,
	name: String,
	key: String,
	protocol_version: u16,
	version: u32,
	total_slots: u32,
	create_ts: i64,
}

#[activity(InitActivity)]
async fn init(ctx: &ActivityCtx, input: &InitInput) -> Result<()> {
	let mut state = ctx.state::<Option<State>>()?;

	*state = Some(State::new(input.namespace_id, input.create_ts));

	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let remaining_slots_key = keys::runner::RemainingSlotsKey::new(input.runner_id);
			let last_ping_ts_key = keys::runner::LastPingTsKey::new(input.runner_id);
			let workflow_id_key = keys::runner::WorkflowIdKey::new(input.runner_id);

			let (remaining_slots_entry, last_ping_ts_entry) = tokio::try_join!(
				tx.read_opt(&remaining_slots_key, Serializable),
				tx.read_opt(&last_ping_ts_key, Serializable),
			)?;
			let now = util::timestamp::now();

			// TODO: Do we still need to check if it already exists? this txn is only run once
			// See if key already exists
			let existing = if let (Some(remaining_slots), Some(last_ping_ts)) =
				(remaining_slots_entry, last_ping_ts_entry)
			{
				Some((remaining_slots, last_ping_ts))
			} else {
				// Initial insert
				None
			};

			let (remaining_slots, last_ping_ts) = if let Some(existing) = existing {
				existing
			}
			// NOTE: These properties are only inserted once
			else {
				tx.write(&workflow_id_key, ctx.workflow_id())?;

				tx.write(
					&keys::runner::NamespaceIdKey::new(input.runner_id),
					input.namespace_id,
				)?;

				tx.write(
					&keys::runner::NameKey::new(input.runner_id),
					input.name.clone(),
				)?;

				tx.write(
					&keys::runner::KeyKey::new(input.runner_id),
					input.key.clone(),
				)?;

				tx.write(
					&keys::runner::VersionKey::new(input.runner_id),
					input.version,
				)?;

				tx.write(&remaining_slots_key, input.total_slots)?;

				tx.write(
					&keys::runner::TotalSlotsKey::new(input.runner_id),
					input.total_slots,
				)?;

				tx.write(
					&keys::runner::CreateTsKey::new(input.runner_id),
					input.create_ts,
				)?;

				tx.write(&last_ping_ts_key, now)?;

				tx.write(
					&keys::runner::ProtocolVersionKey::new(input.runner_id),
					input.protocol_version,
				)?;

				// Populate ns indexes
				tx.write(
					&keys::ns::ActiveRunnerKey::new(
						input.namespace_id,
						input.create_ts,
						input.runner_id,
					),
					ctx.workflow_id(),
				)?;
				tx.write(
					&keys::ns::ActiveRunnerByNameKey::new(
						input.namespace_id,
						input.name.clone(),
						input.create_ts,
						input.runner_id,
					),
					ctx.workflow_id(),
				)?;
				tx.write(
					&keys::ns::AllRunnerKey::new(
						input.namespace_id,
						input.create_ts,
						input.runner_id,
					),
					ctx.workflow_id(),
				)?;
				tx.write(
					&keys::ns::AllRunnerByNameKey::new(
						input.namespace_id,
						input.name.clone(),
						input.create_ts,
						input.runner_id,
					),
					ctx.workflow_id(),
				)?;

				// Write name into namespace runner names list
				tx.write(
					&keys::ns::RunnerNameKey::new(input.namespace_id, input.name.clone()),
					(),
				)?;

				(input.total_slots, now)
			};

			// Set last connect ts
			tx.write(&keys::runner::ConnectedTsKey::new(input.runner_id), now)?;

			let remaining_millislots = (remaining_slots * 1000) / input.total_slots;

			// Insert into index (same as the `update_alloc_idx` op with `AddIdx`)
			tx.write(
				&keys::ns::RunnerAllocIdxKey::new(
					input.namespace_id,
					input.name.clone(),
					input.version,
					remaining_millislots,
					last_ping_ts,
					input.runner_id,
				),
				rivet_data::converted::RunnerAllocIdxKeyData {
					workflow_id: ctx.workflow_id(),
					remaining_slots,
					total_slots: input.total_slots,
					protocol_version: input.protocol_version,
				},
			)?;

			let runner_by_key_key = keys::ns::RunnerByKeyKey::new(
				input.namespace_id,
				input.name.clone(),
				input.key.clone(),
			);

			// Allocate self
			tx.write(
				&runner_by_key_key,
				RunnerByKeyKeyData {
					runner_id: input.runner_id,
					workflow_id: ctx.workflow_id(),
				},
			)?;

			Ok(())
		})
		.custom_instrument(tracing::info_span!("runner_init_tx"))
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct MarkEligibleInput {
	runner_id: Id,
}

#[activity(MarkEligible)]
async fn mark_eligible(ctx: &ActivityCtx, input: &MarkEligibleInput) -> Result<()> {
	// Mark eligible
	ctx.op(crate::ops::runner::update_alloc_idx::Input {
		runners: vec![crate::ops::runner::update_alloc_idx::Runner {
			runner_id: input.runner_id,
			action: crate::ops::runner::update_alloc_idx::Action::AddIdx,
		}],
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ClearDbInput {
	runner_id: Id,
	name: String,
	key: String,
	update_state: RunnerState,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
enum RunnerState {
	Draining,
	Stopped,
}

#[activity(ClearDb)]
async fn clear_db(ctx: &ActivityCtx, input: &ClearDbInput) -> Result<()> {
	let state = ctx.state::<State>()?;
	let namespace_id = state.namespace_id;
	let create_ts = state.create_ts;

	// TODO: Combine into a single udb txn
	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());
			let now = util::timestamp::now();

			// Clear runner by key idx if its still the current runner
			let runner_by_key_key =
				keys::ns::RunnerByKeyKey::new(namespace_id, input.name.clone(), input.key.clone());
			let runner_id = tx
				.read_opt(&runner_by_key_key, Serializable)
				.await?
				.map(|x| x.runner_id);
			if runner_id == Some(input.runner_id) {
				tx.delete(&runner_by_key_key);
			}

			match input.update_state {
				RunnerState::Draining => {
					tx.write(&keys::runner::DrainTsKey::new(input.runner_id), now)?;
					tx.write(&keys::runner::ExpiredTsKey::new(input.runner_id), now)?;
				}
				RunnerState::Stopped => {
					tx.write(&keys::runner::StopTsKey::new(input.runner_id), now)?;

					// Update namespace indexes
					tx.delete(&keys::ns::ActiveRunnerKey::new(
						namespace_id,
						create_ts,
						input.runner_id,
					));
					tx.delete(&keys::ns::ActiveRunnerByNameKey::new(
						namespace_id,
						input.name.clone(),
						create_ts,
						input.runner_id,
					));

					// Clear all actor data like commands
					tx.delete_key_subspace(&keys::runner::ActorDataSubspaceKey::new(
						input.runner_id,
					));
				}
			}

			Ok(())
		})
		.custom_instrument(tracing::info_span!("runner_clear_tx"))
		.await?;

	// Does not clear the data keys like last ping ts, just the allocation idx
	ctx.op(crate::ops::runner::update_alloc_idx::Input {
		runners: vec![crate::ops::runner::update_alloc_idx::Runner {
			runner_id: input.runner_id,
			action: crate::ops::runner::update_alloc_idx::Action::ClearIdx,
		}],
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct FetchRemainingActorsInput {
	runner_id: Id,
}

#[activity(FetchRemainingActors)]
async fn fetch_remaining_actors(
	ctx: &ActivityCtx,
	input: &FetchRemainingActorsInput,
) -> Result<Vec<(Id, u32)>> {
	let actors = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let actor_subspace =
				keys::subspace().subspace(&keys::runner::ActorKey::subspace(input.runner_id));

			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&actor_subspace).into()
				},
				Serializable,
			)
			.map(|res| {
				let (key, generation) = tx.read_entry::<keys::runner::ActorKey>(&res?)?;

				Ok((key.actor_id.into(), generation))
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("runner_fetch_remaining_actors_tx"))
		.await?;

	Ok(actors)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CheckExpiredInput {
	runner_id: Id,
	#[serde(default)]
	draining: bool,
}

#[activity(CheckExpired)]
async fn check_expired(ctx: &ActivityCtx, input: &CheckExpiredInput) -> Result<bool> {
	let runner_lost_threshold = ctx.config().pegboard().runner_lost_threshold();

	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let last_ping_ts = tx
				.read(
					&keys::runner::LastPingTsKey::new(input.runner_id),
					Serializable,
				)
				.await?;

			let now = util::timestamp::now();
			let expired = last_ping_ts < now - runner_lost_threshold;

			if expired {
				tx.write(&keys::runner::ExpiredTsKey::new(input.runner_id), now)?;
			}
			// TODO: remove this branch once runner alloc race bug is fixed
			else if !input.draining {
				let namespace_id_key = keys::runner::NamespaceIdKey::new(input.runner_id);
				let name_key = keys::runner::NameKey::new(input.runner_id);
				let version_key = keys::runner::VersionKey::new(input.runner_id);
				let remaining_slots_key =
					keys::runner::RemainingSlotsKey::new(input.runner_id);
				let total_slots_key = keys::runner::TotalSlotsKey::new(input.runner_id);
				let last_ping_ts_key = keys::runner::LastPingTsKey::new(input.runner_id);

				let (
					namespace_id_entry,
					name_entry,
					version_entry,
					remaining_slots_entry,
					total_slots_entry,
					last_ping_ts_entry,
				) = tokio::try_join!(
					tx.read_opt(&namespace_id_key, Snapshot),
					tx.read_opt(&name_key, Snapshot),
					tx.read_opt(&version_key, Snapshot),
					tx.read_opt(&remaining_slots_key, Snapshot),
					tx.read_opt(&total_slots_key, Snapshot),
					tx.read_opt(&last_ping_ts_key, Snapshot),
				)?;

				let (
					Some(namespace_id),
					Some(name),
					Some(version),
					Some(remaining_slots),
					Some(total_slots),
					Some(old_last_ping_ts),
				) = (
					namespace_id_entry,
					name_entry,
					version_entry,
					remaining_slots_entry,
					total_slots_entry,
					last_ping_ts_entry,
				)
				else {
					tracing::debug!(runner_id=?input.runner_id, "runner has not initiated yet");
					return Ok(expired);
				};

				let remaining_millislots = (remaining_slots * 1000) / total_slots;
				let old_alloc_key = keys::ns::RunnerAllocIdxKey::new(
					namespace_id,
					name.clone(),
					version,
					remaining_millislots,
					old_last_ping_ts,
					input.runner_id,
				);

				if !tx.exists(&old_alloc_key, Snapshot).await? {
					tracing::warn!(runner_id=?input.runner_id, "runner has no alloc idx entry yet is not expired nor draining");
				}
			}

			Ok(expired)
		})
		.custom_instrument(tracing::info_span!("runner_check_expired_tx"))
		.await
		.map_err(Into::into)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub(crate) struct AllocatePendingActorsInput {
	pub namespace_id: Id,
	pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AllocatePendingActorsOutput {
	pub allocations: Vec<ActorAllocation>,
	#[serde(default)]
	pub attempted: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ActorAllocation {
	pub actor_id: Id,
	pub signal: Allocate,
}

#[activity(AllocatePendingActors)]
pub(crate) async fn allocate_pending_actors(
	ctx: &ActivityCtx,
	input: &AllocatePendingActorsInput,
) -> Result<AllocatePendingActorsOutput> {
	// First, fetch all of the pending actors with a snapshot read
	let mut pending_actors = ctx
		.udb()?
		.run(|tx| async move {
			let start = Instant::now();
			let tx = tx.with_subspace(keys::subspace());

			let pending_actor_subspace = keys::subspace().subspace(
				&keys::ns::PendingActorByRunnerNameSelectorKey::subspace(
					input.namespace_id,
					input.name.clone(),
				),
			);
			let mut queue_stream = tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&pending_actor_subspace).into()
				},
				// NOTE: This is not Serializable because we don't want to conflict with all of the keys, just
				// the one we choose
				Snapshot,
			);

			let mut pending_actors = Vec::new();

			loop {
				if start.elapsed() > EARLY_TXN_TIMEOUT {
					tracing::warn!("timed out reading pending actors queue");
					break;
				}

				let Some(queue_entry) = queue_stream.try_next().await? else {
					break;
				};

				pending_actors.push(
					tx.read_entry::<keys::ns::PendingActorByRunnerNameSelectorKey>(&queue_entry)?,
				);
			}

			Ok(pending_actors)
		})
		.custom_instrument(tracing::info_span!("runner_fetch_pending_actors_tx"))
		.await?;

	// Shuffle for good measure
	pending_actors.shuffle(&mut rand::thread_rng());

	let attempted = pending_actors.len();
	let runner_eligible_threshold = ctx.config().pegboard().runner_eligible_threshold();
	let actor_allocation_candidate_sample_size = ctx
		.config()
		.pegboard()
		.actor_allocation_candidate_sample_size();

	// NOTE: This txn should closely resemble the one found in the allocate_actor activity of the actor wf
	// Split the allocation of each actor into a separate txn. this reduces the scope of each individual txn
	// which reduces conflict rate
	let allocations = futures_util::stream::iter(pending_actors)
		.map(|(queue_key, generation)| async move {
			let queue_key = &queue_key;

			ctx.udb()?
				.run(|tx| async move {
					let start = Instant::now();
					let tx = tx.with_subspace(keys::subspace());
					let ping_threshold_ts = util::timestamp::now() - runner_eligible_threshold;

					// Re-check that the queue key still exists in this txn
					if !tx.exists(&queue_key, Snapshot).await? {
						return Ok(None);
					}

					let runner_alloc_subspace =
						keys::subspace().subspace(&keys::ns::RunnerAllocIdxKey::subspace(
							input.namespace_id,
							input.name.clone(),
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
					let mut candidates = Vec::with_capacity(actor_allocation_candidate_sample_size);

					// Select valid runner candidates for allocation
					loop {
						if start.elapsed() > EARLY_TXN_TIMEOUT {
							tracing::warn!("timed out allocating pending actors");
							break;
						}

						let Some(entry) = stream.try_next().await? else {
							break;
						};

						let (old_runner_alloc_key, old_runner_alloc_key_data) =
							tx.read_entry::<keys::ns::RunnerAllocIdxKey>(&entry)?;

						if let Some(highest_version) = highest_version {
							// We have passed all of the runners with the highest version. This is reachable if
							// the ping of the highest version runners makes them ineligible
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

						// Ignore runners without valid ping
						if old_runner_alloc_key.last_ping_ts < ping_threshold_ts {
							continue;
						}

						candidates.push((old_runner_alloc_key, old_runner_alloc_key_data));

						// Max candidate size reached
						if candidates.len() >= actor_allocation_candidate_sample_size {
							break;
						}
					}

					// No candidates, allocation cannot be made
					if candidates.is_empty() {
						return Ok(None);
					}

					// Select a candidate at random, weighted by remaining slots
					let (old_runner_alloc_key, old_runner_alloc_key_data) = candidates
						.choose_weighted(&mut rand::thread_rng(), |(key, _)| {
							key.remaining_millislots
						})?;

					// Add read conflict only for this runner key
					tx.add_conflict_key(&old_runner_alloc_key, ConflictRangeType::Read)?;
					tx.delete(&old_runner_alloc_key);

					// Add read conflict and delete the queue key
					tx.add_conflict_key(&queue_key, ConflictRangeType::Read)?;
					tx.delete(&queue_key);

					let new_remaining_slots =
						old_runner_alloc_key_data.remaining_slots.saturating_sub(1);
					let new_remaining_millislots =
						(new_remaining_slots * 1000) / old_runner_alloc_key_data.total_slots;

					// Write new allocation key with 1 less slot
					tx.write(
						&keys::ns::RunnerAllocIdxKey::new(
							input.namespace_id,
							input.name.clone(),
							old_runner_alloc_key.version,
							new_remaining_millislots,
							old_runner_alloc_key.last_ping_ts,
							old_runner_alloc_key.runner_id,
						),
						rivet_data::converted::RunnerAllocIdxKeyData {
							workflow_id: old_runner_alloc_key_data.workflow_id,
							remaining_slots: new_remaining_slots,
							total_slots: old_runner_alloc_key_data.total_slots,
							protocol_version: old_runner_alloc_key_data.protocol_version,
						},
					)?;

					// Update runner record
					tx.write(
						&keys::runner::RemainingSlotsKey::new(old_runner_alloc_key.runner_id),
						new_remaining_slots,
					)?;

					// Set runner id of actor
					tx.write(
						&keys::actor::RunnerIdKey::new(queue_key.actor_id),
						old_runner_alloc_key.runner_id,
					)?;

					// Insert actor index key
					tx.write(
						&keys::runner::ActorKey::new(
							old_runner_alloc_key.runner_id,
							queue_key.actor_id,
						),
						generation,
					)?;

					return Ok(Some(ActorAllocation {
						actor_id: queue_key.actor_id,
						signal: Allocate {
							runner_id: old_runner_alloc_key.runner_id,
							runner_workflow_id: old_runner_alloc_key_data.workflow_id,
							runner_protocol_version: Some(
								old_runner_alloc_key_data.protocol_version,
							),
						},
					}));
				})
				.custom_instrument(tracing::info_span!("runner_allocate_pending_actors_tx"))
				.await
		})
		.buffer_unordered(1024)
		.filter_map(|res| {
			// Gracefully handle failures because we do not want to fail the entire activity if some
			// allocations were successful
			match res {
				Ok(alloc) => std::future::ready(alloc),
				Err(err) => {
					tracing::error!(?err, "failure during pending actor allocation");

					std::future::ready(None)
				}
			}
		})
		.collect()
		.await;

	Ok(AllocatePendingActorsOutput {
		allocations,
		attempted,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SendMessagesToRunnerInput {
	runner_id: Id,
	messages: Vec<protocol::mk2::ToRunner>,
}

#[activity(SendMessagesToRunner)]
async fn send_messages_to_runner(
	ctx: &ActivityCtx,
	input: &SendMessagesToRunnerInput,
) -> Result<()> {
	let receiver_subject =
		crate::pubsub_subjects::RunnerReceiverSubject::new(input.runner_id).to_string();

	for message in &input.messages {
		let message_serialized = versioned::ToRunnerMk2::wrap_latest(message.clone())
			.serialize_with_embedded_version(PROTOCOL_MK2_VERSION)?;

		ctx.ups()?
			.publish(&receiver_subject, &message_serialized, PublishOpts::one())
			.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DrainOlderVersionsInput {
	namespace_id: Id,
	name: String,
	version: u32,
}

#[activity(DrainOlderVersions)]
async fn drain_older_versions(
	ctx: &ActivityCtx,
	input: &DrainOlderVersionsInput,
) -> Result<crate::ops::runner::drain::Output> {
	ctx.op(crate::ops::runner::drain::Input {
		namespace_id: input.namespace_id,
		name: input.name.clone(),
		version: input.version,
		// Signals are sent by the workflow directly
		send_runner_stop_signals: false,
	})
	.await
}

#[signal("pegboard_runner_init")]
pub struct Init {}

#[signal("pegboard_runner_check_queue")]
pub struct CheckQueue {}

#[signal("pegboard_runner_stop")]
pub struct Stop {
	pub reset_actor_rescheduling: bool,
}

join_signal!(Main {
	Init,
	CheckQueue,
	Stop,
	// Comment to prevent invalid formatting
});
