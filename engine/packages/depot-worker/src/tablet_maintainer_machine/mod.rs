//! One grant-fenced owner discovers loaded volumes and coordinates their lifecycle.

pub mod effect;
pub mod event;
#[cfg(not(target_arch = "wasm32"))]
pub mod executor;
#[cfg(not(target_arch = "wasm32"))]
pub mod runner;
pub mod state;
pub mod types;

use self::{
	effect::MaintainerEffect,
	event::MaintainerEvent,
	state::{MaintainerState, Operation},
	types::*,
};
use crate::tablet_maintainer_machine::types::MAX_COLD_BYTES;
use rivet_data::generated::depot_kv_command_v1::{KvCommandResponse, OverloadReason};
use rivet_data::generated::depot_storage_worker_protocol_v1::{
	RejectResponse, StorageWorkerResponse,
};
use rivet_env::Clocks;
use rivet_state_machine::{Entropy, NoSlot};
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEvent,
};
use serde::{Deserialize, Serialize};

pub type Effects = rivet_state_machine_ephemeral::Effects<MaintainerEffect, MaintainerTimer>;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaintainerSnapshot {
	pub phase: state::MaintainerPhase,
	/// A deterministic recovery failure requiring a different placement or capacity.
	pub recovery_failure: Option<state::RecoveryFailure>,
	/// Latest core-accepted membership, also offered to grant-owned command writers.
	pub replica_set: rivet_data::generated::shared_depot_types_v1::TabletReplicaSet,
	/// Both child stores exist and the coordinator accepted their checkpoint.
	pub split_prepared: bool,
	pub loaded_volumes: u32,
	pub unloading_volumes: u32,
	pub pending_requests: u32,
	pub listed_members: u32,
	/// Volume open requests the core has taken from the inbox since the grant started.
	/// The submitter subtracts it from the opens it queued to count those still waiting.
	pub opens_received: u64,
	pub store: StoreSnapshot,
	pub alarms: AlarmSnapshot,
	pub signals: SignalSnapshot,
}

/// Values the runner reports as signal events and gauges, read from state in constant time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignalSnapshot {
	/// Log bytes this maintainer holds above cold: applied bytes no pass has written to cold
	/// yet, plus entries read for apply and not yet applied. Log entries no read has reached
	/// are not counted, so after a move this starts low and rises while replay reads them.
	pub above_cold_bytes: u64,
	/// Recovered maintainer log ops are still waiting to apply.
	pub maintainer_log_replaying: bool,
	/// No apply, apply read, carried entry or queued store work is outstanding.
	pub apply_idle: bool,
	/// The replica set's learner, while the set is learning or joint.
	pub learner: Option<[u8; 16]>,
	/// Loaded volumes the membership phase has evaluated and still lack its proof.
	pub uncovered_volumes: u32,
}

/// Sampled alarm usage and configured bounds of one tablet's frontier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AlarmSnapshot {
	pub candidates: u32,
	pub operations: u32,
	pub parked: u32,
	pub parked_capacity: u32,
	pub candidate_capacity: u32,
	pub operation_capacity: u32,
}

impl rivet_state_machine::Snapshot for MaintainerSnapshot {}

/// Store admission pressure, published for process-wide gauges only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StoreSnapshot {
	/// Store work waiting in order behind the store queue head.
	pub queued_work: u32,
	/// Session reads waiting only for a read slot.
	pub queued_session_reads: u32,
	/// Tablet-wide operations in flight other than a flush: opening, closing, split
	/// preparation, forks, checkpoints, sweeps and pending-work scans. Keyed work,
	/// passes and flushes are counted separately.
	pub exclusive_in_flight: u32,
	/// Filesystem applies and loads in flight.
	pub keyed_in_flight: u32,
	/// Cold write passes in flight.
	pub passes_in_flight: u32,
	/// Flushes writing their records into the store.
	pub flush_staging_in_flight: u32,
	/// Flushes making their staged records durable.
	pub flush_uploading_in_flight: u32,
	pub ordered_reads_in_flight: u32,
	pub session_reads_in_flight: u32,
	/// Configured read slots shared by ordered and session reads: the client window.
	pub read_capacity: u32,
	/// Configured session reads of one volume while another volume's reads wait; zero is off.
	pub per_volume_read_capacity: u32,
	/// Fork source reads against held images in flight.
	pub held_image_reads_in_flight: u32,
	/// Replica log reads for applies in flight.
	pub replica_log_reads_in_flight: u32,
	/// Fork jobs admitted and not yet finished.
	pub fork_jobs: u32,
	/// Fork cleanup jobs admitted and not yet finished.
	pub fork_cleanup_jobs: u32,
	/// Configured size of each background window: held-image reads, replica log reads,
	/// fork jobs and fork cleanup jobs.
	pub background_read_capacity: u32,
	/// Configured slots for filesystem applies and loads.
	pub keyed_capacity: u32,
	/// Configured flushes that may stage at once.
	pub flush_staging_capacity: u32,
	/// Configured flushes in flight at once, staging or uploading.
	pub flush_upload_capacity: u32,
	/// Configured queued items one admission pass looks at.
	pub admission_scan_limit: u32,
	/// Whether the queue is longer than one admission pass looks, so work past the limit
	/// cannot start ahead of blocked work in front of it.
	pub scan_limited: bool,
	/// Queued store work by kind.
	pub queued_by_kind: state::store::QueueSample,
	/// Why the oldest queued store item cannot start, or `None` when the queue is empty
	/// or its head may start.
	pub head_block: Option<state::store::AdmissionBlock>,
	/// Payload bytes of initial entries unacknowledged creations retain for their retries.
	pub initial_bytes_held: u64,
	/// The configured bound on `initial_bytes_held`.
	pub initial_bytes_capacity: u64,
	/// Cumulative opens whose initial writes were withheld, by budget or by another grant.
	pub initial_writes_withheld: u64,
	/// Volumes loaded, including incarnations being deleted that reserve a slot.
	pub loaded_volumes: u32,
	/// The configured bound on `loaded_volumes`.
	pub loaded_volume_capacity: u32,
}

pub struct TabletMaintainerMachine;

impl EphemeralStateMachine for TabletMaintainerMachine {
	type Slot = NoSlot;
	type State = MaintainerState;
	type Event = MaintainerEvent;
	type Effect = MaintainerEffect;
	type Snapshot = MaintainerSnapshot;
	type Timer = MaintainerTimer;
	const DEBUG_NAME: &'static str = "tablet_maintainer_machine";

	fn inbox_capacity(state: &Self::State) -> usize {
		state.config.inbox_capacity as usize
	}

	fn timer_name(timer: &Self::Timer) -> &'static str {
		match timer {
			MaintainerTimer::Alarms => "alarms",
			MaintainerTimer::IdleUnload => "idle_unload",
			MaintainerTimer::OpenRead(..) => "open_read",
			MaintainerTimer::ForkRequest(..) => "fork_request",
			MaintainerTimer::ForkRetry(..) => "fork_retry",
			MaintainerTimer::MaterializeRetry(..) => "materialize_retry",
			MaintainerTimer::PendingWorkTick => "pending_work_tick",
			MaintainerTimer::HeldSweepRetry => "held_sweep_retry",
			MaintainerTimer::ForkCleanupDue => "fork_cleanup_due",
			MaintainerTimer::RetentionDue => "retention_due",
			MaintainerTimer::CheckpointRequest(..) => "checkpoint_request",
			MaintainerTimer::ForkReferenceRequest(..) => "fork_reference_request",
			MaintainerTimer::FilesystemDue(..) => "filesystem_due",
			MaintainerTimer::FilesystemRetry(..) => "filesystem_retry",
			MaintainerTimer::FilesystemFlushBatch => "filesystem_flush_batch",
			MaintainerTimer::OpenRequest(..) => "open_request",
			MaintainerTimer::RepairWait(..) => "repair_wait",
			MaintainerTimer::CatchUp => "catch_up",
			MaintainerTimer::DeleteRequest(..) => "delete_request",
			MaintainerTimer::DeleteRetry(..) => "delete_retry",
			MaintainerTimer::SplitRetry => "split_retry",
			MaintainerTimer::RetiredRetry(..) => "retired_retry",
			MaintainerTimer::ParentLogReportRetry => "parent_log_report_retry",
			MaintainerTimer::VersionMarksRetry => "version_marks_retry",
			MaintainerTimer::UnloadRecoveryRetry(..) => "unload_recovery_retry",
			MaintainerTimer::UnloadRetry(..) => "unload_retry",
			MaintainerTimer::CloseNotices(..) => "close_notices",
			MaintainerTimer::HolderWatchRetry => "holder_watch_retry",
			MaintainerTimer::ReadRequest(..) => "read_request",
			MaintainerTimer::Operation(..) => "operation",
			MaintainerTimer::ListRetry(..) => "list_retry",
			MaintainerTimer::PromotionRetry(..) => "promotion_retry",
			MaintainerTimer::OrphanSweep => "orphan_sweep",
			MaintainerTimer::WriteToCold => "write_to_cold",
			MaintainerTimer::ColdBatch => "cold_batch",
			MaintainerTimer::CreationExpired(..) => "creation_expired",
			MaintainerTimer::Poll => "poll",
			MaintainerTimer::TrimRetry(..) => "trim_retry",
			MaintainerTimer::MemberRepairRetry(..) => "member_repair_retry",
			MaintainerTimer::MaintainerLogDue => "maintainer_log_due",
			MaintainerTimer::MemtableBudgetShed => "memtable_budget_shed",
			MaintainerTimer::MaintainerLogRetry(..) => "maintainer_log_retry",
		}
	}

	fn start(state: &mut MaintainerState, clocks: Clocks, _: Entropy, effects: &mut Effects) {
		state.now_boot_ms = clocks.boot_ms;
		state::opening::start(state, effects);
		// The recovery scan runs to completion once per maintainer start.
		crate::metrics::record_recovery_scan("pending_work", "start");
		effects.arm_timer_after(MaintainerTimer::PendingWorkTick, 1000);
	}

	/// Each admitted filesystem read request costs the configured simulated time, so reads
	/// queue behind one another at a maintainer the way they do at a loaded one.
	fn simulated_cost_us(state: &MaintainerState, event: &EphemeralStateMachineEvent<Self>) -> u64 {
		if let RuntimeEvent::Input(MaintainerEvent::FilesystemQueryRequest { .. }) = &event.body {
			state.config.simulated_fs_read_cost_us
		} else {
			0
		}
	}

	fn on_event(
		state: &mut MaintainerState,
		event: EphemeralStateMachineEvent<Self>,
		effects: &mut Effects,
	) {
		state.now_boot_ms = state.now_boot_ms.max(effects.now_boot_ms());
		// The oracle names the triggering event when it finds a missed mark.
		#[cfg(debug_assertions)]
		let body = event.body.clone();
		let unload_scope = state::unload::event_scope(&event.body);
		state::unload::mark_scope(state, unload_scope);
		match event.body {
			RuntimeEvent::Input(MaintainerEvent::FilesystemAlarmApplyResponse {
				op_id,
				summary,
			}) => {
				if let Some(Operation::FilesystemApply { key, .. }) = state.operations.get(&op_id) {
					state::alarms::changed(state, key.clone(), Some(&summary), effects);
				}
				state::filesystem::applied(state, op_id, Ok(summary.last_change_position), effects);
			}
			RuntimeEvent::Input(MaintainerEvent::AlarmResponse { op_id, result }) => {
				state::alarms::completed(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::Alarms) => state::alarms::tick(state, effects),
			RuntimeEvent::Input(MaintainerEvent::StopReadinessRequest {
				request_id,
				request,
			}) => state::stop::admit(state, request_id, request, effects),
			RuntimeEvent::Input(MaintainerEvent::HandoffReadinessRequest {
				request_id,
				request,
				stop_intent,
			}) => state::stop::handoff(state, request_id, request, stop_intent, effects),
			RuntimeEvent::Timer(MaintainerTimer::IdleUnload) => {
				state::unload::idle_due(state, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::HolderWatchRetry) => {
				state::holders::retries_due(state)
			}
			RuntimeEvent::Timer(MaintainerTimer::HeldSweepRetry) => {
				state.held_sweep_retry_armed = false;
				state::pending_work::release_held(state, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::ForkCleanupDue) => {
				state::fork_cleanup::due(state, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::PendingWorkTick) => {
				state::pending_work::wake(state, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::PendingWorkResponse { op_id, result }) => {
				state::pending_work::scanned(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::SweepPendingResponse { op_id, result }) => {
				state::pending_work::swept(state, op_id, result, effects)
			}

			RuntimeEvent::Timer(MaintainerTimer::RetentionDue) => {
				state::filesystem::retention_due(state, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ForkRequest {
				request_id,
				spec,
				deadline_boot_ms,
			}) => state::forks::admit(state, request_id, spec, deadline_boot_ms, effects),
			RuntimeEvent::Input(MaintainerEvent::MaterializeResponse { op_id, result }) => {
				state::materialize::stored(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ForkCleanupResponse { op_id, result }) => {
				state::fork_cleanup::stored(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ForkStoreResponse { op_id, result }) => {
				state::forks::stored(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ForkPeerResponse { op_id, result }) => {
				if matches!(
					state.operations.get(&op_id),
					Some(Operation::MaterializePeer { .. })
				) {
					state::materialize::peer(state, op_id, result, effects);
				} else if matches!(
					state.operations.get(&op_id),
					Some(Operation::ForkCleanupPeer { .. })
				) {
					state::fork_cleanup::peer(state, op_id, result, effects);
				} else {
					state::forks::peer(state, op_id, result, effects);
				}
			}
			RuntimeEvent::Timer(MaintainerTimer::ForkRequest(id)) => {
				state::forks::expire(state, id, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::ForkRetry(key)) => {
				state::forks::retry(state, key, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::MaterializeRetry(scope)) => {
				state::materialize::retry_due(state, scope, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ForkReferenceRequest {
				request_id,
				request,
				deadline_boot_ms,
			}) => {
				state::fork_references::admit(state, request_id, request, deadline_boot_ms, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ForkReferenceResponse { op_id, result }) => {
				state::fork_references::completed(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::ForkReferenceRequest(id)) => {
				state::fork_references::expired(state, id, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::CheckpointRequest {
				request_id,
				request,
				deadline_boot_ms,
			}) => state::checkpoints::admit(state, request_id, request, deadline_boot_ms, effects),
			RuntimeEvent::Input(MaintainerEvent::CheckpointResponse { op_id, result }) => {
				state::checkpoints::completed(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::MaintainerOpResponse { op_id, result }) => {
				state::logged_ops::replayed(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::MaintainerLogDue) => {
				state::maintainer_log::due(state, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::MemtableBudgetShed) => {
				state::filesystem_reads::memtable_budget_shed(state, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::MaintainerLogRetry(worker)) => {
				state::maintainer_log::retry(state, worker)
			}
			RuntimeEvent::Timer(MaintainerTimer::CheckpointRequest(id)) => {
				state::checkpoints::expired(state, id, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::FilesystemQueryRequest {
				request_id,
				request,
				deadline_boot_ms,
			}) => state::filesystem_reads::admit(
				state,
				request_id,
				request,
				deadline_boot_ms,
				effects,
			),
			RuntimeEvent::Input(MaintainerEvent::FilesystemQueryResponse { op_id, result }) => {
				state::filesystem_reads::completed(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::FilesystemLoadRequest { key }) => {
				state::filesystem::load(state, key)
			}
			RuntimeEvent::Input(MaintainerEvent::FilesystemLoadResponse { op_id, result }) => {
				state::filesystem::loaded(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::FilesystemCommittedRequest { key, through }) => {
				state::filesystem::committed(state, key, through, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::FilesystemApplyResponse { op_id, result }) => {
				state::filesystem::applied(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::FilesystemFlushStaged { op_id }) => {
				state::filesystem::staged(state, op_id, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::FilesystemFlushResponse { op_id, result }) => {
				state::filesystem::flushed(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::MemtableBudget { full }) => {
				state::filesystem_reads::memtable_budget(state, full, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::FilesystemDue(key)) => {
				state::filesystem::request_due_flush(state, &key, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::FilesystemRetry(key)) => {
				state::filesystem::retry_ready(state, &key)
			}
			RuntimeEvent::Timer(MaintainerTimer::FilesystemFlushBatch) => {
				state::filesystem::flush_batch_due(state)
			}
			RuntimeEvent::Input(MaintainerEvent::KvCommandRequest {
				deadline_boot_ms,
				request_id,
				request,
				request_bytes,
			}) => {
				if state::deletion::holds_key(state, &request.volume_key) {
					let response = if state::deletion::lookup_pending(state, &request.volume_key) {
						crate::kv_refusals::overloaded(OverloadReason::VolumeTransition)
					} else {
						KvCommandResponse::VolumeDeleted
					};
					effects.execute(MaintainerEffect::KvCommandResponse {
						request_id,
						response,
					});
				} else if state.phase == state::MaintainerPhase::Serving {
					effects.execute(MaintainerEffect::ExecuteKvCommand {
						deadline_boot_ms,
						request_id,
						request,
						request_bytes,
					});
				} else {
					effects.execute(MaintainerEffect::KvCommandResponse {
						request_id,
						response: crate::kv_refusals::authority_moved(),
					});
				}
			}

			RuntimeEvent::Input(MaintainerEvent::ReportCatchUpResponse { op_id, result }) => {
				state::catch_up::reported(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::AssignmentUpdated { tablet }) => {
				state::assignment::update(state, tablet, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ManifestObserved {
				tablet_manifest_version,
				tablet,
			}) => state::stop::manifest_observed(state, tablet_manifest_version, tablet, effects),
			RuntimeEvent::Timer(MaintainerTimer::CatchUp) => state::catch_up::start(state, effects),
			RuntimeEvent::Input(MaintainerEvent::OperatorTierRequest { request_id }) => {
				state::operator::tier(state, request_id, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::OperatorUnloadRequest {
				request_id,
				quiet_ms,
				limit,
			}) => state::operator::unload_quiet(state, request_id, quiet_ms, limit, effects),
			RuntimeEvent::Input(MaintainerEvent::DeleteVolumeRequest {
				request_id,
				volume_log_key,
			}) => state::deletion::admit(
				state,
				request_id,
				volume_log_key,
				event.entropy.clone(),
				effects,
			),
			RuntimeEvent::Input(MaintainerEvent::SweepDeletedResponse { op_id, result }) => {
				state::deletion::swept(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::DeleteRequest(key)) => {
				state::deletion::expired(state, key, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::DeleteRetry(key)) => {
				state::drain::retry(state, &key, effects);
				state::deletion::retry(state, key)
			}
			RuntimeEvent::Input(MaintainerEvent::PrepareSplitResponse { op_id, result }) => {
				state::split::prepared(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ReportSplitResponse { op_id, result }) => {
				state::split::reported(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::HolderGone { holder }) => {
				state::holders::gone(state, holder)
			}
			RuntimeEvent::Input(MaintainerEvent::HolderWatchRefused { holder, watch_id }) => {
				state::holders::watch_refused(state, holder, watch_id)
			}
			RuntimeEvent::Input(MaintainerEvent::RetiredTabletsUpdated {
				retired,
				parent_log_recorded,
			}) => {
				state::retired::update(state, retired, effects);
				state::parent_log::recorded(state, parent_log_recorded, effects);
			}
			RuntimeEvent::Input(MaintainerEvent::DeleteRetiredResponse { op_id, result }) => {
				state::retired::deleted(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ReportRetiredResponse { op_id, result }) => {
				state::retired::reported(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ReportParentLogImportedResponse {
				op_id,
				result,
			}) => state::parent_log::reported(state, op_id, result, effects),
			RuntimeEvent::Timer(MaintainerTimer::ParentLogReportRetry) => {
				state::parent_log::retry(state)
			}
			RuntimeEvent::Input(MaintainerEvent::ReportVersionMarksResponse { op_id, result }) => {
				state::version_marks::reported(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::VersionMarksRetry) => {
				state::version_marks::retry(state)
			}
			RuntimeEvent::Timer(MaintainerTimer::RetiredRetry(id)) => {
				state::retired::retry(state, id)
			}
			RuntimeEvent::Timer(MaintainerTimer::SplitRetry) => {
				if state.phase == state::MaintainerPhase::Opening {
					state::opening::finish(state, effects);
				} else {
					state::split::retry(state);
				}
			}
			RuntimeEvent::Timer(MaintainerTimer::UnloadRecoveryRetry(key)) => {
				state::unload::retry_recovery(state, key)
			}

			RuntimeEvent::Input(MaintainerEvent::RecoverVolumeResponse { op_id, result }) => {
				state::repair::completed(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::OpenRequest(id)) => {
				state::lifecycle::expired(state, id, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::RepairWait(id)) => {
				state::lifecycle::repair_wait_expired(state, id, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::CloseVolumeRequest {
				client,
				request_id,
				volume_log_key,
			}) => state::unload::admit(state, request_id, volume_log_key, client, effects),
			RuntimeEvent::Timer(MaintainerTimer::CloseNotices(key)) => {
				state::unload::notices_due(state, key, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::UnloadRetry(key)) => {
				state::unload::retry(state, key, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::ReadRequest(request_id)) => {
				state::filesystem_reads::expired(state, request_id, effects);
			}
			RuntimeEvent::Input(MaintainerEvent::Drain) => {
				if !matches!(
					state.phase,
					state::MaintainerPhase::Fenced
						| state::MaintainerPhase::Stopped
						| state::MaintainerPhase::Draining
				) {
					state::fence(state, effects);
					state.phase = state::MaintainerPhase::Draining;
				}
			}
			RuntimeEvent::Input(MaintainerEvent::ColdClosed { result }) => {
				if matches!(
					state.phase,
					state::MaintainerPhase::Draining | state::MaintainerPhase::Fenced
				) {
					if result.is_err() {
						state.phase = state::MaintainerPhase::Fenced;
					}
					if state.phase == state::MaintainerPhase::Draining {
						state.phase = state::MaintainerPhase::Stopped;
					}
					effects.execute(MaintainerEffect::CloseComplete);
				}
			}
			RuntimeEvent::Input(MaintainerEvent::ReplicaStatusNotices {
				worker_id,
				tablet_grant_epoch,
				tablet_replica_set_version,
				notices,
			}) => state::poll::notices(
				state,
				worker_id,
				tablet_grant_epoch,
				tablet_replica_set_version,
				notices,
				effects,
			),
			RuntimeEvent::Timer(MaintainerTimer::PromotionRetry(worker)) => {
				state::promotion::retry(state, worker)
			}
			RuntimeEvent::Timer(MaintainerTimer::Poll) => state::poll::timer_fired(state),
			RuntimeEvent::Timer(MaintainerTimer::ColdBatch) => {
				state.cold_batch_pending = false;
				state::store::queue_cold_pass(state);
			}
			RuntimeEvent::Timer(MaintainerTimer::TrimRetry(key, worker)) => {
				state::cold::retry_trim(state, key, worker, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::MemberRepairRetry(key, worker)) => {
				state::member_repair::retry(state, key, worker, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::CreationExpired(key)) => {
				// An expired creation sends no initial entries again, whether or not it stays.
				state.release_initial_entries(&key);
				if let Some(volume) = state.loaded_volumes.get_mut(&key)
					&& volume.idle(state.now_boot_ms, state.config.idle_unload_after_ms)
					&& volume
						.creation
						.as_ref()
						.is_some_and(|creation| !creation.acknowledged)
				{
					volume.creation = None;
				}
				state::opening::collect_orphans(
					state,
					"creation_expired",
					state::opening::OrphanScan::Volumes(vec![key]),
					effects,
				);
			}
			RuntimeEvent::Input(MaintainerEvent::WriteToColdResponse { op_id, result }) => {
				state::cold_pass::completed(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::WriteToCold) => {
				state::store::queue_cold_pass(state)
			}
			RuntimeEvent::Input(MaintainerEvent::Fence) => state::fence(state, effects),
			RuntimeEvent::Input(MaintainerEvent::AlarmIndexReadyResponse { result }) => {
				state::alarms::ready_response(state, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::AlarmBootstrapProgress { op_id }) => {
				if matches!(state.operations.get(&op_id), Some(Operation::OpenCold)) {
					effects.arm_timer_after(
						MaintainerTimer::Operation(op_id),
						state.config.tuning().operation_timeout_ms,
					);
				}
			}
			RuntimeEvent::Input(MaintainerEvent::OpenColdResponse { op_id, result }) => {
				state::opening::opened(state, op_id, result, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::OpenVolumeRequest(request)) => {
				state.opens_received += 1;
				state::lifecycle::admit(state, request, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::Repair(event)) => {
				state::operator_repair::on_event(state, event, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ReadOpenKeysResponse { op_id, result }) => {
				state::open_reads::completed(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::OpenRead(request_id)) => {
				state::open_reads::expired(state, request_id, effects)
			}
			RuntimeEvent::Input(MaintainerEvent::ReadRowResponse { op_id, result }) => {
				if matches!(
					state.operations.get(&op_id),
					Some(Operation::ReadDeleteRow { .. })
				) {
					let Some(Operation::ReadDeleteRow { key }) = state.finish(op_id, effects)
					else {
						unreachable!()
					};
					state::deletion::row(state, key, result, event.entropy.clone(), effects);
				} else if matches!(
					state.operations.get(&op_id),
					Some(Operation::ReadUnloadRow { .. })
				) {
					let Some(Operation::ReadUnloadRow { key }) = state.finish(op_id, effects)
					else {
						unreachable!()
					};
					if let Some(result) =
						state::fork_admission::lookup(state, &key.0.volume_key, result)
					{
						state::unload::row(state, key, op_id, result, effects);
					} else {
						// A fork or restore preparation reserves the name, so the close waits for it.
						state::unload::cancel_key(state, &key.0.volume_key, effects);
						state::unload::postpone_idle(state, &key);
					}
				} else if matches!(
					state.operations.get(&op_id),
					Some(Operation::ReadRow { .. })
				) && let Some(Operation::ReadRow { volume_key }) =
					state.finish(op_id, effects)
				{
					state::lifecycle::row(
						state,
						volume_key,
						result,
						event.entropy.clone(),
						effects,
					);
				}
			}
			RuntimeEvent::Input(MaintainerEvent::ReplicaResponse { op_id, result }) => {
				replica_response(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::Operation(op_id)) => {
				match state.operations.get(&op_id) {
					Some(Operation::Alarm(_)) => state::alarms::expired(state, op_id, effects),
					Some(Operation::CatchUp {
						step: state::catch_up::CopyStep::WaitingToLoad { .. },
						..
					}) => {
						state::catch_up::retry_load(state, op_id, effects);
					}
					Some(Operation::MaterializePeer { .. }) => {
						effects.execute(MaintainerEffect::CancelForkPeerRequest { op_id });
						state::materialize::peer(
							state,
							op_id,
							Err("materialization attempt timed out".into()),
							effects,
						);
					}
					Some(Operation::ForkCleanupPeer { .. }) => {
						effects.execute(MaintainerEffect::CancelForkPeerRequest { op_id });
						state::fork_cleanup::peer(
							state,
							op_id,
							Err("fork cleanup attempt timed out".into()),
							effects,
						);
					}
					Some(Operation::ForkPeer { .. }) => {
						effects.execute(MaintainerEffect::CancelForkPeerRequest { op_id });
						state::forks::peer(
							state,
							op_id,
							Err("fork source attempt timed out".into()),
							effects,
						);
					}
					Some(Operation::FilesystemQuery { .. }) => {
						state::filesystem_reads::timed_out(state, op_id, effects);
					}
					Some(Operation::ReadOpenKeys { .. }) => {
						state::open_reads::timed_out(state, op_id, effects);
					}
					Some(Operation::Repair(_)) => {
						state::operator_repair::timed_out(state, op_id, effects);
					}
					Some(
						Operation::PendingWork { .. }
						| Operation::SweepPending { .. }
						| Operation::Checkpoint { .. }
						| Operation::ForkReference { .. }
						| Operation::ForkStore { .. }
						| Operation::ForkCleanupStore { .. }
						| Operation::MaterializeStore { .. }
						| Operation::FilesystemLoad { .. }
						| Operation::FilesystemApply { .. }
						| Operation::FilesystemFlush { .. }
						| Operation::ReadDeleteRow { .. }
						| Operation::SweepDeleted { .. }
						| Operation::OpenCold
						| Operation::PrepareSplit { .. }
						| Operation::DeleteRetired { .. }
						| Operation::ReadUnloadRow { .. }
						| Operation::ReadRow { .. }
						| Operation::WriteToCold { .. }
						| Operation::ReplayOp { .. },
					) => state::store_operation_slow(state, op_id, effects),
					Some(Operation::ReportRetired { .. }) => {
						state::retired::report_timeout(state, op_id, effects)
					}
					Some(Operation::ReportParentLogImported) => {
						state::parent_log::report_timeout(state, op_id, effects)
					}
					Some(Operation::ReportVersionMarks { .. }) => {
						state::version_marks::report_timeout(state, op_id, effects)
					}
					Some(Operation::ReportCatchUp { .. }) => {
						effects.execute(MaintainerEffect::CancelCatchUpReportRequest { op_id });
						state::catch_up::reported(
							state,
							op_id,
							Err("catch-up report timed out".into()),
							effects,
						);
					}
					Some(Operation::ReportSplit) => {
						state::split::report_timeout(state, op_id, effects)
					}
					Some(Operation::RecoverVolume { .. }) => {
						state::repair::cancel_job(state, op_id, effects)
					}
					Some(
						Operation::CatchUp { .. }
						| Operation::DeleteReplica { .. }
						| Operation::DrainClaim { .. }
						| Operation::List { .. }
						| Operation::PromotionList { .. }
						| Operation::Load { .. }
						| Operation::ForgetOrphan { .. }
						| Operation::ForgetVolumes { .. }
						| Operation::ForgetRetiredLog { .. }
						| Operation::FilesystemRead { .. }
						| Operation::Trim { .. }
						| Operation::TrimBatch { .. }
						| Operation::Poll { .. }
						| Operation::MaintainerLog { .. },
					) => {
						effects.execute(MaintainerEffect::CancelReplicaRequest { op_id });
						replica_response(state, op_id, Err("operation timed out".into()), effects);
					}
					None => {}
				}
			}
			RuntimeEvent::Timer(MaintainerTimer::ListRetry(worker)) => {
				let after = state.listing_cursors.remove(&worker).flatten();
				state::opening::list(state, worker, after, effects)
			}
			RuntimeEvent::Timer(MaintainerTimer::OrphanSweep) => state::opening::collect_orphans(
				state,
				"retry",
				state::opening::OrphanScan::Volumes(Vec::new()),
				effects,
			),
		}
		#[cfg(debug_assertions)]
		if let Err(error) = check_incremental_drives(state, event.entropy, effects)
			&& state.drive_oracle_error.is_none()
		{
			state.drive_oracle_error = Some(format!("{error:#} after {body:?}"));
		}
		#[cfg(not(debug_assertions))]
		drives(state, event.entropy, effects, DriveScope::Incremental);
		state::filesystem_reads::shed_unstarted(state, effects);
	}

	fn snapshot(state: &MaintainerState) -> MaintainerSnapshot {
		MaintainerSnapshot {
			phase: state.phase.clone(),
			recovery_failure: state.recovery_failure.clone(),
			replica_set: state.tablet.replica_set.clone(),
			split_prepared: matches!(
				state.split_preparation,
				Some(state::split::SplitPreparation::Complete)
			),
			loaded_volumes: state.loaded_volumes.len() as u32,
			unloading_volumes: state.unloading.len() as u32,
			pending_requests: state.request_count() as u32,
			listed_members: state.listed.len() as u32,
			opens_received: state.opens_received,
			store: state::store::snapshot(state),
			alarms: state::alarms::snapshot(state),
			signals: state::signals::snapshot(state),
		}
	}

	fn debug_validate(state: &MaintainerState) -> anyhow::Result<()> {
		state::alarms::validate(state)?;
		#[cfg(debug_assertions)]
		if let Some(error) = &state.drive_oracle_error {
			anyhow::bail!("{error}");
		}
		state.operations.validate()?;
		state.store_queue.validate()?;
		let held = state
			.loaded_volumes
			.values()
			.filter_map(|volume| volume.creation.as_ref())
			.filter_map(|creation| creation.granted.as_ref())
			.map(|granted| granted.entry_bytes())
			.sum::<u64>();
		anyhow::ensure!(
			held == state.initial_bytes_held,
			"initial_bytes_held disagrees with the retained grants"
		);
		state::poll::validate(state)?;
		state::maintainer_log::validate(state)?;
		state::unload::validate(state)?;
		state::filesystem::validate(state)?;
		let store = state::store::StoreUsage::of(state);
		anyhow::ensure!(
			store.reads() <= state.config.max_cold_reads as usize,
			"store read slots exceeded"
		);
		anyhow::ensure!(
			store.keyed <= state.config.max_keyed_in_flight as usize,
			"store keyed slots exceeded"
		);
		state::footprint::validate(state)?;
		// The operation index was checked against a recomputation above, so its counts
		// stand for the operations here.
		let counts = state.operations.index().counts;
		anyhow::ensure!(
			counts.retired <= 1,
			"retired cleanup concurrency cap exceeded"
		);
		state::catch_up::validate_window(state)?;
		anyhow::ensure!(
			counts.split as usize
				== usize::from(matches!(
					state.split_preparation,
					Some(state::split::SplitPreparation::Running)
				)),
			"split operation lost its preparation state"
		);
		anyhow::ensure!(
			state.phase != state::MaintainerPhase::Serving
				|| state
					.loaded_volumes
					.iter()
					.all(|(key, volume)| !volume.is_deleted()
						|| state.deleting.contains_key(key)
						|| state.deletion_candidates.contains(key)),
			"observed tombstone missing deletion recovery ownership"
		);
		anyhow::ensure!(
			state.reserved_volumes() <= state.config.max_loaded_volumes as usize,
			"loaded volume cap exceeded"
		);
		anyhow::ensure!(
			state.request_count() <= state.config.max_requests as usize,
			"request cap exceeded"
		);
		anyhow::ensure!(
			state::cold::bytes(state) <= MAX_COLD_BYTES,
			"cold byte cap exceeded"
		);
		anyhow::ensure!(
			counts.recover_volume <= state.config.tuning().max_repairs,
			"repair concurrency cap exceeded"
		);
		anyhow::ensure!(
			state.pending.iter().all(|(key, pending)| !matches!(
				pending.step,
				state::OpenStep::WaitingForRepair
			) || state::repair::holds_key(state, key)),
			"reopen lost its recovery cleanup barrier"
		);

		anyhow::ensure!(
			state
				.unloading
				.iter()
				.all(|(key, unload)| !unload.abandoned_create
					|| state.loaded_volumes.get(key).is_none_or(|volume| volume
						.creation
						.as_ref()
						.is_none_or(|creation| !creation.acknowledged))),
			"acknowledged creation classified as abandoned"
		);

		anyhow::ensure!(
			counts.delete_replica <= state.config.tuning().max_delete_requests,
			"delete concurrency cap exceeded"
		);

		Ok(())
	}
}

/// Which volumes the drives evaluate after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DriveScope {
	/// Only the volumes a transition marked, as production runs.
	Incremental,
	/// Every loaded volume, registered when eligible and marked. The debug oracle runs
	/// this on a copy of the state to prove the incremental marks miss nothing.
	#[cfg(debug_assertions)]
	Full,
}

/// Advances every subsystem after an event. Each drive starts only work that the
/// event or an earlier drive enabled.
pub(crate) fn drives(
	state: &mut MaintainerState,
	entropy: Entropy,
	effects: &mut Effects,
	scope: DriveScope,
) {
	state::maintainer_log::drive(state, &entropy, effects);
	state::claim_epochs::drive(state, effects);
	state::drain::drive(state, effects);
	match scope {
		DriveScope::Incremental => {}
		#[cfg(debug_assertions)]
		DriveScope::Full => state::deletion::mark_all(state),
	}
	state::deletion::drive(state, entropy, effects);
	match scope {
		DriveScope::Incremental => {}
		#[cfg(debug_assertions)]
		DriveScope::Full => state::unload::mark_all(state),
	}
	state::unload::drive(state, effects);
	state::checkpoints::resume_after_unload(state, effects);
	state::cold::drive_trims(state, effects);
	state::promotion::drive(state, effects);
	state::poll::drive(state, effects);
	// Give the current copy round a reservation before starting more cold reads.
	state::catch_up::drive(state, effects);
	// A full pass registers and marks every loaded volume after the lifecycle drives
	// settled ownership, exactly where the incremental marks take effect.
	match scope {
		DriveScope::Incremental => {}
		#[cfg(debug_assertions)]
		DriveScope::Full => {
			if state.phase == state::MaintainerPhase::Serving {
				state::filesystem::mark_all(state);
			}
		}
	}
	state::filesystem::drive(state, effects);
	state::checkpoints::drive(state, effects);
	state::fork_references::drive(state, effects);
	state::forks::drive(state, effects);
	// Deletions and restores start drains after the first drain pass. Send their claims in this pass
	// so a drain never waits for an unrelated later event.
	state::drain::drive(state, effects);
	state::split::drive(state, effects);
	state::retired::drive(state, effects);
	state::parent_log::drive(state, effects);
	state::version_marks::drive(state, effects);
	match scope {
		DriveScope::Incremental => state::store::drive(state, effects),
		#[cfg(debug_assertions)]
		DriveScope::Full => state::store::drive_admission(state, effects, true),
	}
	state::catch_up::drive(state, effects);
	// Any drive may have rescheduled a volume's checks, so the timers follow their queue
	// heads last.
	state::poll::arm_head(state, effects);
	state::filesystem::arm_retention(state, effects);
	state::unload::arm_idle(state, effects);
	state::holders::finish_event(state, effects);
	state::fork_cleanup::arm_due(state, effects);
	state::alarms::drive(state, effects);
}

/// Runs the drives incrementally on `state` and as a full pass on a copy, and reports
/// any difference in effects or state. A difference means some change enabled work
/// without marking it, which production would not notice until the next event on that
/// volume.
#[cfg(debug_assertions)]
pub(crate) fn check_incremental_drives(
	state: &mut MaintainerState,
	entropy: Entropy,
	effects: &mut Effects,
) -> anyhow::Result<()> {
	let mut full = state.clone();
	let mut full_effects = effects.clone();
	drives(
		&mut full,
		entropy.clone(),
		&mut full_effects,
		DriveScope::Full,
	);
	drives(state, entropy, effects, DriveScope::Incremental);

	if *effects != full_effects {
		let incremental = effects.clone().drain();
		let full_only = full_effects
			.drain()
			.into_iter()
			.filter(|effect| !incremental.contains(effect))
			.collect::<Vec<_>>();
		anyhow::bail!(
			"incremental maintainer drives emitted {} effects where a full pass emits more or others; full pass only: {full_only:?}",
			incremental.len()
		);
	}
	let differing = state::differing_fields(state, &full);
	// Name the store work only one side queued, since a missed mark usually shows there.
	let full_only_work = full
		.store_queue
		.iter()
		.filter(|work| !state.store_queue.contains(work))
		.collect::<Vec<_>>();
	let incremental_only_work = state
		.store_queue
		.iter()
		.filter(|work| !full.store_queue.contains(work))
		.collect::<Vec<_>>();
	anyhow::ensure!(
		differing.is_empty(),
		"incremental maintainer drives left state that differs from a full pass in {differing:?}; store work only the full pass queued: {full_only_work:?}; only the incremental drives queued: {incremental_only_work:?}"
	);
	Ok(())
}

/// Completion routing uses the retained operation, never a remote worker's claimed identity.
fn replica_response(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<StorageWorkerResponse, String>,
	effects: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(
			Operation::CatchUp { .. }
				| Operation::DeleteReplica { .. }
				| Operation::DrainClaim { .. }
				| Operation::List { .. }
				| Operation::PromotionList { .. }
				| Operation::Load { .. }
				| Operation::ForgetOrphan { .. }
				| Operation::ForgetVolumes { .. }
				| Operation::ForgetRetiredLog { .. }
				| Operation::FilesystemRead { .. }
				| Operation::Trim { .. }
				| Operation::TrimBatch { .. }
				| Operation::Poll { .. }
				| Operation::MaintainerLog { .. }
				| Operation::Repair(state::operator_repair::RepairOperation::Forget { .. })
		)
	) {
		return;
	}
	let operation = state.finish(id, effects).expect("current operation");
	if matches!(
		&result,
		Ok(StorageWorkerResponse::RejectResponse(
			RejectResponse::StaleTabletGrantEpoch(_)
		))
	) {
		state::fence(state, effects);
		return;
	}
	match operation {
		Operation::Repair(state::operator_repair::RepairOperation::Forget { operation_id }) => {
			state::operator_repair::forget::completed(state, operation_id, result, effects)
		}
		Operation::Alarm(_) => unreachable!("alarm responses have their own routing"),
		Operation::CatchUp { key, learner, step } => {
			state::catch_up::completed(state, key, learner, step, result, effects)
		}
		Operation::DeleteReplica {
			key,
			worker,
			action,
		} => state::deletion::replica(state, key, worker, action, result, effects),
		Operation::DrainClaim { key, worker } => {
			state::drain::claimed(state, key, worker, result, effects)
		}
		Operation::ForgetRetiredLog { tablet_id, worker } => {
			state::retired::forgotten(state, tablet_id, worker, result, effects)
		}
		Operation::ForgetVolumes {
			worker,
			volumes,
			detached,
		} => state::unload::forgotten_batch(state, worker, volumes, detached, result, effects),
		Operation::FilesystemRead {
			key,
			worker,
			proof,
			previous,
		} => state::filesystem::read(state, key, worker, proof, previous, result, effects),
		Operation::Trim { key, worker, cold } => {
			state::cold::trimmed(state, key, worker, cold, result, effects)
		}
		Operation::TrimBatch { worker, trims } => {
			state::cold::trimmed_batch(state, worker, trims, result, effects)
		}
		Operation::Poll {
			replica_set_version,
			keys,
			worker,
			barrier,
		} => {
			let result =
				if replica_set_version == state.tablet.replica_set.tablet_replica_set_version {
					result
				} else {
					Err("obsolete replica-set observation".into())
				};
			state::poll::completed(state, keys, worker, barrier, result, effects)
		}
		Operation::PromotionList {
			worker,
			after,
			collecting,
		} => {
			state::promotion::completed(state, worker, after, collecting, result, effects);
		}
		Operation::List { worker, after } => {
			state::opening::listed(state, worker, after, result, effects)
		}
		Operation::Load {
			worker,
			key,
			step: state::lifecycle::ReplicaOpenStep::MissingMember,
		} => state::member_repair::loaded(state, id, key, worker, result, effects),
		Operation::Load { worker, key, step } => {
			state::lifecycle::loaded(
				state,
				id,
				worker.clone(),
				key.clone(),
				step,
				result,
				effects,
			);
			// A load that ended without a replica after its open answered leaves the
			// volume short of a member.
			state::member_repair::absent(state, &key, &worker, effects);
		}
		Operation::MaintainerLog { worker, request } => {
			state::maintainer_log::replica(state, worker, request, result, effects)
		}
		Operation::ForgetOrphan { worker, key } => {
			// A forgotten replica is removed from the members only when it is the one this
			// maintainer observes there, so an answer that arrives after a load created a newer
			// replica on that member keeps the newer one. An absent replica names none.
			let (answered, forgotten_incarnation) = match &result {
				Ok(StorageWorkerResponse::ReplicaStateResponse(response)) => {
					(true, Some(response.state.replica_incarnation))
				}
				Ok(StorageWorkerResponse::RejectResponse(RejectResponse::VolumeNotLoaded)) => {
					(true, None)
				}
				Ok(_) | Err(_) => (false, None),
			};
			if answered {
				if let Some(volume) = state.loaded_volumes.get_mut(&key) {
					let same_replica = match forgotten_incarnation {
						Some(incarnation) => volume
							.members
							.get(&worker)
							.is_some_and(|member| member.replica_incarnation == incarnation),
						None => true,
					};
					if same_replica {
						volume.members.remove(&worker);
					}
					if volume.members.is_empty() {
						state::filesystem::forget(state, &key, effects);
						state.remove_loaded_volume(&key);
					}
				}
				state::opening::collect_orphans(
					state,
					"forget",
					state::opening::OrphanScan::Volumes(vec![key]),
					effects,
				);
			} else {
				// The retry examines only the volumes waiting in the backlog.
				state.orphan_backlog.insert(key);
				effects
					.arm_timer_after(MaintainerTimer::OrphanSweep, state.config.tuning().retry_ms);
			}
		}
		Operation::PendingWork { .. }
		| Operation::SweepPending { .. }
		| Operation::Checkpoint { .. }
		| Operation::ForkReference { .. }
		| Operation::ForkStore { .. }
		| Operation::ForkCleanupStore { .. }
		| Operation::MaterializeStore { .. }
		| Operation::ForkPeer { .. }
		| Operation::ForkCleanupPeer { .. }
		| Operation::MaterializePeer { .. }
		| Operation::FilesystemQuery { .. }
		| Operation::FilesystemLoad { .. }
		| Operation::FilesystemApply { .. }
		| Operation::FilesystemFlush { .. }
		| Operation::DeleteRetired { .. }
		| Operation::ReportRetired { .. }
		| Operation::ReportParentLogImported
		| Operation::ReportVersionMarks { .. }
		| Operation::PrepareSplit { .. }
		| Operation::ReportCatchUp { .. }
		| Operation::ReportSplit
		| Operation::RecoverVolume { .. }
		| Operation::ReadDeleteRow { .. }
		| Operation::SweepDeleted { .. }
		| Operation::OpenCold
		| Operation::ReadRow { .. }
		| Operation::ReadOpenKeys { .. }
		| Operation::Repair(_)
		| Operation::WriteToCold { .. }
		| Operation::ReadUnloadRow { .. }
		| Operation::ReplayOp { .. } => {
			unreachable!("replica operation checked above")
		}
	}
}








