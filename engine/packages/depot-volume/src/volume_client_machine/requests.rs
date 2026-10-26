//! Caller deadlines are independent of replication: an unknown outcome never cancels its entry.

use super::{
	Effects,
	effect::VolumeClientEffect as Effect,
	state::{Barrier, PendingWrite, RecoveryPhase, VolumeClientPhase, VolumeClientState},
	types::*,
};
use depot_types::payload_codec::PayloadDigest;
use rivet_data::generated::shared_depot_types_v1::VolumeLogPosition;
use std::collections::{BTreeMap, BTreeSet};

/// Check identity and all admission bounds before assigning either sequence counter.
#[allow(
	clippy::too_many_arguments,
	reason = "transition inputs mirror the caller request and runtime event envelope"
)]
pub(super) fn admit(
	s: &mut VolumeClientState,
	request_id: RequestId,
	payload: PayloadId,
	encoded_bytes: u32,
	sync: bool,
	digest: PayloadDigest,

	retry_of: Option<WriteToken>,
	request_uuid: [u8; 16],
	fx: &mut Effects,
) {
	if !matches!(
		s.phase,
		VolumeClientPhase::Appending
			| VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry { .. })
	) {
		let error = if let VolumeClientPhase::Fenced { error } = &s.phase {
			match error {
				WriteError::Fenced { .. } => WriteError::Fenced { token: retry_of },
				// A retry's entry may have been admitted before the open failed, so the
				// caller keeps its token and the outcome stays unknown.
				WriteError::HistoryBlocked {
					volume_seq,
					refusal,
					..
				} => WriteError::HistoryBlocked {
					volume_seq: *volume_seq,
					refusal: *refusal,
					token: retry_of,
				},
				WriteError::DeadlineExceeded { .. }
				| WriteError::VolumeNotFound
				| WriteError::RoutingUnavailable
				| WriteError::ColdStoreUnavailable
				| WriteError::LifecycleRejected(_)
				| WriteError::RecoveryFailed
				| WriteError::QuorumUnavailable { .. }
				| WriteError::Overloaded { .. }
				| WriteError::Refused { .. }
				| WriteError::RetryConflict
				| WriteError::RetryIndeterminate { .. }
				| WriteError::EntryTooLarge { .. } => error.clone(),
			}
		} else {
			WriteError::Fenced { token: retry_of }
		};
		rejected(request_id, payload, error, fx);
		return;
	}

	let key = &s.volume.loaded_key().0;
	let volume_incarnation = key.volume_incarnation;
	// Entries at or behind an entry a quorum permanently refuses can never commit.
	let wedge = super::pipeline::wedged(s);
	let retry_token = retry_of.clone();

	// A fully recovered history proves absence only inside the retained identity
	// window. Older requests remain indeterminate; they must never be reissued.
	let mut retry_identity = None;
	if let Some(token) = retry_of {
		if token.volume_incarnation != volume_incarnation || token.payload_digest != digest {
			rejected(request_id, payload, WriteError::RetryConflict, fx);
			return;
		}

		let entry = s.dedupe.get(&token.request_id).cloned().or_else(|| {
			s.writes
				.iter()
				.find(|(_, write)| write.token.request_id == token.request_id && !write.submitted)
				.and_then(|(&seq, _)| s.retained.get(&seq))
				.and_then(|entry| {
					entry.mutation.clone().map(|identity| DedupeEntry {
						identity,
						volume_seq: entry.id.volume_seq,
					})
				})
		});
		if entry.is_none()
			&& s.phase == VolumeClientPhase::Appending
			&& token.client_seq > 0
			&& (s.dedupe.len() < super::types::DEDUPE_WINDOW
				|| s.dedupe
					.values()
					.map(|e| e.identity.client_seq)
					.min()
					.is_some_and(|floor| token.client_seq > floor))
		{
			retry_identity = Some(MutationIdentity {
				client_seq: token.client_seq,
				request_id: token.request_id,
				payload_digest: digest,
			});
		}
		if let Some(entry) = entry {
			if entry.identity.payload_digest != digest
				|| entry.identity.client_seq != token.client_seq
			{
				rejected(request_id, payload, WriteError::RetryConflict, fx);
				return;
			}

			// Return a committed receipt or join the unfinished entry with a new caller deadline.
			let token = WriteToken {
				payload_digest: digest,
				volume_incarnation,
				client_seq: entry.identity.client_seq,
				request_id: entry.identity.request_id,
			};

			if s.phase == VolumeClientPhase::Appending
				&& s.commit_publishing.committed >= entry.volume_seq
			{
				fx.execute(Effect::WriteResponse {
					request_id,
					result: outcome(
						s,
						entry.volume_seq,
						token,
						true,
						Some(s.commit_publishing.committed),
					),
				});
			} else {
				// Joining an entry no quorum can commit would only wait out the deadline.
				if let Some((volume_seq, refusal)) = wedge
					&& entry.volume_seq >= volume_seq
				{
					rejected(
						request_id,
						payload,
						WriteError::HistoryBlocked {
							volume_seq,
							refusal,
							token: Some(token),
						},
						fx,
					);
					return;
				}
				if caller_count(s) >= s.config.max_pending_writes as usize {
					rejected(request_id, payload, WriteError::client_overloaded(), fx);
					return;
				}

				s.writes
					.entry(entry.volume_seq)
					.or_insert_with(|| PendingWrite {
						submitted: true,
						token,
						waiters: BTreeMap::new(),
					})
					.waiters
					.insert(request_id, true);
				fx.arm_timer_after(
					VolumeClientTimer::Quorum(request_id),
					s.config.quorum_timeout_ms,
				);
			}

			fx.execute(Effect::ReleasePayload { payload });
			return;
		} else if retry_identity.is_none() {
			rejected(
				request_id,
				payload,
				WriteError::RetryIndeterminate { token },
				fx,
			);
			return;
		}
	}

	// A new entry would sit behind a blocked quorum's entry and never commit; refuse it
	// before it takes a position, with the refusal that blocks the history.
	if let Some((volume_seq, refusal)) = wedge {
		rejected(
			request_id,
			payload,
			WriteError::HistoryBlocked {
				volume_seq,
				refusal,
				token: retry_token,
			},
			fx,
		);
		return;
	}

	// An entry a quorum could never append in one request is refused for good, so the
	// caller learns the limit instead of retrying; waiting changes nothing about it.
	if encoded_bytes > s.quorum_entry_limit_bytes() {
		rejected(
			request_id,
			payload,
			WriteError::EntryTooLarge {
				max_bytes: s.max_mutation_bytes(),
			},
			fx,
		);
		return;
	}

	// Retained memory holds only entries a quorum has not committed, and commit alone frees it,
	// so a refused write waits for the in-flight appends rather than probing replicas.
	let memory_full = u64::from(encoded_bytes)
		> s.config
			.max_memory_bytes
			.saturating_sub(s.memory_bytes)
			.saturating_sub(super::waiting::reserved_bytes(s));
	let fits_request = u64::from(encoded_bytes) <= s.config.max_memory_bytes
		&& s.next_volume_seq < u64::MAX
		&& s.last_client_seq < u64::MAX;
	// Reserve capacity before assigning identities or retaining the new payload.
	// While a quorum refuses for memory or disk pressure, a new write could only queue
	// behind its retries; refusing it now pushes the overload back to the caller.
	if s.writes.len() >= s.config.max_pending_writes as usize
		|| caller_count(s) >= s.config.max_pending_writes as usize
		|| memory_full
		|| !fits_request
	{
		rejected(request_id, payload, WriteError::client_overloaded(), fx);
		return;
	}
	if super::pipeline::quorum_refusing(s) {
		let retry_after_ms = super::pipeline::quorum_refusal_retry_after_ms(s, fx.clocks().boot_ms);
		rejected(
			request_id,
			payload,
			WriteError::Overloaded {
				retry_after_ms,
				token: None,
			},
			fx,
		);
		return;
	}

	// Allocate an identity only after capacity checks, then rule out a receipt collision.
	let seq = s.next_volume_seq;
	let identity = retry_identity.unwrap_or_else(|| MutationIdentity {
		client_seq: s.last_client_seq + 1,
		request_id: request_uuid,
		payload_digest: digest,
	});

	let token = WriteToken {
		payload_digest: digest,
		volume_incarnation,
		client_seq: identity.client_seq,
		request_id: identity.request_id,
	};

	if s.dedupe.contains_key(&identity.request_id) {
		rejected(request_id, payload, WriteError::RetryConflict, fx);
		return;
	}

	// Replication owns the entry beyond a caller deadline; each waiter has its own timer.
	let was_idle = s.commit_publishing.committed + 1 == s.next_volume_seq;
	s.next_volume_seq += 1;
	s.memory_bytes += u64::from(encoded_bytes);
	s.above_cold.record(seq, encoded_bytes);
	remember(s, identity.clone(), seq);
	s.retained.insert(
		seq,
		EntryRef {
			written_at_unix_ms: fx.clocks().unix_ms,
			id: VolumeLogPosition {
				volume_seq: seq,
				client_epoch: s.client.client_epoch,
			},
			mutation: Some(identity),
			payload,
			encoded_bytes,
		},
	);
	let sync = sync
		|| matches!(
			s.phase,
			VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry { .. })
		);
	let waiters = if sync {
		BTreeMap::from([(request_id, false)])
	} else {
		BTreeMap::new()
	};

	s.writes.insert(
		seq,
		PendingWrite {
			submitted: false,
			token: token.clone(),
			waiters,
		},
	);

	// Replication outlives caller deadlines; asynchronous callers receive admission now.
	if was_idle {
		fx.arm_timer_after(VolumeClientTimer::Stall, s.config.stall_timeout_ms);
		// A burst after a long quiet gap frees its maps again once it commits; a burst that
		// follows the last one closely keeps them.
		let now = fx.clocks().boot_ms;
		s.compact_when_quiet = s
			.quiet_since_boot_ms
			.is_none_or(|since| now.saturating_sub(since) >= s.config.compact_after_quiet_ms);
		s.quiet_since_boot_ms = None;
	}

	if sync {
		fx.arm_timer_after(
			VolumeClientTimer::Quorum(request_id),
			s.config.quorum_timeout_ms,
		);
	} else {
		fx.execute(Effect::WriteResponse {
			request_id,
			result: outcome(s, seq, token, false, None),
		});
	}
}

/// Replies without retaining the rejected payload or reserving replication capacity.
pub(super) fn rejected(
	request_id: RequestId,
	payload: PayloadId,
	error: WriteError,
	fx: &mut Effects,
) {
	fx.execute(Effect::WriteResponse {
		request_id,
		result: Err(error),
	});
	fx.execute(Effect::ReleasePayload { payload });
}

/// Counts joined write waiters and barriers against one caller budget.
pub(super) fn caller_count(s: &VolumeClientState) -> usize {
	s.writes
		.values()
		.map(|write| write.waiters.len())
		.sum::<usize>()
		+ s.barriers.len()
		+ s.waiting.len()
}

/// Replayed and newly admitted mutations advance the same bounded identity window.
pub(super) fn remember(s: &mut VolumeClientState, identity: MutationIdentity, volume_seq: u64) {
	s.last_client_seq = s.last_client_seq.max(identity.client_seq);
	s.dedupe.remember(DedupeEntry {
		identity,
		volume_seq,
	});
}

/// Complete every waiter joined to an entry, without counting duplicated peer acknowledgements.
pub(super) fn completed(s: &mut VolumeClientState, fx: &mut Effects) {
	let finished: Vec<_> = s
		.writes
		.range(..=s.commit_publishing.committed)
		.map(|(&seq, _)| seq)
		.collect();
	for seq in finished {
		let write = s.writes.remove(&seq).unwrap();
		for (request_id, applied_earlier) in write.waiters {
			fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
			fx.execute(Effect::WriteResponse {
				request_id,
				result: outcome(
					s,
					seq,
					write.token.clone(),
					applied_earlier,
					Some(s.commit_publishing.committed),
				),
			});
		}
	}
}

/// Capture position evidence with the receipt, before another recovery or trim changes it.
fn outcome(
	s: &VolumeClientState,
	volume_seq: u64,
	token: WriteToken,
	applied_earlier: bool,
	committed: Option<u64>,
) -> Result<WriteOutcome, WriteError> {
	let position = if volume_seq <= s.cold.volume_seq {
		s.cold.clone()
	} else {
		s.position(volume_seq).ok_or(WriteError::RecoveryFailed)?
	};
	Ok(WriteOutcome {
		volume_seq,
		position,
		token,
		applied_earlier,
		committed,
	})
}

/// Resets the stall alarm only while uncommitted entries remain.
pub(super) fn progress(s: &mut VolumeClientState, fx: &mut Effects) {
	if s.commit_publishing.committed + 1 < s.next_volume_seq {
		fx.arm_timer_after(VolumeClientTimer::Stall, s.config.stall_timeout_ms);
	} else {
		fx.cancel_timer(VolumeClientTimer::Stall);
		if s.quiet_since_boot_ms.is_none() {
			s.quiet_since_boot_ms = Some(fx.clocks().boot_ms);
		}
	}
}

/// Frees the working maps once a burst that followed a long quiet gap has committed and
/// its callers are answered. Runs after every commit advance; a client still busy keeps
/// its maps until it is quiet.
pub(super) fn settle_quiet(s: &mut VolumeClientState) {
	let quiet = s.quiet_since_boot_ms.is_some()
		&& s.writes.is_empty()
		&& s.waiting.is_empty()
		&& s.barriers.is_empty()
		&& s.replicas
			.values()
			.all(|replica| replica.in_flight.is_empty());
	if quiet && s.compact_when_quiet {
		s.compact_when_quiet = false;
		s.free_quiet_containers();
	}
}

/// Stop waiting for a reply while preserving the exact entry for continued replication.
pub(super) fn expired(s: &mut VolumeClientState, request_id: RequestId, fx: &mut Effects) {
	for write in s.writes.values_mut() {
		if write.waiters.remove(&request_id).is_some() {
			// The entry keeps its position and is sent after the deadline whether or not it was
			// sent before, so the caller always gets its token: the outcome is unknown, not
			// refused, and a retry with the token joins the same entry. Giving up on a caller
			// is latency, not evidence of a newer claim, so the client keeps serving.
			fx.execute(Effect::WriteResponse {
				request_id,
				result: Err(WriteError::DeadlineExceeded {
					token: Some(write.token.clone()),
				}),
			});
			return;
		}
	}

	if let Some(barrier) = s.barriers.remove(&request_id) {
		barrier_reply(
			request_id,
			barrier.confirm,
			Err(WriteError::QuorumUnavailable { token: None }),
			fx,
		);
	}
}

/// Releases every caller waiting for a commit once a quorum of shards refuses for a full disk.
///
/// An overload refusal clears within its hint, well inside a caller's quorum deadline, so
/// callers wait it out. A full disk clears only after the maintainer writes the shards' data
/// to cold and trims it, which usually outlasts that deadline, so holding the callers would
/// only turn a typed refusal into a timeout. Each caller gets `Overloaded` with the shards'
/// hint and its write's token: the entry stays admitted and this client keeps resending it
/// at the hints, which is its own intent re-driven on a deadline, so a retry with the token
/// joins the entry. Commit barriers are released the same way without a token. Nothing here
/// unmatches, repairs or fences a replica.
pub(super) fn shed_if_disk_full(s: &mut VolumeClientState, fx: &mut Effects) {
	if !super::pipeline::quorum_disk_full(s) {
		return;
	}
	let retry_after_ms = super::pipeline::quorum_refusal_retry_after_ms(s, fx.clocks().boot_ms);
	for write in s.writes.values_mut() {
		for (request_id, _) in std::mem::take(&mut write.waiters) {
			fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
			fx.execute(Effect::WriteResponse {
				request_id,
				result: Err(WriteError::Overloaded {
					retry_after_ms,
					token: Some(write.token.clone()),
				}),
			});
		}
	}
	for (request_id, barrier) in std::mem::take(&mut s.barriers) {
		fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
		barrier_reply(
			request_id,
			barrier.confirm,
			Err(WriteError::Overloaded {
				retry_after_ms,
				token: None,
			}),
			fx,
		);
	}
}

/// A repeated claim cannot report success for an old sequence until its payload identity is recovered.
pub(super) fn unknown_waiters(s: &mut VolumeClientState, fx: &mut Effects) {
	for (_, write) in std::mem::take(&mut s.writes) {
		for (request_id, _) in write.waiters {
			fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
			fx.execute(Effect::WriteResponse {
				request_id,
				result: Err(WriteError::QuorumUnavailable {
					token: Some(write.token.clone()),
				}),
			});
		}
	}
	// Barriers name positions in the discarded history, not entries in its replacement.
	for (request_id, barrier) in std::mem::take(&mut s.barriers) {
		fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
		barrier_reply(
			request_id,
			barrier.confirm,
			Err(WriteError::QuorumUnavailable { token: None }),
			fx,
		);
	}
}

/// Releases known successes before fencing every remaining write and barrier waiter.
pub(super) fn fail_waiters(s: &mut VolumeClientState, error: &WriteError, fx: &mut Effects) {
	completed(s, fx);
	for (_, write) in std::mem::take(&mut s.writes) {
		for (request_id, _) in write.waiters {
			fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
			fx.execute(Effect::WriteResponse {
				request_id,
				result: Err(if *error == WriteError::VolumeNotFound {
					WriteError::VolumeNotFound
				} else {
					WriteError::Fenced {
						token: Some(write.token.clone()),
					}
				}),
			});
		}
	}

	for (request_id, barrier) in std::mem::take(&mut s.barriers) {
		fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
		barrier_reply(
			request_id,
			barrier.confirm,
			Err(if *error == WriteError::VolumeNotFound {
				WriteError::VolumeNotFound
			} else {
				WriteError::Fenced { token: None }
			}),
			fx,
		);
	}
}

/// Capture the current end once; subsequent writes cannot move this caller's barrier.
pub(super) fn barrier(
	s: &mut VolumeClientState,
	request_id: RequestId,
	confirm: bool,
	history: Option<HistoryIdentity>,
	fx: &mut Effects,
) {
	// A volume opened empty admits writes behind its in-flight claim entry, so it also takes
	// their commit barriers; the claim entry commits first because it precedes them.
	let behind_claim_entry = s.opened_empty
		&& matches!(
			s.phase,
			VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry { .. })
		);
	let stale_history = history.is_some_and(|history| {
		history.client_epoch != s.client.client_epoch
			|| history.claim_entry_volume_seq
				!= s.claim_entry.map(|claim_entry| claim_entry.volume_seq)
	});
	let error = if stale_history || (s.phase != VolumeClientPhase::Appending && !behind_claim_entry)
	{
		Some(WriteError::Fenced { token: None })
	} else if let Some((volume_seq, refusal)) = super::pipeline::wedged(s) {
		// The barrier would capture the current end, at or behind the blocked entry.
		Some(WriteError::HistoryBlocked {
			volume_seq,
			refusal,
			token: None,
		})
	} else if caller_count(s) >= s.config.max_pending_writes as usize {
		Some(WriteError::client_overloaded())
	} else {
		None
	};

	if let Some(error) = error {
		barrier_reply(request_id, confirm, Err(error), fx);
		return;
	}

	s.barriers.insert(
		request_id,
		Barrier {
			volume_seq: s.next_volume_seq - 1,
			confirm,
			confirmed: BTreeSet::new(),
		},
	);
	fx.arm_timer_after(
		VolumeClientTimer::Quorum(request_id),
		s.config.quorum_timeout_ms,
	);
}

/// Confirmations use entry-free replica requests issued after the request; earlier replies do
/// not count.
pub(super) fn barriers(s: &mut VolumeClientState, fx: &mut Effects) {
	for request_id in s.barriers.keys().copied().collect::<Vec<_>>() {
		let barrier = s.barriers[&request_id].clone();
		if s.commit_publishing.committed < barrier.volume_seq {
			continue;
		}

		if !barrier.confirm
			|| super::membership::quorum(&s.replica_set, |worker| {
				barrier.confirmed.contains(worker)
			}) {
			s.barriers.remove(&request_id);
			fx.cancel_timer(VolumeClientTimer::Quorum(request_id));
			barrier_reply(
				request_id,
				barrier.confirm,
				Ok(s.commit_publishing.committed),
				fx,
			);
			continue;
		}

		// Obtain fresh evidence without duplicating confirmation operations already in flight.
		for worker in s.replicas.keys().cloned().collect::<Vec<_>>() {
			let r = &s.replicas[&worker];
			if !r.sendable(s.config.max_in_flight)
				|| r.acked < barrier.volume_seq
				|| barrier.confirmed.contains(&worker)
				|| r.in_flight
					.values()
					.any(|p| p.confirmation == Some(request_id))
			{
				continue;
			}

			super::pipeline::announce(s, worker, Some(request_id), fx);
		}
	}
}

/// Preserves the caller's barrier kind when routing success or failure.
pub(super) fn barrier_reply(
	request_id: RequestId,
	confirm: bool,
	result: Result<u64, WriteError>,
	fx: &mut Effects,
) {
	fx.execute(if confirm {
		Effect::ConfirmResponse { request_id, result }
	} else {
		Effect::WaitForCommitResponse { request_id, result }
	});
}

/// Dispatch is the point after which absence cannot prove that a mutation never reached a replica.
pub(super) fn submitted(s: &mut VolumeClientState, entries: &[EntryRef]) {
	for entry in entries {
		if let Some(write) = s.writes.get_mut(&entry.id.volume_seq) {
			write.submitted = true;
		}
	}
}
