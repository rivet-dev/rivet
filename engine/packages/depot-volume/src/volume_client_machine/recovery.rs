//! Claim an in-sync quorum, append a claim entry on its newest tail, recover that history, and
//! serve once the entry is committed and the history is matched onto a quorum.

use super::{
	Effects,
	effect::VolumeClientEffect as Effect,
	event::RpcError,
	state::{
		MatchStep, ProbeSearch, RecoveryPhase, ReplayStart, VolumeClientPhase, VolumeClientState,
	},
	types::*,
};
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	shared_depot_types_v1::{VolumeLogPosition, VolumeLogReplicaStatus},
};
use rivet_state_machine::Entropy;
use rivet_topology::worker::WorkerId;

/// Selects history from the claimed in-sync quorum, preferring client epoch over suffix
/// length: a longer tail may be an older client's uncommitted history. Committing the
/// claim entry commits every entry beneath it because log matching preserves that prefix.
pub(super) fn begin_recovery(s: &mut VolumeClientState, fx: &mut Effects) {
	if s.phase != VolumeClientPhase::Recovering(RecoveryPhase::Claiming) {
		return;
	}

	let counted: std::collections::BTreeMap<_, _> = s
		.replicas
		.iter()
		.filter(|(w, _)| super::membership::voter(&s.replica_set, w))
		.filter_map(|(w, r)| {
			r.claim
				.as_ref()
				.filter(|p| p.status == VolumeLogReplicaStatus::InSync)
				.map(|p| (w.clone(), p.clone()))
		})
		.collect();
	if !super::membership::quorum(&s.replica_set, |worker| counted.contains_key(worker)) {
		if s.replicas
			.iter()
			.filter(|(w, _)| super::membership::voter(&s.replica_set, w))
			.all(|(_, r)| {
				r.pending_claim.is_none() && !matches!(r.repair, Some(MatchStep::Waiting { .. }))
			}) {
			super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
		}
		return;
	}

	// The maintainer can trim after VolumeOpen, including between claim replies.
	// Only in-sync evidence may advance the cold baseline used for source selection.
	let mut cold = s.cold.clone();
	for proof in counted.values() {
		if proof.cold.volume_seq > cold.volume_seq {
			cold = proof.cold.clone();
		}
	}
	if counted
		.values()
		.any(|proof| proof.cold.volume_seq == cold.volume_seq && proof.cold != cold)
	{
		super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
		return;
	}
	if cold.volume_seq > s.cold.volume_seq {
		// Refresh the durable receipt window together with an advanced cold floor.
		super::lifecycle::open(s, fx);
		return;
	}
	s.cold = cold;
	s.commit_publishing.committed = s.commit_publishing.committed.max(s.cold.volume_seq);

	s.claim_quorum = counted.keys().cloned().collect();
	let (source, proof) = counted
		.iter()
		.max_by_key(|(_, p)| (p.appended.client_epoch, p.appended.volume_seq))
		.unwrap();
	let (source, proof) = (source.clone(), proof.clone());
	// A claimed quorum that ends at one committed tail settles the log: any committed entry
	// above it would be on a quorum, and every quorum shares a member with this one, whose
	// claim refuses older writers. The tail is committed when it is cold, or when the open
	// row's maintainer applied through it, since the maintainer applies only entries a quorum
	// proved committed. Otherwise the first write at the new epoch settles the uncertain
	// suffix, so the client writes a claim entry before it serves.
	let committed_tail = proof.appended == s.cold || applied_covers(s, &proof.appended);
	s.claim_settled = s.purpose == OpenPurpose::Serve
		&& committed_tail
		&& counted.values().all(|p| p.appended == proof.appended);
	if proof.appended.volume_seq < s.cold.volume_seq
		|| proof.cold.volume_seq > s.cold.volume_seq
		|| (proof.appended.volume_seq == s.cold.volume_seq && proof.appended != s.cold)
	{
		super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
		return;
	}

	// Replace old history and retry receipts with evidence from the validated source. A
	// block names an entry of the history being replaced, so it no longer applies.
	for replica in s.replicas.values_mut() {
		replica.blocked = None;
	}
	for (_, entry) in std::mem::take(&mut s.retained) {
		fx.execute(Effect::ReleasePayload {
			payload: entry.payload,
		});
	}

	super::requests::unknown_waiters(s, fx);
	s.dedupe.clear();
	s.history_epochs.clear();
	s.above_cold.clear();
	s.last_client_seq = 0;
	s.claim_entry = None;
	for receipt in s.cold_receipts.clone() {
		super::requests::remember(
			s,
			MutationIdentity {
				client_seq: receipt.client_seq,
				request_id: receipt.request_id,
				payload_digest: receipt.payload_digest,
			},
			receipt.volume_seq,
		);
	}
	s.memory_bytes = 0;
	s.next_volume_seq = proof.appended.volume_seq.saturating_add(1);
	if s.next_volume_seq <= s.commit_publishing.committed {
		super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
		return;
	}

	// Members at the recovered tail hold the source's exact history, so they are matched there
	// now and take the claim entry before the replay reads that history. Once a quorum holds
	// the entry, the maintainer can prove the tail committed whether or not this open finishes
	// within its deadline, so the proof no longer waits on the whole replay.
	match_at_tail(s, &counted, &proof, fx);
	if !s.claim_settled && !retain_claim_entry(s, fx) {
		return;
	}

	if proof.appended.volume_seq == s.cold.volume_seq {
		super::metrics::replay_started(None);
		replayed(s, source, proof, fx);
	} else if let Some(applied) = install_applied_history(s, &proof) {
		super::metrics::replay_started(Some(ReplayStart::Applied));
		let next = applied.volume_seq;
		replay_page(
			s,
			source,
			proof,
			next,
			Some(applied),
			ReplayStart::Applied,
			fx,
		);
	} else {
		super::metrics::replay_started(Some(ReplayStart::Cold));
		let next = s.cold.volume_seq + 1;
		replay_page(s, source, proof, next, None, ReplayStart::Cold, fx);
	}
}

/// Whether the open row's maintainer applied through `tail` with the same epoch there: the
/// applied position is at or past the tail, and the epoch boundaries name the tail's epoch at
/// its sequence. An applied position below the tail, or a different epoch at its sequence,
/// proves nothing about it.
fn applied_covers(s: &VolumeClientState, tail: &VolumeLogPosition) -> bool {
	let Some(history) = s.applied_history.as_ref() else {
		return false;
	};
	if history.applied.volume_seq < tail.volume_seq {
		return false;
	}
	if history.applied.volume_seq == tail.volume_seq {
		return history.applied.client_epoch == tail.client_epoch;
	}
	let epoch_at_tail = history
		.epochs
		.iter()
		.rev()
		.find(|boundary| boundary.volume_seq <= tail.volume_seq)
		.map(|boundary| boundary.client_epoch);
	epoch_at_tail == Some(tail.client_epoch)
}

/// Installs what the open row's maintainer reported about its applied entries, which a replay
/// from cold would otherwise learn by reading every one of them: the epoch boundaries and the
/// newest write receipts. Returns the applied position the replay then starts from, or `None`,
/// changing nothing, when the report is absent or does not describe exactly the entries from
/// just above cold through an applied position the source holds.
fn install_applied_history(
	s: &mut VolumeClientState,
	proof: &VolumeLogReplicaState,
) -> Option<VolumeLogPosition> {
	let history = s.applied_history.as_ref()?;
	let applied = &history.applied;
	let first = history.epochs.first()?;
	let last = history.epochs.last()?;
	let ascending = history.epochs.windows(2).all(|pair| {
		pair[0].volume_seq < pair[1].volume_seq && pair[0].client_epoch < pair[1].client_epoch
	});
	let describes_applied = applied.volume_seq > s.cold.volume_seq
		&& applied.volume_seq <= proof.appended.volume_seq
		&& applied.client_epoch <= proof.appended.client_epoch
		&& first.volume_seq == s.cold.volume_seq + 1
		&& first.client_epoch >= s.cold.client_epoch
		&& last.volume_seq <= applied.volume_seq
		&& last.client_epoch == applied.client_epoch
		&& ascending
		&& history
			.recent_writes
			.iter()
			.all(|receipt| receipt.volume_seq > 0 && receipt.volume_seq <= applied.volume_seq);
	if !describes_applied {
		return None;
	}

	let applied = applied.clone();
	let epochs = history.epochs.clone();
	let receipts = history.recent_writes.clone();
	for boundary in &epochs {
		super::retained::remember_position(s, boundary);
	}
	for receipt in receipts {
		super::requests::remember(
			s,
			MutationIdentity {
				client_seq: receipt.client_seq,
				request_id: receipt.request_id,
				payload_digest: receipt.payload_digest,
			},
			receipt.volume_seq,
		);
	}
	Some(applied)
}

/// Matches every counted member whose tail is the source's, `proof.appended`. They are the
/// members the claim entry goes to before the replay. The tail's position is remembered
/// first, so those appends find their `previous` before the replay fills in the epochs
/// beneath it.
fn match_at_tail(
	s: &mut VolumeClientState,
	counted: &std::collections::BTreeMap<WorkerId, VolumeLogReplicaState>,
	proof: &VolumeLogReplicaState,
	fx: &mut Effects,
) {
	if proof.appended.volume_seq > s.cold.volume_seq {
		super::retained::remember_position(s, &proof.appended);
	}
	for (worker, state) in counted {
		if state.appended == proof.appended {
			match_member_at_tail(s, worker, state, fx);
		}
	}
}

/// Matches `worker` at the recovered tail, which its in-sync `state` under this claim ends
/// at. Log matching makes its log identical to the source's through that position: a shard
/// takes an append only when its own entry at the request's `previous` is that exact
/// position, so one position names one prefix. From here the pipeline sends it the claim
/// entry and counts its acknowledgements.
fn match_member_at_tail(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	state: &VolumeLogReplicaState,
	fx: &mut Effects,
) {
	s.loading_members.remove(worker);
	let Some(replica) = s.replicas.get_mut(worker) else {
		return;
	};
	if let Some(step) = replica.repair.take() {
		for op in step.operation_ids() {
			fx.cancel_timer(VolumeClientTimer::Control(op));
		}
	}
	replica.repair_attempt = 0;
	replica.matched = true;
	replica.repair_floor = None;
	replica.repair_goal = None;
	replica.repair_avoid = None;
	replica.incarnation = state.replica_incarnation;
	replica.acked = state.appended.volume_seq;
	replica.sent = replica.acked;
	replica.claim = Some(state.clone());
}

/// Matches a member whose claim reply arrived during the replay at the recovered tail, when
/// its in-sync state ends exactly there, so it takes the claim entry now instead of after
/// the replay. A member at any other tail waits for matching.
pub(super) fn late_tail_match(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	state: &VolumeLogReplicaState,
	fx: &mut Effects,
) {
	let VolumeClientPhase::Recovering(RecoveryPhase::Replaying { proof, .. }) = &s.phase else {
		return;
	};
	if s.replicas.get(worker).is_none_or(|replica| replica.matched)
		|| state.status != VolumeLogReplicaStatus::InSync
		|| state.appended != proof.appended
		|| state.cold != proof.cold
	{
		return;
	}
	match_member_at_tail(s, worker, state, fx);
}

/// Whether `state`'s tail is still the recovered tail `proof`, or that tail with this client's
/// own claim entry on it, which the pipeline appends to the source while the replay runs.
fn recovered_tail(
	s: &VolumeClientState,
	state: &VolumeLogReplicaState,
	proof: &VolumeLogReplicaState,
) -> bool {
	state.appended == proof.appended
		|| s.claim_entry.is_some_and(|claim| {
			state.appended
				== VolumeLogPosition {
					volume_seq: claim.volume_seq,
					client_epoch: s.client.client_epoch,
				}
		})
}

/// Correlation lives on the replica and is replaced before transport work is emitted.
pub(super) fn claim(s: &mut VolumeClientState, worker: WorkerId, fx: &mut Effects) {
	let op = s.operation();
	s.replicas.get_mut(&worker).unwrap().pending_claim = Some(op);
	fx.arm_timer_after(VolumeClientTimer::Control(op), s.config.claim_timeout_ms);
	fx.execute(Effect::ClaimRequest {
		op_id: op,
		worker_id: worker,
		volume_log_key: s.volume.loaded_key().clone(),
		client: s.client.clone(),
		tablet_replica_set_version: s.replica_set.tablet_replica_set_version,
	});
}

/// Read the next bounded page while keeping the validated source proof.
fn replay_page(
	s: &mut VolumeClientState,
	source: WorkerId,
	proof: VolumeLogReplicaState,
	next: u64,
	verify: Option<VolumeLogPosition>,
	start: ReplayStart,
	fx: &mut Effects,
) {
	let op = s.operation();
	// Ask for no more than the recovered tail. This client's own claim entry may already sit
	// above it on the source; a page that carries it anyway is cut there by `read`.
	let remaining = proof
		.appended
		.volume_seq
		.saturating_sub(next)
		.saturating_add(1);
	let max_entries = remaining.min(1024) as u32;
	s.phase = VolumeClientPhase::Recovering(RecoveryPhase::Replaying {
		source: source.clone(),
		proof,
		op_id: op,
		next,
		verify,
		start,
	});
	fx.arm_timer_after(VolumeClientTimer::Control(op), s.config.append_timeout_ms);
	fx.execute(Effect::ReadVolumeLogEntriesRequest {
		op_id: op,
		worker_id: source,
		volume_log_key: s.volume.loaded_key().clone(),
		start: next,
		max_entries,
		max_bytes: s.config.max_append_bytes,
	});
}

/// Waits out a refused replay read for `hint_ms`, capped like any refusal delay, with no read
/// outstanding. [`replay_resumed`] then reads the same page again.
#[allow(
	clippy::too_many_arguments,
	reason = "mirrors the replay phase's fields"
)]
fn wait_replay(
	s: &mut VolumeClientState,
	source: WorkerId,
	proof: VolumeLogReplicaState,
	next: u64,
	verify: Option<VolumeLogPosition>,
	start: ReplayStart,
	hint_ms: u64,
	fx: &mut Effects,
) {
	let op = s.operation();
	s.phase = VolumeClientPhase::Recovering(RecoveryPhase::Replaying {
		source,
		proof,
		op_id: op,
		next,
		verify,
		start,
	});
	fx.arm_timer_after(
		VolumeClientTimer::ReplayResume(op),
		hint_ms.clamp(1, super::pipeline::MAX_REFUSAL_DELAY_MS),
	);
}

/// Reads the page a refused replay read waited for, if the replay is still at that wait.
pub(super) fn replay_resumed(s: &mut VolumeClientState, op: OpId, fx: &mut Effects) {
	if let VolumeClientPhase::Recovering(RecoveryPhase::Replaying {
		source,
		proof,
		op_id,
		next,
		verify,
		start,
	}) = s.phase.clone()
		&& op_id == op
	{
		replay_page(s, source, proof, next, verify, start, fx);
	}
}

/// Reads a replay page again after a failed or unanswered read, counting the attempt on the
/// source replica. Past `claim_attempts` the open fails.
fn retry_replay(
	s: &mut VolumeClientState,
	source: WorkerId,
	proof: VolumeLogReplicaState,
	next: u64,
	verify: Option<VolumeLogPosition>,
	start: ReplayStart,
	fx: &mut Effects,
) {
	let Some(replica) = s.replicas.get_mut(&source) else {
		super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
		return;
	};
	replica.repair_attempt = replica.repair_attempt.saturating_add(1);
	if replica.repair_attempt > s.config.claim_attempts {
		super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
		return;
	}
	replay_page(s, source, proof, next, verify, start, fx);
}

/// Replace an unavailable read source only with a current voter proving the same
/// claimed tail. The chosen history, partially replayed prefix and caller deadline
/// stay unchanged; each failed source keeps its existing attempt count.
fn alternate_replay_source(s: &mut VolumeClientState, fx: &mut Effects) -> bool {
	let VolumeClientPhase::Recovering(RecoveryPhase::Replaying {
		source,
		proof,
		next,
		verify,
		start,
		..
	}) = s.phase.clone()
	else {
		return false;
	};
	let alternate = s
		.replicas
		.iter()
		.filter(|(worker, replica)| {
			**worker != source
				&& super::membership::voter(&s.replica_set, worker)
				&& replica.repair_attempt <= s.config.claim_attempts
		})
		.filter_map(|(worker, replica)| {
			let candidate = replica.claim.as_ref()?;
			(candidate.status == VolumeLogReplicaStatus::InSync
				&& candidate.claimed_client.as_ref() == Some(&s.client)
				&& candidate.replica_incarnation == replica.incarnation
				&& recovered_tail(s, candidate, &proof)
				&& candidate.cold == proof.cold)
				.then_some((replica.repair_attempt, worker.clone(), candidate.clone()))
		})
		.min_by_key(|(attempt, worker, _)| (*attempt, worker.clone()));
	let Some((_, worker, candidate)) = alternate else {
		return false;
	};
	let Some(failed) = s.replicas.get_mut(&source) else {
		return false;
	};
	failed.repair_attempt = failed.repair_attempt.saturating_add(1);
	replay_page(s, worker, candidate, next, verify, start, fx);
	true
}

/// Drop all payload handles from obsolete pages; a replay page must preserve its source proof.
pub(super) fn read(
	s: &mut VolumeClientState,
	op: OpId,
	worker: &WorkerId,
	result: Result<(VolumeLogReplicaState, Vec<EntryRef>), RpcError>,
	_entropy: Entropy,
	fx: &mut Effects,
) {
	if let Some(target) = super::repair::fetch_target(s, op, worker) {
		super::repair::fetched(s, &target, result, fx);
		return;
	}

	if let VolumeClientPhase::Recovering(RecoveryPhase::Replaying {
		source,
		proof,
		op_id,
		next,
		verify,
		start,
	}) = s.phase.clone()
		&& op == op_id
		&& *worker == source
	{
		fx.cancel_timer(VolumeClientTimer::Control(op));
		// An unavailable source may be replaced only by an identical claimed voter.
		// Otherwise retry the same source within its attempt budget; the claim stays.
		let (state, entries) = match result {
			Ok(page) => page,
			// Cold moved past the replay while the client was opening, so the history it
			// selected no longer starts at a retained position. It has not served yet, so it
			// opens again from the newer cold position.
			Err(RpcError::Rejected(
				rivet_data::generated::depot_storage_worker_protocol_v1::RejectResponse::BelowCold(
					_,
				),
			)) => {
				super::lifecycle::open(s, fx);
				return;
			}
			// The shard or this process's transport refused the read without doing it. That
			// is no evidence about the source, so the page is read again after the hint
			// without counting an attempt; the callers' open deadlines still bound the wait.
			Err(RpcError::Overloaded { retry_after_ms }) => {
				wait_replay(s, source, proof, next, verify, start, retry_after_ms, fx);
				return;
			}
			Err(RpcError::Rejected(
				rivet_data::generated::depot_storage_worker_protocol_v1::RejectResponse::Overloaded(
					refusal,
				),
			)) => {
				let hint = refusal.retry_after_ms;
				wait_replay(s, source, proof, next, verify, start, hint, fx);
				return;
			}
			Err(RpcError::Rejected(
				rivet_data::generated::depot_storage_worker_protocol_v1::RejectResponse::DiskFull(
					refusal,
				),
			)) => {
				let hint = refusal.retry_after_ms;
				wait_replay(s, source, proof, next, verify, start, hint, fx);
				return;
			}
			Err(RpcError::Unavailable) if alternate_replay_source(s, fx) => return,
			Err(_) => {
				retry_replay(s, source, proof, next, verify, start, fx);
				return;
			}
		};

		// Entries above the recovered tail are this client's own claim entry, appended to the
		// source while the replay runs and already known; they are not part of the history
		// being replayed.
		let mut entries = entries;
		let above_tail = entries
			.iter()
			.position(|entry| entry.id.volume_seq > proof.appended.volume_seq)
			.map(|index| entries.split_off(index))
			.unwrap_or_default();
		release(above_tail, fx);

		// Accept only a consecutive page from the unchanged validated source. A replay that
		// starts at the maintainer's applied position also needs the source's entry there to be
		// that position; one that disagrees is evidence of another history, so the open fails.
		let verified = verify
			.as_ref()
			.is_none_or(|position| entries.first().is_some_and(|e| e.id == *position));
		let mut expected = next;
		let valid = verified
			&& same_claim(s, &state, &proof)
			&& state.status == VolumeLogReplicaStatus::InSync
			&& recovered_tail(s, &state, &proof)
			&& !entries.is_empty()
			&& entries.iter().all(|e| {
				let valid =
					e.id.volume_seq == expected && e.id.volume_seq <= proof.appended.volume_seq;
				expected = expected.saturating_add(1);
				valid
			});
		if !valid {
			release(entries, fx);
			super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
			return;
		}

		// Bound retained payloads before installing the page and its retry identities.
		let page_bytes: u64 = entries
			.iter()
			.map(|entry| u64::from(entry.encoded_bytes))
			.sum();
		if page_bytes
			> s.config
				.max_memory_bytes
				.saturating_sub(s.memory_bytes)
				.saturating_sub(super::waiting::reserved_bytes(s))
		{
			release(entries, fx);
			super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
			return;
		}

		super::metrics::replay_page(start, entries.len() as u64, page_bytes);
		for entry in entries {
			if let Some(identity) = &entry.mutation {
				super::requests::remember(s, identity.clone(), entry.id.volume_seq);
			}

			super::retained::remember_position(s, &entry.id);
			s.above_cold
				.record(entry.id.volume_seq, entry.encoded_bytes);
			fx.execute(Effect::ReleasePayload {
				payload: entry.payload,
			});
		}

		// Continue paging until the entire validated suffix has been recovered.
		if expected > proof.appended.volume_seq {
			replayed(s, source, proof, fx);
		} else {
			replay_page(s, source, proof, expected, None, start, fx);
		}
		return;
	}

	// Probe replies refine divergence bounds without retaining their payloads.
	if let Some(MatchStep::Probing {
		op_id,
		at,
		search,
		observed,
	}) = s.replicas.get(worker).and_then(|r| r.repair.clone())
		&& op == op_id
	{
		fx.cancel_timer(VolumeClientTimer::Control(op));
		match result {
			Ok((state, entries)) => {
				let equal = entries.first().is_some_and(|e| {
					e.id.volume_seq == at && s.position(at).as_ref() == Some(&e.id)
				});
				let valid = same_claim(s, &state, &observed) && state.appended == observed.appended;
				release(entries, fx);
				if !valid {
					super::repair::backoff(s, worker, fx);
					return;
				}
				probe_result(s, worker, observed, at, search, equal, fx);
			}
			Err(error) => super::repair::failed(s, worker, error, fx),
		}
		return;
	}

	// Obsolete replies still own payload handles that must be released.
	if let Ok((_, entries)) = result {
		release(entries, fx);
	}
}

/// Releases every payload from a page that will not be retained for replay or repair.
fn release(entries: Vec<EntryRef>, fx: &mut Effects) {
	for entry in entries {
		fx.execute(Effect::ReleasePayload {
			payload: entry.payload,
		});
	}
}

/// Require both the disk claim and the exact replica lifetime observed earlier.
fn same_claim(
	s: &VolumeClientState,
	state: &VolumeLogReplicaState,
	proof: &VolumeLogReplicaState,
) -> bool {
	state.replica_incarnation == proof.replica_incarnation
		&& state.claimed_client.as_ref() == Some(&s.client)
}

/// Publish the chosen history as the source position before matching its peers.
fn replayed(
	s: &mut VolumeClientState,
	source: WorkerId,
	proof: VolumeLogReplicaState,
	fx: &mut Effects,
) {
	// The source was matched at its tail when its history was selected. A failed claim entry
	// append may have backed it off since, or an alternate source may have taken over the
	// replay; either way every page it answered proved it in sync under this claim at the
	// recovered tail, so it is matched there again, as its claim reply would match it.
	if !s.replicas[&source].matched {
		match_member_at_tail(s, &source, &proof, fx);
	}
	s.phase = VolumeClientPhase::Recovering(RecoveryPhase::Matching);
	// A source's selected suffix can contain committed history above cold. Bind every
	// unmatched replica to that exact history, with the claim entry on it, never to a
	// provisional pre-replay floor.
	let through = s.next_volume_seq - 1;
	for replica in s.replicas.values_mut().filter(|replica| !replica.matched) {
		replica.repair_goal = Some(through);
	}
	match_replicas(s, fx);
}

/// Each unmatched replica has exactly one conditional operation in flight.
pub(super) fn match_replicas(s: &mut VolumeClientState, fx: &mut Effects) {
	if !matches!(
		s.phase,
		VolumeClientPhase::Recovering(
			RecoveryPhase::Matching
				| RecoveryPhase::CommittingClaimEntry { .. }
				| RecoveryPhase::Finishing { .. }
		) | VolumeClientPhase::Appending
			| VolumeClientPhase::Draining { .. }
	) {
		return;
	}

	// Limit simultaneous repairs and let each unmatched replica own one continuation.
	for worker in s.replicas.keys().cloned().collect::<Vec<_>>() {
		if s.replicas
			.values()
			.filter(|r| {
				r.repair
					.as_ref()
					.is_some_and(|p| !matches!(p, MatchStep::Waiting { .. }))
					|| r.pending_claim.is_some()
			})
			.count() >= 2
		{
			break;
		}

		let r = &s.replicas[&worker];
		if r.matched || r.repair.is_some() || r.pending_claim.is_some() {
			continue;
		}

		let Some(proof) = r.claim.clone() else {
			claim(s, worker.clone(), fx);
			continue;
		};

		let at = proof.appended.volume_seq.min(s.next_volume_seq - 1);
		probe(
			s,
			&worker,
			proof,
			at,
			ProbeSearch::Backward {
				distance: 1,
				upper: at,
			},
			fx,
		);
	}
}

/// Read an epoch at a candidate position without modifying the replica.
fn probe(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	observed: VolumeLogReplicaState,
	at: u64,
	search: ProbeSearch,
	fx: &mut Effects,
) {
	// A known prefix that already covers the finite repair goal does not need to chase
	// a newer cold floor. Its conditional promotion still checks the current replica lifetime.
	if at == observed.appended.volume_seq
		&& s.position(at).as_ref() == Some(&observed.appended)
		&& s.replicas[worker]
			.repair_goal
			.is_some_and(|goal| at >= goal)
	{
		copy(s, worker, observed, fx);
		return;
	}
	if at <= s.cold.volume_seq {
		cut(s, worker, observed, s.cold.clone(), fx);
		return;
	}

	if at == observed.appended.volume_seq && s.position(at).as_ref() == Some(&observed.appended) {
		cut(s, worker, observed, s.position(at).unwrap(), fx);
		return;
	}

	let op = s.operation();
	s.replicas.get_mut(worker).unwrap().repair = Some(MatchStep::Probing {
		op_id: op,
		at,
		search,
		observed,
	});
	fx.arm_timer_after(VolumeClientTimer::Control(op), s.config.append_timeout_ms);
	fx.execute(Effect::ReadVolumeLogEntriesRequest {
		op_id: op,
		worker_id: worker.clone(),
		volume_log_key: s.volume.loaded_key().clone(),
		start: at,
		max_entries: 1,
		max_bytes: s.config.max_append_bytes,
	});
}
/// Truncation retains a matched prefix or marks the replica repairing at its existing cold floor.
fn cut(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	observed: VolumeLogReplicaState,
	keep: VolumeLogPosition,
	fx: &mut Effects,
) {
	if keep == observed.appended {
		copy(s, worker, observed, fx);
		return;
	}

	// A client cannot advance the replica's cold record. Wait for maintainer trim if
	// its floor is older than the selected cold-store image and the missing prefix is gone.
	let mark_repairing = keep == s.cold;
	if mark_repairing && observed.cold != keep {
		super::repair::backoff(s, worker, fx);
		return;
	}
	let op = s.operation();
	let step = MatchStep::Truncating {
		op_id: op,
		keep: keep.clone(),
		mark_repairing,
	};
	let effect = Effect::TruncateRequest {
		op_id: op,
		worker_id: worker.clone(),
		volume_log_key: s.volume.loaded_key().clone(),
		client: s.client.clone(),
		tablet_replica_set_version: s.replica_set.tablet_replica_set_version,
		expected_appended: observed.appended,
		keep,
		mark_repairing,
	};

	s.replicas.get_mut(worker).unwrap().repair = Some(step);
	fx.arm_timer_after(VolumeClientTimer::Control(op), s.config.append_timeout_ms);
	fx.execute(effect);
}

/// Copy retained entries with their original epochs, fetching cache misses from a matched peer.
/// Ends a member's repair at its validated in-sync tail `state`: it is matched there, so
/// its acknowledgements count from that position, and repair bookkeeping is cleared.
fn matched_at(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	state: VolumeLogReplicaState,
	fx: &mut Effects,
) {
	s.loading_members.remove(worker);
	let r = s.replicas.get_mut(worker).unwrap();
	r.repair = None;
	r.repair_attempt = 0;
	r.matched = true;
	r.repair_floor = None;
	r.repair_goal = None;
	r.repair_avoid = None;
	r.incarnation = state.replica_incarnation;
	r.acked = state.appended.volume_seq;
	r.sent = r.acked;
	r.claim = Some(state);
	// Only the validated prefix counts, exactly as an append acknowledgement at that
	// position would. A claim entry still needs a quorum through its own position before
	// opening the client; a shorter matched prefix cannot commit it. A settled claim's
	// matching has no appends of this epoch yet, so there is nothing to count.
	if super::pipeline::claim_entry_counts(s) {
		super::pipeline::commit_progress(s, fx);
	}
	// The next transition pass releases committed payloads before filling windows.
	// Reserve this peer's consecutive appends first, or a matched short prefix would
	// immediately fall back into repair when that release evicts its missing suffix.
	super::pipeline::send_to(s, worker, fx);
}

pub(super) fn copy(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	state: VolumeLogReplicaState,
	fx: &mut Effects,
) {
	let assigned = s.next_volume_seq - 1;
	if !super::membership::proof_eligible(&s.replica_set, worker, &state)
		|| state.claimed_client.as_ref() != Some(&s.client)
		|| state.appended.volume_seq > assigned
	{
		super::repair::backoff(s, worker, fx);
		return;
	}
	// A block the member's own validated tail disproves, because an earlier append carrying
	// the entry was applied after all, or one that cold has passed, no longer applies.
	let goal = {
		let r = s.replicas.get_mut(worker).unwrap();
		r.repair_floor = Some(state.appended.volume_seq);
		if r.blocked.is_some_and(|blocked| {
			state.appended.volume_seq >= blocked.volume_seq
				|| blocked.volume_seq <= s.cold.volume_seq
		}) {
			r.blocked = None;
		}
		*r.repair_goal.get_or_insert(assigned)
	};
	if state.appended.volume_seq >= goal {
		if s.position(state.appended.volume_seq).as_ref() != Some(&state.appended) {
			super::repair::backoff(s, worker, fx);
			return;
		}
		if state.status == VolumeLogReplicaStatus::Repairing {
			super::repair::promote(s, worker, state, fx);
			return;
		}
		if state.status != VolumeLogReplicaStatus::InSync {
			super::repair::backoff(s, worker, fx);
			return;
		}
		matched_at(s, worker, state, fx);
		return;
	}

	// A member at the entry its shard permanently refuses is as caught up as it can be on
	// this handle: it is matched at that tail, where its acknowledgements count.
	let first = state.appended.volume_seq + 1;
	let blocked_at = s.replicas[worker].blocked.map(|blocked| blocked.volume_seq);
	if blocked_at.is_some_and(|blocked| first >= blocked) {
		if state.status == VolumeLogReplicaStatus::Repairing {
			super::repair::promote(s, worker, state, fx);
			return;
		}
		matched_at(s, worker, state, fx);
		return;
	}

	// Copy a consecutive bounded suffix below any block, fetching from a matched peer on a
	// cache miss.
	let limit = u64::from(s.replicas[worker].append_limit_bytes);
	let mut bytes = 128u64;
	let mut next = first;
	let entries: Vec<_> = s
		.retained
		.range(first..)
		.map(|(_, e)| e)
		.take_while(|e| {
			if e.id.volume_seq != next
				|| blocked_at.is_some_and(|blocked| e.id.volume_seq >= blocked)
			{
				return false;
			}
			next += 1;
			bytes += u64::from(e.encoded_bytes) + u64::from(s.entry_frame_overhead);
			bytes <= limit
		})
		.cloned()
		.collect();
	let Some(end) = entries.last().map(|e| e.id.clone()) else {
		// A retained first entry that does not fit the member's limit on its own can never
		// be copied to it; no smaller copy exists. The member holds everything below it,
		// which its validated state proves, so it is matched at that tail, where its
		// acknowledgements count, and blocked there.
		if let Some(entry) = s.retained.get(&first)
			&& s.single_entry_request_bytes(entry.encoded_bytes) > limit
		{
			if state.status == VolumeLogReplicaStatus::Repairing {
				super::repair::promote(s, worker, state, fx);
				return;
			}
			let max_bytes = s.replicas[worker].append_limit_bytes;
			matched_at(s, worker, state, fx);
			super::pipeline::block(
				s,
				worker,
				first,
				PermanentRefusal::EntryTooLarge { max_bytes },
				fx,
			);
			return;
		}
		super::repair::fetch(s, worker, state, fx);
		return;
	};

	super::requests::submitted(s, &entries);
	let op = s.operation();
	s.replicas.get_mut(worker).unwrap().repair = Some(MatchStep::Copying {
		op_id: op,
		end,
		bytes,
	});
	fx.arm_timer_after(VolumeClientTimer::Control(op), s.config.append_timeout_ms);
	fx.execute(Effect::AppendRequest {
		op_id: op,
		worker_id: worker.clone(),
		volume_log_key: s.volume.loaded_key().clone(),
		client: s.client.clone(),
		tablet_replica_set_version: s.replica_set.tablet_replica_set_version,
		previous: state.appended,
		entries,
		committed_volume_seq: s.commit_publishing.committed,
	});
}
/// Mutation replies are fenced by operation, client, and replica incarnation (except a repairing truncate).
pub(super) fn matched(
	s: &mut VolumeClientState,
	op: OpId,
	worker: &WorkerId,
	result: Result<VolumeLogReplicaState, RpcError>,
	_entropy: Entropy,
	fx: &mut Effects,
) {
	let Some(step) = s.replicas.get(worker).and_then(|r| r.repair.clone()) else {
		return;
	};

	if step.op_id() != op {
		return;
	}

	fx.cancel_timer(VolumeClientTimer::Control(op));
	let state = match result {
		Ok(state) => state,
		// A pass trimmed the target to a newer cold while the copy was on its way, so the copy
		// starts below the target's cold. The refusal carries the target's current state: when
		// it is still this client's claim on the same replica, continue the copy from there at
		// once. Backing off instead races every later pass the same way, which kept a learner
		// repairing, and its catch-up unreported, for as long as writes continued.
		Err(RpcError::Rejected(
			rivet_data::generated::depot_storage_worker_protocol_v1::RejectResponse::BelowCold(
				refusal,
			),
		)) if matches!(step, MatchStep::Copying { .. })
			&& refusal.state.claimed_client.as_ref() == Some(&s.client)
			&& refusal.state.replica_incarnation == s.replicas[worker].incarnation =>
		{
			copy(s, worker, refusal.state, fx);
			return;
		}
		Err(error) => {
			super::repair::failed(s, worker, error, fx);
			return;
		}
	};

	// Validate the exact result promised by this conditional repair operation.
	let expected = match &step {
		MatchStep::Truncating { keep, .. } => keep,
		MatchStep::Copying { end, .. } => end,
		MatchStep::Promoting { expected, .. } => &expected.appended,
		_ => return,
	};
	// Only a conditional promotion may race a newer canonical cold image. Copy and
	// truncate completions still promise one exact position; an arbitrary larger tail
	// cannot replace their proof or the selected history's finite repair goal.
	let newer_cold = matches!(&step, MatchStep::Promoting { expected, .. }
		if state.status == VolumeLogReplicaStatus::InSync
			&& !state.lost_entries
			&& state.appended == state.cold
			&& state.appended.volume_seq > expected.appended.volume_seq
			&& state.appended.client_epoch == expected.appended.client_epoch
			&& state.appended.client_epoch <= s.client.client_epoch
			&& state.appended.volume_seq < s.next_volume_seq
			&& expected.replica_incarnation == state.replica_incarnation
			&& expected.claimed_client.as_ref() == Some(&s.client)
			&& s.position(expected.appended.volume_seq).as_ref() == Some(&expected.appended)
			&& s.position(state.appended.volume_seq).as_ref() == Some(&state.appended)
			&& s.replicas[worker].repair_goal.is_some_and(|goal| expected.appended.volume_seq >= goal));

	let valid = super::membership::proof_eligible(&s.replica_set, worker, &state)
		&& state.claimed_client.as_ref() == Some(&s.client)
		&& (&state.appended == expected || newer_cold)
		&& (matches!(
			step,
			MatchStep::Truncating {
				mark_repairing: true,
				..
			}
		) || state.replica_incarnation == s.replicas[worker].incarnation);
	if !valid {
		super::repair::backoff(s, worker, fx);
		return;
	}

	let goal = super::repair::goal(s);
	let replica = s.replicas.get_mut(worker).unwrap();
	if replica.incarnation != state.replica_incarnation {
		replica.repair_goal = goal;
	}
	replica.incarnation = state.replica_incarnation;
	copy(s, worker, state, fx);
}

/// Retains this opening's claim entry at the next position, under this client's epoch. The
/// pipeline sends it to every matched member from here on, and the stall timer re-drives
/// it. The reserved payload handle denotes the empty claim entry body, materialized by the
/// executor. Returns false, having failed the client, when the entry does not fit.
fn retain_claim_entry(s: &mut VolumeClientState, fx: &mut Effects) -> bool {
	let seq = s.next_volume_seq;
	let Some(next) = seq.checked_add(1) else {
		super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
		return false;
	};
	if s.config
		.max_memory_bytes
		.saturating_sub(s.memory_bytes)
		.saturating_sub(super::waiting::reserved_bytes(s))
		< 25
	{
		super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
		return false;
	}

	let written_at_unix_ms = fx.clocks().unix_ms;
	s.claim_entry = Some(super::state::AppendedClaimEntry {
		volume_seq: seq,
		written_at_unix_ms,
	});
	s.next_volume_seq = next;
	s.retained.insert(
		seq,
		EntryRef {
			written_at_unix_ms,
			id: VolumeLogPosition {
				volume_seq: seq,
				client_epoch: s.client.client_epoch,
			},
			mutation: None,
			payload: PayloadId(0),
			encoded_bytes: 25,
		},
	);
	s.memory_bytes += 25;
	s.above_cold.record(seq, 25);
	true
}

/// Ends matching once a quorum is matched. A settled claim serves at once. Otherwise the
/// client waits for its claim entry, retained when its history was selected, to reach a
/// quorum; that may already have happened while the replay ran, in which case the wait
/// ends in the same pass. The stall timer starts here, as the append timers and refusal
/// delays re-drive the entry during the replay and matching on their own.
pub(super) fn end_matching(s: &mut VolumeClientState, fx: &mut Effects) {
	if s.phase != VolumeClientPhase::Recovering(RecoveryPhase::Matching)
		|| !super::membership::quorum(&s.replica_set, |worker| {
			s.replicas.get(worker).is_some_and(|r| r.matched)
		}) {
		return;
	}

	super::metrics::claim_finished(s.claim_settled);

	// A settled claim serves at once, and its first write builds on the settled tail.
	if s.claim_settled {
		s.commit_publishing.committed = s
			.commit_publishing
			.committed
			.max(s.next_volume_seq.saturating_sub(1));
		s.phase = VolumeClientPhase::Appending;
		super::waiting::ready(s, Ok(()), fx);
		return;
	}

	// Recovery retains the claim entry with the history it selects, so an unsettled claim
	// that reaches matching without one has lost its history.
	let Some(claim) = s.claim_entry else {
		super::lifecycle::fail(s, WriteError::RecoveryFailed, fx);
		return;
	};
	s.phase = VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry {
		volume_seq: claim.volume_seq,
	});
	fx.arm_timer_after(VolumeClientTimer::Stall, s.config.stall_timeout_ms);
	committed(s, fx);
}

/// Open the volume client only after its own claim entry reaches a quorum.
pub(super) fn committed(s: &mut VolumeClientState, fx: &mut Effects) {
	if let VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry { volume_seq }) =
		s.phase
		&& s.commit_publishing.committed >= volume_seq
	{
		if s.purpose == OpenPurpose::Recover {
			s.phase = VolumeClientPhase::Recovering(RecoveryPhase::Finishing { volume_seq });
			fx.arm_timer_after(
				VolumeClientTimer::FinishRecovery,
				s.config.finish_recovery_timeout_ms,
			);
		} else {
			s.phase = VolumeClientPhase::Appending;
			super::waiting::ready(s, Ok(()), fx);
		}
	}
}

/// Discard an incomplete recovery round when its correlated I/O expires.
pub(super) fn expired(
	s: &mut VolumeClientState,
	op: OpId,
	_entropy: Entropy,
	fx: &mut Effects,
) -> bool {
	// An unanswered replay read is read again, and an unanswered repair step backs off that
	// replica alone. Neither makes the client claim again.
	if let VolumeClientPhase::Recovering(RecoveryPhase::Replaying {
		op_id,
		source,
		proof,
		next,
		verify,
		start,
	}) = s.phase.clone()
		&& op_id == op
	{
		if !alternate_replay_source(s, fx) {
			retry_replay(s, source, proof, next, verify, start, fx);
		}
		return true;
	}
	let repairing = s
		.replicas
		.iter()
		.find(|(_, r)| r.repair.as_ref().is_some_and(|step| step.op_id() == op))
		.map(|(worker, _)| worker.clone());
	match repairing {
		Some(worker) => {
			super::repair::backoff(s, &worker, fx);
			true
		}
		None => false,
	}
}

/// Reduce logarithmic probe bounds without ever changing the replica during discovery.
#[allow(
	clippy::too_many_arguments,
	reason = "probe continuation retains the search bounds and correlated replica observation"
)]
fn probe_result(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	observed: VolumeLogReplicaState,
	at: u64,
	search: ProbeSearch,
	equal: bool,
	fx: &mut Effects,
) {
	// Cold can advance while this probe is in flight. Its old bracket no longer
	// proves a retained position, so restart matching at the current cold floor.
	if at <= s.cold.volume_seq
		|| matches!(
			&search,
			ProbeSearch::Binary { low, .. } if *low < s.cold.volume_seq
		) {
		cut(s, worker, observed, s.cold.clone(), fx);
		return;
	}

	// Find a matching lower bound, then narrow the first divergent position.
	let (low, high) = match search {
		ProbeSearch::Backward { distance, upper } if !equal => {
			probe(
				s,
				worker,
				observed,
				at.saturating_sub(distance),
				ProbeSearch::Backward {
					distance: distance.saturating_mul(2),
					upper: at.min(upper),
				},
				fx,
			);
			return;
		}
		ProbeSearch::Backward { upper, .. } => (at, upper),
		ProbeSearch::Binary { low, high } => {
			if equal {
				(at, high)
			} else {
				(low, at)
			}
		}
	};

	// Only a fully narrowed bracket authorizes cutting the divergent suffix.
	if high.saturating_sub(low) <= 1 {
		let Some(keep) = s.position(low) else {
			super::repair::backoff(s, worker, fx);
			return;
		};
		cut(s, worker, observed, keep, fx);
	} else {
		probe(
			s,
			worker,
			observed,
			low + (high - low) / 2,
			ProbeSearch::Binary { low, high },
			fx,
		);
	}
}



/// Accepts an open that created the volume with this client's initial writes. The maintainer
/// wrote them with the replicas and this client's claim, at the sequences after cold, and an
/// in-sync quorum of members answered holding them. So they are committed history under this
/// client's epoch, nothing lies below them and no older client exists: the client takes them
/// as recovered history, marks those members matched, and starts appending with no replay and
/// no claim entry. Members that did not answer are repaired by the normal matching loop. Returns
/// false, releasing the initial writes, when the open did not create the volume with them.
pub(super) fn created(
	s: &mut VolumeClientState,
	created: Option<&VolumeLogPosition>,
	states: &[rivet_data::generated::depot_client_protocol_v1::MemberReplicaState],
	fx: &mut Effects,
) -> bool {
	let initial = std::mem::take(&mut s.initial_writes);
	let through = s.cold.volume_seq + initial.len() as u64;
	let answering: Vec<(WorkerId, VolumeLogReplicaState)> = states
		.iter()
		.map(|member| (WorkerId::from_bytes(member.worker_id), member.state.clone()))
		.filter(|(worker, state)| {
			s.replicas.contains_key(worker)
				&& super::membership::proof_eligible(&s.replica_set, worker, state)
				&& state.status == VolumeLogReplicaStatus::InSync
				&& state.claimed_client.as_ref() == Some(&s.client)
				&& state.cold == s.cold
				&& state.appended.volume_seq == through
				&& state.appended.client_epoch == s.client.client_epoch
		})
		.collect();
	let accepted = !initial.is_empty()
		&& created.is_some_and(|created| {
			created.volume_seq == through && created.client_epoch == s.client.client_epoch
		}) && super::membership::quorum(&s.replica_set, |worker| {
		answering.iter().any(|(answered, _)| answered == worker)
	});
	if !accepted {
		release_writes(&initial, fx);
		return false;
	}

	// The writes are committed history now, held by the answering quorum, so the client keeps
	// their positions and retry identities, exactly as after replaying them. Writes after them
	// continue their `client_seq`.
	for (offset, write) in initial.iter().enumerate() {
		let position = VolumeLogPosition {
			volume_seq: s.cold.volume_seq + 1 + offset as u64,
			client_epoch: s.client.client_epoch,
		};
		super::requests::remember(s, write.identity.clone(), position.volume_seq);
		super::retained::remember_position(s, &position);
		s.above_cold
			.record(position.volume_seq, write.encoded_bytes);
	}
	release_writes(&initial, fx);

	s.next_volume_seq = through + 1;
	s.commit_publishing.committed = s.commit_publishing.committed.max(through);
	s.created = Some(VolumeLogPosition {
		volume_seq: through,
		client_epoch: s.client.client_epoch,
	});
	for (worker, state) in answering {
		s.loading_members.remove(&worker);
		let replica = s.replicas.get_mut(&worker).expect("answering member");
		replica.matched = true;
		replica.repair_floor = None;
		replica.repair_goal = None;
		replica.repair_avoid = None;
		replica.incarnation = state.replica_incarnation;
		replica.acked = through;
		replica.sent = through;
		replica.claim = Some(state);
	}
	s.claim_quorum = s
		.replicas
		.iter()
		.filter(|(_, replica)| replica.matched)
		.map(|(worker, _)| worker.clone())
		.collect();
	s.phase = VolumeClientPhase::Appending;
	super::waiting::ready(s, Ok(()), fx);
	true
}

/// Releases initial writes the open did not apply; the owner writes them as usual.
pub(super) fn release_initial_writes(s: &mut VolumeClientState, fx: &mut Effects) {
	let initial = std::mem::take(&mut s.initial_writes);
	release_writes(&initial, fx);
}

fn release_writes(writes: &[super::state::InitialWrite], fx: &mut Effects) {
	for write in writes {
		fx.execute(Effect::ReleasePayload {
			payload: write.payload,
		});
	}
}
