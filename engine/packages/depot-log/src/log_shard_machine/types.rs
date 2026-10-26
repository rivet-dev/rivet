//! Commands after authority admission and deterministic shard resource limits.

use depot_types::VolumeLogKey;
use rivet_data::generated::depot_storage_worker_protocol_v1 as protocol;
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogEntry,
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition, VolumeLogReplicaStatus},
};
use serde::{Deserialize, Serialize};

/// Correlates one outstanding caller with its reply; independent of storage log sequences.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);

/// A decoded shard operation; the core owns admission, ordering and fencing.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogShardRequest {
	/// Receiving-host deadline. Disk reads never acquire a fresh budget after queueing.
	pub deadline_boot_ms: u64,
	pub request_id: RequestId,
	pub command: LogShardCommand,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogShardCommand {
	/// Returns FORMAT identity for placement without mutating replica or grant state.
	GetDiskIncarnation,
	/// Only Read is unfenced. All mutations name their client or tablet grant once.
	Replica {
		volume_log_key: VolumeLogKey,
		authority: Option<Authority>,
		command: LoadedVolumeReplicaCommand,
	},
	/// A range listing establishes the tablet grant before returning disk states.
	ListTabletReplicas(protocol::ListTabletReplicasRequest),
	/// Unfenced status probe for many replicas. Each key is answered from synced state exactly
	/// as a sequence-zero read would be, without reading segment bytes: at once when no key has
	/// a pending frame, otherwise after the write barrier.
	ReadStatus { volume_log_keys: Vec<VolumeLogKey> },
	/// Conditional forgets of many replicas under one grant check. Each volume is admitted
	/// as a `Forget` with its expected position and claim, and the batch replies once after
	/// the disk write that carries every forget frame.
	ForgetVolumes {
		grant: protocol::TabletGrant,
		volumes: Vec<(VolumeLogKey, VolumeLogPosition, Option<ClientIdentity>)>,
	},
	/// Trims of many replicas under one grant check. Each volume is admitted as a `Trim`
	/// with its cold position and claim, and the batch replies once after the disk write that
	/// carries every trim frame.
	TrimVolumes {
		grant: protocol::TabletGrant,
		volumes: Vec<(VolumeLogKey, VolumeLogPosition, Option<ClientIdentity>)>,
	},
}

/// The decoded wire authority; commands cannot carry a second, conflicting client.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Authority {
	Client {
		identity: ClientIdentity,
		tablet_replica_set_version: u64,
	},
	Grant(protocol::TabletGrant),
	/// A tablet maintainer writing its own log: the grant is checked as for `Grant` and the
	/// client claim as for `Client`.
	GrantedClient {
		grant: protocol::TabletGrant,
		identity: ClientIdentity,
	},
}

impl Authority {
	/// Splits a wire header into its single target and authority.
	pub fn from_header(header: protocol::ReplicaRequestHeader) -> (VolumeLogKey, Self) {
		match header {
			protocol::ReplicaRequestHeader::ClientRequestHeader(header) => (
				header.volume_log_key.into(),
				Self::Client {
					identity: header.client,
					tablet_replica_set_version: header.tablet_replica_set_version,
				},
			),
			protocol::ReplicaRequestHeader::MaintainerRequestHeader(header) => (
				header.volume_log_key.into(),
				Self::Grant(header.tablet_grant),
			),
			protocol::ReplicaRequestHeader::GrantClientRequestHeader(header) => (
				header.volume_log_key.into(),
				Self::GrantedClient {
					grant: header.tablet_grant,
					identity: header.client,
				},
			),
		}
	}
}

/// Replica transitions run after shard authority admission.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoadedVolumeReplicaCommand {
	/// Adds an absent replica; a greater learner generation replaces earlier membership history.
	LoadVolume {
		/// Ordinary claim, applied only when an absent stable-phase replica is created.
		open_claim: Option<rivet_data::generated::shared_depot_types_v1::OpenClaim>,
		/// A learner replaces an older generation once; retries keep its history.
		learner_replica_set_version: Option<u64>,
		disk_incarnation: [u8; 16],
		/// Copied when a learner generation creates or replaces a replica; retries keep its fence.
		claimed_client: Option<ClientIdentity>,
		/// The process holding `claimed_client`, copied with it.
		claimed_holder: Option<rivet_data::generated::shared_depot_types_v1::VolumeHolder>,
		cold: VolumeLogPosition,
		status: VolumeLogReplicaStatus,
		/// A create's first entries, applied with the open claim in the creating frame; see
		/// `LoadVolumeRequest.initialEntries`.
		initial_entries: Vec<VolumeLogEntry>,
		/// Opaque creation evidence stored in the state of a replica this load creates.
		creation: Option<bytes::Bytes>,
	},
	/// Installs the requesting client's fence before replying, with the process that holds it.
	Claim {
		holder: Option<rivet_data::generated::shared_depot_types_v1::VolumeHolder>,
	},
	/// Extends a matched suffix. A grant can copy only committed entries.
	Append {
		previous: VolumeLogPosition,
		entries: Vec<VolumeLogEntry>,
		committed_volume_seq: u64,
	},
	/// Restores in-sync health at the exact incarnation and tail that repair confirmed.
	MarkInSync {
		replica_incarnation: [u8; 16],
		expected_appended: VolumeLogPosition,
	},
	/// **When**
	///
	/// A claimed client repairs a divergent suffix; operator resync also names the
	/// exact observed replica incarnation.
	///
	/// **State**
	///
	/// Checks claim, optional incarnation, and appended position atomically before
	/// dropping a suffix above cold. A stale incarnation changes nothing.
	///
	/// **Effects**
	///
	/// Persists the changed replica before replying, or returns the current fence.
	Truncate {
		expected_replica_incarnation: Option<[u8; 16]>,
		expected_appended: VolumeLogPosition,
		keep_through_volume_seq: u64,
		mark_repairing: bool,
	},
	/// Publishes quorum progress that a replica cannot infer from its own writes.
	AdvanceCommitted { committed_volume_seq: u64 },
	/// Only a maintainer records completed cold-store writes.
	Trim {
		cold: VolumeLogPosition,
		claimed_client: Option<ClientIdentity>,
	},
	/// Rejects future client requests and retains a deletion marker until cleanup.
	MarkVolumeDeleted,
	/// Reads disk state and entries without requiring client or maintainer authority.
	Read {
		first_volume_seq: u64,
		max_bytes: u32,
		/// Also reads the tail a deleted replica retains.
		include_deleted: bool,
	},
	/// Some unloads a fully cold observed tail; None retires a deleted or orphan replica.
	Forget {
		observation: Option<protocol::ForgetObservation>,
		expected_appended: Option<VolumeLogPosition>,
		/// The claim the maintainer observed. A conditional forget under the tablet grant
		/// removes the replica only while its claim still equals this one.
		expected_claim: Option<ClientIdentity>,
	},
}

impl LogShardCommand {
	/// Decodes one request without admitting authority or making storage decisions. A batch
	/// is returned unchanged; its items each decode with `from_item`.
	pub fn from_wire(
		request: protocol::StorageWorkerRequest,
	) -> Result<Self, protocol::StorageWorkerBatchRequest> {
		depot_types::storage_worker::batch_item(request).map(Self::from_item)
	}

	/// Decodes one batched or single request without admitting authority or making storage
	/// decisions.
	pub fn from_item(request: protocol::StorageWorkerBatchItem) -> Self {
		use protocol::StorageWorkerBatchItem as Wire;
		let (header, command) = match request {
			Wire::GetDiskIncarnationRequest => return Self::GetDiskIncarnation,
			Wire::ClaimRequest(r) => (
				r.header,
				LoadedVolumeReplicaCommand::Claim { holder: r.holder },
			),
			Wire::AppendRequest(r) => (
				r.header,
				LoadedVolumeReplicaCommand::Append {
					previous: r.previous,
					entries: r.entries,
					committed_volume_seq: r.committed_volume_seq,
				},
			),
			Wire::TruncateRequest(r) => (
				r.header,
				LoadedVolumeReplicaCommand::Truncate {
					expected_replica_incarnation: r.expected_replica_incarnation,
					expected_appended: r.expected_appended,
					keep_through_volume_seq: r.keep_through_volume_seq,
					mark_repairing: r.mark_repairing,
				},
			),
			Wire::AdvanceCommittedRequest(r) => (
				protocol::ReplicaRequestHeader::ClientRequestHeader(r.header),
				LoadedVolumeReplicaCommand::AdvanceCommitted {
					committed_volume_seq: r.committed_volume_seq,
				},
			),
			Wire::MarkInSyncRequest(r) => (
				r.header,
				LoadedVolumeReplicaCommand::MarkInSync {
					replica_incarnation: r.replica_incarnation,
					expected_appended: r.expected_appended,
				},
			),
			Wire::LoadVolumeRequest(r) => (
				r.header,
				LoadedVolumeReplicaCommand::LoadVolume {
					open_claim: r.open_claim,
					learner_replica_set_version: r.learner_replica_set_version,
					disk_incarnation: r.disk_incarnation,
					claimed_client: r.claimed_client,
					claimed_holder: r.claimed_holder,
					cold: r.cold,
					status: r.status,
					initial_entries: r.initial_entries,
					creation: r.creation,
				},
			),
			Wire::MarkVolumeDeletedRequest(r) => {
				(r.header, LoadedVolumeReplicaCommand::MarkVolumeDeleted)
			}
			Wire::TrimRequest(r) => (
				r.header,
				LoadedVolumeReplicaCommand::Trim {
					cold: r.cold,
					claimed_client: r.claimed_client,
				},
			),
			Wire::ForgetVolumeRequest(r) => (
				r.header,
				LoadedVolumeReplicaCommand::Forget {
					observation: r.observation,
					expected_appended: r.expected_appended,
					expected_claim: r.expected_claim,
				},
			),
			Wire::ReadVolumeLogEntriesRequest(r) => {
				return Self::Replica {
					volume_log_key: r.volume_log_key.into(),
					authority: None,
					command: LoadedVolumeReplicaCommand::Read {
						first_volume_seq: r.first_volume_seq,
						max_bytes: r.max_bytes,
						include_deleted: r.include_deleted,
					},
				};
			}
			Wire::ListTabletReplicasRequest(r) => return Self::ListTabletReplicas(r),
			Wire::ReadVolumeLogStatusRequest(r) => {
				return Self::ReadStatus {
					volume_log_keys: r.volume_log_keys.into_iter().map(Into::into).collect(),
				};
			}
			Wire::TrimVolumesRequest(r) => {
				return Self::TrimVolumes {
					grant: r.tablet_grant,
					volumes: r
						.volumes
						.into_iter()
						.map(|volume| {
							(
								volume.volume_log_key.into(),
								volume.cold,
								volume.claimed_client,
							)
						})
						.collect(),
				};
			}
			Wire::ForgetVolumesRequest(r) => {
				return Self::ForgetVolumes {
					grant: r.tablet_grant,
					volumes: r
						.volumes
						.into_iter()
						.map(|volume| {
							(
								volume.volume_log_key.into(),
								volume.expected_appended,
								volume.expected_claim,
							)
						})
						.collect(),
				};
			}
		};
		let (volume_log_key, authority) = Authority::from_header(header);
		Self::Replica {
			volume_log_key,
			authority: Some(authority),
			command,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogShardConfig {
	pub max_memory_requests: u32,
	pub inbox_capacity: usize,
	pub max_memory_bytes: u64,
	pub max_batch_bytes: u32,
	/// Maximum framed bytes handed to the executor in one write and sync.
	pub max_write_bytes: u64,
	pub max_disk_bytes: u64,
	/// Early flush watermark; hard disk admission remains max_disk_bytes.
	pub write_to_cold_at_disk_percent: u8,
	/// Replicas the shard holds loaded at once; a load past it is refused with
	/// `TooManyLoadedVolumes`. The default is [`DEFAULT_LOADED_REPLICA_BUDGET_BYTES`] at
	/// [`LOADED_REPLICA_BYTES`] each. Entries above cold are bounded separately by
	/// `max_memory_bytes` and the retained payload budgets.
	pub max_loaded_volumes: u32,
	pub write_to_disk_timeout_ms: u64,
	/// Fails the shard when its startup scan makes no progress for this long. The scan reports
	/// each segment it syncs, reads or reads entries back from, so a long retained log does
	/// not count against it, only a disk that stops answering. Repair marking, which rewrites
	/// only replica states and quarantines a damaged suffix, gets this long in total.
	pub recovery_timeout_ms: u64,
	/// Bounds a client waiting for the supervisor to publish a newer replica set.
	pub manifest_refresh_timeout_ms: u64,
	pub checkpoint_timeout_ms: u64,
	pub remove_segments_timeout_ms: u64,
	pub segment_bytes: u64,
	/// Writes each new segment into a spare file zero-filled to `segment_bytes` in the
	/// background, so a write inside it overwrites allocated blocks and syncs only its data.
	/// The spare starts with a segment header that every write updates. Disk admission
	/// charges the active segment's unused allocation and one spare. Off by default so tests
	/// with tiny segments keep exact file lengths; the storage service turns it on unless
	/// `storage.log_shard.preallocate_segments` is false.
	pub preallocate_segments: bool,
	pub checkpoint_bytes: u64,
	/// Publishes every loaded replica's state in the runner snapshot. Each step then costs
	/// O(loaded replicas), which dominates a busy shard. The default keeps tests and embedded
	/// owners fully observable; the storage service turns it off, and
	/// `LogShardMachine::loaded_volume` answers single-replica queries cheaply.
	pub publish_replica_views: bool,
	/// Most volumes one `ForgetVolumesRequest` or `TrimVolumesRequest` may name. A larger batch is rejected as
	/// `RequestTooLarge`, so maintainers must not batch more than this.
	pub max_forget_batch_volumes: u32,
	/// Encoded bytes of retained entries whose payloads the shard may keep in memory after
	/// they sync. Above it, the least recently appended replicas release their synced
	/// payloads first. Unsynced payloads are already bounded by `max_memory_bytes`.
	pub max_retained_payload_bytes: u64,
	/// Encoded bytes of synced payloads one replica keeps after each disk write; older
	/// payloads are released first.
	pub max_retained_payload_bytes_per_volume: u64,
	/// How long a replica appends nothing before it releases every synced payload.
	pub release_idle_payloads_after_ms: u64,
	/// Most reads dispatched to disk and not yet answered. A read that needs disk beyond it
	/// is rejected with `Overloaded` instead of queueing behind the others.
	pub max_disk_reads: u32,
	/// Disk reads executing at once, beside the single write queue. Each running read holds
	/// one scan buffer, which `max_memory_bytes` reserves.
	pub disk_read_concurrency: u32,
	/// How long a replica whose synced batches only appended entries waits for a state
	/// change to notice it before it is noticed alone, so a burst of appends shares one
	/// notice. The maintainer derives commits from the appended positions the notices carry,
	/// so this bounds how long a write waits to be applied. Zero notices appends at once.
	pub append_notice_delay_ms: u64,
	/// Most entry bytes one replica's status notice carries. The notice of the tablet's entry
	/// carrier carries the entries the replica synced since its previous notice, so its maintainer can apply a small write
	/// without reading the replica; a larger range carries none and is read. Zero carries no
	/// entries.
	pub max_notice_entry_bytes: u32,
	/// Most entry bytes one notice request to a maintainer worker carries across all its
	/// replicas. Replicas past it in the request carry no entries.
	pub max_notice_request_entry_bytes: u64,
	/// How long a notice request may stay unanswered before the shard counts it as timed
	/// out and resends its replicas after a backoff. The transport's own timeout normally
	/// answers first; this deadline frees the worker's notice slot even if no answer ever
	/// comes back. Keep it above the transport timeout (`volume_request_timeout_ms`, 5 s by
	/// default); below it, notices still arrive but every slow one is resent.
	pub notice_timeout_ms: u64,
	/// Longest time a submitted request other than an append may wait in the inbox before the
	/// core sees it. A request that waited longer is refused `Overloaded` before any work, so a
	/// busy runner sheds load instead of letting queueing pass the volume client's quorum
	/// timeout.
	pub max_admission_wait_ms: u64,
	/// The same bound for appends, commit announcements, trims and forgets, which carry writes
	/// a volume client already admitted or free memory and disk, so new work is shed before
	/// them. Refusing an admitted write's append only delays its commit, so this bound sits
	/// just under the volume client's 2 s append timeout: an append that waited longer has
	/// been resent, and serving it would be duplicate work.
	pub max_append_admission_wait_ms: u64,
	/// Retry hint on `Overloaded` refusals from a full inbox or budget. Queue-wait refusals
	/// hint the observed wait instead, at least this and at most `MAX_RETRY_AFTER_MS`.
	pub overload_retry_after_ms: u64,
	/// The family versions this worker advertises in its topology heartbeat. Admission refuses
	/// a payload above its family's advertised `maxRead`, and recovery refuses FORMAT marks
	/// above it, so the shard never stores what it told writers it cannot read. The storage
	/// service passes its registration's versions, which a deployment compiles in.
	pub read_versions: Vec<rivet_data::generated::topology_worker_heartbeat_v1::FamilyVersions>,
	/// Simulation only: runner time each admitted or refused request costs, spent on the
	/// simulated clock so a saturated runner queues as a busy core does. Zero in deployments,
	/// where real processing time passes on its own.
	pub simulated_request_cost_us: u64,
}

/// Heap charged per loaded replica when a memory budget sizes `max_loaded_volumes`. A quiet
/// loaded replica measures about 1,160 bytes in the `volume_memory` test, including its place
/// in hash order for listings.
pub const LOADED_REPLICA_BYTES: u64 = 1536;

/// Memory the default `max_loaded_volumes` lets one shard's loaded replicas hold: 1,048,576
/// replicas, enough for a worker to hold one replica of each of about a million volumes.
pub const DEFAULT_LOADED_REPLICA_BUDGET_BYTES: u64 = 1536 * 1024 * 1024;

impl Default for LogShardConfig {
	fn default() -> Self {
		Self {
			max_memory_requests: 1024,
			inbox_capacity: 256,
			// Two full writes can overlap while bounded range reads still have room.
			max_memory_bytes: 64 * 1024 * 1024,
			max_batch_bytes: depot_types::log_shard::DEFAULT_MAX_BATCH_BYTES,
			max_write_bytes: 16 * 1024 * 1024,
			max_disk_bytes: 1024 * 1024 * 1024,
			write_to_cold_at_disk_percent: 75,
			max_loaded_volumes: (DEFAULT_LOADED_REPLICA_BUDGET_BYTES / LOADED_REPLICA_BYTES) as u32,
			write_to_disk_timeout_ms: 30_000,
			recovery_timeout_ms: 120_000,
			manifest_refresh_timeout_ms: 5_000,
			checkpoint_timeout_ms: 30_000,
			remove_segments_timeout_ms: 30_000,
			segment_bytes: 64 * 1024 * 1024,
			preallocate_segments: false,
			checkpoint_bytes: 4 * 1024 * 1024,
			publish_replica_views: true,
			max_forget_batch_volumes: depot_types::log_shard::DEFAULT_MAX_GRANT_BATCH_VOLUMES,
			max_retained_payload_bytes: 16 * 1024 * 1024,
			max_retained_payload_bytes_per_volume: 32 * 1024,
			release_idle_payloads_after_ms: 5_000,
			max_disk_reads: 256,
			disk_read_concurrency: 8,
			append_notice_delay_ms: 100,
			max_notice_entry_bytes: 64 * 1024,
			max_notice_request_entry_bytes: 1024 * 1024,
			notice_timeout_ms: 10_000,
			max_admission_wait_ms: 1_000,
			max_append_admission_wait_ms: 1_800,
			overload_retry_after_ms: 100,
			read_versions: rivet_data::families::compiled_versions(),
			simulated_request_cost_us: 0,
		}
	}
}

/// The longest retry hint an `Overloaded` refusal carries. Callers keep their own deadlines,
/// so a longer hint would only turn a refusal into a missed deadline.
pub const MAX_RETRY_AFTER_MS: u64 = 1_000;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogShardTimer {
	/// **When**
	///
	/// A client request names a replica-set version ahead of the local manifest.
	///
	/// **State**
	///
	/// Removes only that retained request and releases its byte and request reservations.
	///
	/// **Effects**
	///
	/// Replies NotServing without returning an older replica set to the client.
	ManifestRefresh(RequestId),

	/// **When**
	///
	/// Armed when disk work is dispatched, including its wait for a reader, and cancelled
	/// by its `ReadResponse`. The executor also observes the original caller deadline.
	///
	/// **State**
	///
	/// Removes the read from `reads`, releasing its request and byte reservations. A read
	/// that already completed is gone and the timer is ignored. The shard keeps serving: a
	/// slow read is not evidence of a broken disk, and writes keep their own fail-stop timers.
	///
	/// **Effects**
	///
	/// Replies `Overloaded`, which callers retry. A late `ReadResponse` finds no read and
	/// emits nothing.
	Read(RequestId),

	/// Bounds the startup scan while the shard is recovering.
	Recover,

	/// Bounds replacement writes and quarantine after a corrupt recovery outcome.
	MarkRepairing,

	/// Fails only the outstanding write ending at this batch sequence.
	WriteToDisk { last_log_seq: u64 },

	/// Bounds checkpoint replacement at the selected disk boundary.
	WriteCheckpoint { last_included_log_seq: u64 },

	/// Bounds the single outstanding segment-removal operation.
	RemoveSegments,

	/// **When**
	///
	/// The replica that appended least recently has been quiet for
	/// `release_idle_payloads_after_ms`. It is armed after every event at that time and
	/// cancelled when no replica holds payloads.
	///
	/// **State**
	///
	/// Releases every synced payload of each replica quiet that long, keeping their entry
	/// headers. A quiet replica that still holds unsynced payloads is checked again one idle
	/// period later.
	///
	/// **Effects**
	///
	/// None; releasing is a pure state change, and later reads of released entries use
	/// `ReadRequest`.
	ReleaseIdlePayloads,

	/// **When**
	///
	/// The earliest replica whose synced batches only appended entries has waited
	/// `append_notice_delay_ms` without a state change noticing it. It is armed after every
	/// event at that time and cancelled when no such replica remains.
	///
	/// **State**
	///
	/// Marks every due replica for a status notice to its maintainer.
	///
	/// **Effects**
	///
	/// The following dispatch can emit `NotifyMaintainerRequest`.
	AppendNotices,

	/// **When**
	///
	/// The earliest backoff after a failed notice to a maintainer worker has passed. It is
	/// armed after every event at that time and cancelled when no worker backs off.
	///
	/// **State**
	///
	/// Ends the backoff of every worker due by now, keeping its failure count until a notice
	/// to it succeeds.
	///
	/// **Effects**
	///
	/// The following dispatch can resend the worker's marked replicas with
	/// `NotifyMaintainerRequest`.
	NoticeRetry,

	/// **When**
	///
	/// Armed when a notice request to this maintainer worker is sent, and cancelled by the
	/// `NotifyMaintainerResponse` whose `notice_id` matches it. At most one is armed per
	/// maintainer worker, because at most one notice is in flight to each.
	///
	/// **State**
	///
	/// Ends the in-flight notice as `Timeout`: its replicas are marked again and the worker
	/// backs off. A response that arrives later names an older notice and is ignored.
	///
	/// **Effects**
	///
	/// Arms `NoticeRetry` for the backoff.
	NoticeDeadline(rivet_topology::worker::WorkerId),
}

/// Why a read could not be answered from retained entries and reads disk instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiskReadReason {
	/// The page starts below the first retained entry, or the replica retains none.
	BelowFront,
	/// An accepted but unsynced truncate shortened memory below the requested start.
	Truncated,
	/// The first requested entry is not on disk in memory's form yet.
	Unsynced,
	/// The first requested entry's payload was released from memory.
	Released,
}

impl DiskReadReason {
	pub fn label(self) -> &'static str {
		match self {
			Self::BelowFront => "below_front",
			Self::Truncated => "truncated",
			Self::Unsynced => "unsynced",
			Self::Released => "released",
		}
	}
}

/// Why the disk executor could not return a page for a read.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiskReadError {
	/// Opening, seeking or reading the segment failed, or the read outlived its deadline.
	/// The synced history is intact, so only this read fails and the caller retries. A
	/// segment removed after the read was planned also lands here: its entries were already
	/// below cold, so the retry is answered `BelowCold`.
	Unavailable(String),
	/// The bytes at an indexed location fail their checksum or hold another batch or entry.
	/// The shard's history is no longer trustworthy, so the shard fails.
	Corrupt(String),
}

/// Returned after every batch in the write and any new directory entry have synced.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WrittenToDisk {
	pub segment_first_log_seq: u64,
	pub segment_len_bytes: u64,
}

/// An I/O failure invalidates the executor and ends serving.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WriteToDiskError {
	Write(String),
	Sync(String),
	SegmentRoll(String),
}

impl std::fmt::Display for WriteToDiskError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

impl std::error::Error for WriteToDiskError {}

/// The supervisor's current tablet replica sets, replaced as one manifest view.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabletManifestView {
	pub tablet_manifest_version: u64,
	pub tablets: std::collections::BTreeMap<[u8; 16], TabletManifestEntry>,
	/// Split parents the manifest records as permanently retired. A shard raises each one it
	/// holds retirement fences for to the retired grant, as a child's listing would, so a
	/// shard outside every child's replica set still releases them.
	pub retired_tablets: std::collections::BTreeSet<[u8; 16]>,
}

/// How one replica status notice ended. Every outcome but `Sent` resends replicas of the
/// notice after a backoff.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoticeOutcome {
	/// The maintainer worker queued the notices of every tablet except `refused_tablets`,
	/// whose maintainers' inboxes were full, and `unknown_tablets`, which it does not run.
	/// Only those tablets' replicas are resent; an unknown tablet also refreshes the shard's
	/// routing view, since its maintainer has probably moved.
	Sent {
		refused_tablets: Vec<[u8; 16]>,
		unknown_tablets: Vec<[u8; 16]>,
	},
	/// The request could not be sent, or no notifier is installed yet.
	Failed,
	/// No reply arrived in time. The maintainer may have applied the notices; resending
	/// the newest states is harmless.
	Timeout,
	/// The maintainer worker runs none of the tablets the notice names, so this shard's
	/// routing view is stale. It also refreshes the view.
	NotMaintainer,
	/// The maintainer worker answered with another refusal, or the reply could not be
	/// decoded.
	Refused,
}

impl NoticeOutcome {
	pub fn as_str(&self) -> &'static str {
		match self {
			NoticeOutcome::Sent {
				refused_tablets,
				unknown_tablets,
			} if refused_tablets.is_empty() && unknown_tablets.is_empty() => "sent",
			NoticeOutcome::Sent { .. } => "partly_refused",
			NoticeOutcome::Failed => "failed",
			NoticeOutcome::Timeout => "timeout",
			NoticeOutcome::NotMaintainer => "not_maintainer",
			NoticeOutcome::Refused => "refused",
		}
	}
}

/// Why a create's initial entries were not written with it. A metric label only; the opener
/// writes dropped entries itself after its claim entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InitialEntriesDrop {
	/// The replica already existed, so the load created nothing.
	Existing,
	/// The new replica could not be claimed for the opener.
	NotClaimed,
	/// The entries would not fit the create's write or the shard's budgets.
	TooLarge,
	/// The entries did not extend the new replica under the opener's claim.
	Malformed,
}

impl InitialEntriesDrop {
	pub fn as_str(self) -> &'static str {
		match self {
			InitialEntriesDrop::Existing => "existing",
			InitialEntriesDrop::NotClaimed => "not_claimed",
			InitialEntriesDrop::TooLarge => "too_large",
			InitialEntriesDrop::Malformed => "malformed",
		}
	}
}

/// Manifest information supplied by the supervisor for one tablet's client replica-set checks.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabletManifestEntry {
	/// The worker granted to maintain this tablet, which receives its replica status
	/// notices. `None` while the tablet has no maintainer.
	pub maintainer: Option<rivet_topology::worker::WorkerId>,
	pub disks: Vec<rivet_data::generated::shared_depot_types_v1::StorageWorkerDisk>,
	pub tablet_id: [u8; 16],
	pub hash_range: rivet_data::generated::shared_depot_types_v1::PartitionKeyHashRange,
	pub members: Vec<rivet_topology::worker::WorkerId>,
	pub tablet_replica_set_version: u64,
	/// The rejection returns the current phase so a client can also route to learners.
	pub phase: rivet_data::generated::shared_depot_types_v1::TabletPhase,
}
