//! Unfenced reads capture synced disk state: at once for a volume with no pending frames,
//! otherwise after the write barrier that syncs them.

use super::{
	LogShardPhase, LogShardState, fail,
	memory_batches::{PendingReply, ReadBounds, WaitingReply, reject},
};
use crate::log_shard_machine::{
	Effects,
	effect::LogShardEffect,
	types::{DiskReadError, DiskReadReason, LogShardTimer, RequestId},
};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	depot_storage_worker_protocol_v1::{self as protocol, RejectResponse, StorageWorkerResponse},
	shared_depot_types_v1::VolumeLogReplicaStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Answers from synced state at once when the volume has no pending frames; otherwise
/// reserves a reply slot behind the write that syncs them. Never exposes speculative state or
/// overtakes a prior write of the volume.
pub(crate) fn admit(
	state: &mut LogShardState,
	request_id: RequestId,
	deadline_boot_ms: u64,
	volume_log_key: VolumeLogKey,
	first_volume_seq: u64,
	max_bytes: u32,
	include_deleted: bool,
	effects: &mut Effects,
) {
	if state.phase != LogShardPhase::Serving {
		reject(request_id, RejectResponse::NotServing, effects);
		return;
	}
	let deadline_boot_ms = deadline_boot_ms.min(
		effects
			.now_boot_ms()
			.saturating_add(state.config.write_to_disk_timeout_ms),
	);
	if effects.now_boot_ms() >= deadline_boot_ms {
		reject(request_id, super::admission::overloaded(state), effects);
		return;
	}
	let pending = state
		.memory_batches
		.pending()
		.map(|b| b.waiting_replies().len())
		.sum::<usize>();
	if pending + state.reads.len() + state.manifest_waits.len()
		>= state.config.max_memory_requests as usize
		|| (first_volume_seq != 0
			&& admission_bytes(state, max_bytes)
				> state
					.config
					.max_memory_bytes
					.saturating_sub(occupied_bytes(state)))
	{
		reject(request_id, super::admission::overloaded(state), effects);
		return;
	}
	let max_bytes = max_bytes.min(state.read_batch_limit());
	// An idle volume's synced state is already final, so the read needs no write barrier.
	if state.memory_batches.synced_now(&volume_log_key) {
		dispatch(
			state,
			request_id,
			deadline_boot_ms,
			volume_log_key,
			first_volume_seq,
			max_bytes,
			include_deleted,
			effects,
		);
		return;
	}
	state.memory_batches.filling.push_reply(WaitingReply {
		request_id,
		reply: PendingReply::Read {
			deadline_boot_ms,
			volume_log_key,
			first_volume_seq,
			max_bytes,
			include_deleted,
		},
	});
}

/// Answers a batched status probe at once when none of its volumes has pending frames, or
/// reserves one reply slot behind the write barrier. Like a sequence-zero read it holds no
/// bytes, and every status it returns is synced.
/// The key count is bounded by the loaded replica cap, the most a shard can answer for.
pub(crate) fn admit_status(
	state: &mut LogShardState,
	request_id: RequestId,
	volume_log_keys: Vec<VolumeLogKey>,
	effects: &mut Effects,
) {
	if state.phase != LogShardPhase::Serving {
		reject(request_id, RejectResponse::NotServing, effects);
		return;
	}
	if volume_log_keys.len() > state.config.max_loaded_volumes as usize {
		reject(
			request_id,
			RejectResponse::RequestTooLarge(protocol::RequestTooLarge {
				max_bytes: state.config.max_batch_bytes,
			}),
			effects,
		);
		return;
	}
	let pending = state
		.memory_batches
		.pending()
		.map(|b| b.waiting_replies().len())
		.sum::<usize>();
	if pending + state.reads.len() + state.manifest_waits.len()
		>= state.config.max_memory_requests as usize
	{
		reject(request_id, super::admission::overloaded(state), effects);
		return;
	}
	// A probe whose every volume is idle reports synced state already final.
	if volume_log_keys
		.iter()
		.all(|key| state.memory_batches.synced_now(key))
	{
		reply_status(state, request_id, &volume_log_keys, effects);
		return;
	}
	state.memory_batches.filling.push_reply(WaitingReply {
		request_id,
		reply: PendingReply::Status { volume_log_keys },
	});
}

/// Answers each key of a batched status probe in request order, with the same outcome a
/// sequence-zero read admitted and released at this point would have.
pub(crate) fn reply_status(
	state: &LogShardState,
	request_id: RequestId,
	volume_log_keys: &[VolumeLogKey],
	effects: &mut Effects,
) {
	let disk_pressure = disk_pressure(state);
	let statuses = volume_log_keys
		.iter()
		.map(|key| status(state, key, disk_pressure))
		.collect();
	effects.execute(LogShardEffect::Reply {
		request_id,
		response: StorageWorkerResponse::ReadVolumeLogStatusResponse(
			protocol::ReadVolumeLogStatusResponse { statuses },
		),
	});
}

fn status(
	state: &LogShardState,
	key: &VolumeLogKey,
	disk_pressure: bool,
) -> protocol::VolumeLogStatus {
	let replica = state.loaded_volume_replicas.get(key);
	if super::retirement::excluded(state, key) {
		return match replica {
			Some(_) => protocol::VolumeLogStatus::NotServing,
			None => protocol::VolumeLogStatus::VolumeNotLoaded,
		};
	}
	let Some((replica, disk)) = replica.and_then(|r| r.disk_state.clone().map(|disk| (r, disk)))
	else {
		return protocol::VolumeLogStatus::VolumeNotLoaded;
	};
	if disk.status == VolumeLogReplicaStatus::Deleted {
		return protocol::VolumeLogStatus::VolumeLogReplicaDeleted(
			protocol::VolumeLogReplicaDeleted { state: disk },
		);
	}
	protocol::VolumeLogStatus::BelowCold(protocol::BelowCold {
		cold_write: replica.cold_write_status(&disk, disk_pressure),
		state: disk,
	})
}

/// Whether the shard's disk is under cold-write pressure.
pub(crate) fn disk_pressure(state: &LogShardState) -> bool {
	super::status_notices::disk_pressure(state)
}

/// Answers from retained entries when they cover the page, otherwise the ordered executor
/// reads this disk image before any subsequently emitted write.
pub(crate) fn dispatch(
	state: &mut LogShardState,
	request_id: RequestId,
	deadline_boot_ms: u64,
	volume_log_key: VolumeLogKey,
	first_volume_seq: u64,
	max_bytes: u32,
	include_deleted: bool,
	effects: &mut Effects,
) {
	if effects.now_boot_ms() >= deadline_boot_ms {
		reject(request_id, super::admission::overloaded(state), effects);
		return;
	}
	let Some(disk) = state
		.loaded_volume_replicas
		.get(&volume_log_key)
		.and_then(|r| r.disk_state.clone())
	else {
		reject(request_id, RejectResponse::VolumeNotLoaded, effects);
		return;
	};
	if disk.status == VolumeLogReplicaStatus::Deleted && !include_deleted {
		reject(
			request_id,
			RejectResponse::VolumeLogReplicaDeleted(protocol::VolumeLogReplicaDeleted {
				state: disk,
			}),
			effects,
		);
		return;
	}
	if first_volume_seq <= disk.cold.volume_seq {
		reject(
			request_id,
			RejectResponse::BelowCold(protocol::BelowCold {
				cold_write: state.loaded_volume_replicas[&volume_log_key]
					.cold_write_status(&disk, disk_pressure(state)),
				state: disk,
			}),
			effects,
		);
		return;
	}
	let reason = match synced_page(state, &volume_log_key, &disk, first_volume_seq, max_bytes) {
		Ok(response) => {
			effects.execute(LogShardEffect::Reply {
				request_id,
				response,
			});
			return;
		}
		Err(reason) => reason,
	};
	// Reads that need disk are bounded on their own, so a burst of them is turned away
	// here instead of queueing behind each other. Memory pages above never count.
	if state.reads.len() >= state.config.max_disk_reads as usize {
		reject(request_id, super::admission::overloaded(state), effects);
		return;
	}
	// Queueing for disk work consumes the same deadline as the read itself.
	state.reads.insert(request_id, max_bytes);
	effects.arm_timer_at(LogShardTimer::Read(request_id), deadline_boot_ms);
	effects.execute(LogShardEffect::ReadRequest {
		request_id,
		deadline_boot_ms,
		volume_log_key,
		state: disk,
		first_volume_seq,
		max_bytes,
		reason,
	});
}

/// Builds the page the disk executor would return, from entries the replica already retains.
///
/// Every entry above `cold` stays in memory after it reaches disk, but the oldest synced
/// payloads may be released under the shard's payload budget. An entry is on disk once its
/// batch is at or below the newest synced batch. Accepted but unsynced changes can make
/// memory differ from disk: a pending trim drops entries below the requested start, and a
/// pending truncate drops synced entries, possibly past the requested start, or puts
/// unsynced ones in their place. Those cases, and a start inside the released prefix,
/// return the reason and use the disk read instead.
/// A page stops early at the first unsynced entry, which callers already handle as a short
/// page, and follows the executor's byte limit and oversize rejection exactly.
pub(crate) fn synced_page(
	state: &LogShardState,
	volume_log_key: &VolumeLogKey,
	disk: &VolumeLogReplicaState,
	first_volume_seq: u64,
	max_bytes: u32,
) -> Result<StorageWorkerResponse, DiskReadReason> {
	let synced_through = super::payloads::synced_through(state);
	let replica = state
		.loaded_volume_replicas
		.get(volume_log_key)
		.ok_or(DiskReadReason::BelowFront)?;
	let entries = replica.entries();
	let last_volume_seq = disk.appended.volume_seq;
	let mut page = Vec::new();
	if first_volume_seq <= last_volume_seq {
		let front_volume_seq = entries
			.front()
			.ok_or(DiskReadReason::BelowFront)?
			.volume_seq;
		let start = first_volume_seq
			.checked_sub(front_volume_seq)
			.and_then(|start| usize::try_from(start).ok())
			.ok_or(DiskReadReason::BelowFront)?;
		// An accepted but unsynced truncate shortens memory before disk, so a synced start
		// can lie past the retained tail. That range is read from disk instead.
		if start >= entries.len() {
			return Err(DiskReadReason::Truncated);
		}
		// Released payloads form a prefix; a page starting inside it reads disk.
		if start < replica.released() {
			return Err(DiskReadReason::Released);
		}
		let mut bytes = 0u64;
		for retained in entries.range(start..) {
			if retained.volume_seq > last_volume_seq {
				break;
			}
			let synced = retained
				.batch_log_seq
				.is_some_and(|batch_log_seq| batch_log_seq <= synced_through);
			if !synced {
				if page.is_empty() {
					return Err(DiskReadReason::Unsynced);
				}
				break;
			}
			let length = retained.encoded_bytes();
			if bytes + length > u64::from(max_bytes) {
				if page.is_empty() {
					return Ok(StorageWorkerResponse::RejectResponse(
						RejectResponse::RequestTooLarge(protocol::RequestTooLarge { max_bytes }),
					));
				}
				break;
			}
			// Entries after the released prefix hold their payloads.
			let Some(entry) = retained.log_entry() else {
				return Err(DiskReadReason::Released);
			};
			bytes += length;
			page.push(entry);
		}
		if page
			.first()
			.is_none_or(|entry| entry.volume_seq != first_volume_seq)
		{
			return Err(DiskReadReason::BelowFront);
		}
	}
	Ok(StorageWorkerResponse::ReadVolumeLogEntriesResponse(
		protocol::ReadVolumeLogEntriesResponse {
			state: disk.clone(),
			entries: page,
		},
	))
}

/// Releases only a current read. An I/O error fails that read alone: its entries are
/// synced history that writes never rewrite in place, and a broken disk still stops the
/// shard through the write path's own timers. Corrupt history fails the shard.
pub(crate) fn completed(
	state: &mut LogShardState,
	request_id: RequestId,
	result: Result<StorageWorkerResponse, DiskReadError>,
	effects: &mut Effects,
) {
	if state.reads.remove(&request_id).is_none() {
		return;
	}
	effects.cancel_timer(LogShardTimer::Read(request_id));
	match result {
		Ok(response) => effects.execute(LogShardEffect::Reply {
			request_id,
			response,
		}),
		Err(DiskReadError::Unavailable(error)) => {
			tracing::warn!(%error, "log shard disk read failed; the caller retries");
			crate::metrics::record_disk_read_failure(DiskReadFailure::IoError);
			reject(request_id, super::admission::overloaded(state), effects);
		}
		Err(DiskReadError::Corrupt(reason)) => fail(state, reason, effects),
	}
}

/// Fails a read whose disk work outlived its deadline, without failing the shard. Its
/// late response finds no read and is dropped.
pub(crate) fn timed_out(state: &mut LogShardState, request_id: RequestId, effects: &mut Effects) {
	if state.reads.remove(&request_id).is_none() {
		return;
	}
	crate::metrics::record_disk_read_failure(DiskReadFailure::Timeout);
	reject(request_id, super::admission::overloaded(state), effects);
}

/// Why a disk read was answered `Overloaded` instead of with a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiskReadFailure {
	/// The read did not finish within its deadline once it started.
	Timeout,
	/// The executor returned an error for the read.
	IoError,
}

impl DiskReadFailure {
	pub(crate) fn label(self) -> &'static str {
		match self {
			Self::Timeout => "timeout",
			Self::IoError => "io_error",
		}
	}
}

/// A disk reader scans at most two complete framed batches for the read it is running,
/// so each running read holds one scan buffer of this size.
fn scan_bytes(state: &LogShardState) -> u64 {
	2 * (u64::from(state.read_batch_limit()) + depot_types::log_shard::BATCH_FRAMING_BYTES as u64)
}

/// Bytes held by `reads` data reads: one scan buffer per reader they can occupy, at most
/// `disk_read_concurrency`, plus their summed reply bounds minus the largest. A reply is at
/// most the batches its read scanned, so a scan buffer covers one reply, counted as the
/// largest; a lone read therefore needs exactly one scan buffer.
fn read_bytes(state: &LogShardState, reads: u64, sum: u64, largest: u32) -> u64 {
	let buffers = reads.clamp(1, u64::from(state.config.disk_read_concurrency));
	scan_bytes(state)
		.saturating_mul(buffers)
		.saturating_add(sum.saturating_sub(u64::from(largest)))
}

/// Bytes a new data read adds to `occupied_bytes`.
fn admission_bytes(state: &LogShardState, max_bytes: u32) -> u64 {
	let max_bytes = max_bytes.min(state.read_batch_limit());
	let before = occupied_read_bytes(state);
	let (reads, sum, largest) = data_reads(state);
	let largest = largest.map_or(max_bytes, |largest| largest.max(max_bytes));
	let after = read_bytes(
		state,
		reads + 1,
		sum.saturating_add(u64::from(max_bytes)),
		largest,
	);
	after.saturating_sub(before)
}

fn occupied_read_bytes(state: &LogShardState) -> u64 {
	let (reads, sum, largest) = data_reads(state);
	let Some(largest) = largest else {
		return 0;
	};
	read_bytes(state, reads, sum, largest)
}

/// The count, summed and largest reply bounds of data reads queued behind a write barrier
/// or running on a disk reader, from the running totals each keeps. Status probes reserve
/// a request slot and the write barrier but no bytes.
fn data_reads(state: &LogShardState) -> (u64, u64, Option<u32>) {
	let mut reads = 0u64;
	let mut sum = 0u64;
	let mut largest = None::<u32>;
	let bounds = state
		.memory_batches
		.pending()
		.map(|batch| batch.read_bounds())
		.chain(std::iter::once(state.reads.bounds()));
	for bounds in bounds {
		reads = reads.saturating_add(bounds.len());
		sum = sum.saturating_add(bounds.sum());
		largest = largest.max(bounds.largest());
	}
	(reads, sum, largest)
}

/// Outstanding executor reads by caller and the reply byte bound each one reserved,
/// with the running bounds `occupied_bytes` reads.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct OutstandingReads {
	by_request: BTreeMap<RequestId, u32>,
	bounds: ReadBounds,
}

impl OutstandingReads {
	pub(crate) fn insert(&mut self, request_id: RequestId, bound: u32) {
		if let Some(old) = self.by_request.insert(request_id, bound) {
			self.bounds.remove(old);
		}
		self.bounds.add(bound);
	}

	pub(crate) fn remove(&mut self, request_id: &RequestId) -> Option<u32> {
		let bound = self.by_request.remove(request_id)?;
		self.bounds.remove(bound);
		Some(bound)
	}

	pub(crate) fn clear(&mut self) {
		*self = Self::default();
	}

	pub(crate) fn len(&self) -> usize {
		self.by_request.len()
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.by_request.is_empty()
	}

	pub(crate) fn bounds(&self) -> &ReadBounds {
		&self.bounds
	}

	/// Checks the running bounds against the reads, for debug validation.
	pub(crate) fn validate(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.bounds == ReadBounds::recompute(self.by_request.values().copied()),
			"read reservations drifted from the outstanding reads"
		);
		Ok(())
	}
}

/// Shares one byte cap across writes and reads, including reads still behind a write barrier.
pub(crate) fn occupied_bytes(state: &LogShardState) -> u64 {
	let write_bytes = state
		.memory_batches
		.pending()
		.map(|batch| batch.encoded_bytes)
		.sum::<u64>();
	write_bytes
		.saturating_add(occupied_read_bytes(state))
		.saturating_add(super::manifest_wait::bytes(state))
}
