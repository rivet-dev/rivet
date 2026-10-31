//! Discovery cursor and bounded sweeps are nested activities in the maintainer.
//!
//! Discovery recovers durable pending work after a takeover: forks, materializations,
//! fork cleanup, pending sweeps, and deletions (`PendingDeleteKey`) whose log replicas
//! were lost. It pages through every pending row once when the maintainer starts
//! serving, and once more after a member replacement. A live maintainer creates every
//! later pending row itself through a store operation, and the completion of such an
//! operation scans only the pending rows of the volume keys it touched, so the work per
//! write follows that write and not the number of pending rows on the tablet. Fork
//! checks are left out of those scans: each one has a deadline from the transition that
//! learned of it (`fork_cleanup`).
use super::{MaintainerPhase, MaintainerState, Operation, store::StoreWork};
use crate::tablet_maintainer_machine::types::MAX_ORPHAN_SCAN_KEYS;
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{MaintainerTimer, OpId},
};
use depot_fs_core::work::{PendingSweepProgress, PendingVolume, PendingWorkPage, SweepHold};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Discovery {
	/// The scan's next page waits for `PendingWorkTick`.
	Waiting(Scan),
	/// The store queue owns the scan's next page.
	Queued(Scan),
	/// One correlated storage operation reads the scan's page.
	Reading(Scan),
	/// No scan is due: the full pass ran and every volume in
	/// `MaintainerState::pending_volumes` was scanned since it was marked.
	Done,
}
impl Default for Discovery {
	fn default() -> Self {
		Self::Waiting(Scan::All { after: None })
	}
}

/// Which pending rows a discovery scan reads.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Scan {
	/// The pass over every pending row, continuing after the cursor.
	All { after: Option<Vec<u8>> },
	/// One volume key's pending rows other than fork checks, continuing after the cursor.
	Volume {
		volume: PendingVolume,
		after: Option<Vec<u8>>,
	},
}

impl Scan {
	fn after(&self) -> Option<Vec<u8>> {
		match self {
			Scan::All { after } | Scan::Volume { after, .. } => after.clone(),
		}
	}

	fn continued(&self, after: Option<Vec<u8>>) -> Self {
		match self {
			Scan::All { .. } => Scan::All { after },
			Scan::Volume { volume, .. } => Scan::Volume {
				volume: volume.clone(),
				after,
			},
		}
	}
}

pub(crate) fn wake(state: &mut MaintainerState, fx: &mut Effects) {
	if state.pending_work == Discovery::Done {
		return;
	}
	if state.phase == MaintainerPhase::Serving
		&& !state
			.operations
			.values()
			.any(|op| matches!(op, Operation::SweepPending { .. }))
		&& !state
			.store_queue
			.iter()
			.any(|w| matches!(w, StoreWork::SweepPending { .. }))
		&& let Discovery::Waiting(scan) = &state.pending_work
	{
		state.pending_work = Discovery::Queued(scan.clone());
		state.store_queue.push_back(StoreWork::PendingWork);
		return;
	}
	if matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Opening | MaintainerPhase::Frozen
	) {
		fx.arm_timer_after(
			MaintainerTimer::PendingWorkTick,
			state.config.tuning().retry_ms,
		);
	}
}
pub(crate) fn dispatch(state: &mut MaintainerState, fx: &mut Effects) {
	let Discovery::Queued(scan) = &state.pending_work else {
		return;
	};
	let scan = scan.clone();
	if state.phase != MaintainerPhase::Serving {
		state.pending_work = Discovery::Waiting(scan);
		fx.arm_timer_after(
			MaintainerTimer::PendingWorkTick,
			state.config.tuning().retry_ms,
		);
		return;
	}
	let after = scan.after();
	let volume = match &scan {
		Scan::All { .. } => None,
		Scan::Volume { volume, .. } => Some(volume.clone()),
	};
	state.pending_work = Discovery::Reading(scan);
	let op_id = state.begin(
		Operation::PendingWork {
			volume: volume.clone(),
		},
		fx,
	);
	fx.execute(MaintainerEffect::PendingWorkRequest {
		op_id,
		after,
		limit: MAX_ORPHAN_SCAN_KEYS,
		volume,
	});
}
pub(crate) fn scanned(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<PendingWorkPage, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::PendingWork { .. })
	) {
		return;
	}
	state.finish(id, fx);
	let Discovery::Reading(scan) = state.pending_work.clone() else {
		super::fence(state, fx);
		return;
	};
	let Ok(page) = result else {
		super::fence(state, fx);
		return;
	};
	if page.scopes.len() + page.forks.len() + page.cleanup.len() + page.materializations.len()
		> MAX_ORPHAN_SCAN_KEYS as usize
		|| page
			.scopes
			.iter()
			.any(|s| !state.tablet.hash_range.contains(s.hash))
	{
		super::fence(state, fx);
		return;
	}
	// A volume scan read in one page saw every materialization row of its volume.
	if let Scan::Volume {
		volume,
		after: None,
	} = &scan
		&& page.after.is_none()
	{
		super::materialize::volume_scanned(state, volume, &page.materializations, fx);
	}
	for scope in page.materializations {
		if super::materialize::discover(state, scope).is_err() {
			super::fence(state, fx);
			return;
		}
	}
	for work in page.cleanup {
		if super::fork_cleanup::discover(state, work).is_err() {
			super::fence(state, fx);
			return;
		}
	}
	for spec in page.forks {
		if let Err(()) = super::forks::discover(state, spec) {
			super::fence(state, fx);
			return;
		}
	}
	for scope in page.scopes {
		state
			.store_queue
			.push_back(StoreWork::SweepPending { scope });
	}
	// A deletion whose log replicas were lost resumes in the deletion drive while it has
	// cleanup slots. One that does not fit is found by scanning its rows again.
	for (volume_key, row) in page.deletes {
		if state.reserves_at_least(state.config.max_loaded_volumes as usize) {
			match &scan {
				Scan::All { .. } => state.pending_work_again = true,
				Scan::Volume { volume, .. } => {
					state.pending_volumes.insert(volume.clone());
				}
			}
			break;
		}
		super::deletion::observe_row(state, volume_key, row);
	}
	if page.after.is_some() {
		state.pending_work = Discovery::Waiting(scan.continued(page.after));
		fx.arm_timer_after(
			MaintainerTimer::PendingWorkTick,
			state.config.tuning().retry_ms,
		);
		return;
	}
	next_scan(state, fx);
}

/// Starts the next due scan: the full pass when one was asked for, paced by `retry_ms` like
/// its pages, else the next marked volume at once, else discovery is `Done`.
fn next_scan(state: &mut MaintainerState, fx: &mut Effects) {
	let (scan, delay_ms) = if std::mem::take(&mut state.pending_work_again) {
		(Scan::All { after: None }, state.config.tuning().retry_ms)
	} else if let Some(volume) = state.pending_volumes.pop_first() {
		let scan = Scan::Volume {
			volume,
			after: None,
		};
		(scan, 0)
	} else {
		match idle_hand_back(state) {
			Some(HandBack::Volume(volume)) => {
				crate::metrics::record_recovery_scan("pending_work", "idle_job");
				let scan = Scan::Volume {
					volume,
					after: None,
				};
				(scan, 0)
			}
			Some(HandBack::Pass) => {
				crate::metrics::record_recovery_scan("pending_work", "idle_job");
				(Scan::All { after: None }, state.config.tuning().retry_ms)
			}
			None => {
				state.pending_work = Discovery::Done;
				return;
			}
		}
	};
	state.pending_work = Discovery::Waiting(scan);
	fx.arm_timer_after(MaintainerTimer::PendingWorkTick, delay_ms);
}

/// The replica set's members changed, so deletions whose replicas were on a replaced
/// member are found by one more pass from the start.
pub(crate) fn members_replaced(state: &mut MaintainerState, fx: &mut Effects) {
	rescan_all(state, "replacement", fx);
}

/// Runs one more pass over every pending row from the start: after `retry_ms` when discovery
/// is done, else after the scan in progress. `trigger` is a code-defined metric label.
pub(crate) fn rescan_all(state: &mut MaintainerState, trigger: &'static str, fx: &mut Effects) {
	crate::metrics::record_recovery_scan("pending_work", trigger);
	match &state.pending_work {
		Discovery::Done => {
			state.pending_work = Discovery::Waiting(Scan::All { after: None });
			fx.arm_timer_after(
				MaintainerTimer::PendingWorkTick,
				state.config.tuning().retry_ms,
			);
		}
		Discovery::Waiting(Scan::All { after: None }) => {}
		Discovery::Waiting(_) | Discovery::Queued(_) | Discovery::Reading(_) => {
			state.pending_work_again = true;
		}
	}
}

/// A store operation that can write pending rows (a fork, a materialization, a fork
/// cleanup, a fork release or a checkpoint change) finished, touching the pending rows of
/// `volumes`. Each is scanned once, so the rows the operation wrote are swept without a
/// periodic scan. Marks of one volume made before its scan starts share that scan.
pub(crate) fn wrote(state: &mut MaintainerState, volumes: Vec<PendingVolume>, fx: &mut Effects) {
	if volumes.is_empty() {
		return;
	}
	state.pending_volumes.extend(volumes);
	if state.pending_work == Discovery::Done {
		crate::metrics::record_recovery_scan("pending_work", "write");
		next_scan(state, fx);
	}
}

/// Volumes whose pending work discovery found while every job of its kind was taken. A
/// serving maintainer runs no later pass over every pending row, so each job of that kind
/// that ends hands one volume back to discovery (`Skipped::job_ended`), which scans that
/// volume's pending rows alone. Every skip happened while every job was taken and each of
/// those jobs hands back one volume when it ends, so skipped work waits for a free job, never
/// for a pass that would not come.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Skipped {
	volumes: std::collections::BTreeSet<PendingVolume>,
	/// The volume handed back last. The next hand-back starts after it, so a volume that is
	/// skipped again whenever admissions refill the jobs does not keep the others waiting.
	after: Option<PendingVolume>,
	/// A skipped volume did not fit under the cap, so one more full pass runs once the set
	/// drains.
	overflowed: bool,
}

impl Skipped {
	/// Remembers that discovery skipped `volume`'s work. The set holds at most `cap` volumes.
	pub(crate) fn skip(&mut self, volume: PendingVolume, cap: usize) {
		if self.volumes.len() >= cap && !self.volumes.contains(&volume) {
			self.overflowed = true;
			return;
		}
		self.volumes.insert(volume);
	}

	/// `volume`'s work took a job, so it no longer waits for one.
	pub(crate) fn started(&mut self, volume: &PendingVolume) {
		self.volumes.remove(volume);
	}

	pub(crate) fn clear(&mut self) {
		*self = Self::default();
	}

	

	

	/// Takes the next skipped volume, round robin after the last one handed back, or the full
	/// pass an overflow asked for once no volume is left.
	fn hand_back(&mut self) -> Option<HandBack> {
		let next = match &self.after {
			Some(after) => self
				.volumes
				.range((
					std::ops::Bound::Excluded(after.clone()),
					std::ops::Bound::Unbounded,
				))
				.next()
				.or_else(|| self.volumes.first()),
			None => self.volumes.first(),
		}
		.cloned();
		if let Some(volume) = next {
			self.volumes.remove(&volume);
			self.after = Some(volume.clone());
			return Some(HandBack::Volume(volume));
		}
		if std::mem::take(&mut self.overflowed) {
			return Some(HandBack::Pass);
		}
		None
	}

	/// A job of this kind ended: scans the next skipped volume's pending rows, or runs the
	/// full pass an overflow asked for. `trigger` is a code-defined metric label.
	pub(crate) fn job_ended(
		state: &mut MaintainerState,
		skipped: fn(&mut MaintainerState) -> &mut Skipped,
		trigger: &'static str,
		fx: &mut Effects,
	) {
		if state.phase != MaintainerPhase::Serving {
			return;
		}
		match skipped(state).hand_back() {
			Some(HandBack::Volume(volume)) => scan_volume(state, volume, trigger, fx),
			Some(HandBack::Pass) => rescan_all(state, trigger, fx),
			None => {}
		}
	}
}

/// Scans `volume`'s pending rows, after the scan in progress if there is one. `trigger` is a
/// code-defined metric label.
pub(crate) fn scan_volume(
	state: &mut MaintainerState,
	volume: PendingVolume,
	trigger: &'static str,
	fx: &mut Effects,
) {
	state.pending_volumes.insert(volume);
	if state.pending_work == Discovery::Done {
		crate::metrics::record_recovery_scan("pending_work", trigger);
		next_scan(state, fx);
	}
}

/// What a [`Skipped`] set gives back to discovery.
enum HandBack {
	/// Scan this volume's pending rows.
	Volume(PendingVolume),
	/// Run one more pass over every pending row.
	Pass,
}

/// Skipped work whose kind has a free job, for discovery to scan before it goes idle. A
/// handed-back volume's scan may start no job (its row is gone, or another scan started it),
/// so a free job could otherwise wait for a job end that never comes.
fn idle_hand_back(state: &mut MaintainerState) -> Option<HandBack> {
	if state.phase != MaintainerPhase::Serving {
		return None;
	}
	if super::forks::has_free_job(state)
		&& let Some(hand_back) = state.forks.skipped.hand_back()
	{
		return Some(hand_back);
	}
	if super::materialize::has_free_job(state) {
		return super::materialize::skipped(state).hand_back();
	}
	None
}

pub(crate) fn swept(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<PendingSweepProgress, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::SweepPending { .. })
	) {
		return;
	}
	let Some(Operation::SweepPending { scope }) = state.finish(id, fx) else {
		unreachable!()
	};
	match result {
		Ok(PendingSweepProgress::More) => {
			state.held_sweeps.remove(&scope);
			if state.phase == MaintainerPhase::Serving {
				state
					.store_queue
					.push_back(StoreWork::SweepPending { scope });
			}
		}
		Ok(PendingSweepProgress::Complete) => {
			state.held_sweeps.remove(&scope);
		}
		Ok(PendingSweepProgress::Held(SweepHold::Checkpoint)) => {
			// A checkpoint or fork hold may keep the old incarnation for ever, so the sweep
			// leaves memory. Its pending sweep row stays, and the store operation that
			// releases the last hold touches the volume's pending rows, whose scan queues
			// the sweep again.
			state.held_sweeps.remove(&scope);
			if state.held_sweeps.is_empty() {
				cancel_retry(state, fx);
			}
		}
		Ok(PendingSweepProgress::Held(SweepHold::ReadSession)) => {
			// Read sessions expire inside the store without an event, so a retry deadline
			// drives the sweep again.
			state.held_sweeps.insert(scope);
			arm_retry(state, fx);
		}
		Err(error) => {
			tracing::warn!(%error, ?scope, "pending sweep failed in the store; fencing the maintainer");
			super::fence(state, fx)
		}
	}
}

/// Releases the held sweeps of `volume_key`'s incarnations once its replay ended, while other
/// volumes still replay.
pub(crate) fn release_held_of(
	state: &mut MaintainerState,
	volume_key: &depot_types::VolumeKey,
	fx: &mut Effects,
) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let probe = depot_fs_core::keys::slatedb::Scope::new(volume_key, uuid::Uuid::nil());
	let released = state
		.held_sweeps
		.iter()
		.filter(|scope| scope.hash == probe.hash && scope.name == probe.name)
		.cloned()
		.collect::<Vec<_>>();
	for scope in released {
		state.held_sweeps.remove(&scope);
		let work = StoreWork::SweepPending { scope };
		if !state.store_queue.contains(&work) {
			state.store_queue.push_back(work);
		}
	}
	if state.held_sweeps.is_empty() {
		cancel_retry(state, fx);
	}
}

/// Arms `HeldSweepRetry` unless it is armed already, so a later hold never postpones the
/// retry an earlier one is waiting for.
fn arm_retry(state: &mut MaintainerState, fx: &mut Effects) {
	if state.held_sweep_retry_armed {
		return;
	}
	state.held_sweep_retry_armed = true;
	fx.arm_timer_after(
		MaintainerTimer::HeldSweepRetry,
		state.config.tuning().held_sweep_retry_ms,
	);
}

fn cancel_retry(state: &mut MaintainerState, fx: &mut Effects) {
	state.held_sweep_retry_armed = false;
	fx.cancel_timer(MaintainerTimer::HeldSweepRetry);
}

/// Queues every sweep waiting in memory again: those held by a read session, on their retry
/// deadline, and those that waited for replay, once it ended. A sweep that is still held
/// reports `Held` again.
pub(crate) fn release_held(state: &mut MaintainerState, fx: &mut Effects) {
	if state.held_sweeps.is_empty() {
		return;
	}
	if state.phase != MaintainerPhase::Serving {
		arm_retry(state, fx);
		return;
	}
	cancel_retry(state, fx);
	for scope in std::mem::take(&mut state.held_sweeps) {
		let work = StoreWork::SweepPending { scope };
		if !state.store_queue.contains(&work) {
			state.store_queue.push_back(work);
		}
	}
}
