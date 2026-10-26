//! Finite membership goals and existing-client matching under the admitted grant/version.

use super::*;
use crate::tablet_maintainer_machine::state::{
	LoadedVolume, filesystem, holders, maintainer_log, membership, poll, promotion, repair,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Goal {
	/// Fixed phase prefix; ordinary writes and cold publication do not extend it.
	observed: VolumeLogPosition,
	/// Captured durable/committed floor. Matching can never discard this prefix.
	committed_floor: u64,
	match_after_boot_ms: u64,
	/// When membership recovery may claim a held volume whose writer still looks live.
	holder_wait_until_boot_ms: u64,
	verification: Option<poll::VolumeBarrier>,
}

fn discovered(state: &MaintainerState) -> bool {
	if matches!(state.tablet.replica_set.phase, TabletPhase::Joint(_)) {
		return promotion::observed(state);
	}
	matches!(state.tablet.replica_set.phase, TabletPhase::Learning(_))
		&& state
			.tablet
			.replica_set
			.members
			.iter()
			.filter(|id| state.listed.contains(&WorkerId::from_bytes(**id)))
			.count() > state.tablet.replica_set.members.len() / 2
}

fn snapshot_goal(state: &MaintainerState, key: &VolumeLogKey, volume: &LoadedVolume) -> Goal {
	let cold = volume.row(key).map_or(
		VolumeLogPosition {
			volume_seq: 0,
			client_epoch: 0,
		},
		|row| row.cold,
	);
	let committed_floor =
		super::super::filesystem::known_committed(state, key).max(cold.volume_seq);
	let observed = if matches!(state.tablet.replica_set.phase, TabletPhase::Joint(_)) {
		promotion::frontier_position(state, key)
	} else {
		VolumeLogPosition {
			volume_seq: committed_floor,
			client_epoch: cold.client_epoch,
		}
	};
	Goal {
		observed,
		committed_floor,
		match_after_boot_ms: state
			.now_boot_ms
			.saturating_add(state.config.tuning().repair_quiet_ms),
		holder_wait_until_boot_ms: state
			.now_boot_ms
			.saturating_add(state.config.tuning().membership_holder_wait_ms),
		verification: None,
	}
}

/// Coverage is indexed by the keys observations changed. Only discovery or a change
/// to complete-member evidence starts a bounded traversal of all loaded keys.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Checks {
	/// A disk member is draining. Uncovered holders get a shorter matching allowance
	/// in this phase; assignment replacement resets it with the coverage index.
	stop_requested: bool,
	/// Complete-listing bitmap; membership is immutable until assignment::update
	/// resets this index.
	evidence: Option<u16>,
	/// Cursor for rebuilding coverage after global discovery evidence changes.
	scan: Scan,
	/// Observation/lifecycle handlers mark only the keys they changed.
	dirty: std::collections::BTreeSet<VolumeLogKey>,
	/// Evaluated keys still missing their finite phase proof.
	uncovered: std::collections::BTreeSet<VolumeLogKey>,
	/// Matching visits at most CHECK_BUDGET uncovered keys per drive.
	matching_after: Option<VolumeLogKey>,
	/// A copy round visited every loaded volume under this assignment. A new learner
	/// needs every volume loaded once; afterwards only keys still missing their phase
	/// proof can need a copy, so later rounds visit `uncovered` alone.
	copy_swept: bool,
	/// When a round blocked by lifecycle work may next ask for a tiering pass. See
	/// [`pass_for_lifecycle`].
	lifecycle_pass_after_boot_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Scan {
	#[default]
	Start,
	After(VolumeLogKey),
	Complete,
}

impl Checks {
	/// Evaluated volumes still missing this phase's proof.
	pub(crate) fn uncovered_count(&self) -> usize {
		self.uncovered.len()
	}
}

pub(super) const CHECK_BUDGET: usize = 128;

/// Whether a copy round already visited every loaded volume under this assignment.
pub(super) fn copy_swept(state: &MaintainerState) -> bool {
	state.membership_checks.copy_swept
}

/// Asks for one throttled tiering pass when a round is not ready because lifecycle work
/// waits for one, at most once per `memtable_flush_throttle_ms`.
///
/// This is the only cold write catch-up ever requests, and it exists for liveness, not
/// durability. A report needs every unload in the uncovered set closed and no deletion in
/// flight anywhere in the tablet. Both wait for an ordinary pass: a closed volume's unload
/// for the pass that carries its record, and a deletion for the pass that makes its staged
/// row cold. Left alone, that pass comes only at `write_to_cold_after_ms`, and a volume
/// closed during Learning holds the report for that long; under reopen or delete churn
/// Learning could then run into its deadline and a drain past its grace.
///
/// The request is the one a volume makes when its data comes due: the blocking unloads'
/// volumes ask for the next throttled flush, and deletions, or unloads a maintainer log op
/// still names, ask for the maintainer log's pass behind a cold pass that stages deleted
/// rows. Every request coalesces into the throttled flush, so a replacement under churn
/// adds at most one L0 SST per throttle window, and one with no closes or deletions in
/// flight adds none.
pub(super) fn pass_for_lifecycle(state: &mut MaintainerState, fx: &mut Effects) {
	let now_boot_ms = fx.now_boot_ms();
	if now_boot_ms < state.membership_checks.lifecycle_pass_after_boot_ms {
		return;
	}
	// Walk the smaller of the two sets; this runs at most once per throttle window.
	let uncovered = &state.membership_checks.uncovered;
	let blocking = |key: &&VolumeLogKey| filesystem::owns(state, key);
	let unloads: Vec<VolumeLogKey> = if uncovered.len() <= state.unloading.len() {
		uncovered
			.iter()
			.filter(|key| state.unloading.contains_key(*key))
			.filter(blocking)
			.take(CHECK_BUDGET)
			.cloned()
			.collect()
	} else {
		state
			.unloading
			.keys()
			.filter(|key| uncovered.contains(*key))
			.filter(blocking)
			.take(CHECK_BUDGET)
			.cloned()
			.collect()
	};
	if unloads.is_empty() && state.deleting.is_empty() {
		return;
	}
	state.membership_checks.lifecycle_pass_after_boot_ms =
		now_boot_ms.saturating_add(state.config.memtable_flush_throttle_ms);

	// A closed volume's record rides the next flush that carries the volume.
	for key in &unloads {
		filesystem::request_due_flush(state, key, fx);
	}

	// A deletion's row is staged by a cold pass and made cold by the next pass after it. An
	// unload whose volume a maintainer log op still names waits for the log's pass to trim it,
	// which a volume with nothing left to flush does not bring.
	let log_names_unload = unloads
		.iter()
		.any(|key| maintainer_log::names_volume(state, key));
	if !state.deleting.is_empty() && !state.pending_rows.is_empty() {
		super::super::store::queue_cold_pass(state);
	}
	if !state.deleting.is_empty() || log_names_unload {
		maintainer_log::request_pass(state, fx);
	}
}

pub(super) fn mark_copy_swept(state: &mut MaintainerState) {
	state.membership_checks.copy_swept = true;
}

/// The first key after `after` that a copy round visits: every loaded volume in the
/// first round under an assignment, then only evaluated keys still missing their proof.
/// Covered keys need no copy, so their number never adds work to a later round.
pub(super) fn next_copy_key(
	state: &MaintainerState,
	after: Option<&VolumeLogKey>,
) -> Option<VolumeLogKey> {
	let lower = after.map_or(Unbounded, Excluded);
	if copy_swept(state) {
		state
			.membership_checks
			.uncovered
			.range((lower, Unbounded))
			.next()
			.cloned()
	} else {
		state
			.loaded_volumes
			.range((lower, Unbounded))
			.next()
			.map(|(key, _)| key.clone())
	}
}

/// A normal holder allowance outlasts a pod drain. Keep the existing recovery
/// protocol, but reserve time for ownership transfer and cleanup: give a writer that never matches fifteen seconds before claiming
/// it. The renewable goal deadline remains the single source of its time origin.
const STOP_HOLDER_WAIT_MS: u64 = 15_000;

pub(crate) fn stop_requested(state: &mut MaintainerState, worker: &Id) {
	if state.tablet.replica_set.phase != TabletPhase::Stable
		&& state
			.tablet
			.replica_set
			.disks
			.iter()
			.any(|disk| &disk.worker_id == worker)
	{
		state.membership_checks.stop_requested = true;
	}
}

fn holder_wait_until(state: &MaintainerState, goal: &Goal) -> u64 {
	if state.membership_checks.stop_requested {
		goal.holder_wait_until_boot_ms
			.saturating_sub(state.config.tuning().membership_holder_wait_ms)
			.saturating_add(
				state
					.config
					.tuning()
					.membership_holder_wait_ms
					.min(STOP_HOLDER_WAIT_MS),
			)
	} else {
		goal.holder_wait_until_boot_ms
	}
}

fn evidence(state: &MaintainerState) -> u16 {
	let mut bits = 0;
	for (i, worker) in state.members().iter().enumerate() {
		let observed = if matches!(state.tablet.replica_set.phase, TabletPhase::Joint(_)) {
			promotion::member_observed(state, worker.as_bytes())
		} else {
			state.listed.contains(worker)
		};
		if observed {
			bits |= 1 << i;
		}
	}
	bits
}

pub(crate) fn mark(state: &mut MaintainerState, key: &VolumeLogKey) {
	if !matches!(state.tablet.replica_set.phase, TabletPhase::Stable)
		&& state.loaded_volumes.contains_key(key)
	{
		state.membership_checks.dirty.insert(key.clone());
	}
}

pub(crate) fn untrack(state: &mut MaintainerState, key: &VolumeLogKey) {
	state.membership_goals.remove(key);
	state.membership_checks.dirty.remove(key);
	state.membership_checks.uncovered.remove(key);
}

pub(super) fn pending(state: &MaintainerState) -> bool {
	discovered(state)
		&& (!matches!(state.membership_checks.scan, Scan::Complete)
			|| !state.membership_checks.dirty.is_empty())
}

/// Returns the number of keys checked; unchanged events do no volume traversal.
pub(super) fn capture(state: &mut MaintainerState) -> usize {
	if !discovered(state) {
		return 0;
	}
	let evidence = evidence(state);
	if state.membership_checks.evidence != Some(evidence) {
		state.membership_checks.evidence = Some(evidence);
		state.membership_checks.scan = Scan::Start;
	}
	let mut checked = 0;
	while checked < CHECK_BUDGET {
		let next = match &state.membership_checks.scan {
			Scan::Start => state.loaded_volumes.first_key_value(),
			Scan::After(key) => state
				.loaded_volumes
				.range((Excluded(key), Unbounded))
				.next(),
			Scan::Complete => None,
		}
		.map(|(key, _)| key.clone());
		let key = if let Some(key) = next {
			state.membership_checks.scan = Scan::After(key.clone());
			state.membership_checks.dirty.remove(&key);
			key
		} else {
			state.membership_checks.scan = Scan::Complete;
			let Some(key) = state.membership_checks.dirty.pop_first() else {
				break;
			};
			key
		};
		checked += 1;
		let Some(volume) = state.loaded_volumes.get(&key) else {
			untrack(state, &key);
			continue;
		};
		if !state.membership_goals.contains_key(&key) {
			let goal = snapshot_goal(state, &key, volume);
			state.membership_goals.insert(key.clone(), goal);
		}
		if phase_covered(state, &key, &state.membership_goals[&key]) {
			state.membership_checks.uncovered.remove(&key);
		} else {
			state.membership_checks.uncovered.insert(key);
		}
	}
	checked
}

fn covered(state: &MaintainerState, key: &VolumeLogKey, worker: &Id, goal: &Goal) -> bool {
	let Some(volume) = state.loaded_volumes.get(key) else {
		return false;
	};
	let Some(_) = volume
		.row(key)
		.filter(|row| row.status == FsVolumeStatus::Live)
	else {
		return false;
	};
	let observed = if matches!(state.tablet.replica_set.phase, TabletPhase::Joint(_)) {
		promotion::member_observed(state, worker)
	} else {
		state.listed.contains(&WorkerId::from_bytes(*worker))
	};
	observed
		&& volume
			.members
			.get(&WorkerId::from_bytes(*worker))
			.is_some_and(|member| {
				// The member holds the goal's entries; the goal itself was proven committed
				// when it was set, so the member's own commit knowledge is not needed.
				member.status == Status::InSync
					&& member.appended.volume_seq
						>= goal.observed.volume_seq.max(goal.committed_floor)
					&& member.appended.client_epoch >= goal.observed.client_epoch
					&& (!matches!(state.tablet.replica_set.phase, TabletPhase::Learning(_))
						|| state.tablet.replica_set.members.contains(worker)
						|| member.learner_replica_set_version
							>= state.tablet.replica_set.tablet_replica_set_version)
			})
}
type Id = [u8; 16];

fn next_covered(state: &MaintainerState, key: &VolumeLogKey, goal: &Goal) -> bool {
	let next = match &state.tablet.replica_set.phase {
		TabletPhase::Learning(p) => &p.next,
		TabletPhase::Joint(p) => &p.next,
		TabletPhase::Stable => return false,
	};
	goal.verification
		.as_ref()
		.is_none_or(|barrier| poll::volume_refreshed(state, barrier))
		&& next
			.iter()
			.filter(|worker| covered(state, key, worker, goal))
			.count() > next.len() / 2
}

// The learner must enter the voting history before promotion, or the replacement
// leaves only the retained replicas holding the volume. A learner that cannot be
// reached is unknown, never covered: Learning waits for it, and the coordinator's
// Learning deadline aborts a change whose learner never catches up.
fn phase_covered(state: &MaintainerState, key: &VolumeLogKey, goal: &Goal) -> bool {
	if state.unloading.contains_key(key)
		&& super::super::unload::closed_through(state, key)
			.is_some_and(|cold| cold.volume_seq >= goal.committed_floor)
	{
		return true;
	}
	// A newer writer may cover this phase while an older close still owns
	// cleanup on a missing member. Its obsolete certificate cannot veto fresh
	// quorum evidence; conditional forgets cannot remove the newer prefix.
	next_covered(state, key, goal)
		&& (!matches!(state.tablet.replica_set.phase, TabletPhase::Learning(_))
			|| learner(state).is_none_or(|worker| covered(state, key, worker.as_bytes(), goal)))
}

/// Failed extra listings and redundant loads cannot veto a complete quorum proof.
/// Pending lifecycle requests still hold their barriers; sent loads retain their
/// uncertain outcomes and are never counted as coverage until observed InSync.
pub(super) fn ready_to_report(state: &MaintainerState) -> bool {
	let counts = state.operations.index().counts;
	discovered(state)
		&& state.pending.is_empty()
		// Uncertified unloads remain in the per-key uncovered index. Certified
		// closes retain their outstanding cleanup without blocking membership.
		&& state.deleting.is_empty()
		&& counts.recover_volume == 0
		&& counts.forget_orphan == 0
		&& state.membership_checks.evidence == Some(evidence(state))
		&& matches!(state.membership_checks.scan, Scan::Complete)
		&& state.membership_checks.dirty.is_empty()
		&& state.membership_checks.uncovered.is_empty()
		&& state.membership_goals.len() == state.loaded_volumes.len()
		&& maintainer_log::covers_destinations(state)
}

/// A membership operation selects existing bounded matching, without another recovery service.
pub(super) fn match_members(state: &mut MaintainerState, fx: &mut Effects) {
	if !discovered(state)
		|| repair::active_jobs(state) >= state.config.tuning().max_repairs as usize
	{
		return;
	}
	// A recovery copies only to members a listing proved. While the learner has not
	// answered one, recovering a volume whose only gap is the learner would fence its
	// writer and still leave it uncovered, so such a volume waits for the listing.
	let learner_unlisted = matches!(state.tablet.replica_set.phase, TabletPhase::Learning(_))
		&& learner(state).is_some_and(|worker| !state.listed.contains(&worker));

	let after = state.membership_checks.matching_after.clone();
	let candidates: Vec<_> = state
		.membership_checks
		.uncovered
		.range((after.as_ref().map_or(Unbounded, Excluded), Unbounded))
		.take(CHECK_BUDGET)
		.cloned()
		.collect();
	state.membership_checks.matching_after = if candidates.len() == CHECK_BUDGET {
		candidates.last().cloned()
	} else {
		None
	};
	let selected = candidates.iter().find_map(|key| {
		let goal = state.membership_goals.get(key)?;
		let volume = state.loaded_volumes.get(key)?;
		if phase_covered(state, key, goal)
			|| (learner_unlisted && next_covered(state, key, goal))
			|| !lifecycle_eligible(state, key)
			|| goal.verification.as_ref().is_some_and(|barrier| {
				!poll::volume_refreshed(state, barrier)
			})
			// Give the existing writer one quiet interval to finish matching. This
			// deadline belongs to the membership goal, so continuous writes cannot
			// postpone required matching indefinitely.
			|| state.now_boot_ms < goal.match_after_boot_ms
			// A recovery client claims the volume, which fences its writer for good: a writer
			// never reopens itself. A writer that is still active matches the new members
			// itself, so only a quiet volume is recovered here. A volume whose claim names a
			// holder is quiet only once that holder has sent nothing for a whole holder check
			// interval, so a live writer that is merely idle keeps its claim. A held writer that has still not matched once the
			// goal's holder wait passes is recovered anyway: fencing one writer is bounded,
			// while a tablet left short of members is not.
			|| (state.now_boot_ms < volume.last_activity.saturating_add(quiet_after(state, volume))
				&& state.now_boot_ms < holder_wait_until(state, goal))
			|| !membership::quorum(&state.tablet.replica_set, |worker| {
				volume.members.get(worker).is_some_and(|r| r.status == Status::InSync)
			}) {
			return None;
		}
		volume
			.row(key)
			.filter(|row| row.status == FsVolumeStatus::Live)
			.map(|row| (key.clone(), row))
	});
	if let Some((key, row)) = selected {
		repair::recover(
			state,
			key,
			row,
			repair::RecoveryPurpose::Membership {
				grant_epoch: state.tablet.tablet_grant_epoch,
				replica_set_version: state.tablet.replica_set.tablet_replica_set_version,
			},
			fx,
		);
	}
}

/// Only a correlated, quorum-matched history may replace an uncommitted candidate goal.
pub(super) fn recovered(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	through: VolumeLogPosition,
	fx: &mut Effects,
) {
	let Some(goal) = state.membership_goals.get(key) else {
		return;
	};
	let target = goal.observed.volume_seq.max(goal.committed_floor);
	let floor = super::super::filesystem::known_committed(state, key)
		.max(
			state
				.loaded_volumes
				.get(key)
				.and_then(|v| v.row(key))
				.map_or(0, |r| r.cold.volume_seq),
		)
		// Preserve newly observed commitment within the candidate being replaced,
		// without chasing later commits beyond this phase's fixed prefix. Joint's
		// fresh old-majority barrier covers commits made after Learning's goal.
		.min(target)
		.max(goal.committed_floor);
	if through.volume_seq < floor {
		return;
	}
	let barrier = poll::refresh_volume(state, key.clone(), fx);
	let goal = state.membership_goals.get_mut(key).unwrap();
	goal.observed = VolumeLogPosition {
		volume_seq: through.volume_seq.min(target),
		client_epoch: through.client_epoch,
	};
	goal.committed_floor = floor;
	goal.verification = Some(barrier);
	mark(state, key);
}

/// How long a volume must go without client activity before membership recovery may claim
/// it. A held volume's activity moves only on opens, and the holder check keeps it loaded
/// while its holder is live, so it waits a holder check interval; a volume with no recorded
/// holder waits one repair interval.
fn quiet_after(state: &MaintainerState, volume: &LoadedVolume) -> u64 {
	match holders::holder(volume) {
		Some(_) => state
			.config
			.tuning()
			.holder_check_after_ms
			.max(state.config.tuning().repair_quiet_ms),
		None => state.config.tuning().repair_quiet_ms,
	}
}

/// A membership recovery of `key` finished, whatever its result. It fenced the writer it
/// found, so a writer the owner opens afterwards gets a fresh holder wait of its own before
/// recovery may claim the volume again; a failed or incomplete recovery must not turn into a
/// loop that fences every reopened writer.
pub(super) fn membership_recovery_finished(state: &mut MaintainerState, key: &VolumeLogKey) {
	let wait_until = state
		.now_boot_ms
		.saturating_add(state.config.tuning().membership_holder_wait_ms);
	if let Some(goal) = state.membership_goals.get_mut(key) {
		goal.holder_wait_until_boot_ms = wait_until;
	}
}
