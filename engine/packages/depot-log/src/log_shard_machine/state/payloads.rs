//! Synced entry payloads the shard keeps in memory for reads.
//!
//! Every retained entry keeps its header for append matching, segment pins and status, but
//! a synced payload may leave memory: disk already holds it and reads fall back to disk.
//! Three rules release payloads. After each disk write, every replica it touched keeps at
//! most `max_retained_payload_bytes_per_volume`. A replica that appends nothing for
//! `release_idle_payloads_after_ms` releases all of them. Above
//! `max_retained_payload_bytes` across the shard, the least recently appended replicas
//! release theirs first. Unsynced payloads are never released; the pending batches that hold
//! them are bounded by `max_memory_bytes`.

use super::{
	LogShardPhase, LogShardState,
	loaded_volume_replica::LoadedVolumeReplica,
	replica_table::{ReplicaTable, RowId},
};
use crate::log_shard_machine::{Effects, types::LogShardTimer};
use depot_types::VolumeLogKey;
use rivet_state_machine::IndexedDeadlineQueue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Replicas visited per event when the shard is over its payload budget. Replicas that
/// stay over after this many are visited after the next event, usually a disk write that
/// made more payloads releasable.
const BUDGET_VISITS_PER_EVENT: usize = 256;

/// Idle replicas released per timer delivery; more stay due and the timer fires again.
const IDLE_RELEASES_PER_TIMER: usize = 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Payloads {
	/// Encoded bytes of entries whose payloads are held in memory, across replicas. It is
	/// always the sum of `LoadedVolumeReplica::retained_payload_bytes`.
	pub(crate) retained_bytes: u64,
	/// Every replica that holds payloads, by row id and the time it last appended. Recovered
	/// replicas enter at their recovery time.
	pub(crate) by_last_append: IndexedDeadlineQueue<RowId>,
	/// The time `LogShardTimer::ReleaseIdlePayloads` is armed for.
	pub(crate) idle_armed_at: Option<u64>,
	/// Retained entries across replicas, payload held or not. Each keeps its header in
	/// memory.
	pub(crate) entries: u64,
	/// Encoded bytes of those entries: the log tails above cold, which disk also holds.
	pub(crate) entry_bytes: u64,
	/// Replicas that retain entries, payload held or not, by row id. Only these can report a
	/// cold-write status, so a disk pressure change notices only them. Bounded by the loaded
	/// replicas.
	pub(crate) with_entries: BTreeSet<RowId>,
}

impl Payloads {
	/// Accounts one replica being replaced, created or removed, after the table holds the
	/// new version under `id`. `old` holds the totals of the replaced version and `new` the
	/// totals of the installed one. `appended` marks a replacement that added entries, which
	/// makes the replica recent again.
	pub(crate) fn replaced(
		&mut self,
		id: RowId,
		old: Option<ReplicaTotals>,
		new: Option<ReplicaTotals>,
		appended: bool,
		now_boot_ms: u64,
	) {
		let old = old.unwrap_or_default();
		let new_totals = new.unwrap_or_default();
		self.retained_bytes =
			self.retained_bytes - old.retained_payload_bytes + new_totals.retained_payload_bytes;
		self.entries = self.entries - old.entries + new_totals.entries;
		self.entry_bytes = self.entry_bytes - old.entry_bytes + new_totals.entry_bytes;
		if new_totals.entries > 0 {
			self.with_entries.insert(id);
		} else {
			self.with_entries.remove(&id);
		}
		if new_totals.retained_payload_bytes == 0 {
			self.by_last_append.cancel(&id);
		} else if appended || self.by_last_append.scheduled_at(&id).is_none() {
			self.by_last_append.schedule(id, now_boot_ms);
		}
	}

	/// Rebuilds the accounting from a recovered image; every replica counts as recent.
	pub(crate) fn recovered(replicas: &ReplicaTable, now_boot_ms: u64) -> Self {
		let mut payloads = Self::default();
		for (id, replica) in replicas.rows() {
			payloads.replaced(
				id,
				None,
				Some(ReplicaTotals::of(replica)),
				false,
				now_boot_ms,
			);
		}
		payloads
	}

	/// The keys of the replicas that retain entries.
	pub(crate) fn keys_with_entries(&self, replicas: &ReplicaTable) -> Vec<VolumeLogKey> {
		self.with_entries
			.iter()
			.filter_map(|id| replicas.row(*id))
			.map(|replica| replica.volume_log_key.clone())
			.collect()
	}
}

/// The totals one replica version contributes to the shard's payload accounting.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ReplicaTotals {
	pub(crate) retained_payload_bytes: u64,
	pub(crate) entries: u64,
	pub(crate) entry_bytes: u64,
}

impl ReplicaTotals {
	pub(crate) fn of(replica: &LoadedVolumeReplica) -> Self {
		Self {
			retained_payload_bytes: replica.retained_payload_bytes(),
			entries: replica.entries().len() as u64,
			entry_bytes: replica.entry_bytes(),
		}
	}
}

/// The newest batch sequence on disk. While a write is in flight, its batches are not.
pub(crate) fn synced_through(state: &LogShardState) -> u64 {
	match &state.memory_batches.writing_to_disk {
		Some(writing) => writing.first_log_seq.saturating_sub(1),
		None => state.memory_batches.last_log_seq,
	}
}

/// Releases a replica's synced payloads until at most `keep_bytes` stay held.
fn release(state: &mut LogShardState, id: RowId, keep_bytes: u64) {
	let synced_through = synced_through(state);
	let Some(replica) = state.loaded_volume_replicas.row_mut(id) else {
		return;
	};
	let freed = replica.release_synced(synced_through, keep_bytes);
	state.payloads.retained_bytes -= freed;
	if replica.retained_payload_bytes() == 0 {
		state.payloads.by_last_append.cancel(&id);
	}
}

/// Applies the per-volume cap to the replicas a completed disk write touched.
pub(crate) fn synced(state: &mut LogShardState, keys: BTreeSet<VolumeLogKey>) {
	let keep = state.config.max_retained_payload_bytes_per_volume;
	for key in keys {
		if let Some(id) = state.loaded_volume_replicas.row_id(&key) {
			release(state, id, keep);
		}
	}
}

/// Brings the shard back under `max_retained_payload_bytes` by releasing every synced
/// payload of the least recently appended replicas first.
pub(crate) fn enforce(state: &mut LogShardState) {
	if state.payloads.retained_bytes <= state.config.max_retained_payload_bytes {
		return;
	}
	let oldest = state
		.payloads
		.by_last_append
		.iter()
		.take(BUDGET_VISITS_PER_EVENT)
		.map(|(_, id)| *id)
		.collect::<Vec<_>>();
	for id in oldest {
		if state.payloads.retained_bytes <= state.config.max_retained_payload_bytes {
			break;
		}
		release(state, id, 0);
	}
}

/// Releases every synced payload of replicas that have not appended for the idle period.
/// One whose payloads are not all on disk yet is checked again one idle period later.
pub(crate) fn idle_due(state: &mut LogShardState, now_boot_ms: u64) {
	state.payloads.idle_armed_at = None;
	let quiet_since = now_boot_ms.saturating_sub(state.config.release_idle_payloads_after_ms);
	let due = state
		.payloads
		.by_last_append
		.pop_due(quiet_since, IDLE_RELEASES_PER_TIMER);
	for (at, id) in due {
		// Popping removed the row, so `release` cannot cancel it twice; restore it first
		// to keep the queue and the held bytes in step.
		state.payloads.by_last_append.schedule(id, at);
		release(state, id, 0);
		if state.payloads.by_last_append.scheduled_at(&id).is_some() {
			state.payloads.by_last_append.schedule(id, now_boot_ms);
		}
	}
}

/// Keeps `ReleaseIdlePayloads` armed one idle period after the least recent append while
/// the shard serves.
pub(crate) fn arm(state: &mut LogShardState, now_boot_ms: u64, effects: &mut Effects) {
	let wanted = match state.phase {
		LogShardPhase::Serving | LogShardPhase::Draining => {
			state.payloads.by_last_append.next_at().map(|at| {
				at.saturating_add(state.config.release_idle_payloads_after_ms)
					.max(now_boot_ms)
			})
		}
		LogShardPhase::Recovering(_) | LogShardPhase::Failed { .. } | LogShardPhase::Stopped => {
			None
		}
	};
	if wanted == state.payloads.idle_armed_at {
		return;
	}
	match wanted {
		Some(at) => effects.arm_timer_at(LogShardTimer::ReleaseIdlePayloads, at),
		None => effects.cancel_timer(LogShardTimer::ReleaseIdlePayloads),
	}
	state.payloads.idle_armed_at = wanted;
}

/// Checks the held byte total and queue against the replicas, and each replica's released
/// prefix against disk.
pub(crate) fn validate(state: &LogShardState) -> anyhow::Result<()> {
	state.payloads.by_last_append.debug_validate()?;
	let synced_through = synced_through(state);
	let mut total = 0;
	let mut holders = 0;
	let mut entries = 0;
	let mut entry_bytes = 0;
	for (id, replica) in state.loaded_volume_replicas.rows() {
		replica.validate_payloads(synced_through)?;
		let bytes = replica.retained_payload_bytes();
		total += bytes;
		entries += replica.entries().len() as u64;
		entry_bytes += replica.entry_bytes();
		holders += usize::from(bytes > 0);
		anyhow::ensure!(
			(bytes > 0) == state.payloads.by_last_append.scheduled_at(&id).is_some(),
			"replica holding payloads missing from the release order"
		);
		anyhow::ensure!(
			(replica.entries().len() > 0) == state.payloads.with_entries.contains(&id),
			"replica retaining entries missing from the entry index"
		);
	}
	anyhow::ensure!(
		total == state.payloads.retained_bytes,
		"retained payload bytes {} differ from the replicas' {total}",
		state.payloads.retained_bytes
	);
	anyhow::ensure!(
		(entries, entry_bytes) == (state.payloads.entries, state.payloads.entry_bytes),
		"retained entry totals differ from the replicas"
	);
	anyhow::ensure!(
		state.payloads.by_last_append.len() == holders,
		"release order holds replicas without payloads"
	);
	Ok(())
}


