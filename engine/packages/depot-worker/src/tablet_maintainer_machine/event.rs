//! Correlated executor results and client lifecycle admissions.

use super::types::{OpId, OpenVolume, RequestId};
use rivet_data::generated::depot_storage_worker_protocol_v1::StorageWorkerResponse;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaintainerEvent {
	/// **When**
	///
	/// The ordered store completes replay and its optional alarm projection.
	///
	/// **State**
	///
	/// Applies only the retained operation, then offers its certified summary to delivery.
	///
	/// **Effects**
	///
	/// Continues normal filesystem work and cancels a replaced callback before rescheduling.
	FilesystemAlarmApplyResponse {
		op_id: OpId,
		summary: rivet_data::generated::shared_depot_alarms_v1::AlarmSummary,
	},
	/// **When**
	///
	/// The executor completes or cancels one requested alarm operation.
	///
	/// **State**
	///
	/// Ignores stale identities, releases the attempt, and accepts results only while serving.
	///
	/// **Effects**
	///
	/// May invoke a due callback, persist its retry deadline, retry IO, or fence a conflict.
	AlarmResponse {
		op_id: OpId,
		result: Result<crate::alarms::Response, String>,
	},
	/// **When**
	///
	/// The coordinator replies to the child index-ready report.
	///
	/// **State**
	///
	/// Accepts confirmation while opening; failure leaves the report retryable.
	///
	/// **Effects**
	///
	/// Finishes opening or rearms the existing split retry timer.
	AlarmIndexReadyResponse {
		result: Result<
			rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
			String,
		>,
	},
	/// **When**
	///
	/// The ordered store completes a bounded bootstrap page.
	///
	/// **State**
	///
	/// Extends only the matching cold-open operation timeout.
	///
	/// **Effects**
	///
	/// Rearms its native operation timer while import continues.
	AlarmBootstrapProgress { op_id: OpId },
	/// **When**
	///
	/// A coordinator asks whether a disk may stop.
	///
	/// **State**
	///
	/// Reserves a bounded check for the next complete status round.
	///
	/// **Effects**
	///
	/// Polls replicas; replies only with fresh evidence or a typed blocker.
	StopReadinessRequest {
		request_id: RequestId,
		request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	},
	/// **When**
	///
	/// A V2 grant handoff asks this owner to quiesce fresh creates.
	///
	/// **State**
	///
	/// Retains the create barrier without closing clients. A real stop intent from the
	/// supervisor's exact manifest keeps the barrier for the grant's lifetime.
	///
	/// **Effects**
	///
	/// Replies only after admitted initializers and sent loads resolve.
	HandoffReadinessRequest {
		request_id: RequestId,
		request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
		stop_intent: bool,
	},
	/// **When**
	///
	/// The ordered store finishes one materialization read or durable mutation.
	///
	/// **State**
	///
	/// Ignores stale operation identities, advances copying, or publishes a completed
	/// chain to cached records. Storage failure fences the maintainer.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and may request a source page or more local work.
	MaterializeResponse {
		op_id: OpId,
		result: Result<depot_fs_core::forks::materialize::Progress, String>,
	},
	/// **When**
	///
	/// The filesystem executor finishes inspecting or updating one durable cleanup identity.
	///
	/// **State**
	///
	/// Advances only its current bounded job; stale operation ids are ignored.
	///
	/// **Effects**
	///
	/// Starts a regional check or release, retires completed work, or fences uncertain storage.
	ForkCleanupResponse {
		op_id: OpId,
		result: Result<depot_fs_core::forks::cleanup::Progress, String>,
	},
	/// **When**
	///
	/// The destination receives a fork or restore command.
	///
	/// **State**
	///
	/// Reserves its name and bounded callers under the exact immutable spec.
	///
	/// **Effects**
	///
	/// Queues receipt inspection before new preparation and arms the caller deadline.
	ForkRequest {
		request_id: RequestId,
		spec: depot_fs_core::FsForkSpec,
		deadline_boot_ms: u64,
	},

	/// **When**
	///
	/// A bounded local fork step completes.
	///
	/// **State**
	///
	/// Advances only its current job; durable progress remains after caller expiry.
	///
	/// **Effects**
	///
	/// Queues local or regional work, replies on publication, or fences uncertain storage.
	ForkStoreResponse {
		op_id: OpId,
		result: Result<depot_fs_core::forks::StoreResult, String>,
	},

	/// **When**
	///
	/// A regional source attempt completes or fails.
	///
	/// **State**
	///
	/// Keeps the exact durable preparation and ignores obsolete operations.
	///
	/// **Effects**
	///
	/// Persists source images or metadata pages, or schedules retry without abandoning a hold.
	ForkPeerResponse {
		op_id: OpId,
		result: Result<rivet_data::generated::depot_client_protocol_v1::ClientResponse, String>,
	},

	/// **When**
	///
	/// A routed peer requests acquisition, release, or destination presence.
	///
	/// **State**
	///
	/// Reserves a bounded reply and, for loaded Latest sources, a finite apply barrier.
	///
	/// **Effects**
	///
	/// Refreshes source progress when needed, queues storage and arms the caller deadline.
	ForkReferenceRequest {
		request_id: RequestId,
		request: rivet_data::generated::depot_client_protocol_v1::FsForkReferenceRequest,
		deadline_boot_ms: u64,
	},
	/// **When**
	///
	/// Ordered reference storage completes under its admitted grant.
	///
	/// **State**
	///
	/// Consumes the source durability proof, even after caller expiry; stale operations are ignored.
	///
	/// **Effects**
	///
	/// Replies and schedules safe trims, or fences after uncertain storage failure.
	ForkReferenceResponse {
		op_id: OpId,
		result: Result<depot_fs_core::forks::ReferenceResult, String>,
	},

	/// **When**
	///
	/// A bounded scan of persisted pending work finishes.
	///
	/// **State**
	///
	/// Retains its next cursor and schedules only scopes in this tablet.
	///
	/// **Effects**
	///
	/// Queues bounded sweeps, or fences on a storage error.
	PendingWorkResponse {
		op_id: OpId,
		result: Result<depot_fs_core::work::PendingWorkPage, String>,
	},
	/// **When**
	///
	/// One retained incarnation's bounded sweep and cursor flush finish.
	///
	/// **State**
	///
	/// Releases the operation; the durable marker remains the recovery authority.
	///
	/// **Effects**
	///
	/// Allows more queued work, or fences on uncertain storage failure.
	SweepPendingResponse {
		op_id: OpId,
		result: Result<depot_fs_core::work::PendingSweepProgress, String>,
	},

	/// **When**
	///
	/// A routed caller requests checkpoint metadata or retention changes.
	///
	/// **State**
	///
	/// Reserves a bounded caller slot, rejecting frozen or retired targets.
	///
	/// **Effects**
	///
	/// Queues ordered store work and arms the original caller deadline.
	CheckpointRequest {
		request_id: RequestId,
		request: rivet_data::generated::depot_client_protocol_v1::VolumeCheckpointRequest,
		deadline_boot_ms: u64,
	},
	/// **When**
	///
	/// The ordered store finishes checkpoint administration.
	///
	/// **State**
	///
	/// Applies the durable record proof before releasing the store slot.
	///
	/// **Effects**
	///
	/// Replies after fresh authority validation, or fences on uncertain I/O.
	CheckpointResponse {
		op_id: OpId,
		result: Result<depot_fs_core::checkpoints::CheckpointResult, String>,
	},

	/// **When**
	///
	/// The store executor finished applying a recovered maintainer log op.
	///
	/// **State**
	///
	/// Marks the op applied and lets its volume's apply continue to its next op; an error
	/// fences.
	///
	/// **Effects**
	///
	/// The next replayed op of the volume, if it may apply now.
	MaintainerOpResponse {
		op_id: OpId,
		result: Result<crate::tablet_maintainer_machine::types::ReplayOutcome, String>,
	},

	/// **When**
	///
	/// A routed caller asks for a fixed filesystem read session or its rows.
	///
	/// **State**
	///
	/// Reserves a bounded caller slot in the ordered store queue.
	///
	/// **Effects**
	///
	/// Schedules filesystem store work and the caller's original deadline.
	FilesystemQueryRequest {
		request_id: RequestId,
		request: rivet_data::generated::shared_depot_fs_read_v1::FsReadRequest,
		deadline_boot_ms: u64,
	},
	/// **When**
	///
	/// The ordered store operation finishes a filesystem read.
	///
	/// **State**
	///
	/// Releases its operation and caller reservation after checking current authority.
	///
	/// **Effects**
	///
	/// Replies once, or fences on a storage failure.
	FilesystemQueryResponse {
		op_id: OpId,
		result: Result<rivet_data::generated::shared_depot_fs_read_v1::FsReadResponse, String>,
	},

	/// **When**
	///
	/// The owner selects a loaded volume for filesystem replay.
	///
	/// **State**
	///
	/// Queues a bounded authoritative record lookup; duplicates are ignored.
	///
	/// **Effects**
	///
	/// Reads its record through the ordered store queue.
	///
	FilesystemLoadRequest { key: depot_types::VolumeLogKey },
	/// **When**
	///
	/// The ordered store finishes a filesystem record lookup.
	///
	/// **State**
	///
	/// Matching records initialize applied at cold; invalid results fence.
	///
	/// **Effects**
	///
	/// Starts qualified committed reads.
	///
	FilesystemLoadResponse {
		op_id: OpId,
		result: Result<Option<depot_fs_core::FsVolumeRecord>, String>,
	},
	/// **When**
	///
	/// A writer hints that a loaded volume has new committed entries.
	///
	/// **State**
	///
	/// The hint never changes a commit or applied position.
	///
	/// **Effects**
	///
	/// Starts the bounded fallback status round immediately.
	///
	FilesystemCommittedRequest {
		key: depot_types::VolumeLogKey,
		through: u64,
	},
	/// **When**
	///
	/// The store finishes every bounded batch for committed entries.
	///
	/// **State**
	///
	/// Matching success publishes applied. A failed replica reread retries from the
	/// unchanged position; storage or validation failure fences.
	///
	/// **Effects**
	///
	/// Continues tailing and schedules due flush work.
	///
	FilesystemApplyResponse {
		op_id: OpId,
		result: Result<
			rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
			super::types::FilesystemApplyError,
		>,
	},
	/// **When**
	///
	/// The store executor wrote a flush's records after every earlier mutation and
	/// started making them durable.
	///
	/// **State**
	///
	/// The matching staging flush becomes uploading: its records become unpublished,
	/// so opens, applies and loads of them may start, and lookups of them still wait.
	/// A report for an uploading
	/// flush fences.
	///
	/// **Effects**
	///
	/// None directly; the store queue is driven after the event.
	///
	FilesystemFlushStaged { op_id: OpId },

	/// **When**
	///
	/// The flush's upload made its records and all preceding row writes durable, or
	/// staging or the upload failed.
	///
	/// **State**
	///
	/// Matching success advances cold; failure fences without trimming.
	///
	/// **Effects**
	///
	/// Trims each member through the completed position.
	///
	FilesystemFlushResponse {
		op_id: OpId,
		result: Result<(), String>,
	},
	/// **When**
	///
	/// The tablet store's unflushed bytes reached SlateDB's limit, so every store write now
	/// waits for a memtable upload, or a waiting write went through again.
	///
	/// **State**
	///
	/// Records whether the memtable budget is full. Once it has stayed full for
	/// `MEMTABLE_FULL_SHED_AFTER_MS`, reads that would wait for an apply or for queued store
	/// work, and opens whose row lookup would queue behind store work, are refused at
	/// admission; a write going through ends that.
	///
	/// **Effects**
	///
	/// Arms `MemtableBudgetShed` when the budget fills and cancels it when it clears.
	///
	MemtableBudget { full: bool },

	/// **When**
	///
	/// The worker receives a typed command for this tablet's database volume.
	///
	/// **State**
	///
	/// Admission checks the current lifecycle phase; it does not run application code.
	///
	/// **Effects**
	///
	/// Dispatches bounded handler work while serving, otherwise replies with a rejection.
	KvCommandRequest {
		request_id: super::types::RequestId,
		request: rivet_data::generated::depot_kv_command_v1::KvCommandRequest,
		/// The command's received wire size.
		request_bytes: u64,
		/// Receiving-host deadline translated by UPS; retries never reset it.
		deadline_boot_ms: u64,
	},

	/// **When**
	///
	/// The coordinator transport finishes a correlated catch-up report.
	///
	/// **State**
	///
	/// Acceptance waits for the next manifest; failure releases the attempt. Obsolete phase replies are ignored.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and schedules a fresh catch-up round on failure.
	ReportCatchUpResponse {
		op_id: OpId,
		result: Result<
			rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
			String,
		>,
	},

	/// **When**
	///
	/// A supervisor calls update_retired_tablets with the retired parents assigned
	/// to this maintainer, derived from the live range containing each first hash, and
	/// whether the manifest records this tablet's import of its own parent's log.
	///
	/// **State**
	///
	/// Retains unchanged cleanup progress. Withdrawal stops store deletion and reporting;
	/// after store deletion, owed log cleanup and member listings may finish under the
	/// current grant. A record that gained child imports refreshes the cleanup's copy. Changed checkpoint
	/// identities or this tablet's own ID fence the owner. A recorded parent log import ends
	/// this tablet's report; an unrecorded one makes it due once imported.
	///
	/// **Effects**
	///
	/// Queues one parent cleanup at a time after startup; cancels withdrawn reports and
	/// physical-cleanup retries while preserving owed log-forget retries.
	RetiredTabletsUpdated {
		retired: Vec<rivet_data::generated::depot_tablet_manifest_v1::RetiredTablet>,
		parent_log_recorded: bool,
	},

	/// **When**
	///
	/// The ordered storage executor finishes checking or deleting a retired parent.
	///
	/// **State**
	///
	/// A matching deleted result permits reporting after any late-discovered log holders; pins and
	/// errors retain the same parent and checkpoint for retry. Obsolete completions cannot
	/// advance cleanup.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and reports completion or arms a cold-interval retry.
	DeleteRetiredResponse {
		op_id: OpId,
		result: Result<rivet_env::capabilities::slatedb_result::DeleteDatabaseResult, String>,
	},

	/// **When**
	///
	/// The coordinator responds to a retired-parent deletion report.
	///
	/// **State**
	///
	/// Acceptance completes the assigned cleanup; other responses retain the report
	/// for retry. Results from withdrawn or expired operations are ignored.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and arms a retry after an unaccepted report.
	ReportRetiredResponse {
		op_id: OpId,
		result: Result<
			rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
			String,
		>,
	},

	/// **When**
	///
	/// The executor finishes a ReportParentLogImportedRequest while the grant still serves.
	///
	/// **State**
	///
	/// Acceptance records the import; any other result waits for a retry. Results of
	/// finished operations are ignored.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and arms `ParentLogReportRetry` after an unaccepted
	/// report.
	ReportParentLogImportedResponse {
		op_id: OpId,
		result: Result<
			rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
			String,
		>,
	},

	/// **When**
	///
	/// The executor finishes a ReportVersionMarksRequest while the grant still serves.
	///
	/// **State**
	///
	/// Acceptance records the reported marks as known to the coordinator; any other result
	/// waits for a retry. Results of finished or withdrawn operations are ignored.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and arms `VersionMarksRetry` after an unaccepted report.
	ReportVersionMarksResponse {
		op_id: OpId,
		result: Result<
			rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
			String,
		>,
	},

	/// **When**
	///
	/// The supervisor observes a newer replica set under this owner's unchanged grant.
	///
	/// **State**
	///
	/// Replaces voting membership while retaining the open store and pending mutation
	/// barriers. Older versions are ignored; inconsistent ownership or versions fence.
	///
	/// **Effects**
	///
	/// Cancels recovery clients that captured the previous set, lists new members,
	/// and schedules learner catch-up. It does not reopen the cold store.
	AssignmentUpdated {
		tablet: rivet_data::generated::depot_tablet_manifest_v1::Tablet,
	},
	/// **When**
	///
	/// The supervisor observes a validated global manifest, even if this tablet is unchanged.
	///
	/// **State**
	///
	/// Updates assignment and global version coherently. A newer version at the same
	/// owner/grant fences old conditional writes and releases only ordinary quiescence.
	///
	/// **Effects**
	///
	/// Reuses assignment updates and their existing mutation barriers.
	ManifestObserved {
		tablet_manifest_version: u64,
		tablet: rivet_data::generated::depot_tablet_manifest_v1::Tablet,
	},

	/// **When**
	///
	/// An operator asks the serving maintainer to tier its applied data now.
	///
	/// **State**
	///
	/// Marks every registered volume with applied data above cold for a prompt cold write and
	/// queues a cold pass for pending rows. Non-serving owners and a full
	/// caller budget reject the request.
	///
	/// **Effects**
	///
	/// Replies at once with how many volumes it asked to write, then dispatches the flushes and
	/// the pass through the store queue's usual budgets.
	OperatorTierRequest { request_id: super::types::RequestId },

	/// **When**
	///
	/// An operator asks the serving maintainer to unload volumes that have been quiet.
	///
	/// **State**
	///
	/// Begins the unload of at most `limit` loaded volumes, capped per request, that received
	/// no request for `quiet_ms`, raised to the holder check interval, and have no open, unload
	/// or deletion in progress. An unload that still reads its row checks quietness again, so
	/// a volume used in the meantime stays loaded.
	///
	/// **Effects**
	///
	/// Replies at once with the unloads it began and the volumes still loaded; the unloads
	/// proceed like closed volumes', after ordinary tiering has their data.
	OperatorUnloadRequest {
		request_id: super::types::RequestId,
		quiet_ms: u64,
		limit: u32,
	},

	/// **When**
	///
	/// The client transport asks the current maintainer to delete one exact incarnation.
	///
	/// **State**
	///
	/// Reserves its key, then selects a claim or unloaded-row lookup; stale incarnations
	/// cannot delete a recreated volume. Conflicting lifecycle work gets a retryable rejection.
	///
	/// **Effects**
	///
	/// Arms a caller timer and queues row reads or replica requests; an existing quorum
	/// tombstone can reply immediately. Background cleanup continues after caller expiry.
	DeleteVolumeRequest {
		request_id: super::types::RequestId,
		volume_log_key: depot_types::VolumeLogKey,
	},

	/// **When**
	///
	/// The ordered store executor has flushed one bounded deletion batch or failed it.
	///
	/// **State**
	///
	/// Advances only the correlated cursor. Final completion makes replica forgets
	/// eligible; an uncertain storage failure fences the maintainer.
	///
	/// **Effects**
	///
	/// Cancels the operation timer, queues another sweep or sends unconditional forgets,
	/// or closes the store after fencing. Obsolete responses emit nothing.
	SweepDeletedResponse {
		op_id: OpId,
		result: Result<super::types::DeleteSweepResult, String>,
	},

	/// **When**
	///
	/// The ordered store executor finishes retaining the checkpoint or preparing one child clone.
	///
	/// **State**
	///
	/// Advances only the current frozen attempt; failure retains identical inputs for retry.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and queues the next clone/report or arms a retry timer.
	PrepareSplitResponse {
		op_id: OpId,
		result: Result<u64, String>,
	},

	/// **When**
	///
	/// The executor's topology watch for `holder`, started by `WatchHolderRequest`, resolved:
	/// that process's lease expired or was released, or a newer process of its worker claimed.
	///
	/// **State**
	///
	/// Forgets the holder. While serving, each volume parked under it that still names it,
	/// stayed quiet since it parked and is not busy begins a holder-gone unload; the rest
	/// return to the idle rule. A holder with
	/// nothing parked here changes nothing; gone is final, so a late event is never wrong.
	///
	/// **Effects**
	///
	/// `UnwatchHolder` for the resolved watch, and the unloads' row reads through the drives.
	HolderGone {
		holder: rivet_data::generated::shared_depot_types_v1::VolumeHolder,
	},

	/// **When**
	///
	/// The executor could not start the topology watch a `WatchHolderRequest` asked for, for
	/// example at topology's watch capacity.
	///
	/// **State**
	///
	/// Marks the holder unwatched and, while it still has parked volumes, schedules a retry
	/// after `holder_check_after_ms`. A refusal of any watch but the holder's current one is
	/// ignored.
	///
	/// **Effects**
	///
	/// Re-arms `HolderWatchRetry`.
	HolderWatchRefused {
		holder: rivet_data::generated::shared_depot_types_v1::VolumeHolder,
		/// The refused request's watch id; only the holder's current watch counts.
		watch_id: u64,
	},

	/// **When**
	///
	/// The coordinator transport returns the frozen maintainer's split-prepared report.
	///
	/// **State**
	///
	/// Acceptance completes preparation; lost authority fences the owner; transient errors retry.
	/// Old operation IDs have no effect.
	///
	/// **Effects**
	///
	/// Cancels the report timer, arms a retry, or closes the store after fencing.
	ReportSplitResponse {
		op_id: OpId,
		result: Result<
			rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
			String,
		>,
	},

	/// **When**
	///
	/// The executor has completed or canceled a recovery client and joined all of its tasks.
	///
	/// **State**
	///
	/// Retires the matching job, updates pass counts, and ignores obsolete operation IDs.
	/// Releases the key to a waiting reopen or its background unload.
	///
	/// **Effects**
	///
	/// Schedules another queued repair, refreshes replica observations, resumes lifecycle work,
	/// or replies to the operator.
	RecoverVolumeResponse {
		op_id: super::types::OpId,
		result: Result<depot_volume::volume_client_machine::types::RecoveryReport, String>,
	},

	/// **When**
	///
	/// A volume client closes after publishing its final committed position.
	///
	/// **State**
	///
	/// Checks the exact replica claim and schedules background unloading while serving.
	/// A stale claim, a deleted volume or an absent key changes nothing.
	///
	/// **Effects**
	///
	/// Replies promptly, then reads the cold row, finishes cold writes and conditionally forgets replicas.
	CloseVolumeRequest {
		client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
		request_id: super::types::RequestId,
		volume_log_key: depot_types::VolumeLogKey,
	},

	/// **When**
	///
	/// The supervisor stops this owner before releasing its tablet grant.
	///
	/// **State**
	///
	/// Closes admission and retires pending attempts. Entries above cold remain on their replicas.
	///
	/// **Effects**
	///
	/// Rejects pending callers, cancels and joins recovery clients, then closes the ordered
	/// cold-store executor after its current work.
	Drain,

	/// **When**
	///
	/// The executor has closed the cold-store handle and joined its background tasks.
	///
	/// **State**
	///
	/// Completes draining, or retains the fenced reason as the final phase.
	///
	/// **Effects**
	///
	/// Emits CloseComplete to release the owner’s cleanup waiter. Once producers drop their
	/// inbox handles, the runner finishes normally.
	ColdClosed { result: Result<(), String> },

	/// **When**
	///
	/// A member's log shard pushed the synced state of replicas that changed, through the
	/// tablet supervisor, which groups notices by tablet.
	///
	/// **State**
	///
	/// Ignored unless serving and the sender is a member. When the shard's grant epoch or
	/// replica set version differs from this maintainer's, each named loaded volume is
	/// queued for a targeted status poll instead. Otherwise each notice is applied exactly
	/// as a status reply for that member would be.
	///
	/// **Effects**
	///
	/// None directly; the drives may poll, read, flush, trim or unload what the notices
	/// enable.
	ReplicaStatusNotices {
		worker_id: rivet_topology::worker::WorkerId,
		tablet_grant_epoch: u64,
		tablet_replica_set_version: u64,
		notices: Vec<super::types::StatusNotice>,
	},

	/// **When**
	///
	/// The cold-store executor completes a pass's single memtable flush.
	///
	/// **State**
	///
	/// Success retires creation metadata and records completed cuts awaiting trim. Partial
	/// discards stay buffered; failure fences the lifetime.
	///
	/// **Effects**
	///
	/// Cancels the pass timeout and queues member trims for complete cuts; failures reject
	/// pending callers and close the store.
	WriteToColdResponse {
		op_id: OpId,
		result: Result<(), String>,
	},

	/// **When**
	///
	/// The cold-store executor completes the grant-fenced open.
	///
	/// **State**
	///
	/// Success keeps the maintainer log record and starts member discovery; failure fences
	/// this maintainer lifetime.
	///
	/// **Effects**
	///
	/// Cancels the open timer and lists all tablet members, or closes the store.
	OpenColdResponse {
		op_id: OpId,
		/// The tablet store's record of its maintainer log, when it has one.
		result: Result<
			Option<rivet_data::generated::depot_maintainer_log_v1::MaintainerLogRecord>,
			String,
		>,
	},

	/// **When**
	///
	/// A storage worker finishes a listing, load, committed read, trim, status probe, or cleanup.
	///
	/// **State**
	///
	/// A matching operation updates member evidence and its pending quorum; stale results are ignored.
	///
	/// **Effects**
	///
	/// Cancels the operation timer, continues pagination, retries unavailable members,
	/// or replies to a create/open once the required replicas acknowledge.
	ReplicaResponse {
		op_id: OpId,
		result: Result<StorageWorkerResponse, String>,
	},

	/// **When**
	///
	/// A client asks the maintainer to create or open a volume.
	///
	/// **State**
	///
	/// Admits one bounded lifecycle operation per volume key while serving.
	///
	/// **Effects**
	///
	/// Reads an unloaded row, loads member replicas, or immediately rejects admission.
	OpenVolumeRequest(OpenVolume),

	/// **When**
	///
	/// The cold-store executor finishes a lifecycle lookup, a close's deletion check, or a replay floor lookup.
	///
	/// **State**
	///
	/// A current result checks the durable fork reservation before selecting an incarnation or establishing the stored row.
	///
	/// **Effects**
	///
	/// Cancels the lookup timer and queues loads, unloading, or a committed read above the stored floor; missing/deleted rows
	/// reject an open or end unloading without forgetting replicas.
	ReadRowResponse {
		op_id: OpId,
		result: Result<depot_fs_core::forks::LifecycleLookup, String>,
	},

	/// **When**
	///
	/// The filesystem read task finished an open's read-back.
	///
	/// **State**
	///
	/// Releases the read's operation. A read whose open still waits answers it; a failed read
	/// answers it without values and fences nothing, because the read only saves the opener a
	/// later read of its own.
	///
	/// **Effects**
	///
	/// Cancels the open's read timer and replies to the open.
	ReadOpenKeysResponse {
		op_id: OpId,
		result: Result<Vec<Option<vbare::Bytes>>, String>,
	},

	/// An operator repair request or a repair executor's result; see [`RepairEvent`].
	Repair(RepairEvent),

	/// **When**
	///
	/// The supervisor observes loss of this tablet grant or the local topology lease.
	///
	/// **State**
	///
	/// Fences the lifetime and retires all pending operations and callers.
	///
	/// **Effects**
	///
	/// Rejects callers, cancels timers, and closes the cold-store handle and its tasks.
	Fence,
}

impl rivet_state_machine::EventBody for MaintainerEvent {
	fn name(&self) -> &'static str {
		match self {
			Self::FilesystemAlarmApplyResponse { .. } => "filesystem_apply_response",
			Self::AlarmResponse { .. } => "alarm_response",
			MaintainerEvent::StopReadinessRequest { .. } => "stop_readiness_request",
			MaintainerEvent::HandoffReadinessRequest { .. } => "handoff_readiness_request",
			MaintainerEvent::ManifestObserved { .. } => "manifest_observed",
			MaintainerEvent::MaintainerOpResponse { .. } => "maintainer_op_response",
			MaintainerEvent::MaterializeResponse { .. } => "materialize_response",
			MaintainerEvent::ForkCleanupResponse { .. } => "fork_cleanup_response",
			MaintainerEvent::ForkRequest { .. } => "fork_request",
			MaintainerEvent::ForkStoreResponse { .. } => "fork_store_response",
			MaintainerEvent::ForkPeerResponse { .. } => "fork_peer_response",
			MaintainerEvent::ForkReferenceRequest { .. } => "fork_reference_request",
			MaintainerEvent::ForkReferenceResponse { .. } => "fork_reference_response",
			MaintainerEvent::PendingWorkResponse { .. } => "pending_work_response",
			MaintainerEvent::SweepPendingResponse { .. } => "sweep_pending_response",
			MaintainerEvent::CheckpointRequest { .. } => "checkpoint_request",
			MaintainerEvent::CheckpointResponse { .. } => "checkpoint_response",
			MaintainerEvent::FilesystemQueryRequest { .. } => "filesystem_query_request",
			MaintainerEvent::FilesystemQueryResponse { .. } => "filesystem_query_response",
			MaintainerEvent::FilesystemLoadRequest { .. } => "filesystem_load_request",
			MaintainerEvent::FilesystemLoadResponse { .. } => "filesystem_load_response",
			MaintainerEvent::FilesystemCommittedRequest { .. } => "filesystem_committed_request",
			MaintainerEvent::FilesystemApplyResponse { .. } => "filesystem_apply_response",
			MaintainerEvent::FilesystemFlushStaged { .. } => "filesystem_flush_staged",
			MaintainerEvent::FilesystemFlushResponse { .. } => "filesystem_flush_response",
			MaintainerEvent::MemtableBudget { .. } => "memtable_budget",
			MaintainerEvent::KvCommandRequest { .. } => "kv_command_request",
			MaintainerEvent::ReportCatchUpResponse { .. } => "report_catch_up_response",
			MaintainerEvent::RetiredTabletsUpdated { .. } => "retired_tablets_updated",
			MaintainerEvent::DeleteRetiredResponse { .. } => "delete_retired_response",
			MaintainerEvent::ReportRetiredResponse { .. } => "report_retired_response",
			MaintainerEvent::ReportParentLogImportedResponse { .. } => {
				"report_parent_log_imported_response"
			}
			MaintainerEvent::ReportVersionMarksResponse { .. } => "report_version_marks_response",
			MaintainerEvent::AssignmentUpdated { .. } => "assignment_updated",
			MaintainerEvent::OperatorTierRequest { .. } => "operator_tier_request",
			MaintainerEvent::OperatorUnloadRequest { .. } => "operator_unload_request",
			MaintainerEvent::DeleteVolumeRequest { .. } => "delete_volume_request",
			MaintainerEvent::SweepDeletedResponse { .. } => "sweep_deleted_response",
			MaintainerEvent::PrepareSplitResponse { .. } => "prepare_split_response",
			MaintainerEvent::ReportSplitResponse { .. } => "report_split_response",
			MaintainerEvent::HolderGone { .. } => "holder_gone",
			MaintainerEvent::HolderWatchRefused { .. } => "holder_watch_refused",
			MaintainerEvent::RecoverVolumeResponse { .. } => "recover_volume_response",
			MaintainerEvent::CloseVolumeRequest { .. } => "close_volume_request",
			MaintainerEvent::Drain => "drain",
			MaintainerEvent::ColdClosed { .. } => "cold_closed",
			MaintainerEvent::ReplicaStatusNotices { .. } => "replica_status_notices",
			MaintainerEvent::WriteToColdResponse { .. } => "write_to_cold_response",
			MaintainerEvent::AlarmIndexReadyResponse { .. }
			| MaintainerEvent::AlarmBootstrapProgress { .. }
			| MaintainerEvent::OpenColdResponse { .. } => "open_cold_response",
			MaintainerEvent::ReplicaResponse { .. } => "replica_response",
			MaintainerEvent::OpenVolumeRequest(..) => "open_volume_request",
			MaintainerEvent::ReadRowResponse { .. } => "read_row_response",
			MaintainerEvent::ReadOpenKeysResponse { .. } => "read_open_keys_response",
			MaintainerEvent::Repair(event) => event.name(),
			MaintainerEvent::Fence => "fence",
		}
	}
}

/// Operator repair requests and the results of the repair work they start; see
/// `state::operator_repair`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepairEvent {
	/// **When**
	/// An operator requests a page of lifecycle diagnostics.
	/// **State**
	/// Reserves one read slot if serving and within the request budget.
	/// **Effects**
	/// Arms the read deadline and dispatches DiagnosticsRequest, or replies with refusal.
	DiagnosticsRequest {
		request_id: RequestId,
		request: rivet_data::generated::depot_client_protocol_v1::RepairDiagnosticsRequest,
	},
	/// **When**
	/// The diagnostic read completes or fails.
	/// **State**
	/// Releases the matching read slot; ignores a late completion after timeout.
	/// **Effects**
	/// Cancels its deadline and replies with the page or a retryable refusal.
	DiagnosticsResponse {
		op_id: OpId,
		result:
			Result<rivet_data::generated::depot_client_protocol_v1::RepairDiagnosticsPage, String>,
	},
	/// **When**
	///
	/// An operator asks the serving maintainer what it holds for one volume name.
	///
	/// **State**
	///
	/// Refuses the request at admission when the maintainer is not serving, its caller budget
	/// is full or its read slots are taken. Otherwise begins one store read of the name's
	/// records and changes nothing else.
	///
	/// **Effects**
	///
	/// Arms the operation timer and dispatches InspectReadRequest, or replies with the refusal.
	InspectRequest {
		request_id: RequestId,
		volume_key: depot_types::VolumeKey,
	},

	/// **When**
	///
	/// The filesystem read task finished the inspection's store read, or failed it.
	///
	/// **State**
	///
	/// Releases the read's operation; a stale response does nothing. The reply combines the
	/// store's records with the maintainer's memory at this moment. A failed read fences
	/// nothing, since it only informs an operator.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and replies with the inspection, or with `Overloaded` when
	/// the read failed.
	InspectReadResponse {
		op_id: OpId,
		result: Result<super::types::RepairRecords, String>,
	},

	/// **When**
	///
	/// An operator sends a mutating repair call to the serving maintainer.
	///
	/// **State**
	///
	/// Refuses at admission when the maintainer is not serving, the call names another grant,
	/// its operation id is in flight, another call of the name is in flight, the tablet runs
	/// its most calls, or its read slots are taken. A retry of a call this lifetime finished
	/// answers the stored outcome. Otherwise records the call as preparing.
	///
	/// **Effects**
	///
	/// Arms the operation timer and dispatches PrepareRequest, or replies.
	Request {
		request_id: RequestId,
		request: rivet_data::generated::depot_client_protocol_v1::RepairRequest,
	},

	/// **When**
	///
	/// The filesystem read task finished a call's preparing read, or failed it.
	///
	/// **State**
	///
	/// Releases the read's operation; a stale response does nothing. A stored audit row of the
	/// call answers it. Otherwise the call's body plans: a dry run ends with the plan, and a
	/// real call whose digest matches moves to logging its intent.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and replies, or submits the call's intent audit op to the
	/// maintainer log.
	PrepareResponse {
		op_id: OpId,
		result: Result<crate::repair_types::RepairPrepared, String>,
	},

	/// **When**
	///
	/// An operator lists the repair audit rows of one name.
	///
	/// **State**
	///
	/// Refuses at admission like an inspection; otherwise begins one store read.
	///
	/// **Effects**
	///
	/// Arms the operation timer and dispatches AuditListRequest, or replies.
	AuditListRequest {
		request_id: RequestId,
		request: rivet_data::generated::depot_client_protocol_v1::RepairAuditListRequest,
	},

	/// **When**
	///
	/// The filesystem read task finished an audit listing's read, or failed it.
	///
	/// **State**
	///
	/// Releases the read's operation; a stale response does nothing.
	///
	/// **Effects**
	///
	/// Cancels the operation timer and replies with the page, or with `Overloaded`.
	AuditListResponse {
		op_id: OpId,
		result: Result<rivet_data::generated::depot_client_protocol_v1::RepairAuditPage, String>,
	},

	/// **When**
	///
	/// The executor's repair client of a claim finished opening: it claimed the members,
	/// recovered the tail and committed its claim entry, or it failed or stopped.
	///
	/// **State**
	///
	/// Only the claim whose operation is `op_id` and which is opening takes it. Success holds
	/// the claim at its claim entry; failure closes the client.
	///
	/// **Effects**
	///
	/// Rearms the claim's timer to its time to live and completes the acquire call, or sends
	/// ClaimCloseRequest and refuses the call.
	ClaimOpenResponse {
		op_id: OpId,
		result: Result<super::types::RepairClaimOpened, String>,
	},

	/// **When**
	///
	/// A claim's client committed one appended entry on a quorum, or failed it.
	///
	/// **State**
	///
	/// Only the claim appending under `op_id` takes it. Success moves the claim's tail; an
	/// append that wrote nothing keeps the claim; an unknown outcome closes the claim.
	///
	/// **Effects**
	///
	/// Rearms the time to live and completes the append call, or sends ClaimCloseRequest.
	ClaimAppendResponse {
		op_id: OpId,
		result: Result<
			rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
			super::types::RepairAppendFailure,
		>,
	},

	/// **When**
	///
	/// A claim's client closed and its tasks joined.
	///
	/// **State**
	///
	/// Ends the claim: opens of the name are admitted again.
	///
	/// **Effects**
	///
	/// Cancels the claim's timer, completes a waiting release call and resumes opens that
	/// waited for the name.
	ClaimClosedResponse { op_id: OpId },
}

impl RepairEvent {
	fn name(&self) -> &'static str {
		match self {
			RepairEvent::DiagnosticsRequest { .. } => "repair_diagnostics_request",
			RepairEvent::DiagnosticsResponse { .. } => "repair_diagnostics_response",
			RepairEvent::InspectRequest { .. } => "repair_inspect_request",
			RepairEvent::InspectReadResponse { .. } => "repair_inspect_read_response",
			RepairEvent::Request { .. } => "repair_request",
			RepairEvent::PrepareResponse { .. } => "repair_prepare_response",
			RepairEvent::AuditListRequest { .. } => "repair_audit_list_request",
			RepairEvent::ClaimOpenResponse { .. } => "repair_claim_open_response",
			RepairEvent::ClaimAppendResponse { .. } => "repair_claim_append_response",
			RepairEvent::ClaimClosedResponse { .. } => "repair_claim_closed_response",
			RepairEvent::AuditListResponse { .. } => "repair_audit_list_response",
		}
	}
}

impl MaintainerEvent {
	/// One replica status notice from `worker` under `tablet`'s current grant and
	/// replica set, as the tablet supervisor delivers it.
	#[doc(hidden)]
	pub fn notice(
		tablet: &rivet_data::generated::depot_tablet_manifest_v1::Tablet,
		volume_log_key: depot_types::VolumeLogKey,
		worker_id: rivet_topology::worker::WorkerId,
		state: rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState,
	) -> Self {
		MaintainerEvent::ReplicaStatusNotices {
			worker_id,
			tablet_grant_epoch: tablet.tablet_grant_epoch,
			tablet_replica_set_version: tablet.replica_set.tablet_replica_set_version,
			notices: vec![super::types::StatusNotice {
				volume_log_key,
				state: Some(state),
				cold_write: None,
				entries: Vec::new(),
			}],
		}
	}
}
