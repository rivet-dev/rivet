//! Signal events a tablet maintainer emits under the `depot::signal` tracing target: bytes
//! above cold, apply lag, the replay after a grant, learner phases and refusals to clients.
//! Each reports a value an operator or a simulation oracle judges the maintainer by.
//! Messages and field names are stable, because traces are matched on them. The matching
//! gauges sum each maintainer's contribution through guards it holds, so they stay right
//! across many tablets and abrupt task ends.

use crate::tablet_maintainer_machine::{MaintainerSnapshot, state::MaintainerPhase};
use rivet_data::generated::{
	depot_client_protocol_v1::VolumeRejectResponse, depot_log_shard_v1::VolumeLogEntry,
	shared_depot_types_v1::TabletPhase,
};
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
};
use uuid::Uuid;

/// The tracing target of every signal event.
pub(crate) const TARGET: &str = "depot::signal";

/// Length of one apply lag window. Each window ends with one `maintainer apply lag` event.
const APPLY_WINDOW_MS: u64 = 1_000;

/// A replay ends once a full apply window's oldest entries were all written this recently,
/// even if writes keep apply busy.
const CAUGHT_UP_LAG_MS: u64 = 1_000;

/// Least time between two `tablet above cold bytes` events of one tablet.
const ABOVE_COLD_EVENT_SPACING_MS: u64 = 1_000;

/// Marks an apply window that has not started, or a lag no window has measured yet.
const NONE: u64 = u64::MAX;

/// Reports one refusal a maintainer answered to its client, with the retry hint that
/// not-maintainer and overload refusals carry.
pub(crate) fn volume_request_refused(reason: &VolumeRejectResponse) {
	let (label, retry_after_ms) = match reason {
		VolumeRejectResponse::NotMaintainer(refusal) => {
			("not_maintainer", Some(refusal.retry_after_ms))
		}
		VolumeRejectResponse::TabletSplitting => ("tablet_splitting", None),
		VolumeRejectResponse::VolumeNotFound => ("volume_not_found", None),
		VolumeRejectResponse::VolumeExists => ("volume_exists", None),
		VolumeRejectResponse::VolumeDeleted => ("volume_deleted", None),
		VolumeRejectResponse::Overloaded(refusal) => ("overloaded", Some(refusal.retry_after_ms)),
		VolumeRejectResponse::VolumeRecordMissing(_) => ("record_missing", None),
		VolumeRejectResponse::VolumeUnderRepair(refusal) => {
			("under_repair", Some(refusal.retry_after_ms))
		}
	};
	crate::metrics::TABLET_VOLUME_REFUSALS
		.with_label_values(&[label])
		.inc();
	tracing::debug!(
		target: TARGET,
		layer = "maintainer",
		reason = label,
		retry_after_ms = retry_after_ms.unwrap_or(0),
		has_retry_after = retry_after_ms.is_some(),
		"request refused"
	);
}

/// Apply observations shared by one maintainer's apply tasks and its signal observer.
pub(crate) struct ApplySignals {
	tablet_id: Uuid,
	/// Boot time the current window started, or `NONE` before the first apply.
	window_started_boot_ms: AtomicU64,
	window_max_lag_ms: AtomicU64,
	window_entries: AtomicU64,
	window_bytes: AtomicU64,
	/// The oldest-entry lag of the last finished window, or `NONE`.
	last_window_max_lag_ms: AtomicU64,
	applied_entries: AtomicU64,
	applied_bytes: AtomicU64,
}

impl ApplySignals {
	pub(crate) fn new(tablet_id: [u8; 16]) -> Arc<Self> {
		Arc::new(Self {
			tablet_id: Uuid::from_bytes(tablet_id),
			window_started_boot_ms: AtomicU64::new(NONE),
			window_max_lag_ms: AtomicU64::new(0),
			window_entries: AtomicU64::new(0),
			window_bytes: AtomicU64::new(0),
			last_window_max_lag_ms: AtomicU64::new(NONE),
			applied_entries: AtomicU64::new(0),
			applied_bytes: AtomicU64::new(0),
		})
	}

	/// Records one finished apply. Its lag is the time since the writer wrote its oldest
	/// entry, by the writer's wall clock, so clock skew shifts it.
	pub(crate) fn record_apply(&self, entries: &[VolumeLogEntry], clocks: rivet_env::Clocks) {
		let Some(oldest) = entries.first() else {
			return;
		};
		let lag_ms =
			u64::try_from(clocks.unix_ms.saturating_sub(oldest.written_at_unix_ms)).unwrap_or(0);
		let bytes = entries
			.iter()
			.map(depot_types::log_shard::entry_bytes)
			.sum::<u64>();
		let count = entries.len() as u64;
		crate::metrics::MAINTAINER_APPLY_LAG.observe(lag_ms as f64 / 1000.0);
		self.applied_entries.fetch_add(count, Ordering::Relaxed);
		self.applied_bytes.fetch_add(bytes, Ordering::Relaxed);
		self.window_max_lag_ms.fetch_max(lag_ms, Ordering::Relaxed);
		self.window_entries.fetch_add(count, Ordering::Relaxed);
		self.window_bytes.fetch_add(bytes, Ordering::Relaxed);

		// The apply that finds the window over closes it; a racing apply loses the exchange.
		let started = self.window_started_boot_ms.load(Ordering::Relaxed);
		let now = clocks.boot_ms;
		if started == NONE {
			let _ = self.window_started_boot_ms.compare_exchange(
				NONE,
				now,
				Ordering::Relaxed,
				Ordering::Relaxed,
			);
			return;
		}
		let window_ms = now.saturating_sub(started);
		if window_ms < APPLY_WINDOW_MS {
			return;
		}
		if self
			.window_started_boot_ms
			.compare_exchange(started, now, Ordering::Relaxed, Ordering::Relaxed)
			.is_err()
		{
			return;
		}
		let max_lag_ms = self.window_max_lag_ms.swap(0, Ordering::Relaxed);
		let window_entries = self.window_entries.swap(0, Ordering::Relaxed);
		let window_bytes = self.window_bytes.swap(0, Ordering::Relaxed);
		self.last_window_max_lag_ms
			.store(max_lag_ms, Ordering::Relaxed);
		tracing::debug!(
			target: TARGET,
			tablet_id = %self.tablet_id,
			lag_ms = max_lag_ms,
			lag_entries = window_entries,
			lag_bytes = window_bytes,
			window_ms,
			"maintainer apply lag"
		);
	}
}

/// Follows one maintainer's published snapshots and emits the events and gauges that
/// describe its replay, learner phases and bytes above cold. Owned by the snapshot observer
/// task; `close` reports anything still open when the maintainer ends.
pub(crate) struct SignalObserver {
	tablet_id: Uuid,
	tablet_grant_epoch: u64,
	apply: Arc<ApplySignals>,
	above_cold: Option<(u64, u64)>,
	_above_cold_gauge: Option<IntGaugeGuard>,
	replay: Replay,
	learner: Option<LearnerPhase>,
	_uncovered_gauge: Option<IntGaugeGuard>,
}

enum Replay {
	NotStarted,
	Running {
		began_boot_ms: u64,
		applied_entries: u64,
		applied_bytes: u64,
		peak_above_cold_bytes: u64,
		_replaying: IntGaugeGuard,
	},
	Ended,
}

struct LearnerPhase {
	worker: [u8; 16],
	began_boot_ms: u64,
}

impl SignalObserver {
	pub(crate) fn new(tablet_grant_epoch: u64, apply: Arc<ApplySignals>) -> Self {
		Self {
			tablet_id: apply.tablet_id,
			tablet_grant_epoch,
			apply,
			above_cold: None,
			_above_cold_gauge: None,
			replay: Replay::NotStarted,
			learner: None,
			_uncovered_gauge: None,
		}
	}

	pub(crate) fn observe(&mut self, snapshot: &MaintainerSnapshot, now_boot_ms: u64) {
		self.observe_above_cold(snapshot, now_boot_ms);
		self.observe_replay(snapshot, now_boot_ms);
		self.observe_learner(snapshot, now_boot_ms);
	}

	/// Ends whatever is still open, as the maintainer stops.
	pub(crate) fn close(&mut self, now_boot_ms: u64) {
		self.finish_replay("interrupted", now_boot_ms);
		if let Some(phase) = self.learner.take() {
			self.finish_learner(phase, "interrupted", 0, now_boot_ms);
		}
		self._above_cold_gauge = None;
		self._uncovered_gauge = None;
	}

	fn observe_above_cold(&mut self, snapshot: &MaintainerSnapshot, now_boot_ms: u64) {
		let bytes = snapshot.signals.above_cold_bytes;
		let due = match self.above_cold {
			None => true,
			Some((last_bytes, last_boot_ms)) => {
				last_bytes != bytes
					&& now_boot_ms.saturating_sub(last_boot_ms) >= ABOVE_COLD_EVENT_SPACING_MS
			}
		};
		if !due {
			return;
		}
		self.above_cold = Some((bytes, now_boot_ms));
		self._above_cold_gauge = Some(
			crate::metrics::TABLET_ABOVE_COLD_BYTES
				.add_guard(i64::try_from(bytes).unwrap_or(i64::MAX)),
		);
		tracing::debug!(
			target: TARGET,
			tablet_id = %self.tablet_id,
			above_cold_bytes = bytes,
			"tablet above cold bytes"
		);
	}

	/// A replay starts when the newly granted maintainer starts serving and ends once its
	/// recovered maintainer log ops applied and apply drained or kept within
	/// `CAUGHT_UP_LAG_MS` for a whole window.
	fn observe_replay(&mut self, snapshot: &MaintainerSnapshot, now_boot_ms: u64) {
		let serving = matches!(
			snapshot.phase,
			MaintainerPhase::Serving | MaintainerPhase::Frozen
		);
		let above_cold_bytes = snapshot.signals.above_cold_bytes;
		match &mut self.replay {
			Replay::NotStarted if serving => {
				tracing::info!(
					target: TARGET,
					tablet_id = %self.tablet_id,
					reason = self.reason(),
					tablet_grant_epoch = self.tablet_grant_epoch,
					loaded_volumes = snapshot.loaded_volumes,
					above_cold_bytes,
					"maintainer replay began"
				);
				self.replay = Replay::Running {
					began_boot_ms: now_boot_ms,
					applied_entries: self.apply.applied_entries.load(Ordering::Relaxed),
					applied_bytes: self.apply.applied_bytes.load(Ordering::Relaxed),
					peak_above_cold_bytes: above_cold_bytes,
					_replaying: crate::metrics::MAINTAINER_REPLAYING.inc_guard(),
				};
			}
			Replay::NotStarted | Replay::Ended => {}
			Replay::Running {
				peak_above_cold_bytes,
				..
			} => {
				*peak_above_cold_bytes = (*peak_above_cold_bytes).max(above_cold_bytes);
				let lag = self.apply.last_window_max_lag_ms.load(Ordering::Relaxed);
				let drained = snapshot.signals.apply_idle || lag < CAUGHT_UP_LAG_MS;
				if !serving {
					self.finish_replay("interrupted", now_boot_ms);
				} else if !snapshot.signals.maintainer_log_replaying && drained {
					self.finish_replay("caught_up", now_boot_ms);
				}
			}
		}
	}

	fn finish_replay(&mut self, result: &'static str, now_boot_ms: u64) {
		let Replay::Running {
			began_boot_ms,
			applied_entries,
			applied_bytes,
			peak_above_cold_bytes,
			..
		} = std::mem::replace(&mut self.replay, Replay::Ended)
		else {
			return;
		};
		let duration_ms = now_boot_ms.saturating_sub(began_boot_ms);
		crate::metrics::MAINTAINER_REPLAYS
			.with_label_values(&[result])
			.inc();
		crate::metrics::MAINTAINER_REPLAY_SECONDS.observe(duration_ms as f64 / 1000.0);
		tracing::info!(
			target: TARGET,
			tablet_id = %self.tablet_id,
			reason = self.reason(),
			result,
			duration_ms,
			entries = self
				.apply
				.applied_entries
				.load(Ordering::Relaxed)
				.saturating_sub(applied_entries),
			bytes = self
				.apply
				.applied_bytes
				.load(Ordering::Relaxed)
				.saturating_sub(applied_bytes),
			peak_above_cold_bytes,
			"maintainer replay finished"
		);
	}

	/// Why this maintainer replays: the tablet's first grant, or any later one, which a
	/// move, a restart and a fenced maintainer's regrant all take.
	fn reason(&self) -> &'static str {
		if self.tablet_grant_epoch <= 1 {
			"first_grant"
		} else {
			"regrant"
		}
	}

	fn observe_learner(&mut self, snapshot: &MaintainerSnapshot, now_boot_ms: u64) {
		let replica_set = &snapshot.replica_set;
		let learning = match &replica_set.phase {
			TabletPhase::Learning(_) => snapshot.signals.learner,
			TabletPhase::Stable | TabletPhase::Joint(_) => None,
		};
		let uncovered = snapshot.signals.uncovered_volumes;

		if let Some(phase) = self.learner.take() {
			if learning == Some(phase.worker) {
				self.learner = Some(phase);
			} else {
				let promoted = match &replica_set.phase {
					TabletPhase::Joint(joint) => joint.next.contains(&phase.worker),
					TabletPhase::Stable | TabletPhase::Learning(_) => {
						replica_set.members.contains(&phase.worker)
					}
				};
				let result = if promoted { "promoted" } else { "aborted" };
				self.finish_learner(phase, result, uncovered, now_boot_ms);
			}
		}

		if self.learner.is_none()
			&& let Some(worker) = learning
		{
			tracing::info!(
				target: TARGET,
				tablet_id = %self.tablet_id,
				learner_worker_id = %Uuid::from_bytes(worker),
				tablet_replica_set_version = replica_set.tablet_replica_set_version,
				uncovered_volumes = uncovered,
				loaded_volumes = snapshot.loaded_volumes,
				above_cold_bytes = snapshot.signals.above_cold_bytes,
				"learner phase began"
			);
			self.learner = Some(LearnerPhase {
				worker,
				began_boot_ms: now_boot_ms,
			});
		}

		self._uncovered_gauge = match self.learner {
			Some(_) => {
				Some(crate::metrics::LEARNER_UNCOVERED_VOLUMES.add_guard(i64::from(uncovered)))
			}
			None => None,
		};
	}

	fn finish_learner(
		&self,
		phase: LearnerPhase,
		result: &'static str,
		uncovered_volumes: u32,
		now_boot_ms: u64,
	) {
		let duration_ms = now_boot_ms.saturating_sub(phase.began_boot_ms);
		crate::metrics::LEARNER_PHASES
			.with_label_values(&[result])
			.inc();
		crate::metrics::LEARNER_PHASE_SECONDS.observe(duration_ms as f64 / 1000.0);
		tracing::info!(
			target: TARGET,
			tablet_id = %self.tablet_id,
			learner_worker_id = %Uuid::from_bytes(phase.worker),
			result,
			duration_ms,
			uncovered_volumes,
			"learner phase finished"
		);
	}
}


