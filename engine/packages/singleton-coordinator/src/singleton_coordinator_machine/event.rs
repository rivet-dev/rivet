//! Incoming coordinator requests and correlated adapter responses.
use super::types::*;
use rivet_topology::worker::WorkerIdentity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
	/// Topology publishes a new shared worker lease view.
	AuthorityChanged(WorkerLeaseView),

	/// Local shutdown prevents reacquisition after safe scope release.
	StopRequest,

	/// **When**
	///
	/// `Coordinator::close` sends it while the storage service tears down, after its other
	/// services stopped and UPS drained. A graceful stop that handed the tenure over has already
	/// stopped this candidate; a stop that skipped, failed or cut short the handover has not.
	///
	/// **State**
	///
	/// Sets `shutdown` as StopRequest does, then revokes a running or stepping-down scope at once
	/// instead of serving until a step-down deadline, which the teardown budget cannot wait for.
	/// Revoking before an announced deadline is always safe. An in-flight step-down or withdrawal
	/// write is abandoned; a late landing changes the manifest version, so the release's
	/// conditional write and readback reconcile it.
	///
	/// **Effects**
	///
	/// While serving, StopScopeRequest and the stop timer. Once the scope joins safely, the
	/// release resumes from a lease read or a suspended publication; an unsafe join exits the
	/// process and a fenced lease releases nothing. Without a serving scope, the same effects as
	/// StopRequest.
	CloseRequest,

	/// An operator or service exit requests cleanup and relinquishment without draining owned work.
	RelinquishTenure {
		/// Only the currently running tenure may begin cleanup.
		tenure: CoordinatorTenure,

		/// Retained in the event trace; both causes use the same safe cleanup barrier.
		reason: RelinquishReason,
	},

	/// Env supplies labelled operational randomness for the next attempt and retry delay.
	EntropyResponse {
		op: OpId,
		attempt_id: [u8; 16],
		retry_delay_ms: u64,
	},

	/// **When**
	///
	/// The executor answers ReadLeaseRequest, including the readback after every write. A timed
	/// out read is delivered as `Read(Unknown)`.
	///
	/// **State**
	///
	/// Accepted only for the pending read operation. `Unchanged` is resolved to the retained
	/// manifest projection; a definite result replaces `manifest`. The owner stops its scope when
	/// the manifest names another tenure, a candidate decides between claiming, inspecting the
	/// owner's heartbeat and waiting, and a publication classifies its readback.
	///
	/// **Effects**
	///
	/// Cancels the read's I/O timer, then one of AcquireLeaseRequest, ReleaseLeaseRequest,
	/// StepDownRequest, WithdrawStepDownRequest, InspectHeartbeatRequest, StopScopeRequest or
	/// EntropyRequest.
	ReadLeaseResponse { op: OpId, result: ManifestRead },

	/// Exact previous-owner heartbeat response from the existing Topology service.
	InspectHeartbeatResponse {
		op: OpId,
		result: rivet_topology::heartbeat::Read,
	},

	/// Every write response triggers readback, including a conditional conflict.
	AcquireLeaseResponse { op: OpId },

	/// A release acknowledgment is not a new serving token; confirm the manifest projection by readback.
	ReleaseLeaseResponse { op: OpId },

	/// **When**
	///
	/// The executor finished, or gave up on, a StepDownRequest.
	///
	/// **State**
	///
	/// Accepted only while that announcement is the pending publication. The revocation
	/// deadline is already armed and never depends on this response.
	///
	/// **Effects**
	///
	/// A ReadLeaseRequest readback, which confirms the write by its commit ID.
	StepDownResponse { op: OpId },

	/// **When**
	///
	/// The executor finished, or gave up on, a WithdrawStepDownRequest.
	///
	/// **State**
	///
	/// Accepted only while that withdrawal is the pending publication. Only a readback that
	/// finds its commit ID returns the tenure to Running and cancels the step-down alarm.
	///
	/// **Effects**
	///
	/// A ReadLeaseRequest readback.
	WithdrawStepDownResponse { op: OpId },

	/// **When**
	///
	/// The runtime's topology watcher sees a fresh storage listing without the process named
	/// by `Snapshot::watched_owner`: it is draining, released, replaced or missing. Sent once
	/// per owner until that owner is listed again. The listing is Topology's own discovery, so
	/// this costs no extra read.
	///
	/// **State**
	///
	/// Ignored unless `owner` is the unreleased manifest owner. Otherwise `owner_unlisted` is set
	/// and live heartbeat evidence is dropped so the next check inspects again; a
	/// missing-heartbeat wait already in progress is kept.
	///
	/// **Effects**
	///
	/// Re-arms the check alarm to fire now plus jitter when the candidate is waiting; the check
	/// then re-reads the manifest and inspects the heartbeat before any claim.
	OwnerUnlisted { owner: WorkerIdentity },

	/// **When**
	///
	/// The runtime's topology watcher sees a fresh storage listing showing the previously
	/// unlisted owner as Active again.
	///
	/// **State**
	///
	/// Clears `owner_unlisted` when it names `owner`; otherwise ignored.
	///
	/// **Effects**
	///
	/// None. An armed check still fires and, finding no handover pending, schedules the next
	/// known deadline.
	OwnerListed { owner: WorkerIdentity },

	/// Scope controller has fenced and joined all singleton services and reconciled outstanding effects.
	StopScopeResponse {
		tenure: CoordinatorTenure,
		effects_safe: bool,
	},
}
