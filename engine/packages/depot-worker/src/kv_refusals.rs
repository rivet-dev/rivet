//! KV command refusals shared by the command host and the tablet maintainer state machine,
//! which also builds for wasm.

use rivet_data::generated::depot_kv_command_v1::{
	AuthorityMoved, KvCommandResponse, OverloadReason, Overloaded,
};
#[cfg(not(target_arch = "wasm32"))]
use rivet_data::generated::depot_kv_command_v1::{Unavailable, UnavailableReason};

/// How long a caller waits before sending a refused command again. Refusals come from
/// admission limits and short volume transitions such as an open or a close, which settle on
/// this scale.
pub(crate) const OVERLOADED_RETRY_AFTER_MS: u64 = 100;

/// A command the host did not admit. Nothing ran, so the edge may send it again.
pub(crate) fn overloaded(reason: OverloadReason) -> KvCommandResponse {
	overloaded_after(reason, OVERLOADED_RETRY_AFTER_MS)
}

/// A command refused because a layer below the host refused its work with its own hint, which
/// the edge waits instead of the host's default.
pub(crate) fn overloaded_after(reason: OverloadReason, retry_after_ms: u64) -> KvCommandResponse {
	KvCommandResponse::Overloaded(Overloaded {
		reason,
		retry_after_ms,
	})
}

/// Distinguishes the temporary storage condition without changing the public wire reason.
pub(crate) fn volume_transition(cause: &'static str) -> KvCommandResponse {
	volume_transition_after(cause, OVERLOADED_RETRY_AFTER_MS)
}

/// Keeps a lower layer's retry hint while recording the transition's actual source.
pub(crate) fn volume_transition_after(
	cause: &'static str,
	retry_after_ms: u64,
) -> KvCommandResponse {
	tracing::debug!(
		cause,
		retry_after_ms,
		"KV command refused during a volume transition"
	);
	overloaded_after(OverloadReason::VolumeTransition, retry_after_ms)
}

/// An admitted command that could not finish; its outcome may be unknown.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn unavailable(reason: UnavailableReason) -> KvCommandResponse {
	KvCommandResponse::Unavailable(Unavailable { reason })
}

/// The volume's command authority left this host. The caller follows the refreshed route with
/// the same operation id.
pub(crate) fn authority_moved() -> KvCommandResponse {
	KvCommandResponse::AuthorityMoved(AuthorityMoved {
		retry_after_ms: OVERLOADED_RETRY_AFTER_MS,
	})
}
