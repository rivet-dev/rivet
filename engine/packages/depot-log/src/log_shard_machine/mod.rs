//! One state machine owns admission, replica state and disk write barriers for every log in a shard.
//! Callers deliver admitted commands; the storage executor returns sync outcomes.
//! Rejections carrying replica state wait behind the same disk write barrier as successes.

pub mod effect;
pub mod event;
pub mod executor;
pub mod runner;
pub mod snapshot;
pub mod state;
pub mod types;

use self::{
	effect::LogShardEffect,
	event::LogShardEvent,
	snapshot::{
		LoadedVolumeReplicaView, LogShardSnapshot, Resource, ResourceUsage, WriteToDiskCycle,
	},
	state::{
		LogShardPhase, LogShardState, RecoveryStep, disk_segments::CheckpointCycle, fail,
		loaded_volume_replica::admit, memory_batches::dispatch,
	},
	types::LogShardTimer,
};
use depot_types::VolumeLogKey;
use rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState;
use rivet_env::Clocks;
use rivet_state_machine::{Entropy, NoSlot};
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEvent,
};

type Effects = rivet_state_machine_ephemeral::Effects<LogShardEffect, LogShardTimer>;

pub struct LogShardMachine;

/// Cheap reads for observers that track a few replicas. `snapshot` copies every loaded
/// replica, which is too slow to call per event on a shard holding thousands of volumes.
impl LogShardMachine {
	pub fn phase(state: &LogShardState) -> &LogShardPhase {
		&state.phase
	}

	/// One loaded replica's accepted and disk states, or none if it is not loaded.
	pub fn loaded_volume(
		state: &LogShardState,
		volume_log_key: &VolumeLogKey,
	) -> Option<LoadedVolumeReplicaView> {
		state
			.loaded_volume_replicas
			.get(volume_log_key)
			.map(|replica| LoadedVolumeReplicaView {
				volume_log_key: replica.volume_log_key.clone(),
				memory_state: replica.state().clone(),
				disk_state: replica.disk_state.clone(),
			})
	}

	/// Every loaded replica's accepted and disk states by reference, in no fixed order.
	pub fn loaded_volumes(
		state: &LogShardState,
	) -> impl Iterator<
		Item = (
			&VolumeLogKey,
			&VolumeLogReplicaState,
			Option<&VolumeLogReplicaState>,
		),
	> {
		state.loaded_volume_replicas.values().map(|replica| {
			(
				&replica.volume_log_key,
				replica.state(),
				replica.disk_state.as_ref(),
			)
		})
	}

	pub fn loaded_volume_count(state: &LogShardState) -> usize {
		state.loaded_volume_replicas.len()
	}

	pub fn write_to_disk(state: &LogShardState) -> WriteToDiskCycle {
		state
			.memory_batches
			.writing_to_disk
			.as_ref()
			.map_or(WriteToDiskCycle::Idle, |b| {
				WriteToDiskCycle::WritingToDisk {
					first_log_seq: b.first_log_seq,
					last_log_seq: b.last_log_seq,
				}
			})
	}
}

impl EphemeralStateMachine for LogShardMachine {
	type Slot = NoSlot;
	type State = LogShardState;
	type Event = LogShardEvent;
	type Effect = LogShardEffect;
	type Snapshot = LogShardSnapshot;
	type Timer = LogShardTimer;

	fn inbox_capacity(state: &LogShardState) -> usize {
		state.config.inbox_capacity
	}

	const DEBUG_NAME: &'static str = "log_shard_machine";

	fn timer_name(timer: &Self::Timer) -> &'static str {
		match timer {
			LogShardTimer::ManifestRefresh(..) => "manifest_refresh",
			LogShardTimer::Read(..) => "read",
			LogShardTimer::Recover => "recover",
			LogShardTimer::MarkRepairing => "mark_repairing",
			LogShardTimer::WriteToDisk { .. } => "write_to_disk",
			LogShardTimer::WriteCheckpoint { .. } => "write_checkpoint",
			LogShardTimer::RemoveSegments => "remove_segments",
			LogShardTimer::ReleaseIdlePayloads => "release_idle_payloads",
			LogShardTimer::AppendNotices => "append_notices",
			LogShardTimer::NoticeRetry => "notice_retry",
			LogShardTimer::NoticeDeadline(..) => "notice_deadline",
		}
	}

	/// Starts recovery before admitting requests to the shard. `Recover` bounds the time
	/// between the scan's progress reports, not the whole scan.
	fn start(state: &mut LogShardState, _: Clocks, entropy: Entropy, effects: &mut Effects) {
		if matches!(
			state.phase,
			LogShardPhase::Recovering(RecoveryStep::Scanning)
		) {
			effects.arm_timer_after(LogShardTimer::Recover, state.config.recovery_timeout_ms);
			effects.execute(LogShardEffect::RecoverRequest {
				disk_incarnation: *entropy.rng().uuid_v4().as_bytes(),
			});
		}
	}

	/// Applies one input or I/O completion, then schedules the next disk write barrier.
	fn on_event(
		state: &mut LogShardState,
		event: EphemeralStateMachineEvent<Self>,
		effects: &mut Effects,
	) {
		// Identify completions that can unblock maintenance after their state is installed.
		let mut rng = event.entropy.rng();
		let maintenance = matches!(
			&event.body,
			RuntimeEvent::Input(
				LogShardEvent::WriteToDiskResponse { .. }
					| LogShardEvent::WriteCheckpointResponse { .. }
					| LogShardEvent::RemoveSegmentsResponse { .. }
					| LogShardEvent::RecoverResponse(_)
					| LogShardEvent::MarkRepairingResponse(_)
			)
		);
		if matches!(
			state.phase,
			LogShardPhase::Failed { .. } | LogShardPhase::Stopped
		) {
			// Even terminal shards answer new requests so callers can reroute promptly.
			match event.body {
				RuntimeEvent::Input(LogShardEvent::Submitted { requests, .. }) => {
					for request in requests {
						admit(state, request, &mut rng, effects);
					}
				}
				RuntimeEvent::Input(_) | RuntimeEvent::Timer(_) => {}
			}
			return;
		}

		// Route each input to the loop that owns its state and correlation checks.
		match event.body {
			RuntimeEvent::Input(LogShardEvent::TabletManifestUpdated { tablet_manifest }) => {
				state::retirement::install(state, tablet_manifest);
				state::tablet_replica_sets::dispatch(state);
				state::retirement::dispatch(state);
				state::manifest_wait::resume(state, &mut rng, effects);
			}
			RuntimeEvent::Input(LogShardEvent::ReadResponse { request_id, result }) => {
				state::reads::completed(state, request_id, result, effects);
			}
			RuntimeEvent::Timer(LogShardTimer::ManifestRefresh(request_id)) => {
				state::manifest_wait::expired(state, request_id, effects);
			}
			RuntimeEvent::Timer(LogShardTimer::Read(request_id)) => {
				state::reads::timed_out(state, request_id, effects);
			}
			RuntimeEvent::Input(LogShardEvent::Submitted {
				submitted_boot_ms,
				requests,
			}) => {
				state::admission::submitted(state, submitted_boot_ms, requests, &mut rng, effects);
			}
			RuntimeEvent::Input(LogShardEvent::WriteToDiskResponse {
				last_log_seq,
				result,
			}) => {
				state::memory_batches::written_to_disk(state, last_log_seq, result, effects);
			}
			RuntimeEvent::Timer(LogShardTimer::WriteToDisk { last_log_seq: seq }) => {
				state::memory_batches::timed_out(state, seq, effects);
			}
			RuntimeEvent::Timer(LogShardTimer::ReleaseIdlePayloads) => {
				state::payloads::idle_due(state, effects.now_boot_ms());
			}
			RuntimeEvent::Timer(LogShardTimer::AppendNotices) => {
				state::status_notices::appended_due(state, effects.now_boot_ms());
			}
			RuntimeEvent::Timer(LogShardTimer::NoticeRetry) => {
				state::status_notices::retry_due(state, effects.now_boot_ms());
			}
			RuntimeEvent::Timer(LogShardTimer::NoticeDeadline(worker)) => {
				state::status_notices::deadline_due(state, worker, effects);
			}
			RuntimeEvent::Input(LogShardEvent::RecoverResponse(result))
				if matches!(
					state.phase,
					LogShardPhase::Recovering(RecoveryStep::Scanning)
				) =>
			{
				state::recovery::recovered(state, result, &mut rng, effects);
			}
			RuntimeEvent::Input(LogShardEvent::MarkRepairingResponse(result))
				if matches!(
					state.phase,
					LogShardPhase::Recovering(RecoveryStep::MarkingRepairing)
				) =>
			{
				state::recovery::marked_repairing(state, result, effects);
			}
			RuntimeEvent::Input(LogShardEvent::RecoverProgress) => {
				if matches!(
					state.phase,
					LogShardPhase::Recovering(RecoveryStep::Scanning)
				) {
					effects
						.arm_timer_after(LogShardTimer::Recover, state.config.recovery_timeout_ms);
				}
			}
			// Completed or superseded recovery operations cannot restore serving authority.
			RuntimeEvent::Input(
				LogShardEvent::RecoverResponse(_) | LogShardEvent::MarkRepairingResponse(_),
			) => {}
			RuntimeEvent::Timer(LogShardTimer::Recover)
				if matches!(
					state.phase,
					LogShardPhase::Recovering(RecoveryStep::Scanning)
				) =>
			{
				fail(state, "recovery timed out".into(), effects)
			}
			RuntimeEvent::Timer(LogShardTimer::MarkRepairing)
				if matches!(
					state.phase,
					LogShardPhase::Recovering(RecoveryStep::MarkingRepairing)
				) =>
			{
				fail(state, "repair marking timed out".into(), effects)
			}
			RuntimeEvent::Timer(LogShardTimer::Recover | LogShardTimer::MarkRepairing) => {}

			RuntimeEvent::Input(LogShardEvent::Drain) => {
				state::drain(state, effects);
			}
			RuntimeEvent::Input(LogShardEvent::NotifyMaintainerResponse {
				worker,
				notice_id,
				outcome,
			}) => {
				state::status_notices::notified(state, worker, notice_id, outcome, effects);
			}
			RuntimeEvent::Input(LogShardEvent::WriteCheckpointResponse {
				checkpoint_bytes,
				last_included_log_seq,
				result,
			}) => {
				state::disk_segments::checkpoint_written(
					state,
					last_included_log_seq,
					checkpoint_bytes,
					result,
					effects,
				);
			}
			RuntimeEvent::Input(LogShardEvent::RemoveSegmentsResponse { result }) => {
				state::disk_segments::segments_removed(state, result, effects);
			}
			RuntimeEvent::Timer(LogShardTimer::RemoveSegments) => {
				if !state.disk_segments.removing.is_empty() {
					fail(state, "segment removal timed out".into(), effects);
				}
			}
			RuntimeEvent::Timer(LogShardTimer::WriteCheckpoint {
				last_included_log_seq: seq,
			}) => {
				if matches!(state.disk_segments.checkpoint, CheckpointCycle::Writing { last_included_log_seq, .. } if last_included_log_seq == seq)
				{
					fail(state, "checkpoint timed out".into(), effects);
				}
			}
		}

		// Retirement also runs after recovery and each disk barrier, within normal write budgets.
		state::tablet_replica_sets::dispatch(state);
		state::retirement::dispatch(state);

		// Newly admitted work or a completed barrier may make the next batch runnable.
		if maintenance
			&& matches!(
				state.phase,
				LogShardPhase::Serving | LogShardPhase::Draining
			) {
			state::disk_segments::dispatch_maintenance(state, effects);
		}
		dispatch(state, effects);

		// Replica states that a sync changed go to their maintainers.
		state::status_notices::dispatch(state, effects);
		state::status_notices::arm(state, effects);

		// Released retirement fences leave the index one bounded slice per event.
		state.retired_volumes.drain_step();

		// Releasing payloads only drops memory, so it runs after the batch dispatch.
		state::payloads::enforce(state);
		state::payloads::arm(state, effects.now_boot_ms(), effects);
	}

	/// Each admitted request costs the configured simulated time, and each reply a disk write
	/// releases costs half of it. Refusing a request before any work costs a small fraction,
	/// as it does on a real core.
	fn simulated_cost_us(state: &LogShardState, event: &EphemeralStateMachineEvent<Self>) -> u64 {
		let cost = state.config.simulated_request_cost_us;
		match &event.body {
			RuntimeEvent::Input(LogShardEvent::WriteToDiskResponse { .. }) => {
				let replies = state
					.memory_batches
					.writing_to_disk
					.as_ref()
					.map_or(0, |write| write.pending.waiting_replies().len() as u64);
				replies.saturating_mul(cost / 2)
			}
			RuntimeEvent::Input(LogShardEvent::Submitted {
				submitted_boot_ms,
				requests,
			}) => {
				let waited_ms = event.now.boot_ms.saturating_sub(*submitted_boot_ms);
				requests
					.iter()
					.map(|request| {
						if state::admission::sheds(state, waited_ms, request) {
							cost / state::admission::SIMULATED_REFUSAL_COST_DIVISOR
						} else {
							cost
						}
					})
					.sum()
			}
			RuntimeEvent::Input(_) | RuntimeEvent::Timer(_) => 0,
		}
	}

	fn snapshot(state: &LogShardState) -> LogShardSnapshot {
		LogShardSnapshot {
			retained_entries: state.payloads.entries,
			retained_entry_bytes: state.payloads.entry_bytes,
			undelivered_status_notices: state.status_notices.undelivered() as u64,
			synced_log_seq: state::payloads::synced_through(state),
			oldest_pinned_batch_log_seq: state.segment_pins.oldest(),
			segments: state.disk_segments.segments().len() as u32,
			disk_incarnation: state.disk_incarnation,
			phase: state.phase.clone(),
			resources: vec![
				ResourceUsage {
					resource: Resource::PendingRequests,
					used: state
						.memory_batches
						.pending()
						.map(|b| b.waiting_replies().len() as u64)
						.sum::<u64>() + state.reads.len() as u64
						+ state.manifest_waits.len() as u64,
					capacity: state.config.max_memory_requests as u64,
				},
				ResourceUsage {
					resource: Resource::PendingBytes,
					used: state::reads::occupied_bytes(state),
					capacity: state.config.max_memory_bytes,
				},
				ResourceUsage {
					resource: Resource::DiskBytes,
					used: state.disk_segments.disk_bytes(),
					capacity: state.config.max_disk_bytes,
				},
				ResourceUsage {
					resource: Resource::Replicas,
					used: state.loaded_volume_replicas.len() as u64,
					capacity: state.config.max_loaded_volumes as u64,
				},
				ResourceUsage {
					resource: Resource::QuarantinedBytes,
					used: state.disk_segments.quarantined_bytes,
					capacity: state.config.max_disk_bytes,
				},
				ResourceUsage {
					resource: Resource::DiskReads,
					used: state.reads.len() as u64,
					capacity: u64::from(state.config.max_disk_reads),
				},
				ResourceUsage {
					resource: Resource::AdmissionWaitMs,
					used: state.last_admission_wait_ms,
					capacity: state.config.max_admission_wait_ms,
				},
				ResourceUsage {
					resource: Resource::RetainedPayloadBytes,
					used: state.payloads.retained_bytes,
					capacity: state.config.max_retained_payload_bytes,
				},
			],
			write_to_disk: state.memory_batches.writing_to_disk.as_ref().map_or(
				WriteToDiskCycle::Idle,
				|b| WriteToDiskCycle::WritingToDisk {
					first_log_seq: b.first_log_seq,
					last_log_seq: b.last_log_seq,
				},
			),
			filling_requests: state.memory_batches.filling.waiting_replies().len() as u32,
			checkpoint: state.disk_segments.checkpoint.clone(),
			loaded_volumes: if state.config.publish_replica_views {
				// The replica table keeps no key order, so the opt-in view sorts its copy.
				let mut views = state
					.loaded_volume_replicas
					.values()
					.map(|r| LoadedVolumeReplicaView {
						volume_log_key: r.volume_log_key.clone(),
						memory_state: r.state().clone(),
						disk_state: r.disk_state.clone(),
					})
					.collect::<Vec<_>>();
				views.sort_by(|a, b| a.volume_log_key.cmp(&b.volume_log_key));
				views
			} else {
				Vec::new()
			},
		}
	}

	fn debug_validate(state: &LogShardState) -> anyhow::Result<()> {
		state::payloads::validate(state)?;
		anyhow::ensure!(
			state
				.memory_batches
				.pending()
				.map(|b| b.waiting_replies().len())
				.sum::<usize>()
				+ state.reads.len()
				+ state.manifest_waits.len()
				<= state.config.max_memory_requests as usize,
			"request limit exceeded"
		);
		anyhow::ensure!(
			state::reads::occupied_bytes(state) <= state.config.max_memory_bytes,
			"request byte limit exceeded"
		);
		#[cfg(debug_assertions)]
		state.validate_incremental()?;

		Ok(())
	}
}
