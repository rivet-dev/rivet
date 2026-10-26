//! Queue-wait shedding at the shard's admission, ahead of every replica transition.
//!
//! The runner is one serial loop, so when requests arrive faster than it handles them they
//! wait in its inbox, and the wait grows without bound until the inbox fills. A request that
//! already waited past its bound is refused before the core does any work for it: its caller
//! retries after the hint, within its own deadline, while the requests behind it are served
//! quickly enough to commit inside the volume client's quorum timeout. The bounds stay below
//! the volume client's append timeout, past which it would resend the same append and add
//! load instead of shedding it.

use super::{LogShardPhase, LogShardState, loaded_volume_replica::admit, memory_batches::reject};
use crate::log_shard_machine::{
	Effects,
	types::{LoadedVolumeReplicaCommand, LogShardCommand, LogShardRequest, MAX_RETRY_AFTER_MS},
};
use rivet_data::generated::depot_storage_worker_protocol_v1::{self as protocol, RejectResponse};

/// The refusal a full budget answers: nothing was done, retry after the configured hint.
pub(crate) fn overloaded(state: &LogShardState) -> RejectResponse {
	RejectResponse::Overloaded(protocol::Overloaded {
		retry_after_ms: state.config.overload_retry_after_ms,
	})
}

/// The refusal a full disk budget answers: nothing was written. Space frees only when the
/// maintainer writes the shard's data to cold and trims it, which takes longer than a queue
/// drains, so the hint starts at the longest refusal delay. [`jittered`] spreads it before
/// the reply leaves the shard.
pub(crate) fn disk_full() -> RejectResponse {
	RejectResponse::DiskFull(protocol::DiskFull {
		retry_after_ms: MAX_RETRY_AFTER_MS,
	})
}

/// Spreads a full-disk refusal's hint with the overload jitter rule, so callers the shard
/// refused together do not all return together. Every other refusal passes unchanged: an
/// overload hint is jittered where it is chosen.
pub(crate) fn jittered(
	state: &LogShardState,
	reason: RejectResponse,
	rng: &mut rivet_env::rand::LocalRng,
) -> RejectResponse {
	match reason {
		RejectResponse::DiskFull(refusal) => RejectResponse::DiskFull(protocol::DiskFull {
			retry_after_ms: jittered_hint(state, refusal.retry_after_ms, rng),
		}),
		reason => reason,
	}
}

/// Refuses each request of one submission that waited past its bound, or admits it, in
/// order. Appends and commit announcements carry writes the volume client already admitted,
/// and trims and forgets free the memory and disk that overload fills, so they wait up to
/// `max_append_admission_wait_ms`. Every other request, such as the loads and claims that open
/// new volumes, is new work and is shed first, at `max_admission_wait_ms`. Only a serving
/// shard sheds; other phases answer through ordinary admission.
///
/// Waits are not monotonic across submissions, so a volume's later append can be admitted
/// after its earlier one was refused. The later one then fails its sequence check and changes
/// nothing, and the client resends both from its acknowledged position.
pub(crate) fn submitted(
	state: &mut LogShardState,
	submitted_boot_ms: u64,
	requests: Vec<LogShardRequest>,
	rng: &mut rivet_env::rand::LocalRng,
	effects: &mut Effects,
) {
	let waited_ms = effects.now_boot_ms().saturating_sub(submitted_boot_ms);
	state.last_admission_wait_ms = waited_ms;
	crate::metrics::observe_admission_wait(waited_ms);

	let mut shed = 0;
	for request in requests {
		if sheds(state, waited_ms, &request) {
			reject(
				request.request_id,
				queue_refusal(state, waited_ms, rng),
				effects,
			);
			shed += 1;
			continue;
		}
		admit(state, request, rng, effects);
	}
	if shed > 0 {
		crate::metrics::record_admission_shed(crate::metrics::AdmissionShed::QueueWait, shed);
	}
}

/// How many times cheaper the simulated cost model makes a refusal than an admission. A
/// refusal reads one timestamp and replies; an admission runs the replica transition.
pub(crate) const SIMULATED_REFUSAL_COST_DIVISOR: u64 = 20;

/// Whether a serving shard refuses `request` after it waited `waited_ms` in the inbox.
pub(crate) fn sheds(state: &LogShardState, waited_ms: u64, request: &LogShardRequest) -> bool {
	matches!(state.phase, LogShardPhase::Serving) && waited_ms > wait_bound(state, request)
}

/// The longest inbox wait after which `request` is still admitted.
fn wait_bound(state: &LogShardState, request: &LogShardRequest) -> u64 {
	match &request.command {
		LogShardCommand::Replica {
			command:
				LoadedVolumeReplicaCommand::Append { .. }
				| LoadedVolumeReplicaCommand::AdvanceCommitted { .. }
				| LoadedVolumeReplicaCommand::Trim { .. }
				| LoadedVolumeReplicaCommand::Forget { .. }
				| LoadedVolumeReplicaCommand::MarkVolumeDeleted,
			..
		}
		| LogShardCommand::ForgetVolumes { .. }
		| LogShardCommand::TrimVolumes { .. } => state.config.max_append_admission_wait_ms,
		LogShardCommand::Replica {
			command:
				LoadedVolumeReplicaCommand::LoadVolume { .. }
				| LoadedVolumeReplicaCommand::Claim { .. }
				| LoadedVolumeReplicaCommand::MarkInSync { .. }
				| LoadedVolumeReplicaCommand::Truncate { .. }
				| LoadedVolumeReplicaCommand::Read { .. },
			..
		}
		| LogShardCommand::GetDiskIncarnation
		| LogShardCommand::ListTabletReplicas(_)
		| LogShardCommand::ReadStatus { .. } => state.config.max_admission_wait_ms,
	}
}

/// A refusal of an append that would make the next write too long for the disk hints how
/// long the disk takes to drain what is already waiting, since `limit` bytes take
/// `max_append_admission_wait_ms`, jittered so the appends refused together do not return
/// together.
pub(crate) fn drain_refusal(
	state: &LogShardState,
	filling_bytes: u64,
	limit: u64,
	rng: &mut rivet_env::rand::LocalRng,
) -> RejectResponse {
	let drain_ms =
		filling_bytes.saturating_mul(state.config.max_append_admission_wait_ms) / limit.max(1);
	RejectResponse::Overloaded(protocol::Overloaded {
		retry_after_ms: jittered_hint(state, drain_ms, rng),
	})
}

/// A queue-wait refusal hints the observed wait, which is about how long the queue ahead of
/// a retry takes to drain, scaled by a random factor between one half and three halves.
/// Requests refused together, such as the loads and claims of an open burst, would otherwise
/// all retry in the same instant and refill the queue together.
fn queue_refusal(
	state: &LogShardState,
	waited_ms: u64,
	rng: &mut rivet_env::rand::LocalRng,
) -> RejectResponse {
	RejectResponse::Overloaded(protocol::Overloaded {
		retry_after_ms: jittered_hint(state, waited_ms, rng),
	})
}

/// A retry hint near `base_ms`, at least the configured overload hint and at most
/// `MAX_RETRY_AFTER_MS`, drawn from half to one and a half times the base.
fn jittered_hint(state: &LogShardState, base_ms: u64, rng: &mut rivet_env::rand::LocalRng) -> u64 {
	let min = state.config.overload_retry_after_ms;
	let max = MAX_RETRY_AFTER_MS.max(min);
	// Leave headroom under the cap so the jitter spreads hints instead of piling them at it.
	let hint = base_ms.clamp(min, (max * 2 / 3).max(min));
	let jittered = hint.saturating_mul(rng.gen_range(50..150)) / 100;
	jittered.clamp(min, max)
}
