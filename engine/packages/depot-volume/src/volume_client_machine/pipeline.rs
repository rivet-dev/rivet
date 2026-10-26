//! Bounded admission and quorum progress. Every acknowledgement is tied to an issued operation.

use super::{
	Effects,
	effect::VolumeClientEffect as Effect,
	event::RpcError,
	state::{BlockedAppend, PendingAppend, RecoveryPhase, VolumeClientPhase, VolumeClientState},
	types::*,
};
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	depot_storage_worker_protocol_v1::RejectResponse,
	shared_depot_types_v1::{VolumeLogPosition, VolumeLogReplicaStatus},
};
use rivet_topology::worker::WorkerId;

/// Fill each replica independently; a slow third replica cannot stall a quorum.
pub(super) fn send(s: &mut VolumeClientState, fx: &mut Effects) {
	for worker in s.replicas.keys().cloned().collect::<Vec<_>>() {
		send_to(s, &worker, fx);
	}
}

/// Whether this client's appends may be sent and their acknowledgements counted now: while
/// it serves, drains or finishes recovery, and during the replay and matching once it has
/// retained its claim entry, which goes to the members matched at the recovered tail while
/// the rest of recovery runs.
pub(super) fn claim_entry_counts(s: &VolumeClientState) -> bool {
	match &s.phase {
		VolumeClientPhase::Appending
		| VolumeClientPhase::Draining { .. }
		| VolumeClientPhase::Recovering(
			RecoveryPhase::CommittingClaimEntry { .. } | RecoveryPhase::Finishing { .. },
		) => true,
		VolumeClientPhase::Recovering(
			RecoveryPhase::Replaying { .. } | RecoveryPhase::Matching,
		) => s.claim_entry.is_some(),
		VolumeClientPhase::Recovering(RecoveryPhase::Claiming)
		| VolumeClientPhase::Opening { .. }
		| VolumeClientPhase::Closing { .. }
		| VolumeClientPhase::Stopped
		| VolumeClientPhase::Fenced { .. } => false,
	}
}

/// Reserve one peer's ordinary append window before committed payloads may be released.
pub(super) fn send_to(s: &mut VolumeClientState, worker: &WorkerId, fx: &mut Effects) {
	if !claim_entry_counts(s) {
		return;
	}

	// A block that cold has passed no longer stops the replica: the refused entry left
	// the log, so repair catches the replica up from cold like any member that fell behind.
	if s.replicas[worker]
		.blocked
		.is_some_and(|blocked| blocked.volume_seq <= s.cold.volume_seq)
	{
		s.replicas.get_mut(worker).unwrap().blocked = None;
		super::repair::backoff(s, worker, fx);
		return;
	}

	// Each replica has its own send window, progress watermark and append limit.
	loop {
		let r = &s.replicas[worker];
		if !r.sendable(s.config.max_in_flight) || r.sent + 1 >= s.next_volume_seq {
			break;
		}
		let first = r.sent + 1;
		if !r.can_append(first) {
			break;
		}

		let Some(previous) = s.position(r.sent) else {
			break;
		};

		let limit = u64::from(r.append_limit_bytes);
		let blocked_at = r.blocked.map(|blocked| blocked.volume_seq);
		let mut bytes = 128u64;
		let mut next = first;
		let entries: Vec<_> = s
			.retained
			.range(first..)
			.map(|(_, e)| e)
			.take_while(|entry| {
				if entry.id.volume_seq != next
					|| blocked_at.is_some_and(|blocked| entry.id.volume_seq >= blocked)
				{
					return false;
				}
				next += 1;
				bytes += u64::from(entry.encoded_bytes) + u64::from(s.entry_frame_overhead);
				bytes <= limit
			})
			.cloned()
			.collect();
		if entries.is_empty() {
			// A retained first entry that does not fit the replica's limit on its own can
			// never be sent to it; no smaller batch exists.
			if let Some(entry) = s.retained.get(&first)
				&& s.single_entry_request_bytes(entry.encoded_bytes) > limit
			{
				let max_bytes = r.append_limit_bytes;
				block(
					s,
					worker,
					first,
					PermanentRefusal::EntryTooLarge { max_bytes },
					fx,
				);
			}
			break;
		}
		dispatch(s, worker.clone(), previous, entries, fx);
	}
}
/// Reserve the operation before emitting I/O so even an immediate reply is correlated.
pub(super) fn dispatch(
	s: &mut VolumeClientState,
	worker: WorkerId,
	previous: VolumeLogPosition,
	entries: Vec<EntryRef>,
	fx: &mut Effects,
) {
	debug_assert!(!entries.is_empty());
	super::requests::submitted(s, &entries);
	let op_id = OpId(s.next_op);
	s.next_op = s.next_op.checked_add(1).expect("operation ID exhausted");
	let end = entries
		.last()
		.map_or(previous.volume_seq, |e| e.id.volume_seq);
	let bytes = entries.iter().fold(128u64, |bytes, entry| {
		bytes + u64::from(entry.encoded_bytes) + u64::from(s.entry_frame_overhead)
	});
	let r = s.replicas.get_mut(&worker).unwrap();
	r.sent = r.sent.max(end);
	r.in_flight.insert(
		op_id,
		PendingAppend {
			end,
			committed: s.commit_publishing.committed,
			confirmation: None,
			bytes,
		},
	);
	fx.arm_timer_after(VolumeClientTimer::Append(op_id), s.config.append_timeout_ms);
	fx.execute(Effect::AppendRequest {
		op_id,
		worker_id: worker,
		volume_log_key: s.volume.loaded_key().clone(),
		client: s.client.clone(),
		tablet_replica_set_version: s.replica_set.tablet_replica_set_version,
		previous,
		entries,
		committed_volume_seq: s.commit_publishing.committed,
	});
}

/// Ignore unknown or duplicated responses before examining even a fencing rejection.
pub(super) fn answered(
	s: &mut VolumeClientState,
	op: OpId,
	worker: &WorkerId,
	result: Result<VolumeLogReplicaState, RpcError>,
	entropy: rivet_state_machine::Entropy,
	fx: &mut Effects,
) {
	if s.replicas
		.get(worker)
		.and_then(|r| r.repair.as_ref())
		.is_some_and(|r| r.op_id() == op)
	{
		super::recovery::matched(s, op, worker, result, entropy, fx);
		return;
	}

	// Retire only a known append before evaluating its reply or rejection.
	let Some(r) = s.replicas.get_mut(worker) else {
		return;
	};

	let Some(pending) = r.in_flight.remove(&op) else {
		late_acknowledgement(s, op, worker, &result, fx);
		return;
	};

	fx.cancel_timer(VolumeClientTimer::Append(op));
	let cold = match result {
		Err(RpcError::Rejected(RejectResponse::VolumeLogReplicaDeleted(_))) => {
			fence_with_error(s, WriteError::VolumeNotFound, fx);
			return;
		}
		Err(RpcError::Rejected(RejectResponse::StaleTabletReplicaSet(reject))) => {
			if !super::membership::adopt(s, reject.replica_set, fx) {
				super::repair::backoff(s, worker, fx);
			}
			return;
		}
		Err(RpcError::Rejected(RejectResponse::VolumeNotLoaded))
			if s.loading_members.contains(worker)
				|| !super::membership::voter(&s.replica_set, worker) =>
		{
			super::repair::backoff(s, worker, fx);
			return;
		}
		Err(RpcError::Rejected(RejectResponse::VolumeNotLoaded)) => {
			// Appends continue on the loaded members while repair retries this one. Already
			// committed writes are unaffected by unloading the volume.
			super::lifecycle::member_not_loaded(s, worker, fx);
			return;
		}
		Err(RpcError::Rejected(RejectResponse::Overloaded(refusal))) => {
			refused(s, worker, refusal.retry_after_ms, Refusal::Overloaded, fx);
			return;
		}
		// This process's transport refused the append before sending it, such as a full
		// queue to the worker. The replica's log is unchanged and unknown, so this is a
		// refusal like the shard's own: the replica waits the hint and resends its own
		// admitted entries, and is never repaired for it. New writes are refused at
		// admission while a quorum waits, which returns the overload to the caller.
		Err(RpcError::Overloaded { retry_after_ms }) => {
			refused(s, worker, retry_after_ms, Refusal::Overloaded, fx);
			return;
		}
		Err(RpcError::Rejected(RejectResponse::DiskFull(refusal))) => {
			// Disk admission changed nothing. The shard's hint leaves time for cold writes
			// and trims to free space without claiming again.
			refused(s, worker, refusal.retry_after_ms, Refusal::DiskFull, fx);
			super::requests::shed_if_disk_full(s, fx);
			return;
		}
		// A size refusal of an append applied nothing and names the shard's limit. Resending
		// the same batch would be refused the same way, so the replica's batches shrink
		// to the limit instead; an entry that cannot fit alone blocks the replica in `send`.
		Err(RpcError::Rejected(RejectResponse::RequestTooLarge(reject))) if pending.bytes > 0 => {
			too_large(s, worker, pending.bytes, reject.max_bytes, fx);
			return;
		}
		// The shard cannot read an entry's payload encoding. Writers select versions every
		// live storage worker reads, so this means the shard's binary is older than the
		// writer's view; it applied nothing. The replica waits and resends like any refusal,
		// so it catches up once its shard is upgraded, and a quorum refusing sheds new writes
		// at admission meanwhile.
		Err(RpcError::Rejected(RejectResponse::UnsupportedPayloadVersion(_)))
			if pending.bytes > 0 =>
		{
			refused(
				s,
				worker,
				MAX_REFUSAL_DELAY_MS,
				Refusal::UnsupportedPayloadVersion,
				fx,
			);
			return;
		}
		Ok(state)
			if state.status == VolumeLogReplicaStatus::InSync
				&& state.replica_incarnation == r.incarnation
				&& state.claimed_client.as_ref() == Some(&s.client)
				&& state.appended.volume_seq >= pending.end =>
		{
			r.acked = r.acked.max(pending.end);
			state.cold
		}
		Err(RpcError::Rejected(
			RejectResponse::StaleClientEpoch(_) | RejectResponse::ClientConflict(_),
		)) => {
			fence(s, fx);
			return;
		}
		// An answer that is not this client's in-sync claim at the appended tail, and every
		// other rejection or transport failure, backs the replica off into repair. An
		// announcement carries no entry, so a size or payload refusal of one is a shard fault
		// rather than evidence about an entry; it backs off like any other failure.
		Ok(_)
		| Err(RpcError::Rejected(
			RejectResponse::DiskIncarnationMismatch(_)
			| RejectResponse::NotClaimed(_)
			| RejectResponse::VolumeSeqGap(_)
			| RejectResponse::Diverged(_)
			| RejectResponse::AppendedMismatch(_)
			| RejectResponse::BelowCold(_)
			| RejectResponse::VolumeLogReplicaIncarnationMismatch(_)
			| RejectResponse::StaleTabletGrantEpoch(_)
			| RejectResponse::NotServing
			| RejectResponse::RequestTooLarge(_)
			| RejectResponse::TooManyLoadedVolumes(_)
			| RejectResponse::UnsupportedPayloadVersion(_),
		))
		| Err(
			RpcError::LifecycleRejected(_) | RpcError::RoutingUnavailable | RpcError::Unavailable,
		) => {
			super::repair::backoff(s, worker, fx);
			return;
		}
	};

	commit_progress(s, fx);

	// Fresh confirmation evidence belongs only to the barrier that issued this append.
	if let Some(request) = pending.confirmation
		&& let Some(barrier) = s.barriers.get_mut(&request)
	{
		barrier.confirmed.insert(worker.clone());
	}

	super::retained::observe_cold(s, cold, fx);
}

/// Counts a reply to an append this client already retired at its append timeout. Under shard
/// latency above the append timeout every reply arrives after its append was retired, so
/// ignoring them would leave the quorum without progress for as long as the latency lasts.
/// The replica answers only once its state is on disk, and a replica claimed by this client
/// with the same replica incarnation holds, above the position it was matched at, only
/// entries this client sent at their assigned positions, so its appended position, at most
/// the assigned end, is evidence of its durable tail at this epoch. A reply
/// to an append a membership change retired is not a vote under the new set and is ignored,
/// as is any other reply or a replica under repair.
fn late_acknowledgement(
	s: &mut VolumeClientState,
	op: OpId,
	worker: &WorkerId,
	result: &Result<VolumeLogReplicaState, RpcError>,
	fx: &mut Effects,
) {
	if !claim_entry_counts(s) {
		return;
	}
	if op.0 < s.late_ack_floor {
		return;
	}
	let Ok(state) = result else {
		return;
	};
	let assigned = s.next_volume_seq - 1;
	let Some(r) = s.replicas.get_mut(worker) else {
		return;
	};
	if !r.matched
		|| r.repair.is_some()
		|| state.status != VolumeLogReplicaStatus::InSync
		|| state.replica_incarnation != r.incarnation
		|| state.claimed_client.as_ref() != Some(&s.client)
	{
		return;
	}
	let tail = state.appended.volume_seq.min(assigned);
	if tail <= r.acked {
		return;
	}
	// The shard's own tail is evidence: a replica that holds the entry it was blocked at
	// took it after all, so the block no longer applies.
	if r.blocked.is_some_and(|blocked| tail >= blocked.volume_seq) {
		r.blocked = None;
	}
	r.acked = tail;
	r.sent = r.sent.max(tail);
	commit_progress(s, fx);
}

/// Re-evaluate quorum after either an append acknowledgement or completed replica repair.
/// A repaired replica can already contain the entire tail, leaving no append to trigger progress.
pub(super) fn commit_progress(s: &mut VolumeClientState, fx: &mut Effects) {
	// Count only voters in the current phase; Joint requires both majorities.
	let old_committed = s.commit_publishing.committed;
	let committed = super::membership::position(&s.replica_set, |worker| {
		s.replicas
			.get(worker)
			.filter(|r| r.matched)
			.map_or(0, |r| r.acked)
	});
	// Entries of older epochs never commit by count: a quorum that holds them under this
	// claim could still lose them to a writer that claimed a minority holding a newer entry
	// before this one claimed. Only this client's own claim entry on a quorum commits them,
	// so nothing advances until that entry is acknowledged, and nothing at all while recovery
	// has not retained one. A settled claim has no uncommitted older entries to count.
	let floor = match (s.claim_settled, s.claim_entry, &s.phase) {
		(true, _, _) => None,
		(false, Some(claim), _) => Some(claim.volume_seq),
		(false, None, VolumeClientPhase::Recovering(_)) => Some(u64::MAX),
		(false, None, _) => None,
	};
	let committed = if floor.is_some_and(|floor| committed < floor) {
		old_committed
	} else {
		committed
	};
	s.commit_publishing.committed = old_committed.max(committed);
	if s.commit_publishing.committed > old_committed {
		super::requests::progress(s, fx);
	}

	super::recovery::committed(s, fx);
	super::requests::completed(s, fx);
	super::requests::settle_quiet(s);
}

/// Retries preserve entry identities and bytes; obsolete operation IDs stop counting.
pub(super) fn retry_replica(s: &mut VolumeClientState, worker: &WorkerId, fx: &mut Effects) {
	let r = s.replicas.get_mut(worker).unwrap();
	for op in r.in_flight.keys() {
		fx.cancel_timer(VolumeClientTimer::Append(*op));
	}
	r.in_flight.clear();
	r.sent = r.acked;
}

/// The longest retry delay a refusal can impose. Caller deadlines keep running meanwhile,
/// so a longer hint would only turn a refusal into a missed deadline.
pub(super) const MAX_REFUSAL_DELAY_MS: u64 = 1_000;

/// A replica refused an append or announcement for memory or disk pressure, or for a
/// payload version its shard cannot read yet, doing no work.
/// Its outstanding operations are retired and it resends from its acknowledged position
/// once the delay has passed. It stays matched with its claim: a refusal is not evidence
/// about its log, so it never starts repair, reopens or fences, and other replicas keep
/// committing meanwhile. Callers keep their own quorum deadlines, except that a quorum
/// refusing for a full disk releases them; see [`super::requests::shed_if_disk_full`].
pub(super) fn refused(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	retry_after_ms: u64,
	refusal: Refusal,
	fx: &mut Effects,
) {
	retry_replica(s, worker, fx);
	let op = s.operation();
	let delay_ms = retry_after_ms.clamp(1, MAX_REFUSAL_DELAY_MS);
	let until_boot_ms = fx.clocks().boot_ms.saturating_add(delay_ms);
	let r = s.replicas.get_mut(worker).unwrap();
	let delay = super::state::RefusalDelay {
		op_id: op,
		until_boot_ms,
		disk_full: refusal == Refusal::DiskFull,
	};
	if let Some(old) = r.refused.replace(delay) {
		fx.cancel_timer(VolumeClientTimer::Resume(old.op_id));
	}
	fx.arm_timer_after(VolumeClientTimer::Resume(op), delay_ms);
}

/// Why a replica refused an append without doing any work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
	/// A queue or memory budget on the shard, or this process's transport, was full.
	Overloaded,
	/// The shard's disk budget was full.
	DiskFull,
	/// The shard's binary does not read an entry's payload version yet.
	UnsupportedPayloadVersion,
}

/// Whether the replicas waiting out a full-disk refusal form a quorum, so no admitted entry
/// can commit until the shards write to cold and trim.
pub(super) fn quorum_disk_full(s: &VolumeClientState) -> bool {
	super::membership::quorum(&s.replica_set, |worker| {
		s.replicas
			.get(worker)
			.is_some_and(|replica| replica.refused.is_some_and(|delay| delay.disk_full))
	})
}

/// Whether too few replicas are free of a refusal's retry delay to form a quorum.
pub(super) fn quorum_refusing(s: &VolumeClientState) -> bool {
	!quorum_free_at(s, 0)
}

/// Whether the replicas whose refusal delays end by `boot_ms` form a quorum.
fn quorum_free_at(s: &VolumeClientState, boot_ms: u64) -> bool {
	super::membership::quorum(&s.replica_set, |worker| {
		s.replicas.get(worker).is_none_or(|replica| {
			replica
				.refused
				.is_none_or(|delay| delay.until_boot_ms <= boot_ms)
		})
	})
}

/// How long a caller refused while a quorum is refusing waits: until the earliest moment the
/// shards' own hints free a quorum, so the shard's jittered hint reaches the edge.
pub(super) fn quorum_refusal_retry_after_ms(s: &VolumeClientState, now_boot_ms: u64) -> u64 {
	let mut ends: Vec<u64> = s
		.replicas
		.values()
		.filter_map(|replica| replica.refused.map(|delay| delay.until_boot_ms))
		.collect();
	ends.sort_unstable();
	let free_at = ends
		.into_iter()
		.find(|end| quorum_free_at(s, *end))
		.unwrap_or(now_boot_ms.saturating_add(MAX_REFUSAL_DELAY_MS));
	free_at.saturating_sub(now_boot_ms).max(1)
}

/// Ends a refusal's retry delay and resends at once, before the shared pass after the event
/// could take the idle, lagging replica for one that needs repair. A delay that a repair or
/// fence already ended is ignored.
pub(super) fn resume(s: &mut VolumeClientState, op: OpId, fx: &mut Effects) {
	if let Some(r) = s
		.replicas
		.values_mut()
		.find(|r| r.refused.is_some_and(|delay| delay.op_id == op))
	{
		r.refused = None;
		send(s, fx);
	}
}

/// Ends a refusal's retry delay early because the replica leaves the send path.
pub(super) fn clear_refusal(r: &mut super::state::Replica, fx: &mut Effects) {
	if let Some(delay) = r.refused.take() {
		fx.cancel_timer(VolumeClientTimer::Resume(delay.op_id));
	}
}

/// Retries the replica only if this append operation is still outstanding.
pub(super) fn expired(s: &mut VolumeClientState, op: OpId, fx: &mut Effects) {
	if let Some(worker) = s
		.replicas
		.iter()
		.find(|(_, r)| r.in_flight.contains_key(&op))
		.map(|(w, _)| w.clone())
	{
		retry_replica(s, &worker, fx);
		probe_membership(s, &worker, fx);
	}
}

/// A shard refused an append of `request_bytes` as too large and reported `max_bytes`, the
/// most it accepts. The refusal applied nothing and says nothing about the replica's log,
/// so it starts no repair: the replica's append limit drops to the reported limit, and in
/// any case below the refused request, so the next batch is strictly smaller even when the
/// shard's accounting of the same bytes is larger than this client's. The window resends
/// from the acknowledged position in batches that fit; an entry that cannot fit on its own
/// blocks the replica in `send`.
fn too_large(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	request_bytes: u64,
	max_bytes: u32,
	fx: &mut Effects,
) {
	let r = s.replicas.get_mut(worker).unwrap();
	let below_request = request_bytes.saturating_sub(1).min(u64::from(u32::MAX)) as u32;
	let limit = r.append_limit_bytes.min(max_bytes).min(below_request);
	tracing::warn!(
		worker_id = ?worker,
		request_bytes,
		max_bytes,
		append_limit_bytes = limit,
		client_epoch = s.client.client_epoch,
		"log shard refused an append as too large; lowering this replica's append limit, likely a shard write limit below the client's"
	);
	r.append_limit_bytes = limit;
	retry_replica(s, worker, fx);
	// Resend at once, before the shared pass after the event could take the idle, lagging
	// replica for one that needs repair; see `resume`. This is also where an entry that
	// cannot fit the new limit on its own blocks the replica.
	send(s, fx);
}

/// Stops sending `volume_seq` and everything behind it to `worker`: its shard refuses that
/// entry for a reason no retry changes. Appends reaching the block are retired without
/// evidence while those below it stay in flight, entries below the block still go to the
/// replica, and no repair, new claim or fence follows, because a refusal says nothing about its
/// log. It keeps its claim and matched state while it still lacks entries below the block;
/// see [`BlockedAppend`] for what follows. Callers whose writes no quorum can commit any
/// more are failed at once.
pub(super) fn block(
	s: &mut VolumeClientState,
	worker: &WorkerId,
	volume_seq: u64,
	refusal: PermanentRefusal,
	fx: &mut Effects,
) {
	let r = s.replicas.get_mut(worker).unwrap();
	let reaching: Vec<OpId> = r
		.in_flight
		.iter()
		.filter(|(_, pending)| pending.bytes > 0 && pending.end >= volume_seq)
		.map(|(op, _)| *op)
		.collect();
	for op in reaching {
		fx.cancel_timer(VolumeClientTimer::Append(op));
		r.in_flight.remove(&op);
	}
	r.sent = r
		.in_flight
		.values()
		.filter(|pending| pending.bytes > 0)
		.map(|pending| pending.end)
		.max()
		.unwrap_or(r.acked)
		.max(r.acked);
	clear_refusal(r, fx);
	// Repair reaching an existing block reports it again; only a new or lower block counts.
	if r.blocked
		.is_none_or(|existing| existing.volume_seq > volume_seq)
	{
		r.blocked = Some(BlockedAppend {
			volume_seq,
			refusal,
		});
		super::metrics::permanently_refused(refusal.label());
		tracing::error!(
			worker_id = ?worker,
			volume_seq,
			?refusal,
			client_epoch = s.client.client_epoch,
			"log shard permanently refuses an entry; the replica is blocked at it and nothing at or behind it is sent to it again"
		);
	}
	fail_blocked(s, fx);
}

/// The lowest entry no quorum can append any more, with the refusal of the replica blocked
/// at it: a majority of voters is blocked at or below it. Entries below it can still commit
/// on the replicas that are not blocked there, which repair catches up to their blocks.
pub(super) fn wedged(s: &VolumeClientState) -> Option<(u64, PermanentRefusal)> {
	let mut blocks: Vec<BlockedAppend> = s.replicas.values().filter_map(|r| r.blocked).collect();
	blocks.sort_by_key(|blocked| blocked.volume_seq);
	blocks
		.into_iter()
		.find(|candidate| {
			!super::membership::quorum(&s.replica_set, |worker| {
				s.replicas
					.get(worker)
					.is_none_or(|r| r.can_append(candidate.volume_seq))
			})
		})
		.map(|blocked| (blocked.volume_seq, blocked.refusal))
}

/// Fails every caller whose write or barrier can no longer commit because a quorum is
/// blocked at or below it, with the blocking refusal and the write's token. Their entries
/// stay in retained history and are never resent on this handle; a later open recovers
/// them if a replica holds them, and discards them otherwise. A drain no longer waits for
/// them, so a close completes. Once everything below the blocked entry is committed the
/// stall alarm is retired, since nothing is left to resend. Before serving, a blocked quorum
/// means the open can never succeed, so the open fails with the same error. Runs after
/// every event, because a wedge can also form when a payload release unmatches a blocked
/// replica or a membership change drops one; it finds nothing to do on later passes.
pub(super) fn fail_blocked(s: &mut VolumeClientState, fx: &mut Effects) {
	let Some((volume_seq, refusal)) = wedged(s) else {
		return;
	};
	if matches!(
		s.phase,
		VolumeClientPhase::Opening { .. } | VolumeClientPhase::Recovering(_)
	) {
		fence_with_error(
			s,
			WriteError::HistoryBlocked {
				volume_seq,
				refusal,
				token: None,
			},
			fx,
		);
		return;
	}

	let stuck: Vec<u64> = s.writes.range(volume_seq..).map(|(&seq, _)| seq).collect();
	let barriers: Vec<RequestId> = s
		.barriers
		.iter()
		.filter(|(_, barrier)| barrier.volume_seq >= volume_seq)
		.map(|(&request_id, _)| request_id)
		.collect();
	let failed_any = !stuck.is_empty() || !barriers.is_empty();
	for seq in stuck {
		let write = s.writes.remove(&seq).unwrap();
		for (request_id, _) in write.waiters {
			fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
			fx.execute(Effect::WriteResponse {
				request_id,
				result: Err(WriteError::HistoryBlocked {
					volume_seq,
					refusal,
					token: Some(write.token.clone()),
				}),
			});
		}
	}

	for request_id in barriers {
		let barrier = s.barriers.remove(&request_id).unwrap();
		fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
		super::requests::barrier_reply(
			request_id,
			barrier.confirm,
			Err(WriteError::HistoryBlocked {
				volume_seq,
				refusal,
				token: None,
			}),
			fx,
		);
	}
	// Retire the stall alarm once, when the wedge first fails callers; `stalled` itself
	// declines to re-arm for a wedge whose lower history is committed.
	if failed_any && s.commit_publishing.committed + 1 >= volume_seq {
		fx.cancel_timer(VolumeClientTimer::Stall);
	}
}

/// A peer failure may hide a completed shrink from the survivor that already ACKed
/// the uncommitted tail. A probe carries the current set version, so a stale-set refusal
/// refreshes membership without changing the claim or votes. This is the one place a
/// serving client still sends an `AdvanceCommittedRequest`: it is the only replica
/// request that needs neither an entry nor a claim round, and it fires only while a peer
/// is failing with uncommitted entries outstanding. Only another peer's timeout/backoff
/// calls this; a probe ACK cannot start a loop.
pub(super) fn probe_membership(s: &mut VolumeClientState, failed: &WorkerId, fx: &mut Effects) {
	if !matches!(
		s.phase,
		VolumeClientPhase::Appending | VolumeClientPhase::Draining { .. }
	) || s.commit_publishing.committed + 1 >= s.next_volume_seq
	{
		return;
	}
	let peers: Vec<_> = s
		.replicas
		.iter()
		.filter(|(worker, replica)| {
			*worker != failed
				&& replica.sendable(s.config.max_in_flight)
				&& replica.claim.is_some()
				&& replica.acked > s.commit_publishing.committed
				&& replica.repair.is_none()
				&& replica.pending_claim.is_none()
				&& replica.in_flight.is_empty()
		})
		.map(|(worker, _)| worker.clone())
		.collect();
	for worker in peers {
		announce(s, worker, None, fx);
	}
}

/// Reserves a bounded, correlated replica request that carries no entry: the membership
/// probe and the confirm barrier use it for the reply, not for the committed position it
/// carries. The shard already learns commits as a hint on every append, and the
/// maintainer derives them from the tails, so nothing sends one to publish progress.
pub(super) fn announce(
	s: &mut VolumeClientState,
	worker: WorkerId,
	confirmation: Option<RequestId>,
	fx: &mut Effects,
) {
	let op_id = s.operation();
	let r = s.replicas.get_mut(&worker).unwrap();
	r.in_flight.insert(
		op_id,
		PendingAppend {
			end: r.acked,
			committed: s.commit_publishing.committed,
			confirmation,
			bytes: 0,
		},
	);
	fx.arm_timer_after(VolumeClientTimer::Append(op_id), s.config.append_timeout_ms);
	fx.execute(Effect::AdvanceCommittedRequest {
		op_id,
		worker_id: worker,
		volume_log_key: s.volume.loaded_key().clone(),
		client: s.client.clone(),
		tablet_replica_set_version: s.replica_set.tablet_replica_set_version,
		committed_volume_seq: s.commit_publishing.committed,
	});
}

/// Handles the `Stall` alarm: uncommitted history has made no quorum progress for the stall
/// timeout. A stall is latency, not evidence of a newer claim, so a serving client keeps its
/// epoch and every admitted entry. Each matched replica that is not repairing or waiting out a
/// refusal resends from its acknowledged position with the same positions and payloads. A
/// replica that already holds the whole batch acknowledges it without writing it again; one
/// whose tail ends inside the batch rejects it, and repair matches it again. The alarm is
/// armed again while entries stay uncommitted. Only a `StaleClientEpoch` or `ClientConflict`
/// reply fences a serving client.
///
/// A recovery-only client has no caller deadline that ends it, so a stall still ends it and
/// its maintainer schedules recovery again.
pub(super) fn stalled(s: &mut VolumeClientState, fx: &mut Effects) {
	match s.phase {
		VolumeClientPhase::Fenced { .. }
		| VolumeClientPhase::Stopped
		| VolumeClientPhase::Closing { .. }
		| VolumeClientPhase::Opening { .. } => return,
		VolumeClientPhase::Appending
		| VolumeClientPhase::Draining { .. }
		| VolumeClientPhase::Recovering(_) => {}
	}

	if s.purpose == OpenPurpose::Recover {
		fence(s, fx);
		return;
	}

	if s.commit_publishing.committed + 1 >= s.next_volume_seq {
		return;
	}

	// History a quorum permanently refuses is not stalled: resending it changes nothing,
	// and its callers were already answered. It waits for the owner to open a new client.
	// Entries below the blocked one are still ordinary history and keep their redrive.
	if let Some((volume_seq, refusal)) = wedged(s)
		&& s.commit_publishing.committed + 1 >= volume_seq
	{
		tracing::warn!(
			volume_seq,
			?refusal,
			committed = s.commit_publishing.committed,
			client_epoch = s.client.client_epoch,
			"volume client history is blocked at an entry no quorum can append; not resending"
		);
		return;
	}

	super::metrics::stalled();
	tracing::warn!(
		committed = s.commit_publishing.committed,
		next_volume_seq = s.next_volume_seq,
		client_epoch = s.client.client_epoch,
		"volume client made no quorum progress within the stall timeout; resending from acknowledged positions, likely overloaded log shards"
	);

	let redrive: Vec<_> = s
		.replicas
		.iter()
		.filter(|(_, r)| {
			r.matched && r.repair.is_none() && r.refused.is_none() && r.blocked.is_none()
		})
		.map(|(worker, _)| worker.clone())
		.collect();
	for worker in redrive {
		retry_replica(s, &worker, fx);
	}

	fx.arm_timer_after(VolumeClientTimer::Stall, s.config.stall_timeout_ms);
}

/// Retire all outstanding evidence before answering callers on a dead handle.
pub(super) fn fence(s: &mut VolumeClientState, fx: &mut Effects) {
	fence_with_error(s, WriteError::Fenced { token: None }, fx);
}

/// Complete the lifecycle caller as well as pending writes when the handle loses authority.
pub(super) fn fence_with_error(s: &mut VolumeClientState, error: WriteError, fx: &mut Effects) {
	// Preserve the terminal reason when shutdown or a late timer fences an already stopped client.
	if matches!(s.phase, VolumeClientPhase::Fenced { .. }) {
		return;
	}

	let opening = matches!(
		s.phase,
		VolumeClientPhase::Opening { .. } | VolumeClientPhase::Recovering(_)
	) || (s.purpose == OpenPurpose::Recover
		&& matches!(
			s.phase,
			VolumeClientPhase::Draining { .. } | VolumeClientPhase::Closing { .. }
		));
	let closing = match &s.phase {
		VolumeClientPhase::Draining { request_id }
		| VolumeClientPhase::Closing { request_id, .. } => *request_id,
		_ => None,
	};

	if let VolumeClientPhase::Closing { op_id: op, .. } = s.phase {
		fx.cancel_timer(VolumeClientTimer::Control(op));
	}

	// Revoke authority and retire all alarms before releasing pending callers.
	s.phase = VolumeClientPhase::Fenced {
		error: error.clone(),
	};
	super::lifecycle::clear_loading(s);
	fx.cancel_timer(VolumeClientTimer::FinishRecovery);
	fx.cancel_timer(VolumeClientTimer::Stall);
	for r in s.replicas.values_mut() {
		r.repair_goal = None;
		if let Some(step) = r.repair.take() {
			for op in step.operation_ids() {
				fx.cancel_timer(VolumeClientTimer::Control(op));
			}
		}

		if let Some(op) = r.pending_claim.take() {
			fx.cancel_timer(VolumeClientTimer::Control(op));
		}

		for op in r.in_flight.keys() {
			fx.cancel_timer(VolumeClientTimer::Append(*op));
		}
		r.in_flight.clear();
		clear_refusal(r, fx);
	}

	// Complete lifecycle callers as well as writes so no waiter survives fencing.
	super::requests::fail_waiters(s, &error, fx);
	super::waiting::fail(s, &error, fx);
	fx.cancel_timer(VolumeClientTimer::OpenIdle);
	if opening {
		if s.purpose == OpenPurpose::Recover {
			fx.execute(Effect::RecoveryResponse {
				result: Err(error.clone()),
			});
		}
	}

	if let Some(request_id) = closing {
		fx.execute(Effect::CloseResponse {
			request_id,
			result: Err(error),
		});
	}
}
