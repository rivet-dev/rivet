//! Correlation identities and bounds for one tablet maintainer lifetime.

use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpId(pub u64);

/// What the maintainer established about an incarnation's record before applying its
/// entries. Entries only ever write rows under the incarnation's own keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApplyAuthority {
	/// The incarnation is live: the store refuses the apply once its own record is Deleted.
	Current,
	/// A recovered delete applies the final tail its tombstones kept, so a fork of the
	/// deleted incarnation keeps the rows its base chain reads. The record may already say
	/// Deleted.
	RetiredTail,
}

/// Only a failed replica reread is safe to retry under the same store grant.
/// Partial rows remain hidden at their original commit sequence until replay finishes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
pub enum FilesystemApplyError {
	#[error("transaction reread from {worker:?} failed: {message}")]
	ReplicaRead { worker: WorkerId, message: String },
	#[error("{0}")]
	Storage(String),
}

/// At most this many pending-work keys are visited per discovery page.
pub const MAX_ORPHAN_SCAN_KEYS: u32 = 128;

/// Deadline for one operator repair cycle of a volume.
pub const REPAIR_TIMEOUT_MS: u64 = 60_000;

/// Brief coalescing window for ready cuts from different volumes.
pub const COLD_BATCH_DELAY_MS: u64 = 10;

/// Wait before an unacknowledged volume creation is sent again. It exceeds the
/// operation timeout, so a retry never overlaps the attempt before it.
pub const CREATION_RETRY_MS: u64 = 60_000;

/// Keys one open may ask to read back; a larger request is answered without them.
pub const MAX_OPEN_READ_KEYS: u32 = 64;

/// Ceiling of the doubling backoff, from `retry_ms`, between failed reloads of one member.
pub const MEMBER_REPAIR_MAX_BACKOFF_MS: u64 = 30_000;

/// Bytes of cold reads and cold writes the maintainer holds in memory at once. It fits two
/// of the largest cold reads with their keys.
pub const MAX_COLD_BYTES: u64 = 64 * 1024 * 1024;

/// Encoded bytes of maintainer log entries above cold, and ops waiting for an entry, at
/// which new ops are refused as overloaded until passes trim the log.
pub const MAX_MAINTAINER_LOG_BYTES: u64 = 16 * 1024 * 1024;

/// How long a requested close waits for the members' notices of the writer's last
/// appends before its unload decides what it may forget. Writers announce no commits, so
/// those notices are what prove the tail committed. It should exceed the log shard's
/// append notice delay; a member still unproven after it is probed once.
/// Five times the log shard's default append notice delay.
pub const CLOSE_NOTICE_WAIT_MS: u64 = 500;

/// Entries a volume's apply may lag what its members prove committed before an open of it
/// is refused with a retry hint. An opener replays the unapplied entries itself, one page of
/// at most 1024 entries per round trip within its open deadline, so this is one page.
/// Further behind, the open waits for the maintainer's apply, which keeps its progress.
pub const OPEN_APPLY_LAG_ENTRIES: u64 = 1024;

/// Estimated bytes of unapplied entries past which an open is refused, as
/// `OPEN_APPLY_LAG_ENTRIES` is for their count: about a second of an opener's replay on a
/// 1 Gbit/s link, a third of its default open deadline.
pub const OPEN_APPLY_LAG_BYTES: u64 = 64 * 1024 * 1024;

/// How long a volume's apply may make no progress while opens of it are refused. After
/// that, opens are admitted again and replay the unapplied entries themselves, so an apply
/// that cannot advance never makes its volume unopenable.
pub const OPEN_APPLY_STALL_MS: u64 = 5_000;

/// Opening performs multiple sequential object-store fencing and recovery steps.
/// It is not a serving request; admission remains closed until discovery completes.
pub const COLD_OPEN_TIMEOUT_MS: u64 = 60_000;

/// Deadline for a checkpoint or projected clone; expiry fences and waits for storage cleanup.
pub const SPLIT_STORAGE_TIMEOUT_MS: u64 = 60_000;

/// The versions one grant writes its tablet's data at. The supervisor selects them from the
/// topology's read floor when it starts the grant, so a rolling deploy never writes a version
/// that a live storage worker cannot read. They stay fixed for the grant; a floor that rises
/// later takes effect at the tablet's next grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabletWriteVersions {
	/// `depot.fs_rows`, the SlateDB row values. The marks rise to it, and the store encodes
	/// rows at it once a durable mark covers it.
	pub fs_rows: u16,
	/// `depot.maintainer_log`: log entries, the record row and replica creation evidence.
	pub maintainer_log: u16,
}

impl TabletWriteVersions {
	/// The versions `versions` selects for a grant starting now.
	pub fn select(versions: &dyn rivet_topology::versions::WriteVersions) -> anyhow::Result<Self> {
		use rivet_data::families::VersionFamily;
		Ok(Self {
			fs_rows: versions.write_version(VersionFamily::DEPOT_FS_ROWS)?,
			maintainer_log: versions.write_version(VersionFamily::DEPOT_MAINTAINER_LOG)?,
		})
	}

	/// The lowest versions this binary writes, which every binary of a one-step rolling
	/// deploy reads.
	pub fn lowest() -> Self {
		use rivet_data::families::VersionFamily;
		let lowest = |family: VersionFamily| family.info().map_or(1, |info| info.min_read);
		Self {
			fs_rows: lowest(VersionFamily::DEPOT_FS_ROWS),
			maintainer_log: lowest(VersionFamily::DEPOT_MAINTAINER_LOG),
		}
	}

	/// Checks that this binary encodes both versions.
	pub fn validate(&self) -> anyhow::Result<()> {
		use rivet_data::families::VersionFamily;
		for (family, version) in [
			(VersionFamily::DEPOT_FS_ROWS, self.fs_rows),
			(VersionFamily::DEPOT_MAINTAINER_LOG, self.maintainer_log),
		] {
			let info = family
				.info()
				.ok_or_else(|| anyhow::anyhow!("this binary does not know family {family}"))?;
			anyhow::ensure!(
				(info.min_read..=info.max_write).contains(&version),
				"this binary does not write family {family} at version {version}"
			);
		}
		Ok(())
	}
}

/// Maintainer bounds and intervals no deployment sets. Production always runs
/// [`MaintainerTuning::PRODUCTION`]; only test builds may override them, through
/// `MaintainerConfig::tuning`, to reach a bound or a deadline quickly.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaintainerTuning {
	/// Wait before a held pending sweep is tried again when no store operation that can
	/// release its hold has finished. Read sessions expire inside the store.
	pub held_sweep_retry_ms: u64,
	/// Shared window for delete claims, tombstones, and final forgets.
	pub max_delete_requests: u32,
	/// Maximum filesystem row tombstones in one cold deletion sweep.
	pub max_delete_rows: u32,
	/// Maximum concurrently owned recovery-only clients.
	pub max_repairs: u32,
	/// Two fresh listings must observe unchanged clients and tails across this interval.
	pub repair_quiet_ms: u64,
	/// A reopen parked behind a membership recovery that must match a learner cancels that
	/// recovery after this long, so a slow learner cannot hold an actor wake. The serving
	/// client matches the learner while it serves.
	pub repair_open_wait_ms: u64,
	/// Longest volume key a request may name; longer keys are refused.
	pub max_volume_key_bytes: u32,
	/// Replicas one member listing page returns.
	pub listing_page_size: u32,
	/// Deadline for one replica, coordinator or store operation.
	pub operation_timeout_ms: u64,
	/// Deadline for cold-store reads, writes, and metadata operations.
	pub cold_io_timeout_ms: u64,
	/// Volume rows one cold pass writes.
	pub max_rows_per_pass: u32,
	/// Volume records one filesystem flush publishes. A flush makes the whole memtable
	/// durable, and each record is one small row, so this bounds only the SST's record count.
	/// Every unload of a written volume waits for a flush carrying its record, so this caps
	/// the tablet's unload rate at about this many per flush upload.
	pub max_flush_records: u32,
	/// Unload forget requests in flight. Each request may carry up to
	/// `max_forget_batch_volumes` volumes for one member.
	pub max_forgets_in_flight: u32,
	/// Payload bytes of initial writes one creating open may send with its loads. Larger
	/// initial writes are not sent: the volume is created empty and its opener writes them
	/// after its claim entry. It must leave room for the creation's frames in one log shard write.
	pub max_initial_bytes: u64,
	/// Payload bytes of initial entries all unacknowledged creations of this tablet retain
	/// for their retries. An open that would exceed it is created without its initial writes.
	pub max_pending_initial_bytes: u64,
	/// Wait before a failed or refused step is tried again.
	pub retry_ms: u64,
	/// Reloads of members missing from loaded volumes in flight at once; more wait in order.
	/// Zero disables these reloads, leaving a missing member to the next open.
	pub max_member_repair_loads: u32,
	/// Bytes one cold read returns.
	pub max_cold_read_bytes: u32,
	/// Status requests in flight to members at once.
	pub max_status_requests: u32,
	/// Volumes named in one batched status request to a member.
	pub max_status_batch_volumes: u32,
	/// Replica trims in flight. A flush asks every member to trim each volume it published,
	/// and an unload forgets a member only after its trim, so this bounds how fast a burst
	/// of unloads drains. The replica transport coalesces queued trims to one member.
	pub max_trim_requests: u32,
	/// Base interval between status polls of a loaded volume. A push, hint or changed
	/// observation returns a volume to it.
	pub status_poll_ms: u64,
	/// Whether every loaded volume is polled at `status_poll_ms` for as long as it is loaded,
	/// for runtimes whose members push no status notices. With notices, the default, a
	/// volume is polled only while an observation of it is needed: once after it loads, on a
	/// targeted request, and while a commit waits on a failed read source. Lost notices are
	/// resent by the log shard and refused ones turn into targeted polls here, so an idle
	/// volume costs no status reads.
	pub poll_loaded_volumes: bool,
	/// Quiet time after which a volume whose claim names its holder parks under that holder
	/// process, and the interval at which each holder process is asked whether it is gone. A
	/// live holder keeps its parked volumes loaded; a gone holder's volumes unload. A value at
	/// or above `idle_unload_after_ms` disables the check, and held volumes then idle-unload.
	pub holder_check_after_ms: u64,
	/// How long a membership change waits for a held volume's live writer to match the new
	/// members itself. Past it, membership recovery claims the volume anyway, which fences
	/// that writer and makes its owner open it again, so one writer that cannot match never
	/// holds the tablet's change open indefinitely.
	pub membership_holder_wait_ms: u64,
}

impl MaintainerTuning {
	pub const PRODUCTION: Self = Self {
		held_sweep_retry_ms: 60_000,
		max_delete_requests: 16,
		max_delete_rows: 1024,
		max_repairs: 4,
		repair_quiet_ms: 1000,
		repair_open_wait_ms: 1000,
		max_volume_key_bytes: 4096,
		listing_page_size: 128,
		operation_timeout_ms: 5000,
		cold_io_timeout_ms: 60_000,
		max_rows_per_pass: 128,
		max_flush_records: 1024,
		max_forgets_in_flight: 32,
		max_initial_bytes: 256 * 1024,
		max_pending_initial_bytes: 16 * 1024 * 1024,
		retry_ms: 1000,
		max_member_repair_loads: 16,
		max_cold_read_bytes: 4 * 1024 * 1024,
		max_status_requests: 16,
		max_status_batch_volumes: 256,
		max_trim_requests: 512,
		status_poll_ms: 2000,
		poll_loaded_volumes: false,
		// A dead engine's volumes unload within about two of these intervals.
		holder_check_after_ms: 300_000,
		// Several replica set hand-off rounds, so an idle writer adopts the change and matches first.
		membership_holder_wait_ms: 900_000,
	};
}

#[cfg(any(test, feature = "test-support"))]
impl Default for MaintainerTuning {
	fn default() -> Self {
		Self::PRODUCTION
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaintainerConfig {
	/// Overrides of the fixed bounds, for tests only.
	#[cfg(any(test, feature = "test-support"))]
	pub tuning: MaintainerTuning,
	pub alarms: crate::alarms::Limits,
	/// The versions this grant writes its tablet's data at. The supervisor sets them from
	/// the topology for every grant; the default is the lowest versions this binary writes.
	pub write_versions: TabletWriteVersions,
	pub idle_unload_after_ms: u64,
	/// Existing topology datacenter label, supplied by process bootstrap.
	pub local_region: u16,
	/// The worker this maintainer runs on, set by the tablet supervisor from its grant. A
	/// granted maintainer always holds a replica of each of its volumes there, so reads that
	/// may come from any in-sync replica prefer this one and stay in process.
	pub local_worker: Option<WorkerId>,
	/// Minimum wall-clock age before checking an unfinished fork's destination.
	pub fork_check_grace_ms: u64,
	/// Bounded background image copies per maintainer.
	pub max_concurrent_materializations: u32,
	/// Maximum sparse candidate chunks retained by one copy page.
	pub materialize_batch_chunks: u32,
	pub checkpoints: depot_fs_core::checkpoints::CheckpointConfig,
	pub max_requests: u32,
	/// Queued events the maintainer's inbox holds before senders are rejected as overloaded.
	pub inbox_capacity: u32,
	/// Volume opens queued in the inbox, not yet taken by the runner, at which a new open is
	/// refused at submit. Other inbox events do not count.
	pub max_queued_opens: u32,
	/// Union of member listings, including orphan incarnations. Size for all three shards.
	/// A loaded volume costs about 1.8 KiB of maintainer heap (the `maintainer_memory` test),
	/// so the default of 500,000 holds under 1 GiB per tablet.
	pub max_loaded_volumes: u32,
	/// Maximum batching age from the first unflushed log entry, plus up to 10% jitter.
	/// Production sets it from `DepotConfig::write_to_cold_after_ms`.
	pub write_to_cold_after_ms: u64,
	/// Committed encoded suffix bytes that bypass the age timer (at most 256 MiB).
	pub write_to_cold_above_bytes: u64,
	/// Throttle for memtable flushes that nobody urgently waits on, such as tiering passes
	/// of volumes that reached their batching age. Every flush makes one L0 SST, so passes
	/// that come due close together wait up to this long and share one, which keeps L0
	/// within reach of compaction. The data is already on the log shards' disks, so waiting
	/// costs only replay length.
	pub memtable_flush_throttle_ms: u64,
	/// Unflushed applied bytes at which a volume that did not request a flush joins one
	/// that another volume requested. A flush makes the whole tablet memtable durable, so
	/// publishing such a volume's cold position costs only its record row. The largest
	/// volumes join first, up to `max_flush_records` records. `u64::MAX` disables it.
	pub flush_piggyback_min_bytes: u64,
	/// Bytes applied on the tablet since its last pass at which a pass starts at once,
	/// carrying the volumes with the most unflushed bytes. SlateDB uploads a memtable by
	/// itself when it reaches its L0 SST size, but only a pass records the cold positions
	/// that let the logs trim, so the pass must come first. The supervisor sets it to three
	/// quarters of the tablet's L0 SST size.
	pub tablet_flush_above_bytes: u64,
	/// Volumes in one batched forget or trim. It must not exceed the log shards'
	/// `max_forget_batch_volumes`, which rejects larger batches; the storage service sets it
	/// from that same setting.
	pub max_forget_batch_volumes: u32,
	/// Client cold reads in flight at once: session opens, lifecycle row lookups and
	/// session reads together share this window.
	pub max_cold_reads: u32,
	/// Session reads of one volume admitted while reads of other volumes wait, so one large
	/// scan cannot hold every client read slot of its tablet. Slots no other volume waits
	/// for still go to the capped volume. Zero admits session reads in arrival order only.
	pub max_cold_reads_per_volume: u32,
	/// Size of each background cold-read window: held-image source reads, replica log
	/// reads, fork jobs and fork cleanup jobs are each bounded by it separately, so raising
	/// the client window never widens them.
	pub max_background_cold_reads: u32,
	/// Simulation only: runner time each admitted filesystem read request costs, spent on the
	/// simulated clock so a maintainer busy with reads queues them as a loaded one does. Zero
	/// in deployments, where real processing time passes on its own.
	pub simulated_fs_read_cost_us: u64,
	/// Filesystem applies and loads admitted to the tablet store at once. It bounds how
	/// much keyed work runs ahead of a tablet-wide operation or a newly queued read.
	pub max_keyed_in_flight: u32,
	/// Flushes that may stage at once.
	pub max_flush_staging: u32,
	/// Flushes in flight at once, staging or uploading. A staging
	/// flush counts because it uploads next.
	pub max_flush_uploads: u32,
	/// Queued store items one admission pass looks at past work that cannot start yet.
	pub admission_scan_limit: u32,
}

impl Default for MaintainerConfig {
	fn default() -> Self {
		Self {
			#[cfg(any(test, feature = "test-support"))]
			tuning: MaintainerTuning::PRODUCTION,
			alarms: Default::default(),
			write_versions: TabletWriteVersions::lowest(),
			idle_unload_after_ms: 600_000,
			local_worker: None,
			local_region: 0,
			fork_check_grace_ms: 60_000,
			max_concurrent_materializations: 2,
			materialize_batch_chunks: 1024,
			checkpoints: Default::default(),
			max_requests: 1024,
			inbox_capacity: 4096,
			max_queued_opens: 1024,
			max_loaded_volumes: 500_000,
			write_to_cold_after_ms: 60_000,
			write_to_cold_above_bytes: 64 * 1024 * 1024,
			memtable_flush_throttle_ms: 10_000,
			flush_piggyback_min_bytes: 16 * 1024,
			tablet_flush_above_bytes: 48 * 1024 * 1024,
			max_forget_batch_volumes: depot_types::log_shard::DEFAULT_MAX_GRANT_BATCH_VOLUMES,
			max_cold_reads_per_volume: 0,
			max_cold_reads: 64,
			max_background_cold_reads: 4,
			simulated_fs_read_cost_us: 0,
			max_keyed_in_flight: 256,
			max_flush_staging: 1,
			max_flush_uploads: 2,
			admission_scan_limit: 1024,
		}
	}
}

impl MaintainerConfig {
	/// The maintainer's fixed bounds and intervals.
	pub fn tuning(&self) -> &MaintainerTuning {
		#[cfg(any(test, feature = "test-support"))]
		{
			&self.tuning
		}
		#[cfg(not(any(test, feature = "test-support")))]
		{
			&MaintainerTuning::PRODUCTION
		}
	}
}

/// One replica's synced state from its log shard. `state` is absent when the replica
/// is no longer loaded there. `entries` are synced entries the replica learned are
/// committed since its previous notice, as a read from the first of them would return.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StatusNotice {
	pub volume_log_key: depot_types::VolumeLogKey,
	pub state: Option<rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState>,
	pub cold_write:
		Option<rivet_data::generated::depot_storage_worker_protocol_v1::ColdWriteStatus>,
	pub entries: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntry>,
}

/// A caller's identity is retained independently of executor operation attempts.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpenMode {
	/// Requires an existing volume and returns its open evidence.
	Existing,
	/// Creates if absent and returns the create response with reusable open evidence.
	Create,
	/// Creates if absent through VolumeOpenRequest and returns an open response.
	CreateIfAbsent,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpenVolume {
	pub retention: Option<depot_fs_core::FsRetentionPolicy>,
	/// Original UPS deadline, retained across row lookup and replica loading.
	pub deadline_boot_ms: u64,
	/// Logical client identity, reused across create/open retries. Internal recovery may omit it.
	pub client_incarnation: Option<[u8; 16]>,
	pub request_id: RequestId,
	pub worker_id: WorkerId,
	/// The opening process, kept with the claim this open grants. Admission keeps it only when
	/// it names `worker_id`, the transport's source; otherwise the claim's holder is unknown.
	pub holder: Option<rivet_data::generated::shared_depot_types_v1::VolumeHolder>,
	pub volume_key: depot_types::VolumeKey,
	pub mode: OpenMode,
	/// The opener's first writes, written with the replicas when this open creates the volume.
	/// See `VolumeCreateOptions.initial`.
	pub initial: Vec<rivet_data::generated::depot_log_shard_v1::PayloadWrite>,
	/// Keys to read back when the open finds the volume existing with no log tail. See
	/// `VolumeCreateOptions.read`.
	pub read: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MaintainerTimer {
	/// **When**
	///
	/// The earliest cached alarm or retry deadline becomes due while serving.
	///
	/// **State**
	///
	/// Drains due candidates and frees bounded frontier capacity.
	///
	/// **Effects**
	///
	/// Refills bounded ordered pages when capacity frees and arms the next known deadline.
	Alarms,
	/// **When**
	///
	/// The earliest idle check of a loaded volume is due. It is armed at the head of
	/// the idle check queue after every event while serving and cancelled when the
	/// queue is empty or the maintainer stops serving.
	///
	/// **State**
	///
	/// Takes a bounded number of due volumes and rechecks each against its current
	/// `last_activity`. A volume that became active is rescheduled at its new deadline.
	/// An idle volume that orphan cleanup can prove is an orphaned minority is left to
	/// that cleanup. Any other idle live volume without a pending open begins an idle
	/// unload. Neither is queued again: a cancelled unload and a finished open schedule
	/// the next check from the volume's activity. During an operator repair every due
	/// volume is parked until the repair ends.
	///
	/// **Effects**
	///
	/// Starts the existing recovery, flush and forget path, and `ForgetVolumeRequest`s
	/// for due orphans. The timer is rearmed at the new queue head, at once when due
	/// volumes remain.
	IdleUnload,
	/// **When**
	///
	/// An open waiting for its read-back reaches its reply budget, armed when the read began.
	///
	/// **State**
	///
	/// Drops the pending read-back. A read still in flight completes into nothing.
	///
	/// **Effects**
	///
	/// Replies to the open without the read-back values.
	OpenRead(RequestId),
	/// **When**
	///
	/// A fork caller reaches its original deadline.
	///
	/// **State**
	///
	/// Drops only the waiting reply while durable coordination remains owned.
	///
	/// **Effects**
	///
	/// Replies unavailable without cancelling the job.
	ForkRequest(RequestId),

	/// **When**
	///
	/// An uncertain source attempt reaches its retry delay.
	///
	/// **State**
	///
	/// Restores the exact prepared image/cursor as ready.
	///
	/// **Effects**
	///
	/// The normal drive pass dispatches one new attempt.
	ForkRetry(depot_types::VolumeKey),

	/// **When**
	///
	/// A materialization's source read failed or was refused, and its backoff ended.
	///
	/// **State**
	///
	/// Hands the scope's volume back to discovery: scanned now when a job is free, else kept
	/// with the work skipped for want of a job.
	///
	/// **Effects**
	///
	/// At most one scan of that volume's pending rows, which resumes the durable cursor.
	MaterializeRetry(depot_fs_core::keys::slatedb::Scope),

	/// **When**
	///
	/// Persisted sweep discovery is due after startup, after the previous bounded page,
	/// or after a finished store operation that can write pending rows marked the volume
	/// keys it touched. It is never armed once discovery is `Done`.
	///
	/// **State**
	///
	/// Reserves the waiting scan's next page, of every pending row or of one marked volume
	/// key's rows, when its earlier sweeps have drained, otherwise waits `retry_ms` for them.
	///
	/// **Effects**
	///
	/// Queues the scan on the existing store owner.
	PendingWorkTick,
	/// **When**
	///
	/// `held_sweep_retry_ms` after a sweep found its incarnation held by an unexpired read
	/// session. Read sessions expire inside the store, so their release produces no event.
	/// A sweep held by a checkpoint or fork hold arms no deadline; the release of that hold
	/// queues it again.
	///
	/// **State**
	///
	/// Moves every sweep waiting in memory back onto the store queue.
	///
	/// **Effects**
	///
	/// The store queue issues `SweepPendingRequest` for each.
	HeldSweepRetry,
	/// **When**
	///
	/// The earliest queued fork check or release in `fork_cleanup.due` is due. It is
	/// armed at the queue head after every event while serving and cancelled when the
	/// queue is empty or the maintainer stops serving.
	///
	/// **State**
	///
	/// Takes a bounded number of due checks and releases and queues an inspection of
	/// each whose cleanup job is not already running. One that finds the cleanup jobs
	/// full is queued again `retry_ms` later.
	///
	/// **Effects**
	///
	/// The store queue issues `ForkCleanupRequest` with `Action::Inspect` for each; the
	/// timer is rearmed at the new queue head, at once when due entries remain.
	ForkCleanupDue,

	/// **When**
	///
	/// The earliest retention check of a registered volume with a retention policy is
	/// due, even when no new client write arrives. It is armed at the head of the
	/// retention queue after every event while serving.
	///
	/// **State**
	///
	/// Takes a bounded number of due volumes and rederives each deadline from
	/// `retention_checked_boot_ms` and the current policy. An elapsed policy marks the
	/// volume for a coalesced flush at completed applied progress; the flush that
	/// records the new check time schedules it again. A volume that is not due yet is
	/// rescheduled at its deadline, a deleting or quiescing one retries after
	/// `retry_ms`, and one without an active policy is dropped until a record with a
	/// policy schedules it again.
	///
	/// **Effects**
	///
	/// Queues the existing filesystem flush. The timer is rearmed at the new queue head.
	RetentionDue,

	/// **When**
	///
	/// The caller's original checkpoint administration deadline expires.
	///
	/// **State**
	///
	/// Removes the waiting caller; an already sent mutation remains owned.
	///
	/// **Effects**
	///
	/// Replies with an unavailable outcome without cancelling durable work.
	CheckpointRequest(RequestId),
	/// **When**
	///
	/// A reference caller reaches its original deadline.
	///
	/// **State**
	///
	/// Removes its waiting reply while retaining dispatched mutations.
	///
	/// **Effects**
	///
	/// Replies unavailable; the operation still consumes its eventual durable result.
	ForkReferenceRequest(RequestId),

	/// **When**
	///
	/// The first unflushed filesystem entry reaches its age deadline.
	///
	/// **State**
	///
	/// Marks the current applied position due without extending the deadline.
	///
	/// **Effects**
	///
	/// Coalesces a record flush through the store queue.
	///
	FilesystemDue(depot_types::VolumeLogKey),
	/// **When**
	///
	/// A failed committed read reaches its bounded retry delay.
	///
	/// **State**
	///
	/// Releases that volume for another qualified read.
	///
	/// **Effects**
	///
	/// The shared dispatch selects a current InSync source.
	///
	FilesystemRetry(depot_types::VolumeLogKey),
	/// **When**
	///
	/// The filesystem flush coalescing window ends: `COLD_BATCH_DELAY_MS` after a prompt
	/// flush request, or `memtable_flush_throttle_ms` after a throttled one when nothing
	/// prompt moved it earlier.
	///
	/// **State**
	///
	/// Clears the scheduled deadline, so the next request arms a new window, and retains
	/// the due positions while waiting for the store.
	///
	/// **Effects**
	///
	/// Queues one shared record flush.
	///
	FilesystemFlushBatch,

	/// **When**
	///
	/// The open's answer deadline passes: half of the caller's remaining time when the open
	/// was admitted, so the refusal reaches the caller before its own deadline.
	///
	/// **State**
	///
	/// Retires only the matching caller; late loads remain observations for cleanup.
	///
	/// **Effects**
	///
	/// Releases the caller and schedules orphan cleanup.
	OpenRequest(RequestId),

	/// **When**
	///
	/// A reopen parked behind a membership recovery with a required learner has waited
	/// `repair_open_wait_ms` without that recovery joining.
	///
	/// **State**
	///
	/// Marks the key's recovery jobs cancelled when the same caller still waits; the open
	/// resumes once they join, and the serving client matches the learner.
	///
	/// **Effects**
	///
	/// Cancels each running recovery through its executor.
	RepairWait(RequestId),
	/// **When**
	///
	/// The runtime fires after Learning begins serving, or after the previous copy round.
	///
	/// **State**
	///
	/// An idle Learning maintainer starts preparation; other phases or active rounds ignore it.
	///
	/// **Effects**
	///
	/// Starts a bounded status round and queues a cold pass before learner loads and suffix reads.
	CatchUp,
	/// Releases only caller bookkeeping; deletion continues after its uncertain reply.
	DeleteRequest(depot_types::VolumeLogKey),
	/// Makes failed member work eligible for another bounded attempt.
	DeleteRetry(depot_types::VolumeLogKey),
	/// Resumes the same checkpoint, child clone or coordinator report after backoff.
	SplitRetry,
	/// **When**
	///
	/// A pinned parent, a failed forget of its log, or a failed cleanup/report reaches its
	/// retry interval.
	///
	/// **State**
	///
	/// Makes only the assigned waiting cleanup eligible for another attempt; a log forget
	/// drops members that left the replica set.
	///
	/// **Effects**
	///
	/// Dispatch queues deletion, forgets the next member's log replica, or repeats the
	/// coordinator report under the current grant.
	RetiredRetry([u8; 16]),
	/// **When**
	///
	/// An unaccepted parent log import report reaches its retry interval.
	///
	/// **State**
	///
	/// Makes the report due again.
	///
	/// **Effects**
	///
	/// The next drive reports the import again under the current grant.
	ParentLogReportRetry,
	/// **When**
	///
	/// An unaccepted version marks report reaches its retry interval.
	///
	/// **State**
	///
	/// Clears the retry wait.
	///
	/// **Effects**
	///
	/// Dispatch reports the current marks again if the manifest still lacks them.
	VersionMarksRetry,
	/// Releases the bounded retry wait for a closed volume recovery client.
	UnloadRecoveryRetry(depot_types::VolumeLogKey),
	/// Retries unavailable conditional forgets after their backoff.
	UnloadRetry(depot_types::VolumeLogKey),
	/// **When**
	///
	/// A requested close began its unload, armed for `CLOSE_NOTICE_WAIT_MS`: the window in
	/// which the members' shards notice the writer's last appends.
	///
	/// **State**
	///
	/// Ends the unload's wait for notices. A stale timer for an unload that is gone or no
	/// longer waiting changes nothing.
	///
	/// **Effects**
	///
	/// A targeted status probe of the volume when a member still shows a tail above what
	/// the members prove committed; otherwise none. The unload is marked for its next drive.
	CloseNotices(depot_types::VolumeLogKey),
	/// **When**
	///
	/// The earliest retry of a holder watch topology refused is due. Armed at the head of the
	/// retry queue after every event while serving; nothing arms it while every watch holds.
	///
	/// **State**
	///
	/// Takes a bounded number of due holders and marks them for `finish_event`.
	///
	/// **Effects**
	///
	/// `WatchHolderRequest` for each due holder that still has a parked volume here.
	HolderWatchRetry,
	ReadRequest(RequestId),
	Operation(OpId),
	ListRetry(WorkerId),
	/// **When**
	///
	/// A failed promotion listing reaches its retry interval.
	///
	/// **State**
	///
	/// Releases that worker's retry wait under the current Joint assignment.
	///
	/// **Effects**
	///
	/// Dispatch retries a bounded version-fenced listing if it is still needed.
	PromotionRetry(WorkerId),
	/// **When**
	///
	/// A forget, an abandoned create or a finished or expired open asked for orphan
	/// cleanup to run again after `retry_ms`. It is never armed periodically.
	///
	/// **State**
	///
	/// Collects idle loaded volumes that lost their replica quorum, within the forget
	/// window, when every member of a stable replica set has been listed.
	///
	/// **Effects**
	///
	/// `ForgetVolumeRequest`s for orphaned replicas.
	OrphanSweep,
	/// **When**
	///
	/// The age fallback for creation rows expires: `write_to_cold_after_ms` after the
	/// first row entered an empty `pending_rows` queue, or after serving began with rows
	/// waiting. It is cancelled when a pass takes the last waiting row, so nothing is
	/// armed while no row waits.
	///
	/// **State**
	///
	/// Queues one cold-store pass on the store queue.
	///
	/// **Effects**
	///
	/// None directly; the store queue dispatches `WriteToColdRequest`.
	WriteToCold,
	/// **When** The first ready cut or deletion row's coalescing window expires.
	/// **State** Releases the scheduled batch and queues one store pass.
	/// **Effects** Writes ready rows and cuts subject to the existing byte and volume caps.
	ColdBatch,
	CreationExpired(depot_types::VolumeLogKey),
	/// **When**
	///
	/// The earliest scheduled status check of a loaded volume is due. It is armed at
	/// the head of the status schedule after every event and cancelled when the
	/// schedule is empty or the maintainer stops serving.
	///
	/// **State**
	///
	/// Clears the armed marker. The poll drive that follows takes due volumes, in
	/// check-time order, and reschedules each with its backed-off interval.
	///
	/// **Effects**
	///
	/// Batched `ReadVolumeLogStatusRequest`s to every member for the due volumes, and
	/// the timer rearmed at the new head.
	Poll,
	TrimRetry(depot_types::VolumeLogKey, WorkerId),
	/// **When**
	///
	/// A failed reload of a member missing from a loaded volume waited its backoff. Armed
	/// when the load fails or times out while the member is still eligible.
	///
	/// **State**
	///
	/// Requeues the member's repair as ready. A repair that is gone or no longer waiting,
	/// after a fence or an earlier drop, is left alone.
	///
	/// **Effects**
	///
	/// A `LoadVolumeRequest` to the member when a load slot is free and the volume is still
	/// eligible; otherwise none.
	MemberRepairRetry(depot_types::VolumeLogKey, WorkerId),
	/// **When**
	///
	/// The oldest maintainer log entry whose ops are all applied reached the tiering age.
	/// Armed at that age while serving, moved when the oldest such entry changes, and
	/// cancelled when none is left above cold.
	///
	/// **State**
	///
	/// Marks the log as owed a pass.
	///
	/// **Effects**
	///
	/// Schedules the throttled pass, which covers the log's applied entries and writes the
	/// cold position it may trim to.
	MaintainerLogDue,
	/// **When**
	///
	/// The tablet store's memtable budget has stayed full for
	/// `MEMTABLE_FULL_SHED_AFTER_MS` since it filled.
	///
	/// **State**
	///
	/// Starts shedding: reads that would wait for an apply or queued store work, and opens
	/// whose row lookup would queue behind store work, are refused.
	///
	/// **Effects**
	///
	/// Refuses as overloaded every read already parked for an apply or queued behind store
	/// work, and every open whose row lookup is queued behind store work.
	MemtableBudgetShed,
	/// **When**
	///
	/// A maintainer log request to this member failed or was refused, and its retry delay
	/// passed.
	///
	/// **State**
	///
	/// Lets the member take its next request again.
	///
	/// **Effects**
	///
	/// The next drive sends the member the request its replica needs.
	MaintainerLogRetry(WorkerId),
}

/// Immutable recovery work the maintainer selected for one volume.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecoveryRequest {
	/// Optional matching allowance; required membership work uses the existing owner budget.
	pub finish_recovery_timeout_ms: Option<u64>,
	/// Captured required destinations. The admitted replica set and assignment cancellation
	/// retain their version/disk binding; matching still checks each replica incarnation.
	pub required_members: std::collections::BTreeSet<WorkerId>,
	pub volume_log_key: depot_types::VolumeLogKey,
	pub row: rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord,
	pub replica_set: rivet_data::generated::shared_depot_types_v1::TabletReplicaSet,
	pub timeout_ms: u64,
	/// The claim epoch this maintainer issued for the recovery client, which never chooses
	/// its own.
	pub claim_epoch: u64,
}

/// What applying a replayed maintainer log op left to do.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplayOutcome {
	/// The op's change is in the store.
	Applied,
	/// A retention checkpoint pass ran for `key`. `checkpoint_seq` is the volume seq of the
	/// newest retention checkpoint the store holds after it, or `None` when none exists; a
	/// check while the volume is still applied at that seq takes nothing.
	RetentionChecked {
		key: depot_types::VolumeLogKey,
		checkpoint_seq: Option<u64>,
	},
	/// A fork publication whose copied metadata may not have reached cold before the move.
	/// The store holds the fork's preparation again; the op is done once the fork has copied
	/// its metadata again and published.
	ResumeFork(depot_fs_core::FsForkSpec),
}

/// The outcome of one deletion sweep step.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeleteSweepResult {
	/// A read session or a fork or checkpoint hold keeps the data; the step runs again later.
	Held,
	/// The record is released and the data belongs to a pending sweep.
	Complete,
}

/// What an operator inspection read from the store for one name: its pointer row, the fork or
/// restore prepared for it, and the own record row of each incarnation it asked about, in the
/// order asked. Read without ordering against store writes, so it may predate a change in
/// flight; every repair call checks its own fence again.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepairRecords {
	pub pointer: Option<rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord>,
	pub pending_fork: Option<rivet_data::generated::shared_depot_fs_v1::FsPendingFork>,
	pub incarnations: Vec<(
		[u8; 16],
		Option<rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord>,
	)>,
}

/// A repair claim's client to open: a serving volume client over the maintainer's replica
/// transport, claiming `volume_log_key` at the maintainer-issued `claim_epoch` from `row`'s
/// cold position.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepairClaimOpen {
	pub volume_log_key: depot_types::VolumeLogKey,
	pub row: rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord,
	pub replica_set: rivet_data::generated::shared_depot_types_v1::TabletReplicaSet,
	pub claim_epoch: u64,
	pub timeout_ms: u64,
}

/// What an opened repair claim holds: its identity, its committed claim entry, and the members
/// that acknowledged the entry or did not.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepairClaimOpened {
	pub client: rivet_data::generated::shared_depot_types_v1::ClientIdentity,
	pub claim_entry: rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
	pub in_sync: Vec<WorkerId>,
	pub incomplete: Vec<WorkerId>,
}

/// Why a repair claim's append failed: nothing was written, or the entry may have been.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepairAppendFailure {
	NotExecuted(String),
	Unknown(String),
}
