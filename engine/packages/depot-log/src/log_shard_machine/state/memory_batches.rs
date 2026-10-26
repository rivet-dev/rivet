//! Accepted frames and ordered replies share one disk write barrier.

use super::disk_segments::{CheckpointCycle, DiskSegmentUsage};
use super::fail;
use super::{LogShardPhase, LogShardState};
use crate::log_shard_machine::{
	Effects,
	effect::LogShardEffect,
	types::{Authority, LogShardTimer, RequestId, WrittenToDisk},
};
use depot_types::{VolumeLogKey, log_shard::BATCH_FRAMING_BYTES};
use rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState;
use rivet_data::generated::shared_depot_types_v1::TabletReplicaSet;
use rivet_data::generated::{
	depot_log_shard_v1::{DiskBatch, ForgetFrame, Frame},
	depot_storage_worker_protocol_v1::{
		self as protocol, ForgetVolumeResult, ForgetVolumesResponse, RejectResponse,
		ReplicaStateResponse, StorageWorkerResponse, TrimVolumesResponse,
	},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use vbare::Encode;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryBatches {
	/// The batch being filled while one write is in flight. No batching timer delays it:
	/// an idle request dispatches immediately; under load it grows with sync latency.
	pub(crate) filling: MemoryBatch,
	pub(crate) writing_to_disk: Option<WritingToDisk>,
	pub last_log_seq: u64,
	/// When `filling` first held a frame it could not write at once because a write was in
	/// flight. Dispatch reports the difference to its write time as the batch's fill wait
	/// and clears it.
	pub(crate) filling_since_boot_ms: Option<u64>,
	/// A conservative disk write rate in bytes a second, measured from the most recent
	/// completed write. `None` until the first write completes.
	pub(crate) write_bytes_per_sec: Option<u64>,
}

/// How long a write must run before its bytes over its duration bound the disk's rate. A
/// write's bytes over its time never exceed the disk's rate, but for a short write the time
/// is mostly sync latency, so only a write this slow is evidence. A healthy disk writes even
/// a full 16 MiB write faster than this.
pub(crate) const SLOW_WRITE_EVIDENCE_MS: u64 = 100;

/// The largest filling batch the shard lets build up. Before the first completed write,
/// permit one framed batch rather than the full multi-batch write. Afterward, limit it to
/// what the disk writes within `max_append_admission_wait_ms`, using the lower of the
/// completed rate and the rate an in-flight write can still reach. The floor of one batch
/// lets a very slow disk continue writing but cannot protect one batch from a disk whose
/// throughput falls below that batch's write-timeout budget.
pub(crate) fn drain_limit_bytes(state: &LogShardState, now_boot_ms: u64) -> u64 {
	let one_batch = u64::from(state.config.max_batch_bytes)
		+ depot_types::log_shard::BATCH_FRAMING_BYTES as u64;
	let in_flight_rate = state
		.memory_batches
		.writing_to_disk
		.as_ref()
		.and_then(|writing| {
			let elapsed_ms = now_boot_ms.saturating_sub(writing.started_boot_ms);
			if elapsed_ms < SLOW_WRITE_EVIDENCE_MS {
				None
			} else {
				Some(writing.bytes.saturating_mul(1000) / elapsed_ms)
			}
		});
	let bytes_per_sec = match (state.memory_batches.write_bytes_per_sec, in_flight_rate) {
		(Some(measured), Some(in_flight)) => measured.min(in_flight),
		(Some(rate), None) => rate,
		(None, Some(_)) | (None, None) => return one_batch,
	};
	let limit = bytes_per_sec.saturating_mul(state.config.max_append_admission_wait_ms) / 1000;
	limit.max(one_batch)
}

impl MemoryBatches {
	/// Whether a frame-free reply about `volume_log_key` can be answered from synced disk
	/// state at once: no pending batch holds a frame for the volume, and none holds a tablet
	/// grant or replica set frame, which can change how any volume on the tablet is answered.
	/// Otherwise the reply waits for the write that syncs those frames, so no reply exposes
	/// unsynced state and a reply never overtakes an earlier write of its volume.
	pub(crate) fn synced_now(&self, volume_log_key: &VolumeLogKey) -> bool {
		self.pending().all(|batch| {
			batch.index.grant_epochs.is_empty()
				&& batch.index.replica_sets.is_empty()
				&& !batch.index.volume_frames.contains_key(volume_log_key)
		})
	}

	/// Includes filling and in-flight work when accounting for admission.
	pub(crate) fn pending(&self) -> impl Iterator<Item = &MemoryBatch> {
		std::iter::once(&self.filling).chain(self.writing_to_disk.iter().map(|b| &b.pending))
	}

	/// The highest grant epoch a pending frame installs for `tablet_id`.
	pub(crate) fn pending_grant_epoch(&self, tablet_id: &[u8; 16]) -> Option<u64> {
		self.pending()
			.filter_map(|batch| batch.index.grant_epochs.get(tablet_id).copied())
			.max()
	}

	/// The highest-version replica set a pending frame installs for `tablet_id`. Among
	/// equal versions the earliest pending frame wins, as a scan in pending order would.
	pub(crate) fn pending_replica_set(&self, tablet_id: &[u8; 16]) -> Option<&TabletReplicaSet> {
		let mut highest: Option<&TabletReplicaSet> = None;
		for set in self
			.pending()
			.filter_map(|batch| batch.index.replica_sets.get(tablet_id))
		{
			if highest.is_none_or(|highest| {
				set.tablet_replica_set_version > highest.tablet_replica_set_version
			}) {
				highest = Some(set);
			}
		}
		highest
	}

	/// Whether a pending frame forgets this replica.
	pub(crate) fn forgetting(&self, key: &VolumeLogKey) -> bool {
		self.pending()
			.any(|batch| batch.index.forgets.contains(key))
	}

	/// Pending forget frames that record a retirement.
	pub(crate) fn pending_retirement_forgets(&self) -> usize {
		self.pending()
			.map(|batch| batch.index.retirement_forgets as usize)
			.sum()
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct MemoryBatch {
	/// Frames enter only through `push_frames`, which keeps `index` in step.
	frames: Vec<Frame>,
	/// Fence and forget frames of `frames`, so admission checks do not scan them.
	index: FrameIndex,
	/// The disk size of `frames`, kept as frames are pushed.
	pub sizing: BatchSizing,
	/// Batches containing entries released by these unsynced frames.
	pub retained_batch_log_seqs: std::collections::BTreeSet<u64>,
	pub encoded_bytes: u64,
	/// Replies released when this batch syncs. They enter only through `push_reply`,
	/// which keeps `read_bounds` in step.
	waiting_replies: Vec<WaitingReply>,
	/// Reply bounds of the data reads among `waiting_replies`.
	read_bounds: ReadBounds,
}

/// The fence and forget facts of one pending batch's frames. Admission asks these per
/// request; at most two batches are pending, so each question is two map lookups.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct FrameIndex {
	/// The highest grant epoch per tablet.
	grant_epochs: BTreeMap<[u8; 16], u64>,
	/// The highest-version replica set per tablet; the first frame wins a tie.
	replica_sets: BTreeMap<[u8; 16], TabletReplicaSet>,
	forgets: BTreeSet<VolumeLogKey>,
	/// Forget frames that record a retirement.
	retirement_forgets: u32,
	/// Entry, state and forget frames per volume. A volume with none here has no unsynced
	/// change in this batch.
	volume_frames: BTreeMap<VolumeLogKey, u32>,
}

impl FrameIndex {
	fn add(&mut self, frame: &Frame) {
		let volume = match frame {
			Frame::EntryFrame(entry) => Some(&entry.volume_log_key),
			Frame::StateFrame(state) => Some(&state.volume_log_key),
			Frame::ForgetFrame(forget) => Some(&forget.volume_log_key),
			Frame::TabletGrantFrame(_) | Frame::TabletReplicaSetFrame(_) => None,
		};
		if let Some(volume) = volume {
			*self
				.volume_frames
				.entry(VolumeLogKey::from(volume.clone()))
				.or_default() += 1;
		}
		match frame {
			Frame::TabletGrantFrame(grant) => {
				let epoch = self.grant_epochs.entry(grant.tablet_id).or_default();
				*epoch = (*epoch).max(grant.tablet_grant_epoch);
			}
			Frame::TabletReplicaSetFrame(fence) => {
				let replaces = self.replica_sets.get(&fence.tablet_id).is_none_or(|set| {
					fence.replica_set.tablet_replica_set_version > set.tablet_replica_set_version
				});
				if replaces {
					self.replica_sets
						.insert(fence.tablet_id, fence.replica_set.clone());
				}
			}
			Frame::ForgetFrame(forget) => {
				self.forgets
					.insert(VolumeLogKey::from(forget.volume_log_key.clone()));
				if forget.retirement.is_some() {
					self.retirement_forgets += 1;
				}
			}
			Frame::EntryFrame(_) | Frame::StateFrame(_) => {}
		}
	}

	/// Builds the index from frames alone, for debug validation.
	pub(crate) fn recompute(frames: &[Frame]) -> Self {
		let mut index = Self::default();
		for frame in frames {
			index.add(frame);
		}
		index
	}
}

impl MemoryBatch {
	pub(crate) fn frames(&self) -> &[Frame] {
		&self.frames
	}

	pub(crate) fn index(&self) -> &FrameIndex {
		&self.index
	}

	pub(crate) fn waiting_replies(&self) -> &[WaitingReply] {
		&self.waiting_replies
	}

	pub(crate) fn read_bounds(&self) -> &ReadBounds {
		&self.read_bounds
	}

	/// Queues a reply behind this batch's sync, reserving its bytes if it is a data read.
	pub(crate) fn push_reply(&mut self, reply: WaitingReply) {
		if let Some(bound) = reply.reply.data_read_bound() {
			self.read_bounds.add(bound);
		}
		self.waiting_replies.push(reply);
	}

	/// Releases this batch's replies with the batch.
	pub(crate) fn into_waiting_replies(self) -> Vec<WaitingReply> {
		self.waiting_replies
	}

	/// Appends admitted frames and adds their encoded sizes to the batch's disk size.
	pub(crate) fn push_frames(
		&mut self,
		frames: Vec<Frame>,
		max_batch_bytes: u32,
	) -> anyhow::Result<()> {
		let frame_sizes: Vec<u64> = frames
			.iter()
			.map(|frame| frame.encoded_len() as u64)
			.collect();
		self.push_sized_frames(frames, &frame_sizes, max_batch_bytes)
	}

	/// [`push_frames`](Self::push_frames) with sizes the caller already computed for
	/// admission. `frame_sizes[i]` must be the encoded length of `frames[i]`.
	pub(crate) fn push_sized_frames(
		&mut self,
		frames: Vec<Frame>,
		frame_sizes: &[u64],
		max_batch_bytes: u32,
	) -> anyhow::Result<()> {
		debug_assert_eq!(frames.len(), frame_sizes.len());
		for frame_bytes in frame_sizes {
			self.sizing.push(*frame_bytes, max_batch_bytes)?;
		}
		for frame in &frames {
			self.index.add(frame);
		}
		self.frames.extend(frames);
		Ok(())
	}

	/// The disk size this batch would have with frames of the given encoded sizes
	/// appended, without re-encoding the frames already in it.
	pub(crate) fn size_with(
		&self,
		frame_sizes: &[u64],
		max_batch_bytes: u32,
	) -> anyhow::Result<u64> {
		let mut sizing = self.sizing;
		for frame_bytes in frame_sizes {
			sizing.push(*frame_bytes, max_batch_bytes)?;
		}
		Ok(sizing.total())
	}
}

/// Reply byte bounds of outstanding data reads: their sum and a multiset for the largest,
/// so read reservations are answered without visiting every read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ReadBounds {
	sum: u64,
	counts: BTreeMap<u32, u32>,
}

impl ReadBounds {
	/// Builds the bounds of the given reads, for debug validation.
	pub(crate) fn recompute(bounds: impl Iterator<Item = u32>) -> Self {
		let mut recomputed = Self::default();
		for bound in bounds {
			recomputed.add(bound);
		}
		recomputed
	}

	pub(crate) fn add(&mut self, bound: u32) {
		self.sum = self.sum.saturating_add(u64::from(bound));
		*self.counts.entry(bound).or_default() += 1;
	}

	pub(crate) fn remove(&mut self, bound: u32) {
		if let Some(count) = self.counts.get_mut(&bound) {
			self.sum = self.sum.saturating_sub(u64::from(bound));
			*count -= 1;
			if *count == 0 {
				self.counts.remove(&bound);
			}
		}
	}

	/// Reads counted in these bounds.
	pub(crate) fn len(&self) -> u64 {
		self.counts.values().map(|count| u64::from(*count)).sum()
	}

	pub(crate) fn sum(&self) -> u64 {
		self.sum
	}

	pub(crate) fn largest(&self) -> Option<u32> {
		self.counts.last_key_value().map(|(bound, _)| *bound)
	}

	
}

/// What `write_bytes` reports for a batch's frames, kept incrementally: the framed size
/// of every batch `cut_batches` has already closed, plus the batch still open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct BatchSizing {
	closed_bytes: u64,
	open_frame_bytes: u64,
	open_frames: usize,
}

impl BatchSizing {
	/// Adds one frame, closing the open batch exactly where `cut_batches` would.
	pub(crate) fn push(&mut self, frame_bytes: u64, max_batch_bytes: u32) -> anyhow::Result<()> {
		let max_batch_bytes = u64::from(max_batch_bytes);
		anyhow::ensure!(
			batch_bytes(frame_bytes, 1) <= max_batch_bytes,
			"frame exceeds batch limit"
		);
		if batch_bytes(self.open_frame_bytes + frame_bytes, self.open_frames + 1) > max_batch_bytes
		{
			self.closed_bytes +=
				BATCH_FRAMING_BYTES as u64 + batch_bytes(self.open_frame_bytes, self.open_frames);
			self.open_frame_bytes = 0;
			self.open_frames = 0;
		}
		self.open_frame_bytes += frame_bytes;
		self.open_frames += 1;
		Ok(())
	}

	/// Framed bytes of every batch, including each batch's length and checksum header.
	pub(crate) fn total(&self) -> u64 {
		if self.open_frames == 0 {
			self.closed_bytes
		} else {
			self.closed_bytes
				+ BATCH_FRAMING_BYTES as u64
				+ batch_bytes(self.open_frame_bytes, self.open_frames)
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct WaitingReply {
	pub request_id: RequestId,
	pub reply: PendingReply,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum PendingReply {
	Read {
		deadline_boot_ms: u64,
		volume_log_key: VolumeLogKey,
		first_volume_seq: u64,
		max_bytes: u32,
		include_deleted: bool,
	},
	RejectedReplica {
		volume_log_key: VolumeLogKey,
		authority: Authority,
		reason: RejectResponse,
	},
	ReplicaState {
		volume_log_key: VolumeLogKey,
		rejection: Option<ReplicaRejection>,
		authority: Option<Authority>,
	},
	LoadedVolumeReplicas {
		request: protocol::ListTabletReplicasRequest,
	},
	/// A batched status probe; each key is answered from synced disk state on release.
	Status { volume_log_keys: Vec<VolumeLogKey> },
	/// A batched forget or trim under one grant; each volume is answered in request order
	/// on release, exactly as its single request would be.
	Forgets {
		kind: GrantBatch,
		authority: Authority,
		results: Vec<ForgetResult>,
	},
}

/// Which batched request a `PendingReply::Forgets` answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum GrantBatch {
	Forget,
	Trim,
}

/// One volume of a batched forget or trim, as admitted.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ForgetResult {
	/// Rejected at admission with no frame to wait for.
	Rejected(RejectResponse),
	/// Rejected after its grant frame was queued; the grant is rechecked on release.
	RejectedAfterGrant {
		volume_log_key: VolumeLogKey,
		reason: RejectResponse,
	},
	/// Its frame or rule rejection waits for the disk write.
	Replica {
		volume_log_key: VolumeLogKey,
		rejection: Option<ReplicaRejection>,
	},
}

impl PendingReply {
	/// The reply byte bound of a data read. Sequence-zero status probes return disk
	/// metadata without opening a segment, so they reserve no bytes.
	pub(crate) fn data_read_bound(&self) -> Option<u32> {
		match self {
			Self::Read {
				first_volume_seq,
				max_bytes,
				..
			} if *first_volume_seq != 0 => Some(*max_bytes),
			Self::Read { .. }
			| Self::RejectedReplica { .. }
			| Self::LoadedVolumeReplicas { .. }
			| Self::Status { .. }
			| Self::Forgets { .. }
			| Self::ReplicaState { .. } => None,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct WritingToDisk {
	pub first_log_seq: u64,
	pub last_log_seq: u64,
	pub pending: MemoryBatch,
	/// When the write was handed to the disk.
	pub started_boot_ms: u64,
	/// The framed bytes the write carries.
	pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ReplicaRejection {
	VolumeLogReplicaDeleted,
	StaleClientEpoch,
	ClientConflict,
	NotClaimed,
	VolumeSeqGap,
	Diverged,
	AppendedMismatch,
	BelowCold,
	VolumeLogReplicaIncarnationMismatch,
	VolumeNotLoaded,
}

impl ReplicaRejection {
	/// Retains the reason without capturing speculative state for a deferred reply.
	pub(crate) fn from_response(reason: &RejectResponse) -> Option<Self> {
		match reason {
			RejectResponse::DiskIncarnationMismatch(_) => None,
			RejectResponse::StaleClientEpoch(_) => Some(Self::StaleClientEpoch),
			RejectResponse::ClientConflict(_) => Some(Self::ClientConflict),
			RejectResponse::NotClaimed(_) => Some(Self::NotClaimed),
			RejectResponse::VolumeSeqGap(_) => Some(Self::VolumeSeqGap),
			RejectResponse::Diverged(_) => Some(Self::Diverged),
			RejectResponse::AppendedMismatch(_) => Some(Self::AppendedMismatch),
			RejectResponse::BelowCold(_) => Some(Self::BelowCold),
			RejectResponse::VolumeLogReplicaIncarnationMismatch(_) => {
				Some(Self::VolumeLogReplicaIncarnationMismatch)
			}
			RejectResponse::VolumeNotLoaded => Some(Self::VolumeNotLoaded),
			RejectResponse::StaleTabletReplicaSet(_)
			| RejectResponse::StaleTabletGrantEpoch(_)
			| RejectResponse::Overloaded(_)
			| RejectResponse::NotServing
			| RejectResponse::RequestTooLarge(_)
			| RejectResponse::TooManyLoadedVolumes(_)
			| RejectResponse::DiskFull(_)
			| RejectResponse::UnsupportedPayloadVersion(_) => None,
			RejectResponse::VolumeLogReplicaDeleted(_) => Some(Self::VolumeLogReplicaDeleted),
		}
	}

	/// Attaches the current disk state when the disk write barrier releases the reply.
	pub(crate) fn response(self, state: VolumeLogReplicaState) -> RejectResponse {
		match self {
			Self::VolumeLogReplicaDeleted => {
				RejectResponse::VolumeLogReplicaDeleted(protocol::VolumeLogReplicaDeleted { state })
			}
			Self::StaleClientEpoch => {
				RejectResponse::StaleClientEpoch(protocol::StaleClientEpoch { state })
			}
			Self::ClientConflict => {
				RejectResponse::ClientConflict(protocol::ClientConflict { state })
			}
			Self::NotClaimed => RejectResponse::NotClaimed(protocol::NotClaimed { state }),
			Self::VolumeSeqGap => RejectResponse::VolumeSeqGap(protocol::VolumeSeqGap { state }),
			Self::Diverged => RejectResponse::Diverged(protocol::Diverged { state }),
			Self::AppendedMismatch => {
				RejectResponse::AppendedMismatch(protocol::AppendedMismatch { state })
			}
			Self::BelowCold => RejectResponse::BelowCold(protocol::BelowCold {
				cold_write: None,
				state,
			}),
			Self::VolumeLogReplicaIncarnationMismatch => {
				RejectResponse::VolumeLogReplicaIncarnationMismatch(
					protocol::VolumeLogReplicaIncarnationMismatch { state },
				)
			}
			Self::VolumeNotLoaded => RejectResponse::VolumeNotLoaded,
		}
	}
}

/// Replays one disk frame. Live disk writes and startup use exactly the same state reduction.
pub(crate) fn replay_state(
	state: &mut Option<VolumeLogReplicaState>,
	body: &Frame,
) -> anyhow::Result<()> {
	match body {
		// Tablet fences use their own reducer and never change a replica.
		Frame::TabletGrantFrame(_) | Frame::TabletReplicaSetFrame(_) => {}
		Frame::ForgetFrame(_) => *state = None,
		Frame::StateFrame(f) => {
			anyhow::ensure!(
				f.state.cold.volume_seq <= f.state.known_committed_volume_seq
					&& f.state.known_committed_volume_seq <= f.state.appended.volume_seq,
				"invalid committed position"
			);
			*state = Some(f.state.clone());
		}
		Frame::EntryFrame(f) => {
			let state = state
				.as_mut()
				.ok_or_else(|| anyhow::anyhow!("entry without replica state"))?;
			anyhow::ensure!(
				state.appended.volume_seq.checked_add(1) == Some(f.entry.volume_seq)
					&& f.entry.client_epoch >= state.appended.client_epoch,
				"invalid disk entry sequence"
			);
			state.appended.volume_seq = f.entry.volume_seq;
			state.appended.client_epoch = f.entry.client_epoch;
			state.known_committed_volume_seq = state
				.known_committed_volume_seq
				.max(f.known_committed_volume_seq.min(f.entry.volume_seq));
		}
	}
	Ok(())
}

/// Rejects admission immediately without exposing state that may still be speculative.
pub(super) fn reject(request_id: RequestId, reason: RejectResponse, effects: &mut Effects) {
	effects.execute(LogShardEffect::Reply {
		request_id,
		response: StorageWorkerResponse::RejectResponse(reason),
	});
}

/// BARE batches contain a fixed u64 sequence and a variable-length frame count.
pub(super) fn batch_bytes(frame_bytes: u64, count: usize) -> u64 {
	let mut count = count as u64;
	let mut prefix = 1;
	while count >= 128 {
		count >>= 7;
		prefix += 1;
	}
	8 + prefix + frame_bytes
}

/// Splits frames without changing their order; each payload fits one checksummed disk batch.
pub(super) fn cut_batches(
	frames: &[Frame],
	max_batch_bytes: u32,
	previous_log_seq: u64,
) -> anyhow::Result<Vec<DiskBatch>> {
	let mut batches = Vec::new();
	let mut frames_in_batch = Vec::new();
	let mut bytes_in_batch = 0;
	let mut log_seq = previous_log_seq;
	for frame in frames {
		let bytes = frame.encoded_len() as u64;
		anyhow::ensure!(
			batch_bytes(bytes, 1) <= u64::from(max_batch_bytes),
			"frame exceeds batch limit"
		);
		if batch_bytes(bytes_in_batch + bytes, frames_in_batch.len() + 1)
			> u64::from(max_batch_bytes)
		{
			log_seq = log_seq
				.checked_add(1)
				.ok_or_else(|| anyhow::anyhow!("log sequence exhausted"))?;
			batches.push(DiskBatch {
				log_seq,
				frames: std::mem::take(&mut frames_in_batch),
			});
			bytes_in_batch = 0;
		}
		frames_in_batch.push(frame.clone());
		bytes_in_batch += bytes;
	}
	if !frames_in_batch.is_empty() {
		log_seq = log_seq
			.checked_add(1)
			.ok_or_else(|| anyhow::anyhow!("log sequence exhausted"))?;
		batches.push(DiskBatch {
			log_seq,
			frames: frames_in_batch,
		});
	}
	Ok(batches)
}

/// The framed write size of frames with the given encoded sizes, cut exactly as
/// `cut_batches` would cut them. Includes each batch's length and checksum, so the write cap
/// also bounds segment growth. Nothing is encoded or cloned.
pub(super) fn write_bytes_for_sizes(
	frame_sizes: &[u64],
	max_batch_bytes: u32,
) -> anyhow::Result<u64> {
	let mut sizing = BatchSizing::default();
	for frame_bytes in frame_sizes {
		sizing.push(*frame_bytes, max_batch_bytes)?;
	}
	Ok(sizing.total())
}

/// The framed write size of `frames`, found by cutting and encoding them. This is the
/// reference that debug validation and tests check the incremental sizes against.
pub(super) fn write_bytes(frames: &[Frame], max_batch_bytes: u32) -> anyhow::Result<u64> {
	cut_batches(frames, max_batch_bytes, 0)?
		.iter()
		.try_fold(0u64, |total, batch| {
			Ok(total
				+ depot_types::log_shard::encode_batch(batch, max_batch_bytes as usize)?.len()
					as u64)
		})
}

/// Dispatches every accumulated frame as one ordered write and sync.
/// Sends filling work as soon as the previous write returns, or immediately on an
/// idle shard. WriteToDisk is an operation timeout, never a batching delay.
pub(crate) fn dispatch(state: &mut LogShardState, effects: &mut Effects) {
	// Dispatch runs after every event, so the first frame of a batch that cannot go out now
	// entered during this event.
	let now_boot_ms = effects.now_boot_ms();
	if !state.memory_batches.filling.frames.is_empty() {
		state
			.memory_batches
			.filling_since_boot_ms
			.get_or_insert(now_boot_ms);
	}

	if matches!(
		state.phase,
		LogShardPhase::Failed { .. } | LogShardPhase::Stopped | LogShardPhase::Recovering(_)
	) || state.memory_batches.writing_to_disk.is_some()
	{
		return;
	}

	// Frame-free replies still wait behind earlier writes, but need no disk operation themselves.
	let pending = std::mem::take(&mut state.memory_batches.filling);
	let fill_wait_ms = state
		.memory_batches
		.filling_since_boot_ms
		.take()
		.map_or(0, |since| now_boot_ms.saturating_sub(since));
	if pending.frames.is_empty() {
		release_replies(
			state,
			pending.into_waiting_replies(),
			&BTreeMap::new(),
			effects,
		);
	} else {
		let batches = match cut_batches(
			&pending.frames,
			state.config.max_batch_bytes,
			state.memory_batches.last_log_seq,
		) {
			Ok(batches) => batches,
			Err(error) => {
				fail(state, error.to_string(), effects);
				return;
			}
		};
		let first_log_seq = batches.first().expect("nonempty frames").log_seq;
		let last_log_seq = batches.last().expect("nonempty frames").log_seq;

		// Entry locations follow the individual batch boundaries within this write.
		for batch in &batches {
			for frame in &batch.frames {
				if let Frame::EntryFrame(f) = frame
					&& let Some(replica) = state
						.loaded_volume_replicas
						.get_mut(&VolumeLogKey::from(f.volume_log_key.clone()))
					&& let Some(entry) = entry_at(&mut replica.entries, &f.entry)
				{
					entry.batch_log_seq = Some(batch.log_seq);
					state.segment_pins.add(batch.log_seq);
				}
			}
		}

		// Install one barrier and timeout for the entire write before handing it to the executor.
		state.memory_batches.last_log_seq = last_log_seq;
		let bytes = pending.sizing.total();
		state.memory_batches.writing_to_disk = Some(WritingToDisk {
			first_log_seq,
			last_log_seq,
			pending,
			started_boot_ms: now_boot_ms,
			bytes,
		});
		effects.arm_timer_after(
			LogShardTimer::WriteToDisk { last_log_seq },
			state.config.write_to_disk_timeout_ms,
		);
		effects.execute(LogShardEffect::WriteToDiskRequest {
			batches,
			fill_wait_ms,
		});
		return;
	}

	if state.phase == LogShardPhase::Draining
		&& state.reads.is_empty()
		&& matches!(state.disk_segments.checkpoint, CheckpointCycle::Idle)
		&& state.disk_segments.removing.is_empty()
	{
		state.phase = LogShardPhase::Stopped;
		effects.execute(LogShardEffect::Stopped);
	}
}

/// The retained entry a dispatched entry frame wrote, found by its position: entries are
/// consecutive, so it sits at `volume_seq - front`. It must still be unassigned and carry
/// the frame's header; a truncate and re-append at the same position is a different entry.
fn entry_at<'a>(
	entries: &'a mut std::collections::VecDeque<super::loaded_volume_replica::ReplicaEntry>,
	entry: &rivet_data::generated::depot_log_shard_v1::VolumeLogEntry,
) -> Option<&'a mut super::loaded_volume_replica::ReplicaEntry> {
	let front = entries.front()?.volume_seq;
	let index = usize::try_from(entry.volume_seq.checked_sub(front)?).ok()?;
	entries
		.get_mut(index)
		.filter(|retained| retained.batch_log_seq.is_none() && retained.same_header(entry))
}

/// Releases current disk state in acceptance order without retaining speculative snapshots.
pub(crate) fn release_replies(
	state: &mut LogShardState,
	replies: Vec<WaitingReply>,
	removed: &BTreeMap<VolumeLogKey, VolumeLogReplicaState>,
	effects: &mut Effects,
) {
	for reply in replies {
		let (volume_log_key, rejection, authority) = match reply.reply {
			PendingReply::Read {
				deadline_boot_ms,
				volume_log_key,
				first_volume_seq,
				max_bytes,
				include_deleted,
			} => {
				super::reads::dispatch(
					state,
					reply.request_id,
					deadline_boot_ms,
					volume_log_key,
					first_volume_seq,
					max_bytes,
					include_deleted,
					effects,
				);
				continue;
			}
			PendingReply::RejectedReplica {
				volume_log_key,
				authority,
				reason,
			} => {
				let reason =
					super::tablet_grants::check_authority(state, &volume_log_key, &authority)
						.err()
						.unwrap_or(reason);
				reject(reply.request_id, reason, effects);
				continue;
			}
			PendingReply::LoadedVolumeReplicas { request } => {
				super::tablet_grants::reply_listing(state, reply.request_id, request, effects);
				continue;
			}
			PendingReply::Status { volume_log_keys } => {
				super::reads::reply_status(state, reply.request_id, &volume_log_keys, effects);
				continue;
			}
			PendingReply::Forgets {
				kind,
				authority,
				results,
			} => {
				let mut answers = Vec::with_capacity(results.len());
				for result in results {
					let response = match result {
						ForgetResult::Rejected(reason) => {
							StorageWorkerResponse::RejectResponse(reason)
						}
						ForgetResult::RejectedAfterGrant {
							volume_log_key,
							reason,
						} => StorageWorkerResponse::RejectResponse(
							super::tablet_grants::check_authority(
								state,
								&volume_log_key,
								&authority,
							)
							.err()
							.unwrap_or(reason),
						),
						ForgetResult::Replica {
							volume_log_key,
							rejection,
						} => {
							let Some(response) = replica_response(
								state,
								&volume_log_key,
								rejection,
								Some(&authority),
								removed,
							) else {
								fail(state, "reply without disk replica".into(), effects);
								return;
							};
							response
						}
					};
					answers.push(match response {
						StorageWorkerResponse::ReplicaStateResponse(response) => {
							ForgetVolumeResult::ReplicaStateResponse(response)
						}
						StorageWorkerResponse::RejectResponse(reason) => {
							ForgetVolumeResult::RejectResponse(reason)
						}
						StorageWorkerResponse::GetDiskIncarnationResponse(_)
						| StorageWorkerResponse::ReadVolumeLogEntriesResponse(_)
						| StorageWorkerResponse::ListTabletReplicasResponse(_)
						| StorageWorkerResponse::ReadVolumeLogStatusResponse(_)
						| StorageWorkerResponse::ForgetVolumesResponse(_)
						| StorageWorkerResponse::TrimVolumesResponse(_)
						| StorageWorkerResponse::StorageWorkerBatchResponse(_) => {
							fail(
								state,
								"batched forget or trim answered with a non-replica reply".into(),
								effects,
							);
							return;
						}
					});
				}
				effects.execute(LogShardEffect::Reply {
					request_id: reply.request_id,
					response: match kind {
						GrantBatch::Forget => {
							StorageWorkerResponse::ForgetVolumesResponse(ForgetVolumesResponse {
								results: answers,
							})
						}
						GrantBatch::Trim => {
							StorageWorkerResponse::TrimVolumesResponse(TrimVolumesResponse {
								results: answers,
							})
						}
					},
				});
				continue;
			}
			PendingReply::ReplicaState {
				volume_log_key,
				rejection,
				authority,
			} => (volume_log_key, rejection, authority),
		};
		let Some(response) = replica_response(
			state,
			&volume_log_key,
			rejection,
			authority.as_ref(),
			removed,
		) else {
			fail(state, "reply without disk replica".into(), effects);
			return;
		};
		effects.execute(LogShardEffect::Reply {
			request_id: reply.request_id,
			response,
		});
	}
}

/// The answer to a replica command once its write synced: a rejection when its authority
/// no longer holds, otherwise the synced disk state or the rule rejection carrying it.
/// `None` means the replica has no disk state, which the caller treats as a shard failure.
fn replica_response(
	state: &LogShardState,
	volume_log_key: &VolumeLogKey,
	rejection: Option<ReplicaRejection>,
	authority: Option<&Authority>,
	removed: &BTreeMap<VolumeLogKey, VolumeLogReplicaState>,
) -> Option<StorageWorkerResponse> {
	if let Some(authority) = authority
		&& let Err(reason) = super::tablet_grants::check_authority(state, volume_log_key, authority)
	{
		return Some(StorageWorkerResponse::RejectResponse(reason));
	}
	let disk = state
		.loaded_volume_replicas
		.get(volume_log_key)
		.and_then(|r| r.disk_state.clone())
		.or_else(|| removed.get(volume_log_key).cloned())?;
	Some(match rejection {
		Some(reason) => StorageWorkerResponse::RejectResponse(reason.response(disk)),
		None => StorageWorkerResponse::ReplicaStateResponse(ReplicaStateResponse { state: disk }),
	})
}

/// Installs disk state from the synced frames before callers receive replies. The
/// event handler dispatches the next filling batch only after all replies are emitted.
pub(crate) fn install_written_to_disk(
	state: &mut LogShardState,
	log_seq: u64,
	pending: &MemoryBatch,
	written_to_disk: WrittenToDisk,
) -> anyhow::Result<BTreeMap<VolumeLogKey, VolumeLogReplicaState>> {
	let mut removed = BTreeMap::new();
	// Count only newly written bytes, even when the active segment spans a checkpoint.
	let old_len = state
		.disk_segments
		.segments()
		.get(&written_to_disk.segment_first_log_seq)
		.map_or(0, |s| s.len_bytes);
	state.disk_segments.bytes_since_checkpoint += written_to_disk
		.segment_len_bytes
		.checked_sub(old_len)
		.ok_or_else(|| anyhow::anyhow!("segment length decreased"))?;

	// Record the executor-reported segment length before replaying the synced frames.
	state.disk_segments.insert(
		written_to_disk.segment_first_log_seq,
		DiskSegmentUsage {
			last_log_seq: log_seq,
			len_bytes: written_to_disk.segment_len_bytes,
		},
	);

	// Apply the same reducer used by startup recovery to advance disk state.
	for frame in &pending.frames {
		let old_retired = state.retired_volumes.len() as u64;
		super::retired_volumes::replay(
			&mut state.retired_volumes,
			&state.tablet_grants,
			frame,
			&mut state.disk_segments.changed_since_checkpoint,
		);
		let retired_bytes = (state.retired_volumes.len() as u64).saturating_mul(40);
		state.disk_segments.checkpoint_metadata_bytes = state
			.disk_segments
			.checkpoint_metadata_bytes
			.saturating_add(retired_bytes)
			.saturating_sub(old_retired.saturating_mul(40));
		if let Frame::ForgetFrame(ForgetFrame {
			retirement: Some(grant),
			..
		}) = frame && state
			.tablet_manifest
			.retired_tablets
			.contains(&grant.tablet_id)
		{
			state.parent_retirement_due = true;
		}
		let volume_log_key = match frame {
			Frame::EntryFrame(f) => &f.volume_log_key,
			Frame::StateFrame(f) => &f.volume_log_key,
			Frame::ForgetFrame(f) => &f.volume_log_key,
			Frame::TabletReplicaSetFrame(set) => {
				let old_bytes = state
					.tablet_replica_sets
					.get(&set.tablet_id)
					.map_or(0, |old| 16 + old.encoded_len() as u64);
				super::tablet_replica_sets::replay(&mut state.tablet_replica_sets, set);
				let new_bytes = state
					.tablet_replica_sets
					.get(&set.tablet_id)
					.map_or(0, |new| 16 + new.encoded_len() as u64);
				state.disk_segments.checkpoint_metadata_bytes = state
					.disk_segments
					.checkpoint_metadata_bytes
					.saturating_add(new_bytes)
					.saturating_sub(old_bytes);
				continue;
			}
			Frame::TabletGrantFrame(grant) => {
				if !state.tablet_grants.contains_key(&grant.tablet_id) {
					state.disk_segments.checkpoint_metadata_bytes = state
						.disk_segments
						.checkpoint_metadata_bytes
						.saturating_add(grant.encoded_len() as u64);
				}
				super::tablet_grants::replay(&mut state.tablet_grants, grant);
				continue;
			}
		};

		let key = VolumeLogKey::from(volume_log_key.clone());
		state
			.disk_segments
			.changed_since_checkpoint
			.insert(key.clone());
		let replica = state
			.loaded_volume_replicas
			.get_mut(&key)
			.ok_or_else(|| anyhow::anyhow!("disk replica missing"))?;
		if matches!(frame, Frame::ForgetFrame(_)) {
			let disk = replica
				.disk_state
				.clone()
				.ok_or_else(|| anyhow::anyhow!("forget without disk replica"))?;
			removed.insert(key.clone(), disk);
		}
		let key_bytes = volume_log_key.encoded_len() as u64;
		let old_bytes = replica
			.disk_state
			.as_ref()
			.map_or(0, |old| key_bytes + old.encoded_len() as u64);
		replay_state(&mut replica.disk_state, frame)?;
		let new_bytes = replica
			.disk_state
			.as_ref()
			.map_or(0, |new| key_bytes + new.encoded_len() as u64);
		state.disk_segments.checkpoint_metadata_bytes = state
			.disk_segments
			.checkpoint_metadata_bytes
			.saturating_add(new_bytes)
			.saturating_sub(old_bytes);
	}
	for key in removed.keys() {
		if state
			.loaded_volume_replicas
			.get(key)
			.is_some_and(|r| r.disk_state.is_none())
			&& let Some(id) = state.loaded_volume_replicas.row_id(key)
			&& let Some(replica) = state.loaded_volume_replicas.remove(key)
		{
			state.segment_pins.replace(Some(&replica), None);
			state.payloads.replaced(
				id,
				Some(super::payloads::ReplicaTotals::of(&replica)),
				None,
				false,
				0,
			);
			state.replica_hashes.replaced(key, false);
		}
	}
	Ok(removed)
}

/// Installs only the matching synced batch before releasing its ordered replies.
pub(crate) fn written_to_disk(
	state: &mut LogShardState,
	log_seq: u64,
	result: Result<WrittenToDisk, super::super::types::WriteToDiskError>,
	effects: &mut Effects,
) {
	// Obsolete completions must not advance disk writes or release replies.
	if state
		.memory_batches
		.writing_to_disk
		.as_ref()
		.is_some_and(|x| x.last_log_seq == log_seq)
	{
		effects.cancel_timer(LogShardTimer::WriteToDisk {
			last_log_seq: log_seq,
		});
		match result {
			Ok(written_to_disk) => {
				let done = state
					.memory_batches
					.writing_to_disk
					.take()
					.expect("matched write to disk");
				let took_ms = effects.now_boot_ms().saturating_sub(done.started_boot_ms);
				state.memory_batches.write_bytes_per_sec =
					Some(done.bytes.saturating_mul(1000) / took_ms.max(1));
				match install_written_to_disk(
					state,
					done.last_log_seq,
					&done.pending,
					written_to_disk,
				) {
					Err(error) => fail(state, error.to_string(), effects),
					Ok(removed) => {
						// Every replica whose disk state this batch changed is noticed. A
						// replica that only appended entries is noticed after a delay, so a
						// burst of appends folds into one notice.
						let mut changed = BTreeSet::new();
						let mut appended = BTreeSet::new();
						for frame in done.pending.frames() {
							match frame {
								Frame::EntryFrame(f) => {
									appended.insert(VolumeLogKey::from(f.volume_log_key.clone()));
								}
								Frame::StateFrame(f) => {
									changed.insert(VolumeLogKey::from(f.volume_log_key.clone()));
								}
								Frame::ForgetFrame(f) => {
									changed.insert(VolumeLogKey::from(f.volume_log_key.clone()));
								}
								Frame::TabletGrantFrame(_) | Frame::TabletReplicaSetFrame(_) => {}
							}
						}
						appended.retain(|key| !changed.contains(key));
						super::status_notices::changed(state, changed);
						super::status_notices::appended(state, appended, effects.now_boot_ms());
						super::status_notices::observe_disk_pressure(state);
						// The written replicas keep only their per-volume share of payloads.
						let written = done
							.pending
							.frames()
							.iter()
							.filter_map(|frame| match frame {
								Frame::EntryFrame(f) => {
									Some(VolumeLogKey::from(f.volume_log_key.clone()))
								}
								Frame::StateFrame(_)
								| Frame::ForgetFrame(_)
								| Frame::TabletGrantFrame(_)
								| Frame::TabletReplicaSetFrame(_) => None,
							})
							.collect();
						super::payloads::synced(state, written);
						release_replies(
							state,
							done.pending.into_waiting_replies(),
							&removed,
							effects,
						)
					}
				}
			}

			Err(reason) => {
				tracing::error!(log_seq, error = %reason, "log shard disk write operation failed");
				fail(state, reason.to_string(), effects);
			}
		}
	}
}

/// Fails only the disk write operation whose alarm is still current.
/// Fails the shard, and so stops the process, when the outstanding write passes
/// `write_to_disk_timeout_ms`.
///
/// Fencing only this shard and keeping the process would not be safe. The stalled write
/// cannot be cancelled: its bytes may still land, after any later recovery scan, at the
/// offset a restarted shard would append to, so only process exit ends the old disk
/// session. A shard left failed inside a live process would also keep the worker's
/// topology lease, so placement and tablet membership would keep counting a replica that
/// answers nothing, while process exit lets the lease lapse and repair replace it. Volume
/// clients already treat the exited worker's replicas as down and commit on the others.
pub(crate) fn timed_out(state: &mut LogShardState, seq: u64, effects: &mut Effects) {
	if state
		.memory_batches
		.writing_to_disk
		.as_ref()
		.is_some_and(|x| x.last_log_seq == seq)
	{
		fail(state, "write to disk timed out".into(), effects);
	}
}


