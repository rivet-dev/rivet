//! Work selected by the deterministic tablet maintainer.

use super::types::{OpId, RequestId};
use rivet_data::generated::shared_depot_types_v1::VolumeLogPosition;
use rivet_data::generated::{
	depot_client_protocol_v1::ClientResponse,
	depot_storage_worker_protocol_v1::StorageWorkerRequest, depot_tablet_manifest_v1::Tablet,
	shared_depot_fs_v1::FsVolumeRecord,
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaintainerEffect {
	/// **When**
	///
	/// A same-grant assignment installs a newer replica set in the deterministic core.
	///
	/// **Executor**
	///
	/// Offers the published membership snapshot to retained command writers under
	/// the current grant. Open completion and command admission repeat dropped offers.
	RefreshCommandReplicaSets,
	/// **When**
	///
	/// A child finishes importing its exact split checkpoint index.
	///
	/// **Executor**
	///
	/// Reports readiness under the captured grant and returns AlarmIndexReadyResponse.
	ReportAlarmIndexReady {
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::AlarmIndexReady,
	},
	/// **When**
	///
	/// Split preparation captures the exact parent checkpoint manifest.
	///
	/// **Executor**
	///
	/// Reports that source with SplitPrepared and returns ReportSplitResponse.
	ReportAlarmSplitRequest {
		op_id: OpId,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::SplitPrepared,
		manifest_id: u64,
		maintainer: [u8; 16],
	},
	/// **When**
	///
	/// The maintainer reserves a bounded alarm read, callback, or retry-record update.
	///
	/// **Executor**
	///
	/// Performs this one operation under the current grant and returns AlarmResponse,
	/// including cancellation. Callback capacity remains occupied until that response.
	AlarmRequest {
		op_id: OpId,
		request: crate::alarms::Request,
	},
	/// **When**
	///
	/// An alarm attempt expires, its revision is replaced, or serving authority ends.
	///
	/// **Executor**
	///
	/// Drops the in-flight future; its task returns the original AlarmResponse identity.
	CancelAlarmRequest { op_id: OpId },
	/// **When**
	///
	/// A fresh round completes, or the grant cannot admit the check.
	///
	/// **Executor**
	///
	/// Rechecks grant authority and delivers the correlated readiness reply.
	StopReadinessResponse {
		request_id: RequestId,
		response: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
	},
	/// **When**
	///
	/// A discovered materialization owns the ordered local store slot.
	///
	/// **Executor**
	///
	/// Revalidates its incarnation, reads progress or flushes a cursor and copy page,
	/// then returns MaterializeResponse under the captured grant.
	MaterializeRequest {
		op_id: OpId,
		scope: depot_fs_core::keys::slatedb::Scope,
		action: depot_fs_core::forks::materialize::Action,
	},
	/// **When**
	///
	/// A discovered fork check or release owns the shared store slot.
	///
	/// **Executor**
	///
	/// Revalidates its durable identity or commits a confirmed result, returning ForkCleanupResponse under the captured grant.
	ForkCleanupRequest {
		op_id: OpId,
		work: depot_fs_core::forks::cleanup::Work,
		region: u16,
		action: depot_fs_core::forks::cleanup::Action,
		/// Incarnations of the fork parent the maintainer holds loaded. A release never
		/// schedules a sweep of one of them, even when its record is not stored yet.
		loaded_parent_incarnations: Vec<[u8; 16]>,
	},
	/// **When**
	///
	/// The destination workflow owns the ordered store slot.
	///
	/// **Executor**
	///
	/// Executes one local fork step with the captured identity and returns ForkStoreResponse.
	/// A restore of an incarnation whose first record has not reached the store carries the
	/// record the maintainer registered for it as `replaced`.
	ForkStoreRequest {
		op_id: OpId,
		spec: depot_fs_core::FsForkSpec,
		region: u16,
		action: depot_fs_core::forks::StoreAction,
		replaced: Option<depot_fs_core::FsVolumeRecord>,
	},

	/// **When**
	///
	/// Durable preparation needs a source hold or metadata page, or cleanup needs an exact reference check or release.
	///
	/// **Executor**
	///
	/// Sends one regional request outside the storage FIFO, checks the admitted grant at dispatch and reply, and returns ForkPeerResponse.
	ForkPeerRequest {
		op_id: OpId,
		region: u16,
		request: rivet_data::generated::depot_client_protocol_v1::ClientRequest,
		deadline_boot_ms: u64,
	},

	/// **When**
	///
	/// A regional fork attempt times out.
	///
	/// **Executor**
	///
	/// Aborts the owned attempt; its already sent mutation remains uncertain and the core retries the same identity.
	CancelForkPeerRequest { op_id: OpId },

	/// **When**
	///
	/// Reference administration owns the ordered store slot after its apply barrier.
	///
	/// **Executor**
	///
	/// Applies the exact reference operation in the local region and returns
	/// ForkReferenceResponse, including any durable source record proof.
	ForkReferenceRequest {
		op_id: OpId,
		request: rivet_data::generated::depot_client_protocol_v1::FsForkReferenceRequest,
		region: u16,
		barrier: depot_fs_core::forks::ReferenceBarrier,
		record: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		/// The creation time of the maintainer log op an acquire or release applies, which
		/// then is not flushed: the op is its durable record.
		logged: Option<i64>,
		/// Incarnations of the fork parent the maintainer holds loaded. A release never
		/// schedules a sweep of one of them, even when its record is not stored yet.
		loaded_parent_incarnations: Vec<[u8; 16]>,
	},

	/// **When**
	///
	/// Pending-work discovery reaches the ordered store queue: a page of the pass over
	/// every pending row, or of one volume key's rows after a store write touched them.
	///
	/// **Executor**
	///
	/// Scans one bounded page after `after`: of every pending row when `volume` is
	/// `None`, else of that volume key's rows without its fork checks. Returns
	/// PendingWorkResponse.
	PendingWorkRequest {
		op_id: OpId,
		after: Option<Vec<u8>>,
		limit: u32,
		volume: Option<depot_fs_core::work::PendingVolume>,
	},
	/// **When**
	///
	/// A discovered sweep reaches the ordered store queue.
	///
	/// **Executor**
	///
	/// Revalidates every hold, sweeps one page and persists its cursor, then
	/// returns SweepPendingResponse. The final marker is deleted last.
	SweepPendingRequest {
		op_id: OpId,
		scope: depot_fs_core::keys::slatedb::Scope,
	},

	/// **When**
	///
	/// A checkpoint command reaches the ordered store queue's head.
	///
	/// **Executor**
	///
	/// Validates metadata and applies the command, then returns CheckpointResponse under the
	/// same grant. A logged change is written to the memtable only; any other mutation is
	/// flushed with its applied floor.
	/// **When**
	///
	/// A recovered maintainer log op may apply: its volume's apply reached the op's
	/// position, or it names no volume.
	///
	/// **Executor**
	///
	/// Runs the op's guarded store change against the given record and applied position,
	/// writing to the memtable only, and returns MaintainerOpResponse.
	MaintainerOpRequest {
		op_id: OpId,
		op: rivet_data::generated::depot_maintainer_log_v1::MaintainerOp,
		record: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		region: u16,
		/// Incarnations of the fork parent the maintainer holds loaded. A release never
		/// schedules a sweep of one of them, even when its record is not stored yet.
		loaded_parent_incarnations: Vec<[u8; 16]>,
	},

	CheckpointRequest {
		op_id: OpId,
		request: rivet_data::generated::depot_client_protocol_v1::VolumeCheckpointRequest,
		record: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		/// The time of the maintainer log op that is this change's durable record, when the
		/// change has no holder. The executor then writes the change to the memtable only.
		logged: Option<i64>,
	},

	/// **When**
	///
	/// An ordinary reader owns the ordered store slot, or a held-image query
	/// has reserved an independent read slot.
	///
	/// **Executor**
	///
	/// Opens or reads an immutable session at completed applied progress,
	/// checks the grant and returns FilesystemQueryResponse. The maintainer's read
	/// task spawns each query directly, never behind the ordered IO loop, so a slow
	/// mutation cannot delay it. Owned read tasks let source queries finish while an
	/// ordinary reader waits for an inherited chunk.
	FilesystemQueryRequest {
		op_id: OpId,
		request: rivet_data::generated::shared_depot_fs_read_v1::FsReadRequest,
		applied: Option<VolumeLogPosition>,
		/// The highest applied position with no filesystem transaction half applied, which a
		/// latest open serves.
		settled: Option<VolumeLogPosition>,
		record: Option<FsVolumeRecord>,
	},

	/// **When**
	///
	/// A filesystem query reaches its operation deadline.
	///
	/// **Executor**
	///
	/// The read task aborts and joins that query's task. Reads change no durable state,
	/// so cancellation needs no ordering against later store work.
	CancelFilesystemQueryRequest { op_id: OpId },

	/// **When**
	///
	/// A filesystem load owns the store queue.
	///
	/// **Executor**
	///
	/// Reads the incarnation's own record row and returns FilesystemLoadResponse.
	///
	FilesystemLoadRequest {
		op_id: super::types::OpId,
		key: depot_types::VolumeLogKey,
	},
	/// **When**
	///
	/// A qualified committed page owns the store queue.
	///
	/// **Executor**
	///
	/// Runs bounded filesystem apply, checks the grant and returns FilesystemApplyResponse.
	///
	FilesystemApplyRequest {
		/// Replica attempts expire before the store operation's deadline, so an
		/// unreachable replica ends the apply with a retryable read failure.
		read_timeout_ms: u64,
		read_deadline_boot_ms: u64,
		max_read_bytes: u32,
		resolver: depot_fs_core::txn::Resolver,
		source: Option<(
			rivet_topology::worker::WorkerId,
			rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState,
		)>,
		op_id: super::types::OpId,
		key: depot_types::VolumeLogKey,
		previous: rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
		entries: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntry>,
		/// What the core established about the incarnation's record; the executor refuses
		/// an apply whose record disagrees with it.
		authority: super::types::ApplyAuthority,
		/// The final tail a delete's drain fixed, when this apply serves one: the executor
		/// then reads transactions from a tombstone holding it.
		drain_through: Option<rivet_data::generated::shared_depot_types_v1::VolumeLogPosition>,
		/// The volume's record as the maintainer holds it, carried only when the page logs a
		/// retention policy: the store applies the policy to this record's retention rows
		/// when no pass has stored the record yet.
		loaded_record: Option<depot_fs_core::FsVolumeRecord>,
	},
	/// **When**
	///
	/// Store admission starts a queued flush. It carries the due records of volumes with
	/// no conflicting work in flight or queued ahead of it.
	///
	/// **Executor**
	///
	/// A store task stages the records, holding their volume stripes, and returns
	/// FilesystemFlushStaged. The same task then makes them durable, checks the grant
	/// and returns FilesystemFlushResponse; a staging failure returns
	/// FilesystemFlushResponse directly. It also counts `deferred_volumes`, the
	/// requested volumes this flush skipped and owes. Closing the store joins the task.
	/// `maintainer_log`, when present, is the tablet's maintainer log record, staged in
	/// the same batch so the cold position it may trim to is durable with the flush.
	FilesystemFlushRequest {
		op_id: super::types::OpId,
		records: Vec<depot_fs_core::FsVolumeRecord>,
		deferred_volumes: u32,
		maintainer_log:
			Option<crate::tablet_maintainer_machine::state::maintainer_log::RecordWrite>,
	},

	/// **When**
	///
	/// A serving maintainer admits a typed KV command.
	///
	/// **Executor**
	///
	/// Reserves bounded engine and handler capacity, runs the handler off the lifecycle
	/// loop and delivers its response under the original grant and timeout.
	ExecuteKvCommand {
		request_id: RequestId,
		request: rivet_data::generated::depot_kv_command_v1::KvCommandRequest,
		/// The command's received wire size.
		request_bytes: u64,
		/// Receiving-host deadline translated by UPS; retries never reset it.
		deadline_boot_ms: u64,
	},

	/// **When**
	///
	/// A command arrives while the tablet cannot serve it.
	///
	/// **Executor**
	///
	/// Completes the pending command without opening an engine or running a handler.
	KvCommandResponse {
		request_id: RequestId,
		response: rivet_data::generated::depot_kv_command_v1::KvCommandResponse,
	},

	/// **When**
	///
	/// A completed Learning round proves the learner covers every log's observed committed prefix.
	///
	/// **Executor**
	///
	/// Checks the grant at dispatch and reply, sends the phase-fenced report through coordinator discovery,
	/// and returns ReportCatchUpResponse.
	ReportCatchUpRequest {
		op_id: OpId,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::CatchUpReport,
	},

	/// **When**
	///
	/// A report times out or a newer replica-set assignment supersedes it.
	///
	/// **Executor**
	///
	/// Cancels the local transport attempt; the coordinator still fences any already-sent report by version and grant.
	CancelCatchUpReportRequest { op_id: OpId },

	/// **When**
	///
	/// **When**
	///
	/// An assigned retired parent reaches its turn in the ordered storage queue.
	///
	/// **Executor**
	///
	/// Checks clone pins through the guarded SlateDB adapter. If none remain,
	/// deletes the retained checkpoint and database, then returns DeleteRetiredResponse.
	DeleteRetiredRequest {
		op_id: OpId,
		retired: rivet_data::generated::depot_tablet_manifest_v1::RetiredTablet,
	},

	/// **When**
	///
	/// Parent deletion succeeds, or its unaccepted coordinator report is retried.
	///
	/// **Executor**
	///
	/// Sends the same RetiredTabletDeleted through the injected coordinator transport,
	/// checks grant authority at dispatch and reply, and returns ReportRetiredResponse.
	ReportRetiredRequest {
		op_id: OpId,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::RetiredTabletDeleted,
	},

	/// **When**
	///
	/// A retired-parent report times out or its cleanup assignment is withdrawn.
	///
	/// **Executor**
	///
	/// Cancels the local transport attempt. Remote acceptance can still occur;
	/// retrying the same parent identity reconciles that outcome.
	CancelRetiredReportRequest { op_id: OpId },

	/// **When**
	///
	/// A serving split child's own log holds its parent's ops, the manifest has not recorded
	/// that, and no report is in flight or waiting to retry.
	///
	/// **Executor**
	///
	/// Sends the ParentLogImported through the injected coordinator transport, checks grant
	/// authority at dispatch and reply, and returns ReportParentLogImportedResponse.
	ReportParentLogImportedRequest {
		op_id: OpId,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::ParentLogImported,
	},

	/// **When**
	///
	/// A parent log import report times out.
	///
	/// **Executor**
	///
	/// Cancels the local transport attempt. Remote acceptance can still occur; the next
	/// report or manifest reconciles that outcome.
	CancelParentLogReportRequest { op_id: OpId },

	/// **When**
	///
	/// A serving maintainer's tablet data marks rose above the manifest's and no report is
	/// in flight or waiting to retry.
	///
	/// **Executor**
	///
	/// Sends the VersionMarksReport through the injected coordinator transport, checks
	/// grant authority at dispatch and reply, and returns ReportVersionMarksResponse.
	ReportVersionMarksRequest {
		op_id: OpId,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::VersionMarksReport,
	},

	/// **When**
	///
	/// The opened store's record names a family version this binary cannot read. Emitted
	/// just before the maintainer fences instead of replaying.
	///
	/// **Executor**
	///
	/// Logs the family, the mark and this binary's read range, and counts the refusal.
	/// Returns no event.
	RecordVersionRefused {
		family: u16,
		mark: u16,
		/// `None` when this binary does not know the family.
		max_read: Option<u16>,
	},

	/// **When**
	///
	/// A version marks report times out.
	///
	/// **Executor**
	///
	/// Cancels the local transport attempt. Remote acceptance can still occur; the
	/// coordinator merges marks, so the retried report is harmless.
	CancelVersionMarksReportRequest { op_id: OpId },

	/// **When**
	///
	/// The last voting replica of an unloading incarnation confirmed its forget, so the volume
	/// left memory. `duration_ms` runs from the unload's begin to that confirmation.
	///
	/// **Executor**
	///
	/// Observes the unload duration histogram. No input follows.
	RecordUnloadDuration { duration_ms: u64 },

	/// **When**
	///
	/// A Deleted row has reached cold storage and its cleanup owns the store queue.
	///
	/// **Executor**
	///
	/// Hands the exact incarnation's data to its pending sweep and releases its pointer and
	/// pending delete in one unflushed batch, or reports `Held` while a read session or a fork or
	/// checkpoint hold keeps the data. Returns SweepDeletedResponse.
	SweepDeletedRequest {
		op_id: OpId,
		volume_log_key: depot_types::VolumeLogKey,
		max_rows: u32,
	},

	/// **When**
	///
	/// Frozen preparation reaches its turn in the store queue after quorum listing.
	///
	/// **Executor**
	///
	/// Retains the named checkpoint or verifies/creates the exact projected child through
	/// the guarded SlateDB adapter. Returns PrepareSplitResponse after the storage call finishes,
	/// or Fence if the database is closed or this owner loses authority.
	PrepareSplitRequest {
		op_id: OpId,
		tablet: Tablet,
		step: super::state::split::SplitStep,
	},

	/// **When**
	///
	/// The first quiet volume of a holder process parks here, or a refused watch's retry is due.
	///
	/// **Executor**
	///
	/// Takes the holder's shared topology watch, one per process for every tablet on this
	/// worker, and keeps it until `UnwatchHolder`. Returns `HolderGone` once the process is
	/// gone, or `HolderWatchRefused` when the watch cannot start.
	WatchHolderRequest {
		holder: rivet_data::generated::shared_depot_types_v1::VolumeHolder,
		/// Echoed by `HolderWatchRefused`, so a refusal of an older watch is ignored.
		watch_id: u64,
	},
	/// **When**
	///
	/// A holder has no parked volume here any more, its watch resolved, or the maintainer stops.
	///
	/// **Executor**
	///
	/// Drops this maintainer's watch handle; topology ends the watch after its last handle.
	/// Returns no event.
	UnwatchHolder {
		holder: rivet_data::generated::shared_depot_types_v1::VolumeHolder,
	},

	/// **When**
	///
	/// Both projected children exist, or an unaccepted report finishes its backoff.
	///
	/// **Executor**
	///
	/// Rechecks grant authority and sends the unchanged SplitPrepared through the injected
	/// coordinator transport, returning ReportSplitResponse. Coordinator discovery is wired later.
	ReportSplitRequest {
		op_id: OpId,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::SplitPrepared,
	},

	/// **When**
	///
	/// An unanswered split report reaches its deadline.
	///
	/// **Executor**
	///
	/// Cancels the local report task. A remote acceptance remains possible; the next report
	/// repeats the same checkpoint and grant so it can be reconciled by the coordinator.
	CancelSplitReportRequest { op_id: OpId },

	/// **When**
	///
	/// Operator discovery selects a quiet volume needing repair, or a closed volume has an
	/// unpublished suffix. Both paths require a healthy quorum and share the job cap.
	///
	/// **Executor**
	///
	/// Starts the existing recovery-only client with the selected row and fixed member set.
	/// Waits for its report, shuts it down, and returns RecoverVolumeResponse only after joining it.
	RecoverVolumeRequest {
		op_id: OpId,
		request: super::types::RecoveryRequest,
	},

	/// **When**
	///
	/// A client reopens the volume, the job expires, or this repair pass loses its authority.
	///
	/// **Executor**
	///
	/// Signals cancellation to the owned recovery job. Its response follows joined cleanup,
	/// so the core retains its concurrency reservation until that response arrives.
	CancelRecoveryRequest { op_id: OpId },

	/// **When**
	///
	/// The core has processed the store's close result and published its terminal phase.
	///
	/// **Executor**
	///
	/// Releases the owner's close waiter. The owner drops its inbox handle and joins
	/// the runner and store task before reporting shutdown complete.
	CloseComplete,

	/// **When**
	///
	/// A replica operation expires, or reopening cancels an earlier close's forget decision.
	///
	/// **Executor**
	///
	/// Cancels the local RPC task before replacement work is dispatched. A request
	/// already sent may still finish remotely; its old operation ID cannot affect state.
	CancelReplicaRequest { op_id: OpId },

	/// **When**
	///
	/// The ordered storage queue selects pending lifecycle metadata rows. Row lookups and session opens of volumes the
	/// pass does not carry may run beside it.
	///
	/// **Executor**
	///
	/// Stages lifecycle metadata and, with `upload`, awaits one memtable flush before
	/// returning WriteToColdResponse. Without it the rows only reach the memtable, and the
	/// tablet's next ordinary pass makes them cold. Filesystem apply and flush own data
	/// progress.
	WriteToColdRequest {
		op_id: OpId,
		rows: Vec<(depot_types::VolumeKey, FsVolumeRecord)>,
		upload: bool,
	},

	/// **When**
	///
	/// The initial start hook opens the tablet before any member discovery or serving.
	///
	/// **Executor**
	///
	/// Opens SlateDB at the grant epoch, requiring an existing database for split children
	/// and disabling compaction and garbage collection for a frozen split parent. Reads the
	/// maintainer log record, and from then on encodes rows at `rows_version`, the grant's
	/// `depot.fs_rows` version, capped by the record's durable `depot.fs_rows` mark. Returns
	/// OpenColdResponse.
	OpenColdRequest {
		op_id: OpId,
		tablet: Tablet,
		rows_version: u16,
	},

	/// **When**
	///
	/// Opening needs another listing page, lifecycle needs a replica, cleanup can forget
	/// a log, or learner catch-up needs a load, committed read or grant append.
	///
	/// **Executor**
	///
	/// Resolves the exact worker through topology and sends the encoded request under the
	/// admitted grant; returns a correlated response without deciding quorum success.
	ReplicaRequest {
		op_id: OpId,
		worker_id: WorkerId,
		request: StorageWorkerRequest,
	},

	/// **When**
	///
	/// Opening needs stored metadata, closing checks for deletion, or recovered replicas need the stored cold floor.
	///
	/// **Executor**
	///
	/// The filesystem read task reads the typed volume row and exact pending-fork key under the store's mutation lock,
	/// then returns ReadRowResponse. Without `incarnation` the row is the name's pointer, which an open of an unloaded
	/// name reads; with it, that incarnation's own row, which deletes and unloads of a known incarnation read. It runs
	/// beside any IO-loop mutation the core admitted it next to, and fences instead when the grant ended or the store
	/// closed.
	ReadRowRequest {
		op_id: OpId,
		volume_key: depot_types::VolumeKey,
		incarnation: Option<[u8; 16]>,
	},

	/// **When**
	///
	/// An open found the volume existing with no log tail at a position the store applied, and
	/// the caller named keys to read back while a read slot was free.
	///
	/// **Executor**
	///
	/// The filesystem read task opens a session at `applied`, looks up the KV file named
	/// `read.file` under the root, gets each key, closes the session, and returns
	/// ReadOpenKeysResponse. It fences instead when the grant ended or the store closed.
	ReadOpenKeysRequest {
		op_id: OpId,
		key: depot_types::VolumeLogKey,
		record: FsVolumeRecord,
		applied: VolumeLogPosition,
		read: rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys,
	},

	/// **When**
	///
	/// Admission fails or an admitted lifecycle request reaches its reply condition.
	///
	/// **Executor**
	///
	/// Rechecks the admitted local lease and tablet grant, then delivers the response to the caller.
	Reply {
		request_id: RequestId,
		response: ClientResponse,
	},

	/// Work an operator repair request selected; see [`RepairEffect`].
	Repair(RepairEffect),

	/// **When**
	///
	/// Authority is lost or an uncertain cold-store operation ends this lifetime.
	///
	/// **Executor**
	///
	/// Cancels and joins recovery clients, then closes the ordered store executor and joins
	/// its flusher, compactor and garbage collector before returning ColdClosed. The close
	/// drops the memtable without uploading it; the next maintainer replays it from the logs.
	CloseCold,
}

impl rivet_state_machine::Effect for MaintainerEffect {
	fn name(&self) -> &'static str {
		match self {
			Self::RefreshCommandReplicaSets => "refresh_command_replica_sets",
			Self::AlarmRequest { .. } => "alarm_request",
			Self::CancelAlarmRequest { .. } => "cancel_alarm_request",
			MaintainerEffect::StopReadinessResponse { .. } => "stop_readiness_response",
			MaintainerEffect::MaintainerOpRequest { .. } => "maintainer_op_request",
			MaintainerEffect::MaterializeRequest { .. } => "materialize_request",
			MaintainerEffect::ForkCleanupRequest { .. } => "fork_cleanup_request",
			MaintainerEffect::ForkStoreRequest { .. } => "fork_store_request",
			MaintainerEffect::ForkPeerRequest { .. } => "fork_peer_request",
			MaintainerEffect::CancelForkPeerRequest { .. } => "cancel_fork_peer_request",
			MaintainerEffect::ForkReferenceRequest { .. } => "fork_reference_request",
			MaintainerEffect::PendingWorkRequest { .. } => "pending_work_request",
			MaintainerEffect::SweepPendingRequest { .. } => "sweep_pending_request",
			MaintainerEffect::CheckpointRequest { .. } => "checkpoint_request",
			MaintainerEffect::FilesystemQueryRequest { .. } => "filesystem_query_request",
			MaintainerEffect::CancelFilesystemQueryRequest { .. } => {
				"cancel_filesystem_query_request"
			}
			MaintainerEffect::FilesystemLoadRequest { .. } => "filesystem_load_request",
			MaintainerEffect::FilesystemApplyRequest { .. } => "filesystem_apply_request",
			MaintainerEffect::FilesystemFlushRequest { .. } => "filesystem_flush_request",
			MaintainerEffect::ExecuteKvCommand { .. } => "execute_kv_command",
			MaintainerEffect::KvCommandResponse { .. } => "kv_command_response",
			MaintainerEffect::ReportCatchUpRequest { .. } => "report_catch_up_request",
			MaintainerEffect::CancelCatchUpReportRequest { .. } => "cancel_catch_up_report_request",
			MaintainerEffect::DeleteRetiredRequest { .. } => "delete_retired_request",
			MaintainerEffect::ReportRetiredRequest { .. } => "report_retired_request",
			MaintainerEffect::ReportParentLogImportedRequest { .. } => {
				"report_parent_log_imported_request"
			}
			MaintainerEffect::CancelParentLogReportRequest { .. } => {
				"cancel_parent_log_report_request"
			}
			MaintainerEffect::CancelRetiredReportRequest { .. } => "cancel_retired_report_request",
			MaintainerEffect::ReportVersionMarksRequest { .. } => "report_version_marks_request",
			MaintainerEffect::RecordVersionRefused { .. } => "record_version_refused",
			MaintainerEffect::CancelVersionMarksReportRequest { .. } => {
				"cancel_version_marks_report_request"
			}
			MaintainerEffect::ReadOpenKeysRequest { .. } => "read_open_keys_request",
			MaintainerEffect::RecordUnloadDuration { .. } => "record_unload_duration",
			MaintainerEffect::SweepDeletedRequest { .. } => "sweep_deleted_request",
			MaintainerEffect::PrepareSplitRequest { .. } => "prepare_split_request",
			MaintainerEffect::ReportAlarmIndexReady { .. }
			| MaintainerEffect::ReportAlarmSplitRequest { .. }
			| MaintainerEffect::ReportSplitRequest { .. } => "report_split_request",
			MaintainerEffect::WatchHolderRequest { .. } => "watch_holder_request",
			MaintainerEffect::UnwatchHolder { .. } => "unwatch_holder",
			MaintainerEffect::CancelSplitReportRequest { .. } => "cancel_split_report_request",
			MaintainerEffect::RecoverVolumeRequest { .. } => "recover_volume_request",
			MaintainerEffect::CancelRecoveryRequest { .. } => "cancel_recovery_request",
			MaintainerEffect::CloseComplete => "close_complete",
			MaintainerEffect::CancelReplicaRequest { .. } => "cancel_replica_request",
			MaintainerEffect::WriteToColdRequest { .. } => "write_to_cold_request",
			MaintainerEffect::OpenColdRequest { .. } => "open_cold_request",
			MaintainerEffect::ReplicaRequest { .. } => "replica_request",
			MaintainerEffect::ReadRowRequest { .. } => "read_row_request",
			MaintainerEffect::Reply { .. } => "reply",
			MaintainerEffect::Repair(effect) => effect.name(),
			MaintainerEffect::CloseCold => "close_cold",
		}
	}
}

/// Work selected by operator repair requests; see `state::operator_repair`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepairEffect {
	/// **When**
	/// An admitted operator diagnostic needs one bounded read of a volume's metadata.
	/// **Executor**
	/// Reads only the selected metadata prefix or exact alarm, rechecks serving authority,
	/// and returns the correlated DiagnosticsResponse. It never changes lifecycle state.
	DiagnosticsRequest {
		op_id: OpId,
		request: rivet_data::generated::depot_client_protocol_v1::RepairDiagnosticsRequest,
	},
	/// **When**
	///
	/// An admitted inspection reads the name's records from the store.
	///
	/// **Executor**
	///
	/// The filesystem read task reads the name's pointer row, its pending fork row and the own
	/// record row of each listed incarnation, unordered against store writes, and returns
	/// InspectReadResponse. It fences instead when the grant ended or the store closed.
	InspectReadRequest {
		op_id: OpId,
		volume_key: depot_types::VolumeKey,
		incarnations: Vec<[u8; 16]>,
	},

	/// **When**
	///
	/// An admitted mutating repair call reads what it needs before it plans.
	///
	/// **Executor**
	///
	/// The filesystem read task reads the call's audit row and the facts `reads` names: a
	/// rebuild's probe opens a one-shot session against the planned record to read whether the
	/// incarnation's format row and root inode exist at its cold position, and an append's rows
	/// are read in one session at the volume's applied position. Then it returns
	/// PrepareResponse. It fences instead when the grant ended or the store closed.
	PrepareRequest {
		op_id: OpId,
		volume_log_key: depot_types::VolumeLogKey,
		operation_id: [u8; 16],
		reads: crate::repair_types::PrepareReads,
	},

	/// **When**
	///
	/// An admitted audit listing reads a name's audit rows.
	///
	/// **Executor**
	///
	/// The filesystem read task reads up to `limit` rows after `after` and returns
	/// AuditListResponse. It fences instead when the grant ended or the store closed.
	AuditListRequest {
		op_id: OpId,
		volume_key: depot_types::VolumeKey,
		after: Option<[u8; 16]>,
		limit: u32,
	},

	/// **When**
	///
	/// A real claim acquire ran after its intent was logged and an epoch was issued.
	///
	/// **Executor**
	///
	/// The repair clients start a serving volume client over the replica transport, whose
	/// open is answered locally with `row` and a claim at `claim_epoch`. The client claims the
	/// members, recovers the tail and commits a claim entry; the executor waits for it within
	/// `timeout_ms` and returns ClaimOpenResponse. Every replica request first checks the
	/// maintainer's grant and lease.
	ClaimOpenRequest {
		op_id: OpId,
		request: super::types::RepairClaimOpen,
	},

	/// **When**
	///
	/// A real append ran against a held claim at its expected tail.
	///
	/// **Executor**
	///
	/// The claim's client writes `body` as its next entry and waits for a disk quorum, then
	/// returns ClaimAppendResponse with the entry's position or why it failed.
	ClaimAppendRequest {
		op_id: OpId,
		volume_log_key: depot_types::VolumeLogKey,
		body: rivet_data::generated::depot_log_shard_v1::PayloadWrite,
	},

	/// **When**
	///
	/// A claim is released, expires, fails or loses its replica set.
	///
	/// **Executor**
	///
	/// Closes the claim's client and joins its tasks, then returns ClaimClosedResponse. A claim
	/// with no client answers at once.
	ClaimCloseRequest {
		op_id: OpId,
		volume_log_key: depot_types::VolumeLogKey,
	},
}

impl RepairEffect {
	fn name(&self) -> &'static str {
		match self {
			RepairEffect::DiagnosticsRequest { .. } => "repair_diagnostics_request",
			RepairEffect::InspectReadRequest { .. } => "repair_inspect_read_request",
			RepairEffect::PrepareRequest { .. } => "repair_prepare_request",
			RepairEffect::AuditListRequest { .. } => "repair_audit_list_request",
			RepairEffect::ClaimOpenRequest { .. } => "repair_claim_open_request",
			RepairEffect::ClaimAppendRequest { .. } => "repair_claim_append_request",
			RepairEffect::ClaimCloseRequest { .. } => "repair_claim_close_request",
		}
	}
}
