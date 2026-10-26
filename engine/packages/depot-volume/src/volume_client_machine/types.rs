//! Request identities, payload references and volume client bounds.

use depot_types::payload_codec::PayloadDigest;
use rivet_data::generated::shared_depot_types_v1::VolumeLogPosition;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);

/// Local admission history; a self-reopen can append a claim entry at the same epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HistoryIdentity {
	pub client_epoch: u64,
	pub claim_entry_volume_seq: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpId(pub u64);

/// Executor-owned bytes. A handle stays alive while retained history refers to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PayloadId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryRef {
	/// Immutable scheduling timestamp from the admitted write or fetched log entry.
	pub written_at_unix_ms: i64,

	pub id: VolumeLogPosition,
	pub mutation: Option<MutationIdentity>,
	pub payload: PayloadId,

	/// Encoded entry size, measured at the payload boundary before admission.
	pub encoded_bytes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VolumeClientConfig {
	/// Time allowed for remaining members after a recovery-only claim entry commits.
	pub finish_recovery_timeout_ms: u64,
	pub max_pending_writes: u32,
	pub max_memory_bytes: u64,
	pub max_append_bytes: u32,
	pub max_in_flight: u32,
	pub append_timeout_ms: u64,
	pub claim_timeout_ms: u64,
	pub claim_attempts: u32,
	pub quorum_timeout_ms: u64,
	/// Time an open caller waits for recovery to finish. When the last waiter gives up, the
	/// client abandons recovery and the next open starts again, so this must cover a recovery
	/// under load, not just one quorum round trip. Defaults to 3 seconds.
	pub open_timeout_ms: u64,
	pub stall_timeout_ms: u64,
	/// The quiet gap between two bursts of writes above which the client frees the working
	/// maps a burst allocated once the burst commits. A client that writes again within this
	/// window keeps them, so a steady writer never reallocates them; a quiet client holds
	/// no timer for it, the next write's gap decides. Defaults to 60 seconds.
	pub compact_after_quiet_ms: u64,
	/// Encoded bytes of initial writes a creating open may carry. It must leave room for the
	/// creation's own frames in one log shard write. Larger initial writes are not sent; the
	/// volume is created empty and the owner writes them as usual. Defaults to 256 KiB.
	pub max_initial_bytes: u64,
	/// Codec for mutation payloads before they are logged. Each entry records its codec, so a
	/// change applies to new writes and every older entry stays readable. Defaults to LZ4.
	pub payload_codec: rivet_data::generated::depot_log_shard_v1::PayloadCodec,
}

impl Default for VolumeClientConfig {
	fn default() -> Self {
		Self {
			finish_recovery_timeout_ms: 10_000,
			max_pending_writes: 1024,
			max_memory_bytes: 64 * 1024 * 1024,
			max_append_bytes: 4 * 1024 * 1024,
			max_in_flight: 16,
			append_timeout_ms: 2000,
			claim_timeout_ms: 2000,
			claim_attempts: 5,
			quorum_timeout_ms: 3000,
			open_timeout_ms: 3000,
			stall_timeout_ms: 30_000,
			compact_after_quiet_ms: 60_000,
			max_initial_bytes: 256 * 1024,
			payload_codec: rivet_data::generated::depot_log_shard_v1::PayloadCodec::Lz4,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WriteOutcome {
	pub volume_seq: u64,
	/// The mutation's position, or a cold floor covering a deduplicated mutation.
	/// This is durable when `committed` is present or its history-bound barrier succeeds.
	pub position: VolumeLogPosition,
	pub token: WriteToken,
	pub applied_earlier: bool,
	pub committed: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WriteError {
	/// The original caller deadline expired. Without a token nothing was admitted. With one, the
	/// entry was admitted and the same client keeps replicating it, so it may still commit: a
	/// retry with that token joins the entry and reports it committed, never appends it twice.
	DeadlineExceeded {
		token: Option<WriteToken>,
	},
	VolumeNotFound,
	RoutingUnavailable,
	ColdStoreUnavailable,
	LifecycleRejected(rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse),
	RecoveryFailed,
	QuorumUnavailable {
		token: Option<WriteToken>,
	},
	/// The write was refused at admission, before taking a position: this client's own
	/// budgets are full, a quorum of shards is refusing, or the maintainer refused under load.
	/// Without a token nothing was admitted, so the caller may send it again after
	/// `retry_after_ms`. With one, the entry was admitted but a quorum of shards refuses it for
	/// a full disk; the client keeps the entry and resends it, so it may still commit, and a
	/// retry with the token after `retry_after_ms` joins it rather than appending it twice.
	Overloaded {
		retry_after_ms: u64,
		token: Option<WriteToken>,
	},
	/// The maintainer refused the open, or it found no route or no answer. The client does
	/// not resend it: nothing was admitted, so the caller may open again after
	/// `retry_after_ms`, the refusal's own hint with jitter.
	Refused {
		reason: OpenRefusal,
		retry_after_ms: u64,
	},
	Fenced {
		token: Option<WriteToken>,
	},
	RetryConflict,
	RetryIndeterminate {
		token: WriteToken,
	},
	/// The write's encoded entry is larger than the log shards accept in one append, so it
	/// was not admitted and nothing was sent. The limit is what a quorum of shards accepts;
	/// the caller must split the write or an operator must raise the shards' write limit.
	EntryTooLarge {
		max_bytes: u32,
	},
	/// An admitted entry at `volume_seq` cannot be appended to a quorum on this handle: every
	/// shard that refused it did so for a reason that does not change with time or retries.
	/// With a token, the caller's write took a position at or behind that entry and its
	/// outcome is unknown: the handle keeps the entry, fences nothing and never resends it,
	/// but a later open recovers it if a replica holds it and the shards then accept it, and
	/// discards it otherwise. Without a token, the write took no position because it would
	/// have sat behind the blocked entry. The owner opens a new client to move on.
	HistoryBlocked {
		volume_seq: u64,
		refusal: PermanentRefusal,
		token: Option<WriteToken>,
	},
}

/// Why a log shard refuses an entry's bytes however often they are sent. Unlike an overload
/// or full-disk refusal, waiting changes nothing, so the client never retries one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermanentRefusal {
	/// The entry alone is larger than the shard accepts in one append.
	EntryTooLarge { max_bytes: u32 },
}

impl PermanentRefusal {
	/// Bounded metric and log label.
	pub fn label(&self) -> &'static str {
		match self {
			Self::EntryTooLarge { .. } => "entry_too_large",
		}
	}
}

/// Why a refused open did not reach the volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpenRefusal {
	/// The answering worker does not hold the tablet's maintainer grant, as while the grant
	/// moves to a new maintainer.
	NotMaintainer,
	/// No maintainer route is known for the volume's tablet.
	NoRoute,
	/// The maintainer refused the open under load.
	Overloaded,
	/// The volume's tablet is splitting.
	TabletSplitting,
	/// The maintainer did not answer, or its transport failed.
	Unreachable,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum VolumeClientTimer {
	/// Expires a caller still waiting for recovery without extending other callers.
	Waiting(RequestId),
	/// Bounds an eager open with no registered application caller.
	OpenIdle,
	/// Ends a recovery-only repair window without reporting unreachable members as repaired.
	FinishRecovery,

	/// Ends one caller's wait without cancelling replication of its entry.
	Quorum(RequestId),

	/// Re-drives uncommitted history that has made no quorum progress for the stall timeout.
	/// A serving client keeps its epoch and its entries; only a recovery-only client ends.
	/// History a quorum permanently refuses is not stalled: once everything below the
	/// blocked entry is committed the alarm is retired instead of resending.
	Stall,

	/// Bounds a correlated lifecycle, claim, replay, or repair operation.
	Control(OpId),

	/// Retires an outstanding append so its replica can retry from acknowledged history.
	Append(OpId),

	/// Ends a replica's `Overloaded` or `DiskFull` refusal delay; the replica then
	/// resends from its acknowledged position.
	Resume(OpId),

	/// Ends the wait after a replay read was refused; the page is read again without
	/// counting an attempt against the source.
	ReplayResume(OpId),
}

/// A retry is identified independently of its reusable sequence position.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WriteToken {
	pub payload_digest: PayloadDigest,
	pub volume_incarnation: [u8; 16],
	/// Retry identity, not ordering: matching log positions orders entries.
	pub client_seq: u64,
	pub request_id: [u8; 16],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutationIdentity {
	/// Retry identity, not ordering: matching log positions orders entries.
	pub client_seq: u64,
	pub request_id: [u8; 16],
	pub payload_digest: PayloadDigest,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DedupeEntry {
	pub identity: MutationIdentity,
	pub volume_seq: u64,
}

/// How many recent mutations a client remembers for answering retried requests.
pub const DEDUPE_WINDOW: usize = 64;

/// The encoded size of every entry above cold that the client knows, one per consecutive
/// position from `first_seq`, with their total. Sizes stay after committed payloads leave RAM
/// and go when cold passes them, so the total is the client's log above cold. A recovery
/// that starts at the maintainer's applied position knows only the sizes of entries from
/// there. Positions are dense, so the window costs four bytes per entry and updating it
/// costs one push per entry and one pop per entry that goes cold. An empty window is always
/// the default value, whichever way it emptied.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct AboveCold {
	first_seq: u64,
	sizes: VecDeque<u32>,
	bytes: u64,
}

impl AboveCold {
	pub(crate) fn bytes(&self) -> u64 {
		self.bytes
	}

	

	/// The position one past the last recorded entry, or `None` for an empty window.
	pub(crate) fn end_seq(&self) -> Option<u64> {
		if self.sizes.is_empty() {
			None
		} else {
			Some(self.first_seq + self.sizes.len() as u64)
		}
	}

	/// Records the size of the entry at `seq`, replacing the size recorded there before. An
	/// open retains its claim entry on the recovered tail before the replay reads the history
	/// beneath it, so entries below the window extend it downwards; the positions between
	/// count as unknown until the replay records them. Appends extend it upwards by one.
	/// An entry further above the end can only come from a bug; the window then restarts at
	/// it and undercounts rather than growing with the size of the gap.
	pub(crate) fn record(&mut self, seq: u64, encoded_bytes: u32) {
		match self.end_seq() {
			Some(end) if seq < self.first_seq => {
				let unknown = self.first_seq - seq - 1;
				for _ in 0..unknown {
					self.sizes.push_front(0);
				}
				self.sizes.push_front(encoded_bytes);
				self.first_seq = seq;
				debug_assert_eq!(self.end_seq(), Some(end));
			}
			Some(end) if seq < end => {
				let index = (seq - self.first_seq) as usize;
				let previous = std::mem::replace(&mut self.sizes[index], encoded_bytes);
				self.bytes -= u64::from(previous);
			}
			Some(end) if seq == end => self.sizes.push_back(encoded_bytes),
			Some(_) | None => {
				self.clear();
				self.first_seq = seq;
				self.sizes.push_back(encoded_bytes);
			}
		}
		self.bytes += u64::from(encoded_bytes);
	}

	/// Forgets the entries at and below `cold_seq`.
	pub(crate) fn trim_through(&mut self, cold_seq: u64) {
		while self.first_seq <= cold_seq {
			let Some(size) = self.sizes.pop_front() else {
				break;
			};
			self.bytes -= u64::from(size);
			self.first_seq += 1;
		}
		if self.sizes.is_empty() {
			self.clear();
		}
	}

	pub(crate) fn clear(&mut self) {
		self.bytes = 0;
		self.first_seq = 0;
		if self.sizes.is_empty() {
			self.sizes = VecDeque::new();
		} else {
			self.sizes.clear();
		}
	}

	/// Releases the capacity a burst left behind once the client is quiet: a window that cold
	/// has mostly trimmed keeps only what it still holds, and an empty one holds nothing; see
	/// `VolumeClientState::free_quiet_containers`.
	pub(crate) fn shrink_to_fit(&mut self) {
		if self.sizes.is_empty() {
			self.sizes = VecDeque::new();
		} else {
			self.sizes.shrink_to_fit();
		}
	}
}

/// The recent mutations a client answers retries from, sorted by request id. The request id
/// lives once, inside each identity, and the rows sit in one exact-size vector instead of a
/// map whose nodes would repeat the key and leave slack.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DedupeWindow(Vec<DedupeEntry>);

impl DedupeWindow {
	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	fn position(&self, request_id: &[u8; 16]) -> Result<usize, usize> {
		self.0
			.binary_search_by(|entry| entry.identity.request_id.cmp(request_id))
	}

	pub fn get(&self, request_id: &[u8; 16]) -> Option<&DedupeEntry> {
		self.position(request_id).ok().map(|index| &self.0[index])
	}

	pub fn contains_key(&self, request_id: &[u8; 16]) -> bool {
		self.position(request_id).is_ok()
	}

	pub fn values(&self) -> std::slice::Iter<'_, DedupeEntry> {
		self.0.iter()
	}

	pub fn clear(&mut self) {
		self.0 = Vec::new();
	}

	/// Remembers one mutation, replacing an entry with the same request id, then forgets
	/// the lowest client seq while the window is over [`DEDUPE_WINDOW`].
	pub fn remember(&mut self, entry: DedupeEntry) {
		match self.position(&entry.identity.request_id) {
			Ok(index) => self.0[index] = entry,
			Err(index) => self.0.insert(index, entry),
		}
		while self.0.len() > DEDUPE_WINDOW {
			let oldest = self
				.0
				.iter()
				.enumerate()
				.min_by_key(|(_, entry)| entry.identity.client_seq)
				.map(|(index, _)| index);
			match oldest {
				Some(index) => {
					self.0.remove(index);
				}
				None => break,
			}
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorDisposition {
	NotExecuted,
	OutcomeUnknown,
	Permanent,
}

/// How long a caller waits before resending a write this client refused because its own
/// pending-write or memory budget is full. Those budgets free as in-flight appends commit,
/// which takes about one quorum round trip.
pub const CLIENT_OVERLOAD_RETRY_AFTER_MS: u64 = 100;

impl WriteError {
	/// The write this client refused because its own budgets are full.
	pub fn client_overloaded() -> Self {
		Self::Overloaded {
			retry_after_ms: CLIENT_OVERLOAD_RETRY_AFTER_MS,
			token: None,
		}
	}

	/// The hint of a refusal that admitted nothing, which the edge waits before sending the
	/// request again. Other errors carry none.
	pub fn retry_after_ms(&self) -> Option<u64> {
		match self {
			Self::Overloaded {
				retry_after_ms,
				token: None,
			}
			| Self::Refused { retry_after_ms, .. } => Some(*retry_after_ms),
			// The entry was admitted and may still commit, so resending it as a new request
			// could apply it twice; only a retry with the token may follow the hint.
			Self::Overloaded { token: Some(_), .. } => None,
			Self::LifecycleRejected(reject) => depot_types::client::retry_after_ms(reject),
			Self::DeadlineExceeded { .. }
			| Self::VolumeNotFound
			| Self::RoutingUnavailable
			| Self::ColdStoreUnavailable
			| Self::RecoveryFailed
			| Self::QuorumUnavailable { .. }
			| Self::Fenced { .. }
			| Self::RetryConflict
			| Self::RetryIndeterminate { .. }
			| Self::EntryTooLarge { .. }
			| Self::HistoryBlocked { .. } => None,
		}
	}

	/// Distinguishes safe resubmission from outcomes that require the original retry identity.
	pub fn disposition(&self) -> ErrorDisposition {
		use rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse;

		match self {
			Self::DeadlineExceeded { token: None } => ErrorDisposition::NotExecuted,
			Self::DeadlineExceeded { token: Some(_) } => ErrorDisposition::OutcomeUnknown,
			// Nothing was dispatched: the write was refused before taking a position.
			Self::Overloaded { token: None, .. }
			| Self::RoutingUnavailable
			| Self::Refused { .. }
			| Self::EntryTooLarge { .. }
			| Self::HistoryBlocked { token: None, .. } => ErrorDisposition::NotExecuted,
			// The entry took a position; a later open may still recover and commit it.
			Self::HistoryBlocked { token: Some(_), .. }
			| Self::Overloaded { token: Some(_), .. } => ErrorDisposition::OutcomeUnknown,
			// A lifecycle rejection answers the open, before any write is dispatched.
			Self::LifecycleRejected(reason) => match reason {
				VolumeRejectResponse::VolumeNotFound
				| VolumeRejectResponse::VolumeExists
				| VolumeRejectResponse::VolumeDeleted
				| VolumeRejectResponse::VolumeRecordMissing(_) => ErrorDisposition::Permanent,
				VolumeRejectResponse::NotMaintainer(_)
				| VolumeRejectResponse::TabletSplitting
				| VolumeRejectResponse::Overloaded(_)
				| VolumeRejectResponse::VolumeUnderRepair(_) => ErrorDisposition::NotExecuted,
			},
			Self::ColdStoreUnavailable
			| Self::QuorumUnavailable { .. }
			| Self::Fenced { .. }
			| Self::RetryIndeterminate { .. } => ErrorDisposition::OutcomeUnknown,
			Self::VolumeNotFound | Self::RecoveryFailed | Self::RetryConflict => {
				ErrorDisposition::Permanent
			}
		}
	}
}

/// Recovery-only handles commit a claim entry and repair replicas without admitting application writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpenPurpose {
	Serve,
	Recover,
}

/// Members confirmed at the recovered tail before close; incomplete members need another repair attempt.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecoveryReport {
	pub through: VolumeLogPosition,
	pub in_sync: Vec<rivet_topology::worker::WorkerId>,
	pub incomplete: Vec<rivet_topology::worker::WorkerId>,
}


