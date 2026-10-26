//! Grant-scoped member observations and bounded lifecycle operations.

pub(crate) mod alarms;
pub(crate) mod assignment;
pub(crate) mod catch_up;
pub(crate) mod claim_epochs;
pub mod cold;
pub(crate) mod cold_pass;
pub(crate) mod deletion;
pub(crate) mod drain;
pub mod filesystem;
pub(crate) mod filesystem_reads;
pub mod footprint;
pub(crate) mod holders;
pub(crate) mod lifecycle;
pub(crate) mod logged_ops;
pub mod maintainer_log;
pub(crate) mod member_repair;
pub mod members;
pub(crate) mod membership;
pub(crate) mod open_reads;
pub(crate) mod opening;
pub(crate) mod operations;
pub(crate) mod operator;
pub(crate) mod operator_repair;
pub mod parent_log;
pub mod poll;
pub(crate) mod promotion;
pub mod record;
pub(crate) mod repair;
pub(crate) mod replica_creation;
pub(crate) mod retired;
pub(crate) mod signals;
pub mod split;
pub(crate) mod stop;
pub(crate) mod store;
pub(crate) mod unload;
pub(crate) mod version_marks;
pub(crate) mod volume_keys;

pub(crate) mod checkpoints;
pub(crate) mod fork_admission;
pub(crate) mod fork_cleanup;
pub(crate) mod fork_references;
pub(crate) mod forks;
pub(crate) mod materialize;
pub(crate) mod pending_work;

use super::{Effects, effect::MaintainerEffect, types::*};
use crate::tablet_maintainer_machine::types::{
	COLD_OPEN_TIMEOUT_MS, CREATION_RETRY_MS, MAX_COLD_BYTES, SPLIT_STORAGE_TIMEOUT_MS,
};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	depot_tablet_manifest_v1::Tablet,
	shared_depot_fs_v1::{FsVolumeRecord, FsVolumeStatus},
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition, VolumeLogReplicaStatus},
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaintainerPhase {
	Opening,
	Serving,
	Frozen,
	Draining,
	Stopped,
	Fenced,
}

/// A condition a fresh grant on this worker cannot repair without changing capacity or data.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryFailure {
	/// Listing found more replica reservations than the maintainer can hold.
	CapacityExceeded {
		max_loaded_volumes: u32,
		required_capacity_at_least: u32,
	},
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoadedVolume {
	pub members: members::Members,
	/// Last client activity or committed advance on this maintainer clock.
	pub last_activity: u64,
	/// The initial row drawn by this lifetime allows a failed create to finish the
	/// same incarnation, even before any member has replied.
	pub creation: Option<Box<Creation>>,
	/// An unloaded row starting a new load attempt, from a cold lookup or a
	/// confirmed unload quorum. Until an open reply or recovery admits a client,
	/// retries may create empty voting replicas
	/// at this cold position. Once acknowledged, missing replicas require repair.
	pub unacknowledged_cold_open: Option<Box<FsVolumeRecord>>,
	/// Last row read from or flushed to the cold store, including a partial unload.
	pub stored_row: Option<record::SharedRecord>,
	/// When this volume's members are next polled for status, and how long the poll
	/// after that waits.
	pub status: poll::StatusSchedule,
}

/// Current-lifetime creation evidence survives caller retries until the row reaches cold storage.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Creation {
	pub row: FsVolumeRecord,
	/// Once the core acknowledges a quorum, absence is no longer proof of an abandoned create.
	pub acknowledged: bool,
	/// The newest open claim this creation granted, with the initial entries sent under it.
	/// Grants only rise: an open that does not repeat this grant exactly is claimed above it.
	pub granted: Option<GrantedOpen>,
}

/// One open claim a creation granted, and the exact initial entries sent with it.
///
/// Replicas created under the claim may hold those entries, so its epoch belongs to that one
/// client and those bytes. Only a retry by the same client with the same initial writes gets
/// the same claim and the same entries, `written_at` included. Every other open is claimed
/// above it and sent no entries, so two different entries never share a position and epoch.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GrantedOpen {
	pub client: ClientIdentity,
	/// Empty when the open carried no initial writes, and after the creation is acknowledged
	/// or expires, when no retry can send them again.
	pub entries: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntry>,
	/// How many entries were sent under this claim. It outlives `entries`, so a grant whose
	/// entries were released is never repeated for an open that sends none.
	pub entries_sent: u64,
}

impl GrantedOpen {
	/// Payload bytes of the retained entries, counted against `max_pending_initial_bytes`.
	pub(crate) fn entry_bytes(&self) -> u64 {
		initial_entry_bytes(&self.entries)
	}
}

/// [`MaintainerState::release_initial_entries`] for a volume already borrowed from the state.
pub(crate) fn release_volume_initial_entries(volume: &mut LoadedVolume, held: &mut u64) {
	let granted = volume
		.creation
		.as_mut()
		.and_then(|creation| creation.granted.as_mut());
	if let Some(granted) = granted {
		let bytes = granted.entry_bytes();
		granted.entries.clear();
		*held = held.saturating_sub(bytes);
	}
}

/// Payload bytes of initial entries.
pub(crate) fn initial_entry_bytes(
	entries: &[rivet_data::generated::depot_log_shard_v1::VolumeLogEntry],
) -> u64 {
	entries
		.iter()
		.map(|entry| match &entry.body {
			rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::PayloadWrite(write) => {
				write.payload.len() as u64
			}
			rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::ClaimEntry => 0,
		})
		.sum()
}

impl LoadedVolume {
	pub(crate) fn idle(&self, now: u64, interval: u64) -> bool {
		now.saturating_sub(self.last_activity) >= interval
	}
	/// Copies the highest client claim reported by an in-sync member for a trim.
	/// The receiving replica retains a newer claim or an equal-epoch incarnation.
	pub(crate) fn trim_claim(&self) -> Option<ClientIdentity> {
		self.members
			.values()
			.filter(|replica| replica.status == VolumeLogReplicaStatus::InSync)
			.filter_map(|replica| replica.claimed_client.as_ref())
			.max_by_key(|client| client.client_epoch)
			.cloned()
	}

	/// Deleted evidence dominates; cold comes from in-sync replicas or a confirmed stored row.
	pub fn row(&self, key: &VolumeLogKey) -> Option<FsVolumeRecord> {
		// A member a delete loaded after applying its tail carries that tail as its
		// tombstone's cold position; the members the delete marked keep the real one. The
		// lowest is the position the store is known to hold, and anything above it is a
		// tail a recovered delete still applies.
		let deleted = self
			.members
			.values()
			.filter(|r| r.status == VolumeLogReplicaStatus::Deleted)
			.min_by_key(|r| r.cold.volume_seq);
		let best = deleted.or_else(|| {
			self.members
				.values()
				.filter(|r| r.status == VolumeLogReplicaStatus::InSync)
				.max_by_key(|r| r.cold.volume_seq)
		});
		let metadata = self
			.stored_row
			.as_deref()
			.or_else(|| self.creation.as_ref().map(|v| &v.row))
			.or(self.unacknowledged_cold_open.as_deref());
		// Without a row, the replicas' creation evidence still names the base chain and
		// retention the volume was created with.
		let evidence = metadata
			.is_none()
			.then(|| self.replica_creation())
			.flatten();
		let observed = best.map(|best| FsVolumeRecord {
			recent_writes: metadata.map_or_else(Vec::new, |row| row.recent_writes.clone()),
			base_chain: match (metadata, &evidence) {
				(Some(row), _) => row.base_chain.clone(),
				(None, Some(evidence)) => evidence.base_chain.clone(),
				(None, None) => Vec::new(),
			},
			retention: match (metadata, &evidence) {
				(Some(row), _) => row.retention.clone(),
				(None, Some(evidence)) => evidence.retention.clone(),
				(None, None) => None,
			},
			// An incarnation with neither metadata nor evidence counts as the oldest of its
			// name; recovery reads its own stored record before acting on that.
			incarnation_epoch: match (metadata, &evidence) {
				(Some(row), _) => row.incarnation_epoch,
				(None, Some(evidence)) => evidence.incarnation_epoch,
				(None, None) => 0,
			},
			partition_key: key.0.volume_key.partition_key.clone(),
			name: key.0.volume_key.name.clone(),
			volume_incarnation: key.0.volume_incarnation,
			status: if deleted.is_some() {
				FsVolumeStatus::Deleted
			} else {
				FsVolumeStatus::Live
			},
			cold: best.cold.clone(),
		});
		if self
			.stored_row
			.as_ref()
			.is_some_and(|row| row.status == FsVolumeStatus::Deleted)
		{
			return self.stored_row.as_deref().cloned();
		}
		if deleted.is_some() {
			return observed;
		}
		match (observed, self.stored_row.as_deref()) {
			(Some(observed), Some(stored)) if stored.cold.volume_seq > observed.cold.volume_seq => {
				Some(stored.clone())
			}
			(Some(observed), _) => Some(observed),
			(None, Some(stored)) => Some(stored.clone()),
			(None, None) => self
				.creation
				.as_ref()
				.map(|creation| creation.row.clone())
				.or_else(|| self.unacknowledged_cold_open.as_deref().cloned()),
		}
	}

	/// Whether `row` would report `Deleted`, without building the row.
	pub fn is_deleted(&self) -> bool {
		let deleted = |row: &FsVolumeRecord| row.status == FsVolumeStatus::Deleted;
		if self.stored_row.as_deref().is_some_and(deleted)
			|| self
				.members
				.values()
				.any(|replica| replica.status == VolumeLogReplicaStatus::Deleted)
		{
			return true;
		}

		// Otherwise a live observation or a stored row decides, and neither is deleted.
		let observed = self
			.members
			.values()
			.any(|replica| replica.status == VolumeLogReplicaStatus::InSync);
		if observed || self.stored_row.is_some() {
			return false;
		}
		match &self.creation {
			Some(creation) => deleted(&creation.row),
			None => self
				.unacknowledged_cold_open
				.as_deref()
				.is_some_and(deleted),
		}
	}
}



#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Operation {
	/// One bounded alarm IO or callback attempt, including an immutable delayed retry.
	Alarm(alarms::Attempt),
	MaterializeStore {
		scope: depot_fs_core::keys::slatedb::Scope,
	},
	MaterializePeer {
		scope: depot_fs_core::keys::slatedb::Scope,
	},
	ForkCleanupStore {
		work: depot_fs_core::forks::cleanup::Work,
	},
	ForkCleanupPeer {
		work: depot_fs_core::forks::cleanup::Work,
	},
	ForkStore {
		volume_key: depot_types::VolumeKey,
		/// The fork's parent when it lives in this tablet, fixed at dispatch so the
		/// operation's store footprint can be computed from the operation alone.
		parent_volume_key: Option<depot_types::VolumeKey>,
	},
	ForkPeer {
		volume_key: depot_types::VolumeKey,
	},
	ForkReference {
		request_id: RequestId,
		/// The spec's parent and destination volumes that live in this tablet.
		volumes: Vec<depot_types::VolumeKey>,
		/// Whether this is a release, fixed at dispatch: its completion scans the pending
		/// rows of `volumes`, even after the request's own entry expired.
		releases: bool,
		/// The maintainer log op that is this acquire's or release's durable record, and
		/// its creation time. The operation holds its footprint while the op commits.
		log: Option<(maintainer_log::LogToken, i64)>,
	},
	/// One page of pending-work discovery: of every pending row when `volume` is `None`,
	/// else of that volume key's rows only.
	PendingWork {
		volume: Option<depot_fs_core::work::PendingVolume>,
	},
	SweepPending {
		scope: depot_fs_core::keys::slatedb::Scope,
	},
	Checkpoint {
		request_id: RequestId,
		/// The checkpointed volume, fixed at dispatch.
		volume_key: depot_types::VolumeKey,
		/// Whether this is a checkpoint delete, the one change that can release a retired
		/// incarnation's last hold; its completion scans the volume's pending rows.
		releases: bool,
		/// A change without a holder: its maintainer log op and the op's time. The store
		/// change waits for the op to commit; see `logged_ops`.
		log: Option<(maintainer_log::LogToken, i64)>,
	},
	FilesystemQuery {
		request_id: RequestId,
		order: filesystem_reads::QueryOrder,
		/// The volume of an ordered query, fixed at dispatch so the operation index
		/// can be recomputed from operations alone. Whether the query still blocks its
		/// volume is decided by its caller remaining in `pending_fs_reads`.
		volume_key: Option<depot_types::VolumeKey>,
	},
	FilesystemLoad {
		key: VolumeLogKey,
	},
	FilesystemRead {
		key: VolumeLogKey,
		worker: WorkerId,
		proof: VolumeLogReplicaState,
		previous: VolumeLogPosition,
	},
	FilesystemApply {
		resolved: depot_fs_core::txn::Resolver,
		applied_bytes: u64,
		receipts: Vec<rivet_data::generated::shared_depot_fs_v1::FsWriteReceipt>,
		/// The page's entries where the client epoch changes from the entry before it in the
		/// page, starting with its first entry. Merged into the volume's `epochs` on success.
		epochs: Vec<VolumeLogPosition>,
		/// The policy of the page's last `FsRetentionSet`, which the loaded record takes
		/// when the apply succeeds. `Some(None)` restores the default policy.
		retention: Option<Option<rivet_data::generated::shared_depot_fs_v1::FsRetentionPolicy>>,
		/// Whether the page releases a checkpoint or fork hold, which can leave a pending
		/// sweep of a retired incarnation for discovery to find.
		releases_holds: bool,
		key: VolumeLogKey,
		through: VolumeLogPosition,
		bytes: u64,
		first_write_unix_ms: i64,
	},
	/// One flush of due volume records. One flush may stage while another uploads, on
	/// disjoint records, so at most two are in flight. While `Staging`, the records are
	/// being written. While `Uploading`, the staged memtable is being made
	/// durable and later applies, loads and opens may run; each record's cold position
	/// is published only when its own upload completes, in any order.
	FilesystemFlush {
		records: Vec<depot_fs_core::FsVolumeRecord>,
		stage: filesystem::FlushStage,
		/// This pass's dispatch count in the maintainer log's accounting of coverage.
		dispatch: u64,
	},
	PromotionList {
		worker: WorkerId,
		after: Option<VolumeLogKey>,
		collecting: bool,
	},

	CatchUp {
		key: VolumeLogKey,
		learner: WorkerId,
		step: catch_up::CopyStep,
	},
	ReadDeleteRow {
		key: VolumeLogKey,
	},
	DeleteReplica {
		key: VolumeLogKey,
		worker: WorkerId,
		action: deletion::DeleteAction,
	},
	/// One claim of a drain; see `drain`.
	DrainClaim {
		key: VolumeLogKey,
		worker: WorkerId,
	},
	SweepDeleted {
		key: VolumeLogKey,
	},
	PrepareSplit {
		step: split::SplitStep,
	},
	ReportCatchUp {
		tablet_replica_set_version: u64,
	},
	ReportSplit,
	DeleteRetired {
		tablet_id: [u8; 16],
	},
	/// One unconditional forget of a retired parent's maintainer log replica on `worker`,
	/// sent by the parent's cleanup owner once every child imported the log.
	ForgetRetiredLog {
		tablet_id: [u8; 16],
		worker: rivet_topology::worker::WorkerId,
	},
	ReportRetired {
		tablet_id: [u8; 16],
	},
	/// A split child's report that its own log holds its parent's ops; see `parent_log`.
	ReportParentLogImported,
	/// A coordinator report of the tablet's data version marks, retained for acceptance.
	ReportVersionMarks {
		version_marks: Vec<rivet_data::generated::shared_depot_types_v1::VersionMark>,
	},
	RecoverVolume {
		key: VolumeLogKey,
		purpose: repair::RecoveryPurpose,
		/// Captured at admission; later discovery hints cannot preempt required matching.
		required_members: BTreeSet<WorkerId>,
		cancelled: bool,
	},
	ReadUnloadRow {
		key: VolumeLogKey,
	},
	/// Conditional forgets of fully cold volumes on one member, each expecting its
	/// replica at the given cold position. One request covers up to
	/// `max_forget_batch_volumes` volumes.
	ForgetVolumes {
		worker: WorkerId,
		volumes: Vec<(VolumeLogKey, VolumeLogPosition)>,
		/// Volumes whose close was cancelled while this request was in flight. Their answers
		/// go to `unload::late_forget`, never to a later close of the same volume.
		detached: BTreeSet<VolumeLogKey>,
	},

	WriteToCold {
		rows: BTreeMap<VolumeLogKey, FsVolumeRecord>,
		/// The pass dispatch count of an uploading pass; see
		/// `maintainer_log::store_pass_dispatched`. `None` for a pass that only stages
		/// deleted rows and uploads nothing.
		dispatch: Option<u64>,
	},

	Trim {
		key: VolumeLogKey,
		worker: WorkerId,
		cold: VolumeLogPosition,
	},
	/// Trims of several volumes on one member under one tablet grant check, each to its
	/// published cold position. Each completes as its own `Trim` would.
	TrimBatch {
		worker: WorkerId,
		trims: Vec<(VolumeLogKey, VolumeLogPosition)>,
	},
	/// One batched status read of `keys` from `worker`. `barrier` names the barrier the
	/// read belongs to; scheduled and targeted reads have none.
	Poll {
		replica_set_version: u64,
		keys: Vec<VolumeLogKey>,
		worker: WorkerId,
		barrier: Option<poll::PollBarrier>,
	},
	OpenCold,
	List {
		worker: WorkerId,
		after: Option<VolumeLogKey>,
	},
	ReadRow {
		volume_key: depot_types::VolumeKey,
	},
	/// Work of an operator repair request; see `operator_repair`.
	Repair(operator_repair::RepairOperation),
	/// An open's read-back of named keys; see `open_reads`.
	ReadOpenKeys {
		request_id: RequestId,
		volume_key: depot_types::VolumeKey,
	},
	Load {
		step: lifecycle::ReplicaOpenStep,
		worker: WorkerId,
		key: VolumeLogKey,
	},
	ForgetOrphan {
		worker: WorkerId,
		key: VolumeLogKey,
	},
	/// One request of the maintainer log to a member; see `maintainer_log`.
	MaintainerLog {
		worker: WorkerId,
		request: maintainer_log::LogRequest,
	},
	/// The store apply of one recovered maintainer log op. `volume_key` is the volume whose
	/// rows the op changes, fixed at dispatch for the footprint.
	ReplayOp {
		token: maintainer_log::LogToken,
		volume_key: Option<depot_types::VolumeKey>,
	},
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct PendingOpen {
	pub caller: OpenVolume,
	pub step: OpenStep,
}

/// A reopen owns its key while canceled recovery is joining; it cannot start loading early.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum OpenStep {
	/// A recovery owner is canceling; loading and client replies wait for its completion.
	WaitingForRepair,
	/// The row lookup is queued or in flight under this caller’s reserved key.
	ReadingRow,
	/// Member responses accumulate until a quorum succeeds or the attempt fails.
	Loading(LoadingVolume),
}

/// Birth proof for a claim issued while reconfiguring a Joint replica set. An attempt
/// born before the disk barriers cannot gain a promotion proof from later replies.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum JointColdOpen {
	AwaitingBarrier,
	Observed(Tablet),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct LoadingVolume {
	/// Pins new replicas to this attempt's claim; Observed also permits cold promotion.
	pub joint_cold_open: Option<JointColdOpen>,
	pub open_claim: Option<rivet_data::generated::shared_depot_types_v1::OpenClaim>,
	/// Only this attempt's reconciled load responses, never cached observations.
	pub states: BTreeMap<WorkerId, VolumeLogReplicaState>,
	/// This incarnation was drawn in this lifetime and no caller has received its quorum result.
	pub unacknowledged_create: bool,
	pub key: VolumeLogKey,
	pub row: FsVolumeRecord,
	pub succeeded: BTreeSet<WorkerId>,
	pub operations: BTreeSet<OpId>,
	/// Where the opener's initial writes end, when this attempt creates the volume and sends
	/// them with the loads. The open reports them committed only if every answering member
	/// holds them.
	pub initial_through: Option<VolumeLogPosition>,
	/// The open reached its quorum and waits for the remaining members so it can read back
	/// its named keys; see `open_reads`. Its read timer bounds the wait.
	pub read_wait: bool,
	/// Members this attempt is reloading in sync after proving them caught up; see
	/// `lifecycle::promote_caught_up`. Set at most once per attempt.
	pub promoted: Option<Promoted>,
	/// Replica incarnations a forget of a cancelled close removed during this attempt, by
	/// member. An answer from such a replica is stale and never counts toward the quorum.
	pub lost: BTreeMap<WorkerId, [u8; 16]>,
}

/// Caught-up `Repairing` members an open forgets and loads again in sync, with the newest
/// claim its answers showed, which each reload copies.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Promoted {
	pub workers: BTreeSet<WorkerId>,
	pub claimed_client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	pub claimed_holder: Option<rivet_data::generated::shared_depot_types_v1::VolumeHolder>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaintainerState {
	pub(crate) alarms: alarms::Alarms,
	pub(crate) stop_checks: stop::Checks,
	/// Delivered with the exact tablet assignment; fences abandoned ordinary handoffs.
	pub(crate) manifest_version: u64,
	pub(crate) now_boot_ms: u64,
	/// Diagnostics only. When this lifetime last warned about a store operation that
	/// outlived its deadline, and how many such misses it counted since without warning.
	/// A slow store misses deadlines on many operations at once, so one warning per
	/// deadline window per tablet names the problem without flooding the log.
	pub(crate) slow_store_warning: Option<SlowStoreWarning>,
	/// The tablet's own durable log of maintainer changes.
	pub maintainer_log: maintainer_log::MaintainerLog,
	/// The claim epochs this lifetime issued under its grant.
	pub(crate) claim_epochs: claim_epochs::ClaimEpochs,
	pub(crate) forks: forks::Forks,
	pub(crate) fork_cleanup: fork_cleanup::Cleanup,
	pub(crate) materializations: materialize::Materializations,
	/// Bounded grant-local proof that no durable fork preparation held this name.
	/// New fork admission invalidates it before queuing any persistence.
	pub(crate) fork_checked: BTreeSet<depot_types::VolumeKey>,
	pub(crate) pending_work: pending_work::Discovery,
	/// Volume keys whose pending rows a finished store operation touched and that
	/// discovery has not scanned since. Bounded by the volume keys with store work in
	/// flight; marks of one key share its scan.
	pub(crate) pending_volumes: std::collections::BTreeSet<depot_fs_core::work::PendingVolume>,
	/// A store operation that can write pending work finished while a discovery scan
	/// was already past the start, so the scan runs once more when it ends.
	pub(crate) pending_work_again: bool,
	/// Pending sweeps waiting in memory: held by an unexpired read session, retried on
	/// `HeldSweepRetry`, or waiting for their volume's replay or drain to end. A sweep held by
	/// a checkpoint or fork hold is not kept here; the scan of its volume's pending rows after
	/// the hold's release queues it again.
	pub(crate) held_sweeps: BTreeSet<depot_fs_core::keys::slatedb::Scope>,
	/// Whether `HeldSweepRetry` is armed. A sweep a read session holds needs the deadline
	/// whatever else waits in `held_sweeps`, and arming it only once keeps later holds from
	/// postponing an earlier one's retry.
	pub(crate) held_sweep_retry_armed: bool,
	pub(crate) pending_fork_references: BTreeMap<RequestId, fork_references::Pending>,
	pub(crate) pending_checkpoints: BTreeMap<RequestId, checkpoints::Pending>,
	pub filesystem: filesystem::Filesystem,
	/// Operator repair calls of this lifetime; see `operator_repair`.
	pub(crate) repairs: operator_repair::Repairs,
	pub(crate) retired: BTreeMap<[u8; 16], retired::Cleanup>,
	/// This split child's report of its parent log import; see `parent_log`.
	pub(crate) parent_log: parent_log::Report,
	pub(crate) version_report: version_marks::VersionReport,
	pub(crate) deleting: BTreeMap<VolumeLogKey, deletion::Deletion>,
	/// Incarnations whose final tail is being applied before a delete or restore stops apply.
	pub(crate) drains: BTreeMap<VolumeLogKey, drain::Drain>,
	/// Tombstone observations awaiting deletion recovery; live volumes require no scan.
	pub(crate) deletion_candidates: BTreeSet<VolumeLogKey>,
	pub config: MaintainerConfig,
	pub(crate) catch_up: catch_up::CatchUpCycle,
	pub(crate) membership_goals: BTreeMap<VolumeLogKey, catch_up::Goal>,
	pub(crate) membership_checks: catch_up::Checks,
	pub(crate) promotion: promotion::Promotion,
	/// Size-incompatible copies wait for cold/learner progress without stopping the tablet.
	pub(crate) blocked_copies: BTreeMap<VolumeLogKey, catch_up::BlockedCopy>,
	pub(crate) split_preparation: Option<split::SplitPreparation>,
	/// Members missing from loaded volumes that this maintainer is reloading.
	pub(crate) member_repairs: member_repair::MemberRepairs,
	pub tablet: Tablet,
	pub phase: MaintainerPhase,
	/// A terminal recovery error retained in the fenced snapshot for the supervisor.
	pub recovery_failure: Option<RecoveryFailure>,
	pub loaded_volumes: BTreeMap<VolumeLogKey, LoadedVolume>,
	pub(crate) unloading: BTreeMap<VolumeLogKey, unload::Unloading>,
	pub(crate) listed: BTreeSet<WorkerId>,
	pub(crate) listing_cursors: BTreeMap<WorkerId, Option<VolumeLogKey>>,
	pub(crate) operations: operations::Operations,
	pub(crate) pending: BTreeMap<depot_types::VolumeKey, PendingOpen>,
	/// Creating opens of a key whose current incarnation is being deleted, at most one per key.
	/// Each waits under its own deadline until the deletion releases the key, then is admitted
	/// again and draws a new incarnation. The old incarnation's data is swept in the background,
	/// so the wait covers the replica tombstones and the record handoff, not the data sweep.
	pub(crate) awaiting_deletion: BTreeMap<depot_types::VolumeKey, OpenVolume>,
	/// The volume key of every open whose deadline timer is armed, so an expiry finds its
	/// caller in `pending` or `awaiting_deletion` without scanning them.
	pub(crate) open_deadlines: lifecycle::OpenDeadlines,
	/// Opens whose reply waits for a read-back of named keys, by caller.
	pub(crate) open_reads: BTreeMap<RequestId, open_reads::OpenRead>,
	pub(crate) pending_fs_reads:
		BTreeMap<RequestId, rivet_data::generated::shared_depot_fs_read_v1::FsReadRequest>,
	/// Session opens at a position this maintainer has not applied yet, by volume, with the
	/// position each waits for. Every one is also in `pending_fs_reads` with its deadline
	/// timer armed; an apply that reaches its position queues it again, and its timer answers
	/// it otherwise. An entry whose caller already left is skipped when it is woken.
	pub lagging_opens: BTreeMap<VolumeLogKey, Vec<(u64, RequestId)>>,
	/// When the tablet store's unflushed bytes reached SlateDB's limit, as the store last
	/// reported, or `None` while they are below it. While full, every store write waits for a
	/// memtable upload.
	pub(crate) memtable_budget_full_since: Option<u64>,
	/// Whether the memtable budget has stayed full for `MEMTABLE_FULL_SHED_AFTER_MS`, so
	/// reads that would wait for an apply or queued store work are refused.
	pub(crate) memtable_budget_shedding: bool,
	/// Ordered reads admitted during this event while shedding. Each is refused after the
	/// event's store admission unless it already started.
	pub(crate) admitted_while_budget_full: Vec<RequestId>,
	/// Volumes of the opens admitted during this event while shedding whose row lookup was
	/// queued. Each open is refused after the event's store admission unless its lookup
	/// already started.
	pub(crate) opens_admitted_while_budget_full: Vec<depot_types::VolumeKey>,
	/// Creation rows wait for the next cold-store pass, which does not delay client replies.
	pub(crate) pending_rows: cold_pass::PendingRows,
	/// Rows at the front of `pending_rows` that the last pass skipped because work of
	/// their volume was in flight or queued ahead of it. The queued pass holds their
	/// later work behind it. Recomputed at every pass dispatch.
	pub(crate) pass_deferred: BTreeSet<VolumeLogKey>,
	/// The cold position and status of the last row this grant wrote for each loaded volume,
	/// by a flush or a cold write pass. This maintainer is the only writer under its grant, so
	/// while a volume's stored row still matches, the store holds exactly that row and an
	/// unload need not read it back. A row a predecessor may have written is never here.
	pub(crate) rows_written: BTreeMap<VolumeLogKey, (VolumeLogPosition, FsVolumeStatus)>,
	/// Closes naming a claim newer than any observed, waiting for a status probe of their
	/// volume. At most one per loaded volume.
	pub(crate) pending_closes:
		BTreeMap<VolumeLogKey, rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	pub(crate) unload_drive: unload::UnloadDrive,
	pub(crate) deletion_drive: deletion::DeletionDrive,
	/// Loaded volume keys interned once; the per-volume queues and parked sets key on them.
	pub(crate) volume_keys: volume_keys::VolumeKeys,
	/// When each loaded volume is next checked for idle unload.
	pub(crate) idle_checks: unload::IdleChecks,
	/// Quiet held volumes by holder process, with one topology watch per holder.
	pub(crate) held_volumes: holders::HeldVolumes,
	pub(crate) store_queue: store::StoreQueue,
	/// Inputs of the last settled store admission. While they are unchanged, another
	/// admission pass would start nothing, so `store::drive` skips it.
	pub(crate) store_admission: Option<store::AdmissionInputs>,
	/// Where the last settled store admission scan stopped; see `store::AdmissionCursor`.
	pub(crate) admission_cursor: Option<store::AdmissionCursor>,
	/// Admitted session reads waiting for a read slot, in arrival order. They do not
	/// wait on the store queue; see `store` for the sharing rule.
	pub(crate) session_reads: VecDeque<RequestId>,
	pub(crate) cold_batch_pending: bool,
	pub(crate) trim_ready: BTreeSet<(VolumeLogKey, WorkerId)>,
	/// Member trims waiting for their `TrimRetry`. No drive resends one before it fires,
	/// so a member that fails trims at once is not retried in the same instant.
	pub(crate) trim_backoff: BTreeSet<(VolumeLogKey, WorkerId)>,
	pub(crate) pending_trims: BTreeMap<VolumeLogKey, VolumeLogPosition>,
	pub(crate) poll: poll::PollCycle,
	/// Per-volume status schedule and barrier generations.
	pub(crate) status_polls: poll::StatusPolls,
	/// Volumes whose members a commit notice asked to observe outside a full round.
	pub(crate) poll_targets: std::collections::BTreeSet<VolumeLogKey>,
	/// Proven orphans whose forgets the in-flight cap deferred. Every orphan collection
	/// examines them again, so a finished forget picks up the next one.
	pub(crate) orphan_backlog: std::collections::BTreeSet<VolumeLogKey>,
	/// Payload bytes of initial entries retained by unacknowledged creations for their
	/// retries, bounded by `max_pending_initial_bytes`. Changed only through `record_grant`
	/// and `release_initial_entries`.
	pub(crate) initial_bytes_held: u64,
	/// Opens whose initial writes were not sent because they exceeded a budget. Cumulative;
	/// published for a metric.
	pub(crate) initial_writes_withheld: u64,
	/// Volume open requests taken from the inbox. Cumulative; published so a submitter
	/// can count the opens still queued in the inbox against its early refusal cap.
	pub(crate) opens_received: u64,
	next_op_id: u64,
	/// The first disagreement between the incremental drives and a full pass, found by
	/// the debug oracle in `on_event` and reported by `debug_validate`.
	#[cfg(debug_assertions)]
	pub(crate) drive_oracle_error: Option<String>,
}

impl MaintainerState {
	/// Whether reserved volumes reach `limit`. The exact count visits every deletion, so it
	/// runs only when loaded volumes and deletions together could reach the limit, which
	/// keeps opens and deletes cheap while many deletions are in progress.
	pub(crate) fn reserves_at_least(&self, limit: usize) -> bool {
		self.loaded_volumes.len() + self.deleting.len() >= limit && self.reserved_volumes() >= limit
	}

	/// Row lookups reserve capacity before an unloaded deletion creates a loaded item.
	pub(crate) fn reserved_volumes(&self) -> usize {
		self.loaded_volumes.len()
			+ self
				.deleting
				.iter()
				.filter(|(key, _)| !self.loaded_volumes.contains_key(key))
				.count()
	}

	/// All pending client operations share one admission budget.
	pub(crate) fn request_count(&self) -> usize {
		stop::pending(self)
			+ forks::pending_requests(self)
			+ self.pending_fork_references.len()
			+ self.pending_checkpoints.len()
			+ self.pending.len()
			+ self.awaiting_deletion.len()
			+ self.pending_fs_reads.len()
			+ deletion::pending_requests(self)
	}

	/// Validates the fixed assignment before starting any grant-fenced I/O.
	pub fn new(tablet: Tablet, config: MaintainerConfig) -> anyhow::Result<Self> {
		membership::validate(&tablet.replica_set)?;
		config.checkpoints.validate()?;
		config.write_versions.validate()?;
		// A shorter wait would let membership recovery claim, and so fence, a live writer
		// before it has had a holder check interval to match the new members itself.
		anyhow::ensure!(
			config.tuning().membership_holder_wait_ms
				>= config
					.tuning()
					.holder_check_after_ms
					.max(config.tuning().repair_quiet_ms),
			"membership_holder_wait_ms must be at least holder_check_after_ms and repair_quiet_ms"
		);
		config.alarms.validate()?;
		anyhow::ensure!(
			tablet.tablet_grant_epoch > 0
				&& tablet.tablet_grant_epoch < u64::MAX
				&& tablet.hash_range.is_valid(),
			"invalid tablet assignment"
		);
		anyhow::ensure!(
			config.idle_unload_after_ms > 0
				&& config.tuning().held_sweep_retry_ms > 0
				&& config.tuning().max_delete_requests > 0
				&& config.tuning().max_delete_rows > 0
				&& config.tuning().max_repairs > 0
				&& config.tuning().repair_quiet_ms > 0
				&& config.max_requests > 0
				&& config.max_loaded_volumes > 0
				&& config.tuning().max_volume_key_bytes > 0
				&& config.tuning().listing_page_size > 0
				&& config.tuning().operation_timeout_ms > 0
				&& config.tuning().cold_io_timeout_ms > 0
				&& config.write_to_cold_after_ms > 0
				&& (1..=256 * 1024 * 1024).contains(&config.write_to_cold_above_bytes)
				&& config.tuning().max_rows_per_pass > 0
				&& config.tuning().max_flush_records > 0
				&& config.tuning().max_forgets_in_flight > 0
				&& config.max_forget_batch_volumes > 0
				&& CREATION_RETRY_MS > config.tuning().operation_timeout_ms
				&& config.tuning().max_initial_bytes <= config.tuning().max_pending_initial_bytes
				&& config.tuning().retry_ms > 0
				&& config.max_cold_reads > 0
				&& config.max_background_cold_reads > 0
				&& config.max_keyed_in_flight > 0
				&& config.max_flush_staging > 0
				&& config.max_flush_uploads >= config.max_flush_staging
				&& config.admission_scan_limit > 0
				&& config.max_concurrent_materializations > 0
				&& config.materialize_batch_chunks > 0
				&& config.tuning().max_cold_read_bytes > 0
				&& MAX_COLD_BYTES
					>= 2 * (u64::from(config.tuning().max_cold_read_bytes)
						+ u64::from(config.tuning().max_volume_key_bytes)
						+ 64) && config.tuning().max_status_requests > 0
				&& config.tuning().max_trim_requests > 0
				&& config.tuning().status_poll_ms > 0
				&& config.tuning().max_status_batch_volumes > 0,
			"invalid maintainer limits"
		);
		if let Some(split) = &tablet.split {
			anyhow::ensure!(
				matches!(
					tablet.replica_set.phase,
					rivet_data::generated::shared_depot_types_v1::TabletPhase::Stable
				),
				"split cannot overlap replacement"
			);
			anyhow::ensure!(
				tablet.hash_range.start < split.split_key_hash
					&& tablet.hash_range.contains(split.split_key_hash)
					&& split.left_tablet_id != tablet.id
					&& split.right_tablet_id != tablet.id
					&& split.left_tablet_id != split.right_tablet_id
					&& split.left_tablet_id != [0; 16]
					&& split.right_tablet_id != [0; 16]
					&& split.checkpoint_id != [0; 16],
				"invalid tablet split"
			);
		}
		Ok(Self {
			now_boot_ms: 0,
			slow_store_warning: None,
			maintainer_log: Default::default(),
			claim_epochs: Default::default(),
			filesystem: Default::default(),
			repairs: Default::default(),
			alarms: Default::default(),
			retired: BTreeMap::new(),
			parent_log: parent_log::Report::default(),
			version_report: Default::default(),
			catch_up: Default::default(),
			promotion: promotion::Promotion::for_phase(&tablet.replica_set.phase),
			membership_goals: BTreeMap::new(),
			membership_checks: Default::default(),
			blocked_copies: Default::default(),
			deleting: BTreeMap::new(),
			drains: BTreeMap::new(),
			deletion_candidates: BTreeSet::new(),
			split_preparation: tablet
				.split
				.as_ref()
				.map(|_| split::SplitPreparation::Ready(split::SplitStep::Checkpoint)),
			config,
			tablet,
			phase: MaintainerPhase::Opening,
			recovery_failure: None,
			loaded_volumes: BTreeMap::new(),
			member_repairs: Default::default(),
			unloading: BTreeMap::new(),
			listed: BTreeSet::new(),
			listing_cursors: BTreeMap::new(),
			operations: operations::Operations::default(),
			pending: BTreeMap::new(),
			awaiting_deletion: BTreeMap::new(),
			open_deadlines: Default::default(),
			open_reads: BTreeMap::new(),
			initial_bytes_held: 0,
			initial_writes_withheld: 0,
			opens_received: 0,
			pending_fs_reads: BTreeMap::new(),
			lagging_opens: BTreeMap::new(),
			memtable_budget_full_since: None,
			memtable_budget_shedding: false,
			admitted_while_budget_full: Vec::new(),
			opens_admitted_while_budget_full: Vec::new(),
			pending_checkpoints: BTreeMap::new(),
			pending_fork_references: BTreeMap::new(),
			fork_checked: BTreeSet::new(),
			forks: Default::default(),
			fork_cleanup: Default::default(),
			materializations: Default::default(),
			pending_work: Default::default(),
			pending_volumes: Default::default(),
			pending_work_again: false,
			held_sweeps: BTreeSet::new(),
			held_sweep_retry_armed: false,
			pending_rows: Default::default(),
			pass_deferred: BTreeSet::new(),
			rows_written: BTreeMap::new(),
			pending_closes: BTreeMap::new(),
			unload_drive: Default::default(),
			deletion_drive: Default::default(),
			volume_keys: Default::default(),
			idle_checks: Default::default(),
			held_volumes: Default::default(),
			store_queue: Default::default(),
			store_admission: None,
			admission_cursor: None,
			session_reads: VecDeque::new(),
			next_op_id: 0,
			cold_batch_pending: false,
			pending_trims: BTreeMap::new(),
			trim_ready: BTreeSet::new(),
			trim_backoff: BTreeSet::new(),
			poll: Default::default(),
			status_polls: Default::default(),
			poll_targets: Default::default(),
			orphan_backlog: Default::default(),
			stop_checks: Default::default(),
			manifest_version: 0,
			#[cfg(debug_assertions)]
			drive_oracle_error: None,
		})
	}

	/// The deadline armed for `operation` when it begins. A store operation that outlives it
	/// waits another deadline of the same length.
	pub(crate) fn timeout_ms(&self, operation: &Operation) -> u64 {
		match operation {
			Operation::Alarm(alarms::Attempt::Running(crate::alarms::Request::Call { .. })) => {
				self.config.alarms.handler_timeout_ms
			}
			Operation::Alarm(alarms::Attempt::Waiting(_)) => self.config.alarms.retry_min_ms,
			Operation::Alarm(_) => self.config.alarms.io_timeout_ms,
			Operation::OpenCold => COLD_OPEN_TIMEOUT_MS,
			Operation::WriteToCold { .. }
			| Operation::ReadRow { .. }
			| Operation::ReadOpenKeys { .. }
			| Operation::Repair(_)
			| Operation::ReadUnloadRow { .. }
			| Operation::ReadDeleteRow { .. }
			| Operation::SweepDeleted { .. }
			| Operation::FilesystemQuery { .. }
			| Operation::FilesystemLoad { .. }
			| Operation::FilesystemApply { .. }
			| Operation::FilesystemFlush { .. }
			| Operation::Checkpoint { .. }
			| Operation::ForkReference { .. }
			| Operation::ForkStore { .. }
			| Operation::ForkCleanupStore { .. }
			| Operation::MaterializeStore { .. }
			| Operation::PendingWork { .. }
			| Operation::SweepPending { .. }
			| Operation::ReplayOp { .. } => self.config.tuning().cold_io_timeout_ms,
			Operation::PrepareSplit { .. } | Operation::DeleteRetired { .. } => {
				SPLIT_STORAGE_TIMEOUT_MS
			}
			Operation::MaterializePeer { .. }
			| Operation::ForkCleanupPeer { .. }
			| Operation::ForkPeer { .. }
			| Operation::PromotionList { .. }
			| Operation::CatchUp { .. }
			| Operation::DeleteReplica { .. }
			| Operation::DrainClaim { .. }
			| Operation::ReportCatchUp { .. }
			| Operation::ReportSplit
			| Operation::ReportRetired { .. }
			| Operation::ForgetRetiredLog { .. }
			| Operation::ReportParentLogImported
			| Operation::ReportVersionMarks { .. }
			| Operation::RecoverVolume { .. }
			| Operation::ForgetVolumes { .. }
			| Operation::FilesystemRead { .. }
			| Operation::Trim { .. }
			| Operation::TrimBatch { .. }
			| Operation::Poll { .. }
			| Operation::List { .. }
			| Operation::Load { .. }
			| Operation::ForgetOrphan { .. }
			| Operation::MaintainerLog { .. } => self.config.tuning().operation_timeout_ms,
		}
	}

	/// Reserves an operation identity and its timeout before emitting executor work.
	pub(crate) fn begin(&mut self, operation: Operation, effects: &mut Effects) -> OpId {
		self.next_op_id = self
			.next_op_id
			.checked_add(1)
			.expect("operation identity exhausted");
		let op_id = OpId(self.next_op_id);
		let timeout_ms = self.timeout_ms(&operation);
		let queued_alarm = matches!(operation, Operation::Alarm(alarms::Attempt::Queued(_)));
		if let Operation::Load { key, .. } = &operation {
			stop::load_started(self, op_id, key);
		}
		self.operations.insert(op_id, operation);
		if !queued_alarm {
			effects.arm_timer_after(MaintainerTimer::Operation(op_id), timeout_ms);
		}
		op_id
	}

	/// Removes a finished operation and cancels its timeout. A finished store operation
	/// that can write pending rows, a checkpoint or fork hold release included, scans the
	/// pending rows of the volumes it touched.
	pub(crate) fn finish(&mut self, op_id: OpId, effects: &mut Effects) -> Option<Operation> {
		let operation = self.operations.remove(&op_id)?;
		if matches!(operation, Operation::Load { .. }) {
			stop::load_finished(self, op_id);
		}
		effects.cancel_timer(MaintainerTimer::Operation(op_id));
		let volumes = pending_volumes_written(&operation);
		pending_work::wrote(self, volumes, effects);
		unload::operation_finished(self, &operation);
		Some(operation)
	}

	/// Removes a loaded volume together with its status schedule. Every removal from
	/// `loaded_volumes` goes through here.
	pub fn remove_loaded_volume(&mut self, key: &VolumeLogKey) -> Option<LoadedVolume> {
		self.release_initial_entries(key);
		catch_up::untrack(self, key);
		poll::untrack(self, key);
		self.pending_closes.remove(key);
		self.rows_written.remove(key);
		self.filesystem.committed_floors.remove(key);
		self.filesystem.waiting_opens.remove(key);
		self.filesystem.record_missing.remove(key);
		if let Some(id) = self.volume_keys.find(key) {
			holders::unpark(&mut self.held_volumes, id);
			self.idle_checks.queue.cancel(&id);
			self.filesystem.retention.cancel(&id);
		}
		self.volume_keys.release(key);
		self.loaded_volumes.remove(key)
	}

	/// Drops the initial entries a creation retained for its retries and their bytes from
	/// `initial_bytes_held`. Called wherever a creation is acknowledged, expires or goes away;
	/// the grant's identity stays while the creation does.
	pub(crate) fn release_initial_entries(&mut self, key: &VolumeLogKey) {
		if let Some(volume) = self.loaded_volumes.get_mut(key) {
			release_volume_initial_entries(volume, &mut self.initial_bytes_held);
		}
	}

	/// Removes every loaded volume and its status schedule, for test setups.
	#[doc(hidden)]
	pub fn clear_loaded_volumes(&mut self) {
		let keys = self.loaded_volumes.keys().cloned().collect::<Vec<_>>();
		for key in keys {
			self.remove_loaded_volume(&key);
		}
	}

	/// Checks every loaded volume for idle unload at the next idle timer, scheduling any
	/// that is not scheduled yet. Tests that fire the timer use this to stand in for
	/// elapsed deadlines; each check still rederives its real deadline.
	#[doc(hidden)]
	pub fn check_idle_now(&mut self) {
		let keys = self.loaded_volumes.keys().cloned().collect::<Vec<_>>();
		for key in keys {
			let id = self.volume_keys.intern(&key);
			self.idle_checks.queue.schedule(id, self.now_boot_ms);
		}
	}

	/// Checks every registered volume's retention at the next retention timer.
	#[doc(hidden)]
	pub fn check_retention_now(&mut self) {
		let keys = self.filesystem.volumes.keys().cloned().collect::<Vec<_>>();
		for key in keys {
			let id = self.volume_keys.intern(&key);
			self.filesystem.retention.schedule(id, self.now_boot_ms);
		}
	}

	pub(crate) fn members(&self) -> Vec<WorkerId> {
		membership::destinations(&self.tablet.replica_set)
	}

	/// Returns the disk admitted by this assignment; an absent member cannot authorize a load.
	pub(crate) fn disk_incarnation(&self, worker: &WorkerId) -> Option<[u8; 16]> {
		self.tablet
			.replica_set
			.disks
			.iter()
			.find(|disk| disk.worker_id == *worker.as_bytes())
			.map(|disk| disk.disk_incarnation)
	}

	pub(crate) fn grant(
		&self,
	) -> rivet_data::generated::depot_storage_worker_protocol_v1::TabletGrant {
		rivet_data::generated::depot_storage_worker_protocol_v1::TabletGrant {
			tablet_id: self.tablet.id,
			tablet_grant_epoch: self.tablet.tablet_grant_epoch,
		}
	}
}

impl rivet_state_machine::StateMachineState for MaintainerState {}

/// Names the fields in which two states differ, ignoring the bookkeeping that only
/// decides which volumes the drives evaluate: filesystem candidates, the unload drive's
/// marks, the store admission record, and the debug oracle's own report.
#[cfg(debug_assertions)]
pub(crate) fn differing_fields(a: &MaintainerState, b: &MaintainerState) -> Vec<&'static str> {
	let mut differing = Vec::new();
	macro_rules! compare {
		($($field:ident),* $(,)?) => {
			$(
				if a.$field != b.$field {
					differing.push(stringify!($field));
				}
			)*
		};
	}
	compare!(
		alarms,
		stop_checks,
		now_boot_ms,
		maintainer_log,
		forks,
		fork_cleanup,
		materializations,
		fork_checked,
		pending_work,
		pending_volumes,
		pending_work_again,
		held_sweeps,
		held_sweep_retry_armed,
		pending_fork_references,
		pending_checkpoints,
		retired,
		deleting,
		deletion_candidates,
		config,
		catch_up,
		promotion,
		blocked_copies,
		split_preparation,
		member_repairs,
		tablet,
		phase,
		loaded_volumes,
		unloading,
		listed,
		membership_goals,
		membership_checks,
		listing_cursors,
		operations,
		pending,
		pending_fs_reads,
		lagging_opens,
		repairs,
		memtable_budget_full_since,
		memtable_budget_shedding,
		pending_rows,
		pending_closes,
		idle_checks,
		held_volumes,
		store_queue,
		session_reads,
		cold_batch_pending,
		trim_ready,
		trim_backoff,
		pending_trims,
		poll,
		status_polls,
		poll_targets,
		next_op_id,
	);
	if !filesystem::same_ignoring_marks(&a.filesystem, &b.filesystem) {
		differing.push("filesystem");
	}
	differing
}

/// The volume keys whose pending rows a finished store operation may have written, for
/// discovery to scan. Empty for an operation that writes no pending row discovery would
/// find. A fork reference writes one only when it releases a hold, and a checkpoint change
/// only when it deletes a checkpoint: either can leave a retired incarnation's sweep free to
/// run. The pending check an acquire writes gets its deadline from the acquire itself.
fn pending_volumes_written(operation: &Operation) -> Vec<depot_fs_core::work::PendingVolume> {
	use depot_fs_core::work::PendingVolume;
	match operation {
		Operation::MaterializeStore { scope } => vec![PendingVolume::of_scope(scope)],
		Operation::ForkCleanupStore { work } => vec![PendingVolume::of_scope(&work.scope)],
		Operation::ForkStore {
			volume_key,
			parent_volume_key,
		} => std::iter::once(volume_key)
			.chain(parent_volume_key)
			.map(PendingVolume::of)
			.collect(),
		Operation::Checkpoint {
			volume_key,
			releases: true,
			..
		} => vec![PendingVolume::of(volume_key)],
		Operation::Checkpoint {
			releases: false, ..
		} => Vec::new(),
		Operation::SweepDeleted { key, .. } => vec![PendingVolume::of(&key.0.volume_key)],
		Operation::ForkReference {
			volumes,
			releases: true,
			..
		} => volumes.iter().map(PendingVolume::of).collect(),
		Operation::ForkReference {
			releases: false, ..
		} => Vec::new(),
		Operation::FilesystemApply {
			releases_holds: true,
			key,
			..
		} => vec![depot_fs_core::work::PendingVolume::of(&key.0.volume_key)],
		Operation::ReplayOp {
			volume_key: Some(volume_key),
			..
		} => vec![PendingVolume::of(volume_key)],
		// A pass that writes a retired incarnation's record releases its retention
		// checkpoints, which can leave its sweep free to run.
		Operation::FilesystemFlush { records, .. } => records
			.iter()
			.filter(|record| record.status == depot_fs_core::FsVolumeStatus::Deleted)
			.map(|record| PendingVolume::of(&record.volume_key()))
			.collect(),
		Operation::MaterializePeer { .. }
		| Operation::ForkCleanupPeer { .. }
		| Operation::ForkPeer { .. }
		| Operation::PendingWork { .. }
		| Operation::SweepPending { .. }
		| Operation::FilesystemQuery { .. }
		| Operation::FilesystemLoad { .. }
		| Operation::FilesystemRead { .. }
		| Operation::FilesystemApply { .. }
		| Operation::PromotionList { .. }
		| Operation::CatchUp { .. }
		| Operation::ReadDeleteRow { .. }
		| Operation::DeleteReplica { .. }
		| Operation::DrainClaim { .. }
		| Operation::PrepareSplit { .. }
		| Operation::ReportCatchUp { .. }
		| Operation::ReportSplit
		| Operation::DeleteRetired { .. }
		| Operation::ReportRetired { .. }
		| Operation::ForgetRetiredLog { .. }
		| Operation::ReportParentLogImported
		| Operation::ReportVersionMarks { .. }
		| Operation::RecoverVolume { .. }
		| Operation::ReadUnloadRow { .. }
		| Operation::ForgetVolumes { .. }
		| Operation::WriteToCold { .. }
		| Operation::Trim { .. }
		| Operation::TrimBatch { .. }
		| Operation::Poll { .. }
		| Operation::OpenCold
		| Operation::List { .. }
		| Operation::ReadRow { .. }
		| Operation::ReadOpenKeys { .. }
		| Operation::Repair(_)
		| Operation::Load { .. }
		| Operation::ForgetOrphan { .. }
		| Operation::Alarm(_)
		| Operation::MaintainerLog { .. }
		| Operation::ReplayOp {
			volume_key: None, ..
		} => Vec::new(),
	}
}

/// A store operation that outlived its deadline is slow, not lost. Its task keeps the
/// SlateDB call, which cannot be cancelled, and always reports a result, and its footprint
/// stays held until then. Slowness is not evidence of a newer owner: fencing would discard
/// the applied memtable and hand a successor a larger replay against the same store, whose
/// own open then outlives the same deadline. So the maintainer keeps its grant, counts the
/// missed deadline and waits another one. A newer owner still ends this lifetime through
/// the store's fencing error or the revoked grant.
pub(crate) fn store_operation_slow(
	state: &mut MaintainerState,
	op_id: OpId,
	effects: &mut Effects,
) {
	// A lifetime that is already ending waits for nothing; its close joins the task.
	if matches!(
		state.phase,
		MaintainerPhase::Fenced | MaintainerPhase::Stopped | MaintainerPhase::Draining
	) {
		return;
	}
	let Some(operation) = state.operations.get(&op_id) else {
		return;
	};
	let name = store_operation_name(operation);
	let timeout_ms = state.timeout_ms(operation);
	crate::metrics::TABLET_STORE_OPERATIONS_SLOW
		.with_label_values(&[name])
		.inc();
	effects.arm_timer_after(MaintainerTimer::Operation(op_id), timeout_ms);

	// Warn at most once per deadline window; the counter above records every miss.
	let now_boot_ms = effects.now_boot_ms();
	if let Some(suppressed) =
		SlowStoreWarning::admit(&mut state.slow_store_warning, now_boot_ms, timeout_ms)
	{
		tracing::warn!(
			tablet_id = ?state.tablet.id,
			grant_epoch = state.tablet.tablet_grant_epoch,
			phase = ?state.phase,
			operation = name,
			timeout_ms,
			suppressed,
			"tablet store operation outlived its deadline; the cold store is slow, so the maintainer keeps its grant and waits"
		);
	}
}

/// The last slow-store warning of one maintainer lifetime.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SlowStoreWarning {
	pub(crate) warned_boot_ms: u64,
	/// Deadline misses counted since that warning without logging one.
	pub(crate) suppressed: u64,
}

impl SlowStoreWarning {
	/// Decides whether a deadline miss at `now_boot_ms` logs. Returns the misses suppressed
	/// since the last warning when one `window_ms` has passed since it, or on the first miss,
	/// and otherwise counts this miss as suppressed.
	pub(crate) fn admit(
		warning: &mut Option<SlowStoreWarning>,
		now_boot_ms: u64,
		window_ms: u64,
	) -> Option<u64> {
		match warning {
			Some(last) if now_boot_ms < last.warned_boot_ms.saturating_add(window_ms) => {
				last.suppressed += 1;
				None
			}
			Some(_) | None => {
				let suppressed = warning.as_ref().map_or(0, |last| last.suppressed);
				*warning = Some(SlowStoreWarning {
					warned_boot_ms: now_boot_ms,
					suppressed: 0,
				});
				Some(suppressed)
			}
		}
	}
}



/// The metric label of a store operation, the operations whose deadline
/// `store_operation_slow` handles.
fn store_operation_name(operation: &Operation) -> &'static str {
	match operation {
		Operation::PendingWork { .. } => "pending_work",
		Operation::SweepPending { .. } => "sweep_pending",
		Operation::Checkpoint { .. } => "checkpoint",
		Operation::ForkReference { .. } => "fork_reference",
		Operation::ForkStore { .. } => "fork_store",
		Operation::ForkCleanupStore { .. } => "fork_cleanup_store",
		Operation::MaterializeStore { .. } => "materialize_store",
		Operation::FilesystemLoad { .. } => "filesystem_load",
		Operation::FilesystemApply { .. } => "filesystem_apply",
		Operation::FilesystemFlush { .. } => "filesystem_flush",
		Operation::ReadDeleteRow { .. } => "read_delete_row",
		Operation::SweepDeleted { .. } => "sweep_deleted",
		Operation::OpenCold => "open_cold",
		Operation::PrepareSplit { .. } => "prepare_split",
		Operation::DeleteRetired { .. } => "delete_retired",
		Operation::ReadUnloadRow { .. } => "read_unload_row",
		Operation::ReadRow { .. } => "read_row",
		Operation::WriteToCold { .. } => "write_to_cold",
		Operation::ReplayOp { .. } => "replay_op",
		Operation::Alarm(_)
		| Operation::MaterializePeer { .. }
		| Operation::ForkCleanupPeer { .. }
		| Operation::ForkPeer { .. }
		| Operation::PromotionList { .. }
		| Operation::CatchUp { .. }
		| Operation::DeleteReplica { .. }
		| Operation::DrainClaim { .. }
		| Operation::ReportCatchUp { .. }
		| Operation::ReportSplit
		| Operation::ReportRetired { .. }
		| Operation::ForgetRetiredLog { .. }
		| Operation::ReportParentLogImported
		| Operation::ReportVersionMarks { .. }
		| Operation::RecoverVolume { .. }
		| Operation::ForgetVolumes { .. }
		| Operation::FilesystemRead { .. }
		| Operation::FilesystemQuery { .. }
		| Operation::ReadOpenKeys { .. }
		| Operation::Repair(_)
		| Operation::Trim { .. }
		| Operation::TrimBatch { .. }
		| Operation::Poll { .. }
		| Operation::List { .. }
		| Operation::Load { .. }
		| Operation::ForgetOrphan { .. }
		| Operation::MaintainerLog { .. } => "other",
	}
}

/// An uncertain store operation or lost grant ends the lifetime, never advances it to a newer grant.
///
/// The caller's source location is logged and counted, because dozens of paths fence and a
/// fenced maintainer otherwise leaves no trace of which one ended its lifetime.
#[track_caller]
pub(crate) fn fence(state: &mut MaintainerState, effects: &mut Effects) {
	if matches!(
		state.phase,
		MaintainerPhase::Fenced | MaintainerPhase::Stopped | MaintainerPhase::Draining
	) {
		return;
	}
	let location = std::panic::Location::caller();
	let site = fence_site(location);
	tracing::warn!(
		tablet_id = ?state.tablet.id,
		grant_epoch = state.tablet.tablet_grant_epoch,
		phase = ?state.phase,
		site,
		line = location.line(),
		"tablet maintainer fenced; its supervisor recovers the tablet under a new grant"
	);
	crate::metrics::TABLET_MAINTAINER_FENCES
		.with_label_values(&[site])
		.inc();
	state.phase = MaintainerPhase::Fenced;
	stop::cancel(state, effects);
	for key in state.filesystem.volumes.keys() {
		effects.cancel_timer(MaintainerTimer::FilesystemDue(key.clone()));
		effects.cancel_timer(MaintainerTimer::FilesystemRetry(key.clone()));
	}
	effects.cancel_timer(MaintainerTimer::FilesystemFlushBatch);
	effects.cancel_timer(MaintainerTimer::MaintainerLogDue);
	effects.cancel_timer(MaintainerTimer::MemtableBudgetShed);
	for worker in std::mem::take(&mut state.maintainer_log)
		.members
		.into_keys()
	{
		effects.cancel_timer(MaintainerTimer::MaintainerLogRetry(worker));
	}
	state.poll_targets.clear();
	state.orphan_backlog.clear();
	state.pending_closes.clear();
	state.unload_drive = Default::default();
	state.deletion_drive = Default::default();
	effects.cancel_timer(MaintainerTimer::RetentionDue);
	effects.cancel_timer(MaintainerTimer::IdleUnload);
	state.idle_checks.armed_at = None;
	holders::stop(state, effects);
	effects.cancel_timer(MaintainerTimer::PendingWorkTick);
	effects.cancel_timer(MaintainerTimer::HeldSweepRetry);
	state.held_sweep_retry_armed = false;
	state.filesystem = Default::default();
	state.catch_up = Default::default();
	state.membership_goals.clear();
	state.membership_checks = Default::default();
	state.blocked_copies.clear();
	effects.cancel_timer(MaintainerTimer::CatchUp);
	retired::cancel(state, effects);
	version_marks::cancel(state, effects);
	state.store_queue.clear();
	state.session_reads.clear();
	state.split_preparation = None;
	effects.cancel_timer(MaintainerTimer::SplitRetry);
	state.unloading.clear();
	for (key, delete) in std::mem::take(&mut state.deleting) {
		effects.cancel_timer(MaintainerTimer::DeleteRequest(key.clone()));
		effects.cancel_timer(MaintainerTimer::DeleteRetry(key));
		if let Some(id) = delete.caller {
			lifecycle::reject(id, depot_types::client::not_maintainer(), effects);
		}
	}
	effects.cancel_timer(MaintainerTimer::OrphanSweep);
	effects.cancel_timer(MaintainerTimer::WriteToCold);
	effects.cancel_timer(MaintainerTimer::ColdBatch);
	state.cold_batch_pending = false;
	effects.cancel_timer(MaintainerTimer::Poll);
	poll::fence(state);
	member_repair::clear(state, effects);
	effects.cancel_timer(MaintainerTimer::Alarms);
	for (id, operation) in state.operations.take_all() {
		if matches!(
			operation,
			Operation::Alarm(
				alarms::Attempt::Running(_)
					| alarms::Attempt::Cancelling(_)
					| alarms::Attempt::Obsolete(_)
			)
		) {
			effects.execute(MaintainerEffect::CancelAlarmRequest { op_id: id });
		}
		effects.cancel_timer(MaintainerTimer::Operation(id));
	}
	for worker in state.members() {
		effects.cancel_timer(MaintainerTimer::ListRetry(worker));
	}
	// An open waiting for its read-back loses its maintainer like any other caller.
	for (id, _) in std::mem::take(&mut state.open_reads) {
		effects.cancel_timer(MaintainerTimer::OpenRead(id));
		lifecycle::reject(id, depot_types::client::not_maintainer(), effects);
	}
	for (_, pending) in std::mem::take(&mut state.pending) {
		lifecycle::reject(
			pending.caller.request_id,
			depot_types::client::not_maintainer(),
			effects,
		);
	}
	for (_, caller) in std::mem::take(&mut state.awaiting_deletion) {
		lifecycle::reject(
			caller.request_id,
			depot_types::client::not_maintainer(),
			effects,
		);
	}
	forks::fence(state, effects);
	effects.cancel_timer(MaintainerTimer::ForkCleanupDue);
	state.fork_cleanup = Default::default();
	materialize::fence(state, effects);
	for (request_id, _) in std::mem::take(&mut state.pending_fork_references) {
		effects.cancel_timer(MaintainerTimer::ForkReferenceRequest(request_id));
		lifecycle::reject(request_id, depot_types::client::not_maintainer(), effects);
	}
	for (request_id, _) in std::mem::take(&mut state.pending_checkpoints) {
		effects.cancel_timer(MaintainerTimer::CheckpointRequest(request_id));
		lifecycle::reject(request_id, depot_types::client::not_maintainer(), effects);
	}

	state.lagging_opens.clear();
	for (request_id, _) in std::mem::take(&mut state.pending_fs_reads) {
		effects.cancel_timer(MaintainerTimer::ReadRequest(request_id));
		lifecycle::reject(request_id, depot_types::client::not_maintainer(), effects);
	}
	for key in state.loaded_volumes.keys() {
		effects.cancel_timer(MaintainerTimer::CreationExpired(key.clone()));
		effects.cancel_timer(MaintainerTimer::UnloadRetry(key.clone()));
		effects.cancel_timer(MaintainerTimer::UnloadRecoveryRetry(key.clone()));
		for worker in state.members() {
			effects.cancel_timer(MaintainerTimer::TrimRetry(key.clone(), worker));
		}
	}
	state.trim_backoff.clear();
	effects.execute(MaintainerEffect::CloseCold);
}

/// Schedules a volume that just entered `loaded_volumes` for its status polls and its
/// idle check. A volume that is already scheduled keeps its schedules.
pub(crate) fn track_loaded(state: &mut MaintainerState, key: &VolumeLogKey) {
	catch_up::mark(state, key);
	poll::track(state, key);
	unload::track_idle(state, key);
}

/// As `track_loaded`, for a volume whose open loads every member: the load replies observe
/// each member, status notices report later changes, and `member_repair::open_completed`
/// handles a member the open could not load, so no status poll follows. Runtimes without a
/// notice transport still poll.
pub(crate) fn track_opened(state: &mut MaintainerState, key: &VolumeLogKey) {
	catch_up::mark(state, key);
	if state.config.tuning().poll_loaded_volumes {
		poll::track(state, key);
		poll::reset(state, key);
	}
	unload::track_idle(state, key);
}

/// Arms `timer` at the head of a deadline queue while serving, at `now` when the head
/// is already due, and cancels it otherwise. `armed_at` remembers the armed time, so an
/// unchanged head emits no effect; the timer's handler clears it when it fires.
pub(crate) fn arm_queue_head(
	armed_at: &mut Option<u64>,
	next_at: Option<u64>,
	phase: &MaintainerPhase,
	now: u64,
	timer: MaintainerTimer,
	effects: &mut Effects,
) {
	let wanted = match phase {
		MaintainerPhase::Serving => next_at.map(|at| at.max(now)),
		MaintainerPhase::Opening
		| MaintainerPhase::Frozen
		| MaintainerPhase::Draining
		| MaintainerPhase::Stopped
		| MaintainerPhase::Fenced => None,
	};
	if wanted == *armed_at {
		return;
	}
	match wanted {
		Some(at) => effects.arm_timer_at(timer, at),
		None => effects.cancel_timer(timer),
	}
	*armed_at = wanted;
}

/// The incarnations of `volume_key` this maintainer holds loaded. A loaded incarnation is
/// live or still being deleted, and its own lifecycle owns its sweep, so a fork release
/// must not schedule one for it. Its record may not be stored yet: a first record rides
/// the tiering pass.
pub(crate) fn loaded_incarnations(
	state: &MaintainerState,
	volume_key: &depot_types::VolumeKey,
) -> Vec<[u8; 16]> {
	state
		.loaded_volumes
		.range(incarnations(volume_key))
		.map(|(key, _)| key.0.volume_incarnation)
		.collect()
}

/// The span of a map keyed by `VolumeLogKey` that holds every incarnation of `volume_key`.
/// Keys order by volume key before incarnation, so these entries are contiguous; lookups by
/// volume key use this range instead of scanning every loaded volume.
pub(crate) fn incarnations(
	volume_key: &depot_types::VolumeKey,
) -> std::ops::RangeInclusive<VolumeLogKey> {
	let bound = |volume_incarnation| {
		VolumeLogKey::from(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: volume_key.clone(),
			volume_incarnation,
		})
	};
	bound([0; 16])..=bound([0xff; 16])
}

/// Whether a volume's partition key lies in this tablet's hash range.
pub(crate) fn in_tablet(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	state
		.tablet
		.hash_range
		.contains(depot_types::partition_key_hash(key))
}

/// The fence call site's file relative to the maintainer module: a bounded label set fixed by
/// the code, taken from a static string without allocating.
pub(crate) fn fence_site(location: &'static std::panic::Location<'static>) -> &'static str {
	let file = location.file();
	file.find("tablet_maintainer_machine/")
		.map_or(file, |at| &file[at + "tablet_maintainer_machine/".len()..])
}
