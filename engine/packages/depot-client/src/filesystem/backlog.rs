//! Commit admission against the tablet's apply. A writer whose own admitted bytes run too far
//! ahead of what the maintainer has applied is refused before its next write is built, with a
//! typed retry hint, instead of growing a backlog that log retention, a maintainer move's
//! replay and every read waiting on the apply must pay for.

use super::{
	Error, Filesystem, Result,
	client::{MAX_HELD_LAYERS, NoteDeadline, State, entry_epoch},
};
use std::collections::VecDeque;

/// Least bytes between two samples of the admitted total, so the samples stay few however
/// small the commits are. An estimate counts at most this much more than the true backlog.
const SAMPLE_BYTES: u64 = 1024 * 1024;

/// The retry hint of a write refused for the backlog. Applies drain at least tens of MiB a
/// second on a healthy store, so this is about how long a refused writer's excess takes.
pub(crate) const BACKLOG_RETRY_AFTER_MS: u64 = 500;

/// Least time between two applied probes of one filesystem, so a refused writer retrying
/// often does not send one maintainer read per attempt.
const PROBE_INTERVAL_MS: u64 = BACKLOG_RETRY_AFTER_MS;

/// Held layers above which a write admission probes the applied position even under the
/// backlog cap, so a writer that reads nothing still releases them.
const PROBE_HELD_LAYERS: usize = MAX_HELD_LAYERS / 4;

/// Longest a probe waits for the maintainer. It reads at the snapshot's floor, which the
/// maintainer normally already serves in one round trip; one that does not answer by then
/// counts as behind. The probe holds the filesystem's state, so this stays short.
const PROBE_TIMEOUT_MS: u64 = 50;

/// This opening's admitted payload bytes, sampled by log sequence, against the newest
/// position the maintainer reported applied.
#[derive(Debug, Default)]
pub(super) struct UnappliedBacklog {
	/// Payload bytes this opening admitted.
	admitted_bytes: u64,
	/// Admitted bytes through the newest sample the maintainer has applied.
	applied_bytes: u64,
	/// The sequence of the newest admitted entry.
	last_seq: u64,
	/// `(seq, admitted bytes through seq)`, ascending, at least `SAMPLE_BYTES` apart.
	samples: VecDeque<(u64, u64)>,
	/// Until this boot time no new applied probe is sent.
	probe_after_boot_ms: u64,
}

impl UnappliedBacklog {
	/// Counts an admitted entry of `bytes` at `seq`.
	pub(super) fn admitted(&mut self, seq: u64, bytes: u64) {
		self.admitted_bytes = self.admitted_bytes.saturating_add(bytes);
		self.last_seq = self.last_seq.max(seq);
		let last = self
			.samples
			.back()
			.map_or(self.applied_bytes, |(_, total)| *total);
		if self.admitted_bytes.saturating_sub(last) >= SAMPLE_BYTES {
			self.samples.push_back((seq, self.admitted_bytes));
		}
	}

	/// The admitted bytes the maintainer has not reported applied, given its newest applied
	/// sequence. Never less than the true backlog.
	fn unapplied(&mut self, applied_seq: u64) -> u64 {
		while let Some((seq, total)) = self.samples.front().copied()
			&& seq <= applied_seq
		{
			self.applied_bytes = total;
			self.samples.pop_front();
		}
		if applied_seq >= self.last_seq {
			self.applied_bytes = self.admitted_bytes;
			self.samples.clear();
		}
		self.admitted_bytes.saturating_sub(self.applied_bytes)
	}
}

impl Filesystem {
	/// Sends one cheap read at the snapshot's floor, so the maintainer's reply raises the
	/// observed applied position. Its rows are not needed, and a refusal or timeout leaves the
	/// observed position as it is.
	pub(super) async fn observe_applied(&self, state: &State, deadline: u64) {
		let _ = state
			.cold
			.snapshot
			.get_inode(depot_fs_core::ROOT_INO, state.last_seq, deadline)
			.await;
	}

	/// Moves the floor to the newest layer the maintainer has reported applied, so the layers a
	/// probe just learned are applied stop counting against the backlog and the memory budget.
	fn release_applied_layers(&self, state: &mut State) {
		if state.staged_format.is_some() {
			return;
		}
		let Ok(status) = self.journal.status() else {
			return;
		};
		// A move to cold itself is the refresh's, which takes cold's own position.
		let floor = state
			.cold
			.snapshot
			.position()
			.volume_seq
			.max(status.cold.volume_seq);
		let applied = state.last_seq.min(self.cold_store.observed_applied());
		let Some(seq) = state
			.layers
			.iter()
			.map(|note| note.layer.seq)
			.filter(|seq| *seq > floor && *seq <= applied)
			.max()
		else {
			return;
		};
		let position = rivet_data::generated::shared_depot_types_v1::VolumeLogPosition {
			volume_seq: seq,
			client_epoch: entry_epoch(state, seq),
		};
		self.move_floor(state, position, "applied");
	}

	/// Refuses a new write before it is built when this opening's admitted bytes are more than
	/// `max_unapplied_bytes`, or its entries more than `MAX_HELD_LAYERS`, ahead of the
	/// maintainer's applied position. The refusal changed nothing, so the edge may send the
	/// write again after its hint. Writes already inside a transaction are never refused here:
	/// a transaction is admitted once, at its start.
	///
	/// The applied position comes from the maintainer's read replies. A writer that reads
	/// nothing would otherwise see a stale one, so a backlog over the cap, or many held layers,
	/// first sends one cheap read at the snapshot's floor to learn the current position, at
	/// most once per probe interval and bounded by `PROBE_TIMEOUT_MS`.
	pub(super) async fn shed_unapplied_backlog(
		&self,
		state: &mut State,
		deadline: u64,
	) -> Result<()> {
		let limit = self.config.max_unapplied_bytes as u64;
		let over = |state: &mut State, applied: u64| {
			state.backlog.unapplied(applied) > limit
				|| state.last_seq.saturating_sub(applied) > MAX_HELD_LAYERS as u64
		};
		// Held layers wait for a reply that reports them applied, which a writer that never
		// misses would not get, so many of them also call for a probe.
		let held = state
			.layers
			.iter()
			.filter(|note| note.deadline == NoteDeadline::UntilApplied)
			.count();
		let applied = self.cold_store.observed_applied();
		let over_now = over(state, applied);
		if !over_now && held <= PROBE_HELD_LAYERS {
			return Ok(());
		}
		let now = self.now()?.boot_ms;
		if now >= state.backlog.probe_after_boot_ms {
			state.backlog.probe_after_boot_ms = now.saturating_add(PROBE_INTERVAL_MS);
			let probe_deadline = deadline.min(now.saturating_add(PROBE_TIMEOUT_MS));
			self.observe_applied(state, probe_deadline).await;
			self.release_applied_layers(state);
		}
		let applied = self.cold_store.observed_applied();
		if !over(state, applied) {
			return Ok(());
		}
		crate::metrics::FS_OVERLOADED
			.with_label_values(&["unapplied_backlog"])
			.inc();
		tracing::debug!(
			key = ?self.key,
			limit,
			"refusing a filesystem write while the tablet's apply is behind; a slow cold store or compaction is likely holding back memtable flushes"
		);
		Err(Error::ApplyBacklog {
			retry_after_ms: BACKLOG_RETRY_AFTER_MS,
		})
	}
}
