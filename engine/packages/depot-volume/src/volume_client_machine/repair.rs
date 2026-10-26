//! Bounded background matching and the in-sync-promotion barrier.

use super::{
	Effects,
	effect::VolumeClientEffect as Effect,
	event::RpcError,
	state::{MatchStep, RecoveryPhase, VolumeClientPhase, VolumeClientState},
	types::*,
};
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState, depot_storage_worker_protocol_v1::RejectResponse,
	shared_depot_types_v1::VolumeLogReplicaStatus,
};
use rivet_topology::worker::WorkerId;

/// Before replay selects history there is no safe matching goal. Afterwards every
/// already assigned position may have committed, including a write with a lost reply.
pub(super) fn goal(s: &VolumeClientState) -> Option<u64> {
	matches!(
		s.phase,
		VolumeClientPhase::Appending
			| VolumeClientPhase::Draining { .. }
			| VolumeClientPhase::Recovering(
				RecoveryPhase::Matching
					| RecoveryPhase::CommittingClaimEntry { .. }
					| RecoveryPhase::Finishing { .. }
			)
	)
	.then_some(s.next_volume_seq - 1)
}

/// Remove every old operation before retrying. A waiting repair consumes no repair slot.
pub(super) fn backoff(s: &mut VolumeClientState, worker: &WorkerId, fx: &mut Effects) {
	let r = &s.replicas[worker];
	let delay = (50u64 << r.repair_attempt.min(6)).min(3000);
	backoff_for(s, worker, delay, fx);
}

/// The wait after an overload refusal: the shard's hint, capped, but never shorter than the
/// replica's ordinary backoff, so refusals still spend the claim attempt budget no faster
/// than other failures and an unjittered inbox-full hint does not set the pace.
pub(super) fn refusal_delay(s: &VolumeClientState, worker: &WorkerId, hint_ms: u64) -> u64 {
	let attempt = s.replicas[worker].repair_attempt;
	let backoff = (50u64 << attempt.min(6)).min(3000);
	hint_ms
		.clamp(1, super::pipeline::MAX_REFUSAL_DELAY_MS)
		.max(backoff)
}

/// Like [`backoff`], but waits `delay_ms` before the next attempt, such as the retry hint
/// of an overload refusal.
pub(super) fn backoff_for(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	delay_ms: u64,
	fx: &mut Effects,
) {
	let through = goal(s);
	for barrier in s.barriers.values_mut() {
		barrier.confirmed.remove(worker);
	}

	let op = s.operation();
	let r = s.replicas.get_mut(worker).unwrap();
	for old in r.in_flight.keys() {
		fx.cancel_timer(VolumeClientTimer::Append(*old));
	}
	r.in_flight.clear();
	if let Some(old) = r.pending_claim.take() {
		fx.cancel_timer(VolumeClientTimer::Control(old));
	}

	if let Some(step) = r.repair.take() {
		for old in step.operation_ids() {
			fx.cancel_timer(VolumeClientTimer::Control(old));
		}
	}

	// Stop counting this replica while its bounded retry delay is active.
	super::pipeline::clear_refusal(r, fx);
	r.unmatch(through);
	r.claim = None;
	r.repair_attempt = r.repair_attempt.saturating_add(1);
	r.repair = Some(MatchStep::Waiting { op_id: op });
	fx.arm_timer_after(VolumeClientTimer::Control(op), delay_ms);
	super::pipeline::probe_membership(s, worker, fx);
}

/// Fences lost client authority, reopens missing volumes, and backs off other failures.
pub(super) fn failed(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	error: RpcError,
	fx: &mut Effects,
) {
	match error {
		RpcError::Rejected(RejectResponse::VolumeLogReplicaDeleted(_)) => {
			super::pipeline::fence_with_error(s, WriteError::VolumeNotFound, fx);
		}
		RpcError::Rejected(RejectResponse::StaleTabletReplicaSet(reject)) => {
			if !super::membership::adopt(s, reject.replica_set, fx) {
				backoff(s, worker, fx);
			}
		}
		RpcError::Rejected(RejectResponse::VolumeNotLoaded)
			if s.loading_members.contains(worker)
				|| !super::membership::voter(&s.replica_set, worker) =>
		{
			backoff(s, worker, fx)
		}
		RpcError::Rejected(
			RejectResponse::StaleClientEpoch(_) | RejectResponse::ClientConflict(_),
		) => super::pipeline::fence(s, fx),
		// Repair keeps retrying an unloaded member until the maintainer reloads it. Only a
		// volume no longer loaded on a quorum reopens; committed writes remain committed.
		RpcError::Rejected(RejectResponse::VolumeNotLoaded) => {
			super::lifecycle::member_not_loaded(s, worker, fx)
		}
		// A refusal did nothing; wait out the shard's hint, which it jitters so refused
		// opens do not all retry together.
		RpcError::Rejected(RejectResponse::Overloaded(refusal)) => {
			let delay = refusal_delay(s, worker, refusal.retry_after_ms);
			backoff_for(s, worker, delay, fx);
		}
		// The transport refused the step before sending it; like a shard refusal, the step
		// waits the hint and is sent again.
		RpcError::Overloaded { retry_after_ms } => {
			let delay = refusal_delay(s, worker, retry_after_ms);
			backoff_for(s, worker, delay, fx);
		}
		// A full disk refused the repair step without writing it, so the step waits the
		// shard's hint, as an append does, for space to be freed.
		RpcError::Rejected(RejectResponse::DiskFull(refusal)) => {
			let delay = refusal_delay(s, worker, refusal.retry_after_ms);
			backoff_for(s, worker, delay, fx);
		}
		// A size refusal applied nothing and names the limit. A copy of several entries is
		// rebuilt under it; a copy or fetch of one entry that cannot fit blocks the member.
		RpcError::Rejected(RejectResponse::RequestTooLarge(reject)) => {
			too_large(s, worker, reject.max_bytes, fx)
		}
		// The member's shard cannot read the copied entry's payload encoding until its
		// binary is upgraded. Nothing was written, so the step waits and is sent again, and
		// the member catches up once its shard reads the version.
		RpcError::Rejected(RejectResponse::UnsupportedPayloadVersion(_)) => {
			let delay = refusal_delay(s, worker, super::pipeline::MAX_REFUSAL_DELAY_MS);
			backoff_for(s, worker, delay, fx);
		}
		// Every other rejection and a transport failure back the step off and retry it.
		RpcError::Rejected(
			RejectResponse::DiskIncarnationMismatch(_)
			| RejectResponse::NotClaimed(_)
			| RejectResponse::VolumeSeqGap(_)
			| RejectResponse::Diverged(_)
			| RejectResponse::AppendedMismatch(_)
			| RejectResponse::BelowCold(_)
			| RejectResponse::VolumeLogReplicaIncarnationMismatch(_)
			| RejectResponse::StaleTabletGrantEpoch(_)
			| RejectResponse::NotServing
			| RejectResponse::TooManyLoadedVolumes(_),
		)
		| RpcError::LifecycleRejected(_)
		| RpcError::RoutingUnavailable
		| RpcError::Unavailable => backoff(s, worker, fx),
	}
}

/// Ends the member's repair at a permanent refusal: the step is retired, the member stays
/// unmatched and out of every quorum, and it is blocked at `first` instead of being
/// claimed again and sent the same entry again; see [`super::pipeline::block`].
pub(super) fn block(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	first: u64,
	refusal: PermanentRefusal,
	fx: &mut Effects,
) {
	let r = s.replicas.get_mut(worker).unwrap();
	if let Some(step) = r.repair.take() {
		for op in step.operation_ids() {
			fx.cancel_timer(VolumeClientTimer::Control(op));
		}
	}
	r.matched = false;
	super::pipeline::block(s, worker, first, refusal, fx);
}

/// A copy or fetch was refused for size with `max_bytes` as the shard's limit. A copy of
/// several entries lowers the member's append limit, as an append refusal does, and backs
/// off to be rebuilt under it, because the step keeps no replica state to continue from. A
/// copy whose first entry cannot fit on its own blocks the member there, as does a fetch
/// whose source cannot page even its first entry within this client's read bound, since
/// the client can then never obtain the entry for the member.
fn too_large(s: &mut VolumeClientState, worker: &WorkerId, max_bytes: u32, fx: &mut Effects) {
	let r = &s.replicas[worker];
	match r.repair.clone() {
		Some(MatchStep::Copying {
			bytes: request_bytes,
			..
		}) => {
			let Some(first) = r.repair_floor.map(|floor| floor + 1) else {
				backoff(s, worker, fx);
				return;
			};
			let below_request = request_bytes.saturating_sub(1).min(u64::from(u32::MAX)) as u32;
			let limit = r.append_limit_bytes.min(max_bytes).min(below_request);
			let first_fits = s.retained.get(&first).is_none_or(|entry| {
				s.single_entry_request_bytes(entry.encoded_bytes) <= u64::from(limit)
			});
			tracing::warn!(
				worker_id = ?worker,
				request_bytes,
				max_bytes,
				append_limit_bytes = limit,
				"log shard refused a repair copy as too large; lowering this replica's append limit, likely a shard write limit below the client's"
			);
			s.replicas.get_mut(worker).unwrap().append_limit_bytes = limit;
			if first_fits {
				backoff(s, worker, fx);
			} else {
				block(
					s,
					worker,
					first,
					PermanentRefusal::EntryTooLarge { max_bytes: limit },
					fx,
				);
			}
		}
		Some(MatchStep::Fetching { target, .. }) => block(
			s,
			worker,
			target.appended.volume_seq + 1,
			PermanentRefusal::EntryTooLarge { max_bytes },
			fx,
		),
		Some(
			MatchStep::Waiting { .. }
			| MatchStep::Promoting { .. }
			| MatchStep::Probing { .. }
			| MatchStep::Truncating { .. },
		)
		| None => backoff(s, worker, fx),
	}
}

/// Marks a fully copied replica in sync using its exact incarnation and tail.
/// No second claim round is needed; the claim entry still must commit on an in-sync quorum.
pub(super) fn promote(
	s: &mut VolumeClientState,
	target: &WorkerId,
	expected: VolumeLogReplicaState,
	fx: &mut Effects,
) {
	let op = s.operation();
	s.replicas.get_mut(target).unwrap().repair = Some(MatchStep::Promoting {
		op_id: op,
		expected: expected.clone(),
	});
	fx.arm_timer_after(VolumeClientTimer::Control(op), s.config.append_timeout_ms);
	fx.execute(Effect::MarkInSyncRequest {
		op_id: op,
		worker_id: target.clone(),
		volume_log_key: s.volume.loaded_key().clone(),
		client: s.client.clone(),
		tablet_replica_set_version: s.replica_set.tablet_replica_set_version,
		replica_incarnation: expected.replica_incarnation,
		expected_appended: expected.appended,
	});
}

/// Repair timeouts retry only that replica, preserving the serving quorum's progress.
pub(super) fn expired(s: &mut VolumeClientState, op: OpId, fx: &mut Effects) -> bool {
	let target = s.replicas.iter().find_map(|(w, r)| {
		r.repair
			.as_ref()
			.filter(|step| step.operation_ids().contains(&op))
			.map(|_| w.clone())
	});
	let Some(target) = target else {
		return false;
	};
	if matches!(s.replicas[&target].repair, Some(MatchStep::Waiting { .. })) {
		s.replicas.get_mut(&target).unwrap().repair = None;
		if s.phase == VolumeClientPhase::Recovering(RecoveryPhase::Claiming) {
			// Lagging manifests retry at the same epoch, bounded by the claim attempt budget.
			if super::membership::voter(&s.replica_set, &target)
				&& (s.loading_members.contains(&target)
					|| s.replicas[&target].repair_attempt < s.config.claim_attempts)
			{
				super::recovery::claim(s, target, fx);
			}
			super::recovery::begin_recovery(s, fx);
		}
	} else {
		if let Some(MatchStep::Fetching { source, .. }) = &s.replicas[&target].repair {
			let source = source.clone();
			s.replicas.get_mut(&target).unwrap().repair_avoid = Some(source);
		}
		backoff(s, &target, fx);
	}
	true
}

/// Fetch a missing retained range only from a matched peer that already acknowledges it.
pub(super) fn fetch(
	s: &mut VolumeClientState,
	target: &WorkerId,
	state: VolumeLogReplicaState,
	fx: &mut Effects,
) {
	// A source whose last fetch for this target timed out is tried only when no other matched
	// peer holds the range, so a repair is not pinned to an unresponsive worker.
	let avoid = s.replicas[target].repair_avoid.clone();
	let candidates = || {
		s.replicas
			.iter()
			.filter(|(w, r)| *w != target && r.matched && r.acked > state.appended.volume_seq)
	};
	let source = candidates()
		.find(|(w, _)| Some(*w) != avoid.as_ref())
		.or_else(|| candidates().next())
		.map(|(w, r)| (w.clone(), r.incarnation));
	let Some((source, source_incarnation)) = source else {
		backoff(s, target, fx);
		return;
	};

	let start = state.appended.volume_seq + 1;
	let op = s.operation();
	s.replicas.get_mut(target).unwrap().repair = Some(MatchStep::Fetching {
		op_id: op,
		source: source.clone(),
		source_incarnation,
		target: state,
	});
	fx.arm_timer_after(VolumeClientTimer::Control(op), s.config.append_timeout_ms);
	fx.execute(Effect::ReadVolumeLogEntriesRequest {
		op_id: op,
		worker_id: source,
		volume_log_key: s.volume.loaded_key().clone(),
		start,
		max_entries: 1024,
		max_bytes: s.config.max_append_bytes,
	});
}

/// Resolves a fetch reply only when both its source worker and operation still match.
pub(super) fn fetch_target(s: &VolumeClientState, op: OpId, worker: &WorkerId) -> Option<WorkerId> {
	s.replicas.iter().find_map(|(target, r)| match &r.repair {
		Some(MatchStep::Fetching { op_id, source, .. }) if *op_id == op && source == worker => {
			Some(target.clone())
		}
		_ => None,
	})
}

/// Validate the source's current claim before retaining any fetched bytes for copying.
pub(super) fn fetched(
	s: &mut VolumeClientState,
	target: &WorkerId,
	result: Result<(VolumeLogReplicaState, Vec<EntryRef>), RpcError>,
	fx: &mut Effects,
) {
	let Some(MatchStep::Fetching {
		op_id,
		source,
		source_incarnation,
		target: destination,
	}) = s.replicas[target].repair.clone()
	else {
		return;
	};

	fx.cancel_timer(VolumeClientTimer::Control(op_id));
	let (state, entries) = match result {
		Ok(result) => result,
		Err(error) => {
			s.replicas.get_mut(target).unwrap().repair_avoid = Some(source.clone());
			failed(s, target, error, fx);
			return;
		}
	};

	// Validate the complete page and source lifetime before retaining any payloads.
	let mut next = destination.appended.volume_seq + 1;
	let valid = state.status == VolumeLogReplicaStatus::InSync
		&& state.replica_incarnation == source_incarnation
		&& state.claimed_client.as_ref() == Some(&s.client)
		&& !entries.is_empty()
		&& entries.iter().all(|e| {
			let valid = e.id.volume_seq == next
				&& e.id.volume_seq < s.next_volume_seq
				&& e.id.volume_seq <= state.appended.volume_seq;
			next = next.saturating_add(1);
			valid
		});
	if !valid {
		for e in entries {
			fx.execute(Effect::ReleasePayload { payload: e.payload });
		}
		backoff(s, target, fx);
		backoff(s, &source, fx);
		return;
	}

	// Validate every duplicate before charging or retaining the page. On disagreement
	// even the later unused payloads must be released before this handle is fenced.
	if entries.iter().any(|entry| {
		s.retained
			.get(&entry.id.volume_seq)
			.is_some_and(|held| held.id != entry.id)
	}) {
		for entry in entries {
			fx.execute(Effect::ReleasePayload {
				payload: entry.payload,
			});
		}
		super::pipeline::fence(s, fx);
		return;
	}

	// Duplicate entries consume no additional retained-memory budget.
	let bytes: u64 = entries
		.iter()
		.filter(|e| !s.retained.contains_key(&e.id.volume_seq))
		.map(|e| u64::from(e.encoded_bytes))
		.sum();
	if bytes
		> s.config
			.max_memory_bytes
			.saturating_sub(s.memory_bytes)
			.saturating_sub(super::waiting::reserved_bytes(s))
	{
		for e in entries {
			fx.execute(Effect::ReleasePayload { payload: e.payload });
		}
		backoff(s, target, fx);
		return;
	}

	// Retain the validated page, release duplicates, and resume copying to the target.
	s.memory_bytes += bytes;
	for entry in entries {
		match s.retained.entry(entry.id.volume_seq) {
			std::collections::btree_map::Entry::Vacant(slot) => {
				slot.insert(entry);
			}
			std::collections::btree_map::Entry::Occupied(_) => {
				fx.execute(Effect::ReleasePayload {
					payload: entry.payload,
				});
			}
		}
	}

	super::recovery::copy(s, target, destination, fx);
}
