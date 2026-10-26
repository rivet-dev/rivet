//! Recovery-only handles repair history onto a committed claim entry, then close without serving
//! application writes.

use super::{
	Effects,
	effect::VolumeClientEffect as Effect,
	state::{RecoveryPhase, VolumeClientPhase, VolumeClientState},
	types::*,
};

/// Finish once required members and publication quorums are ready. Ordinary recovery
/// matches every destination early, then reports optional stragglers at its bounded deadline.
/// A member is caught up once it acknowledged the claim entry; the maintainer derives the
/// commit from the members' tails, so nothing is announced to them.
pub(super) fn finish(s: &mut VolumeClientState, timed_out: bool, fx: &mut Effects) {
	let VolumeClientPhase::Recovering(RecoveryPhase::Finishing { volume_seq }) = s.phase else {
		return;
	};

	let ready: std::collections::BTreeSet<_> = s
		.replicas
		.iter()
		.filter(|(_, r)| r.matched && r.acked >= volume_seq)
		.map(|(worker, _)| worker.clone())
		.collect();
	let required_ready = s
		.required_recovery_members
		.iter()
		.all(|worker| ready.contains(worker));
	let quorum_ready = super::membership::quorum(&s.replica_set, |worker| ready.contains(worker));
	// An admitted membership requirement replaces only best-effort matching of optional
	// peers. It never replaces the phase's publication quorums or the learner's matching proof.
	if !timed_out
		&& (!required_ready
			|| !quorum_ready
			|| (s.required_recovery_members.is_empty() && ready.len() < s.replicas.len()))
	{
		return;
	}
	fx.cancel_timer(VolumeClientTimer::FinishRecovery);

	// Closing requires the claim entry on a quorum, even when a remaining peer is unreachable.
	if !quorum_ready || !required_ready {
		super::lifecycle::fail(s, WriteError::QuorumUnavailable { token: None }, fx);
		return;
	}
	s.phase = VolumeClientPhase::Draining { request_id: None };
}

/// Produce the report only after the correlated maintainer release has ended.
pub(super) fn closed(s: &VolumeClientState, result: Result<(), WriteError>, fx: &mut Effects) {
	let result = result.and_then(|()| {
		let through = s
			.position(s.next_volume_seq - 1)
			.ok_or(WriteError::RecoveryFailed)?;
		let (in_sync, incomplete) = s
			.replicas
			.iter()
			.partition::<Vec<_>, _>(|(_, r)| r.matched && r.acked >= through.volume_seq);
		Ok(RecoveryReport {
			through,
			in_sync: in_sync
				.into_iter()
				.map(|(worker, _)| worker.clone())
				.collect(),
			incomplete: incomplete
				.into_iter()
				.map(|(worker, _)| worker.clone())
				.collect(),
		})
	});
	fx.execute(Effect::RecoveryResponse { result });
}


