//! Quorum commitment releases payloads. Epoch boundaries retain matching evidence;
//! lagging peers fetch missing bytes from another matched replica.

use super::{
	Effects,
	effect::VolumeClientEffect as Effect,
	pipeline,
	state::{VolumeClientPhase, VolumeClientState},
};
use rivet_data::generated::shared_depot_types_v1::VolumeLogPosition;

/// Only known committed history may be evicted; a conflicting epoch revokes this handle.
pub(super) fn observe_cold(s: &mut VolumeClientState, cold: VolumeLogPosition, fx: &mut Effects) {
	if s.phase != VolumeClientPhase::Appending || cold.volume_seq < s.cold.volume_seq {
		return;
	}
	if s.position(cold.volume_seq).as_ref() != Some(&cold) {
		pipeline::fence(s, fx);
		return;
	}
	if cold.volume_seq > s.commit_publishing.committed {
		return;
	}
	let boundary = s
		.history_epochs
		.range(..=cold.volume_seq)
		.next_back()
		.map(|(seq, _)| *seq);
	if let Some(boundary) = boundary {
		s.history_epochs.retain(|seq, _| *seq >= boundary);
	}
	s.above_cold.trim_through(cold.volume_seq);
	s.cold = cold;
}

/// Prepared appends own their bytes, so one slow peer cannot pin the committed history.
pub(super) fn release_committed_payloads(s: &mut VolumeClientState, fx: &mut Effects) {
	if !matches!(
		s.phase,
		VolumeClientPhase::Appending
			| VolumeClientPhase::Draining { .. }
			| VolumeClientPhase::Recovering(
				super::state::RecoveryPhase::CommittingClaimEntry { .. }
					| super::state::RecoveryPhase::Finishing { .. }
			)
	) {
		return;
	}
	let goal = super::repair::goal(s);

	// A replica waiting out an overload refusal resends from its acknowledgement, so the
	// committed entries it lacks stay retained for that delay, at most one refusal hint, while
	// they hold under half the memory budget. Past that it catches up through repair instead.
	// A replica under repair keeps the committed entries above its observed position the
	// same way, so its copy sends them from memory; a peer may already have moved them into
	// cold, which no client can copy from.
	let pin_refused = s.memory_bytes <= s.config.max_memory_bytes / 2;
	let release_through = s
		.replicas
		.values()
		.filter(|_| pin_refused)
		.filter_map(
			|replica| match (replica.matched, replica.refused, replica.repair_floor) {
				(true, Some(_), _) => Some(replica.acked),
				// A blocked replica still takes the entries below its block from memory; once
				// it holds them all, nothing more is kept for it.
				(true, None, _)
					if replica
						.blocked
						.is_some_and(|blocked| replica.acked + 1 < blocked.volume_seq) =>
				{
					Some(replica.acked)
				}
				// The floor stays across a repair's backoff and the claim that restarts it, which is
				// when a copy that races passes would otherwise lose its entries. It is cleared
				// when the replica is matched, and a replica that leaves the set takes it along;
				// the half-budget gate bounds what a stuck repair can hold.
				(false, _, Some(floor)) => Some(floor),
				_ => None,
			},
		)
		.min()
		.map_or(s.commit_publishing.committed, |acked| {
			acked.min(s.commit_publishing.committed)
		});

	// A lagging matched peer can no longer be pipelined through an evicted prefix.
	// Claim again it so matching waits for its cold floor or copies from another replica. A
	// blocked peer that holds everything below its block is not lagging: nothing more is
	// sent to it, and its acknowledgements still vote for the entries it holds.
	for replica in s.replicas.values_mut() {
		if replica.matched
			&& replica.acked < release_through
			&& replica.in_flight.is_empty()
			&& replica
				.blocked
				.is_none_or(|blocked| replica.acked + 1 < blocked.volume_seq)
		{
			super::pipeline::clear_refusal(replica, fx);
			replica.unmatch(goal);
			replica.claim = None;
			replica.sent = replica.acked;
		}
	}
	while s
		.retained
		.first_key_value()
		.is_some_and(|(&seq, _)| seq <= release_through)
	{
		let (_, entry) = s.retained.pop_first().unwrap();
		remember_position(s, &entry.id);
		s.memory_bytes -= u64::from(entry.encoded_bytes);
		fx.execute(Effect::ReleasePayload {
			payload: entry.payload,
		});
	}
}

/// Consecutive entries in one epoch share one boundary, irrespective of payload size.
pub(super) fn remember_position(s: &mut VolumeClientState, at: &VolumeLogPosition) {
	if s.history_epochs
		.range(..=at.volume_seq)
		.next_back()
		.map(|(_, epoch)| *epoch)
		!= Some(at.client_epoch)
	{
		s.history_epochs.insert(at.volume_seq, at.client_epoch);
	}
}

/// Repair append effects materialize their bytes before these release effects run.
/// The selected source retains the claimed suffix, so another repair can fetch it again. The
/// client's own claim entry stays retained until it commits, so it can be sent again.
pub(super) fn release_recovery_payloads(s: &mut VolumeClientState, fx: &mut Effects) {
	if s.phase != VolumeClientPhase::Recovering(super::state::RecoveryPhase::Matching) {
		return;
	}
	let keep_from = s.claim_entry.map_or(u64::MAX, |claim| claim.volume_seq);
	let kept = s.retained.split_off(&keep_from);
	let released = std::mem::replace(&mut s.retained, kept);
	for (_, entry) in released {
		remember_position(s, &entry.id);
		s.memory_bytes -= u64::from(entry.encoded_bytes);
		fx.execute(Effect::ReleasePayload {
			payload: entry.payload,
		});
	}
}
