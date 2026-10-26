//! Bounded callers waiting for open share recovery, but keep independent original deadlines.

use super::{
	Effects,
	effect::VolumeClientEffect as Effect,
	state::{RecoveryPhase, VolumeClientPhase, VolumeClientState},
	types::*,
};
use depot_types::payload_codec::PayloadDigest;
use serde::{Deserialize, Serialize};

/// These requests own no log position until recovery has selected the inherited history.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Request {
	Open,

	Write {
		request_uuid: [u8; 16],
		payload: PayloadId,
		encoded_bytes: u32,
		sync: bool,
		payload_digest: PayloadDigest,

		retry_of: Option<WriteToken>,
	},
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Caller {
	deadline_boot_ms: u64,
	pub(crate) request: Request,
}

pub(super) fn reserved_bytes(s: &VolumeClientState) -> u64 {
	s.waiting
		.values()
		.map(|caller| match &caller.request {
			Request::Write { encoded_bytes, .. } => u64::from(*encoded_bytes),
			Request::Open => 0,
		})
		.sum()
}

fn recovering(s: &VolumeClientState) -> bool {
	s.purpose == OpenPurpose::Serve
		&& matches!(
			s.phase,
			VolumeClientPhase::Opening { .. } | VolumeClientPhase::Recovering(_)
		)
}

/// Whether a write may take its position now, right behind an in-flight claim entry. Only a volume
/// opened empty qualifies: its claim entry is its first entry, so nothing recovered can precede
/// the write, and a failed claim entry fails the write with it.
fn sends_behind_claim_entry(s: &VolumeClientState, request: &Request) -> bool {
	s.opened_empty
		&& matches!(request, Request::Write { .. })
		&& matches!(
			s.phase,
			VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry { .. })
		)
}

/// Queue immutable caller inputs before any log position is allocated.
pub(super) fn admit(s: &mut VolumeClientState, id: RequestId, request: Request, fx: &mut Effects) {
	if !recovering(s) || sends_behind_claim_entry(s, &request) {
		execute(s, id, request, fx);
		return;
	}
	let bytes = match &request {
		Request::Write { encoded_bytes, .. } => u64::from(*encoded_bytes),
		_ => 0,
	};
	// An entry no quorum could ever append is refused for good, not queued behind recovery.
	if bytes > u64::from(s.quorum_entry_limit_bytes()) {
		reject(
			id,
			request,
			WriteError::EntryTooLarge {
				max_bytes: s.max_mutation_bytes(),
			},
			fx,
		);
		return;
	}
	if super::requests::caller_count(s) >= s.config.max_pending_writes as usize
		|| bytes
			> s.config
				.max_memory_bytes
				.saturating_sub(s.memory_bytes)
				.saturating_sub(super::waiting::reserved_bytes(s))
				.saturating_sub(
					if matches!(
						s.phase,
						VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry { .. })
					) {
						0
					} else {
						25
					},
				) {
		reject(id, request, WriteError::client_overloaded(), fx);
		return;
	}
	let timeout_ms = match &request {
		Request::Open => s.config.open_timeout_ms,
		Request::Write { .. } => s.config.quorum_timeout_ms,
	};
	let deadline_boot_ms = fx.clocks().boot_ms.saturating_add(timeout_ms);
	s.waiting.insert(
		id,
		Caller {
			deadline_boot_ms,
			request,
		},
	);
	fx.cancel_timer(VolumeClientTimer::OpenIdle);
	fx.arm_timer_at(VolumeClientTimer::Waiting(id), deadline_boot_ms);
}

/// In the pass that ends matching behind an uncommitted claim entry, submit only writes that
/// fit one append alongside it; never wait for batching. Other callers are dispatched once the
/// claim entry has committed.
pub(super) fn dispatch(s: &mut VolumeClientState, matching_ended: bool, fx: &mut Effects) {
	let claim_entry = matching_ended
		&& matches!(
			s.phase,
			VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry { .. })
		);
	if !claim_entry && s.phase != VolumeClientPhase::Appending {
		return;
	}
	let mut bytes = 128 + 25 + u64::from(s.entry_frame_overhead);
	for id in s.waiting.keys().copied().collect::<Vec<_>>() {
		if claim_entry {
			let Request::Write {
				encoded_bytes,
				retry_of,
				..
			} = &s.waiting[&id].request
			else {
				continue;
			};
			if retry_of.is_none() {
				let next = bytes + u64::from(*encoded_bytes) + u64::from(s.entry_frame_overhead);
				if next > u64::from(s.config.max_append_bytes) {
					break;
				}
				bytes = next;
			}
		}
		let caller = s.waiting.remove(&id).unwrap();
		fx.cancel_timer(VolumeClientTimer::Waiting(id));
		execute(s, id, caller.request, fx);
		// Admission may reply immediately; only live callers retain the original alarm.
		if s.writes
			.values()
			.any(|write| write.waiters.contains_key(&id))
		{
			fx.arm_timer_at(VolumeClientTimer::Quorum(id), caller.deadline_boot_ms);
		}
	}
}

fn execute(s: &mut VolumeClientState, id: RequestId, request: Request, fx: &mut Effects) {
	match request {
		Request::Open => {
			let result = match &s.phase {
				VolumeClientPhase::Appending => Ok(()),
				VolumeClientPhase::Fenced { error } => Err(error.clone()),
				_ => Err(WriteError::Fenced { token: None }),
			};
			fx.execute(Effect::OpenResponse {
				request_id: id,
				result,
			});
		}
		Request::Write {
			request_uuid,
			payload,
			encoded_bytes,
			sync,
			payload_digest,
			retry_of,
		} => super::requests::admit(
			s,
			id,
			payload,
			encoded_bytes,
			sync,
			payload_digest,
			retry_of,
			request_uuid,
			fx,
		),
	}
}

fn reject(id: RequestId, request: Request, error: WriteError, fx: &mut Effects) {
	match request {
		Request::Open => fx.execute(Effect::OpenResponse {
			request_id: id,
			result: Err(error),
		}),
		Request::Write {
			payload, retry_of, ..
		} => {
			// An existing retry token describes an earlier attempt whose result may still be unknown.
			let error = if matches!(error, WriteError::DeadlineExceeded { .. }) {
				WriteError::DeadlineExceeded { token: retry_of }
			} else {
				error
			};
			super::requests::rejected(id, payload, error, fx);
		}
	}
}

pub(super) fn expired(s: &mut VolumeClientState, id: RequestId, fx: &mut Effects) {
	if let Some(caller) = s.waiting.remove(&id) {
		reject(
			id,
			caller.request,
			WriteError::DeadlineExceeded { token: None },
			fx,
		);
	}
	abandon(s, fx);
}

/// The last waiter leaves no reason to acquire authority, but outstanding opens still own cleanup.
pub(super) fn abandon(s: &mut VolumeClientState, fx: &mut Effects) {
	if recovering(s)
		&& s.waiting.is_empty()
		&& s.writes.values().all(|write| write.waiters.is_empty())
	{
		super::lifecycle::fail(s, WriteError::DeadlineExceeded { token: None }, fx);
	}
}

pub(super) fn ready(s: &mut VolumeClientState, result: Result<(), WriteError>, fx: &mut Effects) {
	let ids: Vec<_> = s
		.waiting
		.iter()
		.filter_map(|(&id, caller)| matches!(caller.request, Request::Open).then_some(id))
		.collect();
	for id in ids {
		s.waiting.remove(&id);
		fx.cancel_timer(VolumeClientTimer::Waiting(id));
		fx.execute(Effect::OpenResponse {
			request_id: id,
			result: result.clone(),
		});
	}
	fx.cancel_timer(VolumeClientTimer::OpenIdle);
}

pub(super) fn fail(s: &mut VolumeClientState, error: &WriteError, fx: &mut Effects) {
	for (id, caller) in std::mem::take(&mut s.waiting) {
		fx.cancel_timer(VolumeClientTimer::Waiting(id));
		reject(id, caller.request, error.clone(), fx);
	}
}
