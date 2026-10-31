//! Open, claim and close decisions. Every reply must belong to the current attempt.

use super::{
	Effects,
	effect::VolumeClientEffect as Effect,
	event::RpcError,
	state::{RecoveryPhase, VolumeClientPhase, VolumeClientState, VolumeIdentity},
	types::*,
};
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState, depot_storage_worker_protocol_v1::RejectResponse,
	shared_depot_fs_v1::FsVolumeStatus,
};
use rivet_state_machine::Entropy;
use rivet_topology::worker::WorkerId;

/// A member answered `VolumeNotLoaded`. The client marks it not loaded and keeps going on the
/// members that are: the maintainer reloads a member missing from a loaded volume on its own,
/// and the member's repair catches it up once it answers again. When the loaded members no
/// longer form a quorum the client fails: a writer never reopens itself, and its owner opens
/// a new client, which claims the volume at a newer epoch.
pub(super) fn member_not_loaded(s: &mut VolumeClientState, worker: &WorkerId, fx: &mut Effects) {
	s.loading_members.insert(worker.clone());
	if loaded_quorum(s) {
		super::repair::backoff(s, worker, fx);
	} else {
		fail(s, WriteError::QuorumUnavailable { token: None }, fx);
	}
}

/// Whether the members not known to be unloaded still form a quorum of the replica set.
pub(super) fn loaded_quorum(s: &VolumeClientState) -> bool {
	super::membership::quorum(&s.replica_set, |member| !s.loading_members.contains(member))
}

/// Retire old operations before asking the maintainer to create replicas for an unloaded volume.
pub(super) fn open(s: &mut VolumeClientState, fx: &mut Effects) {
	// Only a client that never served opens. A serving, draining or closing client that would
	// need a new open has lost its claim's quorum and fails; its owner opens a new client.
	if matches!(
		s.phase,
		VolumeClientPhase::Appending
			| VolumeClientPhase::Draining { .. }
			| VolumeClientPhase::Closing { .. }
	) {
		fail(s, WriteError::QuorumUnavailable { token: None }, fx);
		return;
	}
	clear_loading(s);

	// Retire old-history waits before an open response can advance the cold baseline.
	super::requests::unknown_waiters(s, fx);
	fx.cancel_timer(VolumeClientTimer::FinishRecovery);

	for r in s.replicas.values_mut() {
		for op in r.in_flight.keys() {
			fx.cancel_timer(VolumeClientTimer::Append(*op));
		}

		if let Some(op) = r.pending_claim.take() {
			fx.cancel_timer(VolumeClientTimer::Control(op));
		}
		r.in_flight.clear();
		if let Some(step) = r.repair.take() {
			for op in step.operation_ids() {
				fx.cancel_timer(VolumeClientTimer::Control(op));
			}
		}
		r.matched = false;
		r.repair_goal = None;
		r.claim = None;
		// A block names an entry of the history this open replaces.
		r.blocked = None;
	}
	// A late reply to an append of the previous attempt, such as its claim entry, is no vote
	// under the history this open recovers.
	s.late_ack_floor = s.next_op;

	// Reopen only after all evidence from the previous attempt is retired.
	let op = s.operation();
	s.open_attempts.insert(op);
	s.phase = VolumeClientPhase::Opening { op_id: Some(op) };
	fx.arm_timer_after(VolumeClientTimer::Control(op), s.config.claim_timeout_ms);
	fx.execute(Effect::VolumeOpenRequest {
		deadline_boot_ms: fx
			.clocks()
			.boot_ms
			.saturating_add(s.config.claim_timeout_ms),
		op_id: op,
		volume_key: s.volume.logical_key().clone(),
		client_incarnation: s.client.client_incarnation,
		create_if_absent: s.volume.create_if_absent(),
		initial: match s.volume.create_if_absent() {
			true => s.initial_writes.clone(),
			false => Vec::new(),
		},
		read: match s.volume.create_if_absent() {
			true => s.read_keys.clone(),
			false => None,
		},
	});
}

/// Accepts only the current open result and validates its volume lifetime and cold floor.
///
/// `replayed` marks a response kept from an earlier create and handed to this new client
/// ([`VolumeClientState::from_kept_open_response`]), rather than one the maintainer just
/// answered. Its member states may be stale: another
/// worker may have claimed the volume at a newer epoch and written it since. The client
/// therefore claims every member again instead of adopting those states as its claims, so a
/// newer claim fences it before it serves a read or write from the old view.
pub(super) fn opened(
	s: &mut VolumeClientState,
	op: OpId,
	result: Result<rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse, RpcError>,
	replayed: bool,
	entropy: Entropy,
	fx: &mut Effects,
) {
	if !s.open_attempts.remove(&op) {
		return;
	}
	fx.cancel_timer(VolumeClientTimer::Control(op));
	if s.phase != (VolumeClientPhase::Opening { op_id: Some(op) }) {
		return;
	}

	let opened = match result {
		Ok(row) => row,
		Err(error) => {
			match (s.purpose, open_refusal(&error)) {
				(OpenPurpose::Serve, Some((refusal, hint_ms))) => {
					refuse_open(s, refusal, hint_ms, entropy, fx)
				}
				(OpenPurpose::Serve | OpenPurpose::Recover, _) => {
					fail(s, lifecycle_error(error), fx)
				}
			}
			return;
		}
	};
	let row = opened.record;
	let applied_history = opened.applied;
	let mut seen = std::collections::BTreeSet::new();
	if opened.states.iter().any(|member| {
		let worker = WorkerId::from_bytes(member.worker_id);
		!seen.insert(worker)
	}) {
		fail(s, WriteError::RecoveryFailed, fx);
		return;
	}
	s.loading_members = s
		.replicas
		.keys()
		.filter(|w| !seen.contains(*w))
		.cloned()
		.collect();
	let matches = match &s.volume {
		VolumeIdentity::Unresolved { .. } => true,
		VolumeIdentity::Known(key) => key.0.volume_incarnation == row.volume_incarnation,
	};
	if !matches
		|| row.volume_key() != *s.volume.logical_key()
		|| row.status != FsVolumeStatus::Live
		|| row.cold.volume_seq < s.cold.volume_seq
	{
		fail(s, WriteError::RecoveryFailed, fx);
		return;
	}

	// Only the first open of a volume can have created it, or read back its keys.
	let asked_to_create = s.volume.create_if_absent();
	if asked_to_create {
		s.read_back = opened.read.clone().map(std::sync::Arc::new);
	}
	// Resolve only the first open. An already known incarnation is never replaced on reopen.
	if let VolumeIdentity::Unresolved { key, .. } = &s.volume {
		s.volume = VolumeIdentity::Known(std::sync::Arc::new(
			rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
				volume_key: key.clone(),
				volume_incarnation: row.volume_incarnation,
			}
			.into(),
		));
	}

	// The cold position remains authoritative even after the volume was unloaded.
	s.observed_epoch = s
		.observed_epoch
		.max(row.cold.client_epoch)
		.max(s.client.client_epoch);
	s.cold_receipts = row.recent_writes;
	s.applied_history = applied_history;
	s.has_base_chain = !row.base_chain.is_empty();
	s.cold = row.cold;
	s.commit_publishing.committed = s.commit_publishing.committed.max(s.cold.volume_seq);
	s.next_volume_seq = s.next_volume_seq.max(s.cold.volume_seq.saturating_add(1));
	s.claim_attempt = 0;
	for member in &opened.states {
		s.observed_epoch = s
			.observed_epoch
			.max(member.state.appended.client_epoch)
			.max(member.state.cold.client_epoch)
			.max(
				member
					.state
					.claimed_client
					.as_ref()
					.map_or(0, |c| c.client_epoch),
			);
	}
	s.granted_claim = false;
	s.opened_empty = false;

	// The maintainer issues every claim epoch. A writer never chooses its own, so an open the
	// maintainer answered without a claim for this client cannot serve.
	let granted = opened
		.open_claim
		.as_ref()
		.map(|open| open.client.clone())
		.filter(|client| client.client_incarnation == s.client.client_incarnation);
	let Some(granted) = granted else {
		super::recovery::release_initial_writes(s, fx);
		fail(s, WriteError::RecoveryFailed, fx);
		return;
	};
	s.granted_claim = true;
	// Kept member states cannot prove the volume is still empty.
	s.opened_empty = !replayed
		&& asked_to_create
		&& s.cold.volume_seq == 0
		&& !s.has_base_chain
		&& opened
			.states
			.iter()
			.all(|member| member.state.appended.volume_seq == 0);
	s.client = granted;
	s.observed_epoch = s.observed_epoch.max(s.client.client_epoch);
	s.claim_attempt = 1;
	s.phase = VolumeClientPhase::Recovering(RecoveryPhase::Claiming);
	for member in opened.states.iter().filter(|_| !replayed) {
		let worker = WorkerId::from_bytes(member.worker_id);
		if let Some(replica) = s.replicas.get_mut(&worker)
			&& super::membership::proof_eligible(&s.replica_set, &worker, &member.state)
			&& member.state.status
				== rivet_data::generated::shared_depot_types_v1::VolumeLogReplicaStatus::InSync
			&& member.state.claimed_client.as_ref() == Some(&s.client)
			&& member.state.cold == s.cold
		{
			replica.incarnation = member.state.replica_incarnation;
			replica.claim = Some(member.state.clone());
		}
	}
	// Members the loads did not claim, such as during a membership change, are claimed at
	// the same granted epoch.
	for worker in s.replicas.keys().cloned().collect::<Vec<_>>() {
		if s.replicas[&worker].claim.is_none() {
			super::recovery::claim(s, worker, fx);
		}
	}
	if super::recovery::created(s, opened.created.as_ref(), &opened.states, fx) {
		return;
	}
	super::recovery::begin_recovery(s, fx);
}

/// Count distinct disk in-sync replies. Late responses can attach the third empty replica.
pub(super) fn claimed(
	s: &mut VolumeClientState,
	op: OpId,
	worker: WorkerId,
	result: Result<VolumeLogReplicaState, RpcError>,
	_entropy: Entropy,
	fx: &mut Effects,
) {
	let goal = super::repair::goal(s);
	let Some(r) = s.replicas.get_mut(&worker) else {
		return;
	};

	if r.pending_claim != Some(op) {
		return;
	}
	r.pending_claim = None;
	if matches!(
		&result,
		Ok(_)
			| Err(RpcError::Rejected(
				RejectResponse::StaleClientEpoch(_) | RejectResponse::ClientConflict(_)
			))
	) {
		s.loading_members.remove(&worker);
	}
	fx.cancel_timer(VolumeClientTimer::Control(op));
	match result {
		Err(RpcError::Rejected(RejectResponse::VolumeLogReplicaDeleted(_))) => {
			fail(s, WriteError::VolumeNotFound, fx);
			return;
		}
		Err(RpcError::Rejected(RejectResponse::StaleTabletReplicaSet(reject))) => {
			if !super::membership::adopt(s, reject.replica_set, fx) {
				super::repair::backoff(s, &worker, fx);
				super::recovery::begin_recovery(s, fx);
			}
			return;
		}
		Err(RpcError::Rejected(RejectResponse::VolumeNotLoaded))
			if s.loading_members.contains(&worker)
				|| !super::membership::voter(&s.replica_set, &worker) =>
		{
			super::repair::backoff(s, &worker, fx);
			super::recovery::begin_recovery(s, fx);
			return;
		}
		Ok(state)
			if state.claimed_client.as_ref() == Some(&s.client)
				&& super::membership::proof_eligible(&s.replica_set, &worker, &state) =>
		{
			s.observed_epoch = s
				.observed_epoch
				.max(state.appended.client_epoch)
				.max(state.cold.client_epoch);
			if r.incarnation != state.replica_incarnation {
				r.repair_goal = goal;
			}
			r.claim = Some(state.clone());
			r.incarnation = state.replica_incarnation;
			super::recovery::late_tail_match(s, &worker, &state, fx);
		}
		// A newer claim fenced this client. It never claims again on its own: its owner opens
		// a new client, which the maintainer grants a newer epoch.
		Err(RpcError::Rejected(RejectResponse::StaleClientEpoch(reject))) => {
			observe(s, &reject.state);
			super::pipeline::fence(s, fx);
			return;
		}
		Err(RpcError::Rejected(RejectResponse::ClientConflict(reject))) => {
			observe(s, &reject.state);
			super::pipeline::fence(s, fx);
			return;
		}
		Err(RpcError::Rejected(RejectResponse::VolumeNotLoaded)) => {
			// The claim continues on the loaded members. A volume no longer loaded on a
			// quorum ends this client; committed writes are unaffected either way.
			member_not_loaded(s, &worker, fx);
			if !matches!(s.phase, VolumeClientPhase::Fenced { .. }) {
				super::recovery::begin_recovery(s, fx);
			}
			return;
		}
		// A refused claim did nothing; retry after the shard's jittered hint, so the claims of
		// an open burst it refused together do not all return at once.
		Err(RpcError::Rejected(RejectResponse::Overloaded(refusal))) => {
			let delay = super::repair::refusal_delay(s, &worker, refusal.retry_after_ms);
			super::repair::backoff_for(s, &worker, delay, fx);
			super::recovery::begin_recovery(s, fx);
			return;
		}
		// The transport refused the claim before sending it, so the claim waits its hint the
		// same way.
		Err(RpcError::Overloaded { retry_after_ms }) => {
			let delay = super::repair::refusal_delay(s, &worker, retry_after_ms);
			super::repair::backoff_for(s, &worker, delay, fx);
			super::recovery::begin_recovery(s, fx);
			return;
		}
		// A full disk refused the claim without writing it. The claim waits the shard's hint,
		// as an append does, for cold writes and trims to free space, instead of claiming
		// again at the ordinary backoff.
		Err(RpcError::Rejected(RejectResponse::DiskFull(refusal))) => {
			let delay = super::repair::refusal_delay(s, &worker, refusal.retry_after_ms);
			super::repair::backoff_for(s, &worker, delay, fx);
			super::recovery::begin_recovery(s, fx);
			return;
		}
		// A claim that is not this client's on a proof-eligible member, and every other
		// rejection or transport failure, backs the member off and claims it again later.
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
			super::repair::backoff(s, &worker, fx);
			super::recovery::begin_recovery(s, fx);
			return;
		}
	}

	super::recovery::begin_recovery(s, fx);
	super::recovery::match_replicas(s, fx);
}

/// Forgets which members were unloaded; the next open reply reports them again.
pub(super) fn clear_loading(s: &mut VolumeClientState) {
	s.loading_members.clear();
}

/// Raises the next claim's epoch floor from every epoch written to disk in the reply.
fn observe(s: &mut VolumeClientState, state: &VolumeLogReplicaState) {
	s.observed_epoch = s
		.observed_epoch
		.max(state.appended.client_epoch)
		.max(state.cold.client_epoch)
		.max(state.claimed_client.as_ref().map_or(0, |w| w.client_epoch));
}

/// Routes current control deadlines to repair, recovery, or lifecycle failure handling.
pub(super) fn expired(s: &mut VolumeClientState, op: OpId, entropy: Entropy, fx: &mut Effects) {
	if s.open_attempts.remove(&op) {
		if s.phase == (VolumeClientPhase::Opening { op_id: Some(op) }) {
			refuse_open(
				s,
				OpenRefusal::Unreachable,
				OPEN_REFUSAL_RETRY_AFTER_MS,
				entropy,
				fx,
			);
		}
		return;
	}
	if super::repair::expired(s, op, fx) {
		return;
	}

	if super::recovery::expired(s, op, entropy.clone(), fx) {
		return;
	}

	if let Some(worker) = s.replicas.iter().find_map(|(worker, r)| {
		if r.pending_claim == Some(op) {
			Some(worker.clone())
		} else {
			None
		}
	}) {
		if !super::membership::voter(&s.replica_set, &worker)
			|| s.phase == VolumeClientPhase::Recovering(RecoveryPhase::Claiming)
		{
			// An unanswered claim leaves the member out; the open goes on with the members
			// that answered and fails once no quorum can answer.
			super::repair::backoff(s, &worker, fx);
			super::recovery::begin_recovery(s, fx);
		} else {
			for r in s.replicas.values_mut() {
				if r.pending_claim == Some(op) {
					r.pending_claim = None;
				}
			}
		}
	} else if s.phase == (VolumeClientPhase::Opening { op_id: Some(op) }) {
		refuse_open(
			s,
			OpenRefusal::Unreachable,
			OPEN_REFUSAL_RETRY_AFTER_MS,
			entropy,
			fx,
		);
	} else if let VolumeClientPhase::Closing { op_id, .. } = s.phase
		&& op == op_id
	{
		closed(s, op, Err(RpcError::Unavailable), fx);
	}
}

/// Stops admission before an explicit close. The client's owner closes it, including once it
/// is idle. Closing promptly frees memory on three workers once the maintainer writes to cold
/// and unloads. Correctness does not depend on close: the maintainer recovers idle volumes
/// before unloading.
pub(super) fn close(s: &mut VolumeClientState, request_id: RequestId, fx: &mut Effects) {
	if s.phase == VolumeClientPhase::Appending {
		clear_loading(s);
		s.phase = VolumeClientPhase::Draining {
			request_id: Some(request_id),
		};
	} else {
		fx.execute(Effect::CloseResponse {
			request_id,
			result: Err(WriteError::Fenced { token: None }),
		});
	}
}

/// Release through the maintainer only after accepted writes reach quorum. Nothing announces
/// the final committed position: the quorum's tails prove it to the maintainer, which reads
/// them from the shards' notices.
pub(super) fn drain(s: &mut VolumeClientState, fx: &mut Effects) {
	let VolumeClientPhase::Draining { request_id } = s.phase.clone() else {
		return;
	};

	if !s.writes.is_empty() || s.commit_publishing.committed + 1 < s.next_volume_seq {
		return;
	}

	for r in s.replicas.values_mut() {
		for op in r.in_flight.keys() {
			fx.cancel_timer(VolumeClientTimer::Append(*op));
		}
		r.in_flight.clear();
		if let Some(step) = r.repair.take() {
			for op in step.operation_ids() {
				fx.cancel_timer(VolumeClientTimer::Control(op));
			}
		}

		if let Some(op) = r.pending_claim.take() {
			fx.cancel_timer(VolumeClientTimer::Control(op));
		}
	}

	// Release through the maintainer only after the replica work has been retired.
	let op = s.operation();
	s.phase = VolumeClientPhase::Closing {
		op_id: op,
		request_id,
	};

	fx.arm_timer_after(VolumeClientTimer::Control(op), s.config.claim_timeout_ms);
	fx.execute(Effect::VolumeCloseRequest {
		client: s.client.clone(),
		op_id: op,
		volume_log_key: s.volume.loaded_key().clone(),
	});
}

/// Completes only the current release attempt and stops periodic volume client activity.
pub(super) fn closed(
	s: &mut VolumeClientState,
	op: OpId,
	result: Result<(), RpcError>,
	fx: &mut Effects,
) {
	let VolumeClientPhase::Closing { op_id, request_id } = s.phase.clone() else {
		return;
	};

	if op != op_id {
		return;
	}

	fx.cancel_timer(VolumeClientTimer::Control(op));
	s.phase = VolumeClientPhase::Stopped;
	s.close_acknowledged = result.is_ok();
	let result = match result {
		// The drain already made every write durable on a quorum, so a close only lets the
		// maintainer unload the volume sooner. A maintainer that is unreachable, moved or busy
		// unloads the idle volume on its own, so its silence still completes the close.
		Err(
			RpcError::Unavailable
			| RpcError::RoutingUnavailable
			| RpcError::Overloaded { .. }
			| RpcError::LifecycleRejected(
				rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse::NotMaintainer(_)
				| rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse::Overloaded(_)
				| rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse::TabletSplitting,
			),
		) if s.purpose == OpenPurpose::Serve => Ok(()),
		result => result.map_err(lifecycle_error),
	};
	if s.purpose == OpenPurpose::Recover {
		super::recovery_only::closed(s, result, fx);
	} else if let Some(request_id) = request_id {
		fx.execute(Effect::CloseResponse { request_id, result });
	}
}

/// Fences the handle and reports the lifecycle failure to its pending callers.
pub(super) fn fail(s: &mut VolumeClientState, error: WriteError, fx: &mut Effects) {
	super::pipeline::fence_with_error(s, error, fx);
}

/// Keep definitive lifecycle rejections distinct from a maintainer request with an unknown outcome.
fn lifecycle_error(error: RpcError) -> WriteError {
	use rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse as Reject;
	match error {
		RpcError::LifecycleRejected(Reject::VolumeNotFound | Reject::VolumeDeleted) => {
			WriteError::VolumeNotFound
		}
		RpcError::LifecycleRejected(Reject::Overloaded(refusal)) => WriteError::Overloaded {
			retry_after_ms: refusal.retry_after_ms,
			token: None,
		},
		// An operator repair claim holds the volume; nothing ran, and it clears on release.
		RpcError::LifecycleRejected(Reject::VolumeUnderRepair(refusal)) => WriteError::Overloaded {
			retry_after_ms: refusal.retry_after_ms,
			token: None,
		},
		RpcError::Overloaded { retry_after_ms } => WriteError::Overloaded {
			retry_after_ms,
			token: None,
		},
		RpcError::LifecycleRejected(reason) => WriteError::LifecycleRejected(reason),
		RpcError::RoutingUnavailable => WriteError::RoutingUnavailable,
		RpcError::Unavailable => WriteError::ColdStoreUnavailable,
		RpcError::Rejected(_) => WriteError::RecoveryFailed,
	}
}

/// The refusal a serving open returns to its caller, with the hint the refusal carried, or
/// `None` when the open must fail. A volume slot refuses an open queued behind its own
/// unsettled close the same way, so both paths classify a maintainer answer identically.
pub fn open_refusal(error: &RpcError) -> Option<(OpenRefusal, u64)> {
	use rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse as Reject;
	match error {
		RpcError::Unavailable => Some((OpenRefusal::Unreachable, OPEN_REFUSAL_RETRY_AFTER_MS)),
		RpcError::RoutingUnavailable => Some((OpenRefusal::NoRoute, OPEN_REFUSAL_RETRY_AFTER_MS)),
		RpcError::LifecycleRejected(Reject::NotMaintainer(refusal)) => {
			Some((OpenRefusal::NotMaintainer, refusal.retry_after_ms))
		}
		RpcError::LifecycleRejected(Reject::Overloaded(refusal)) => {
			Some((OpenRefusal::Overloaded, refusal.retry_after_ms))
		}
		// A repair claim refuses opens until it is released, as shed load clears.
		RpcError::LifecycleRejected(Reject::VolumeUnderRepair(refusal)) => {
			Some((OpenRefusal::Overloaded, refusal.retry_after_ms))
		}
		RpcError::Overloaded { retry_after_ms } => Some((OpenRefusal::Overloaded, *retry_after_ms)),
		RpcError::LifecycleRejected(Reject::TabletSplitting) => {
			Some((OpenRefusal::TabletSplitting, OPEN_REFUSAL_RETRY_AFTER_MS))
		}
		RpcError::LifecycleRejected(
			Reject::VolumeNotFound
			| Reject::VolumeExists
			| Reject::VolumeDeleted
			| Reject::VolumeRecordMissing(_),
		)
		| RpcError::Rejected(_) => None,
	}
}

/// The hint of an open refusal whose answer carries none: a missing route, an unreachable
/// maintainer or a splitting tablet. Each settles on the scale of a route refresh.
pub(crate) const OPEN_REFUSAL_RETRY_AFTER_MS: u64 = 100;

/// The hint a refused open returns: the refusal's own hint plus a uniform draw of up to half
/// of it, so openers refused together do not return together. The hint stays a floor.
pub(crate) fn open_refusal_retry_after_ms(hint_ms: u64, entropy: Entropy) -> u64 {
	let hint_ms = hint_ms.max(1);
	hint_ms + entropy.rng().gen_range(0..hint_ms / 2 + 1)
}

/// Ends a serving open the maintainer refused, returning the refusal to its callers with a
/// hint. The edge owns retries of a user request, so the client never resends the open
/// itself: a refusal admitted nothing, and the owner opens a new client when the edge sends
/// the request again. A creating open that got no answer and carried initial writes may have
/// created the volume with them, so only a definite answer is a refusal; that create fails
/// with an unknown outcome. One that carried no writes is idempotent: at worst it created an
/// empty volume, which the next creating open simply opens, so it is a refusal too.
fn refuse_open(
	s: &mut VolumeClientState,
	refusal: OpenRefusal,
	hint_ms: u64,
	entropy: Entropy,
	fx: &mut Effects,
) {
	let unanswered_create = refusal == OpenRefusal::Unreachable
		&& s.volume.create_if_absent()
		&& !s.initial_writes.is_empty();
	if s.purpose == OpenPurpose::Recover || unanswered_create {
		fail(s, WriteError::ColdStoreUnavailable, fx);
		return;
	}
	let retry_after_ms = open_refusal_retry_after_ms(hint_ms, entropy);
	fail(
		s,
		WriteError::Refused {
			reason: refusal,
			retry_after_ms,
		},
		fx,
	);
}


