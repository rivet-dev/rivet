//! Outgoing work; executors do not decide ownership or retry policy.
use super::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
	/// Draw one nonzero ID and bounded jitter through Env, returning EntropyResponse.
	EntropyRequest {
		op: OpId,
		min_delay_ms: u64,
		max_delay_ms: u64,
	},
	/// **When**
	///
	/// A check alarm or a safe-release transition needs the current manifest, and after every
	/// conditional write to learn its outcome.
	///
	/// **Executor**
	///
	/// Reads the shared publication object with If-None-Match set to `known_version` when it is
	/// present, returning ReadLeaseResponse with `Unchanged` or the decoded read.
	ReadLeaseRequest {
		op: OpId,
		known_version: Option<String>,
	},
	/// Resolve exact expiry evidence, returning InspectHeartbeatResponse.
	InspectHeartbeatRequest {
		op: OpId,
		owner: rivet_topology::worker::WorkerIdentity,
	},
	/// Conditional publication; preserve application metadata and bind ownership in the same object.
	AcquireLeaseRequest { op: OpId, attempt: ClaimAttempt },
	/// Sent only after safe scope cleanup; update the exact observed shared manifest conditionally.
	ReleaseLeaseRequest {
		op: OpId,
		observed: ObservedLease,
		commit_id: [u8; 16],
	},
	/// **When**
	///
	/// A serving owner is asked to shut down, loses eligibility or reads an operator's release
	/// request while its manifest shows no step-down, or a stepping-down owner reads a manifest
	/// that lost its announcement. The revocation deadline is armed in the same transition,
	/// before this is sent.
	///
	/// **Executor**
	///
	/// Conditionally writes `step_down_ms` and a fresh `commit_id` on the exact observed
	/// version, preserving the tenure and operator intent, and returns StepDownResponse; the
	/// core always reads back.
	StepDownRequest {
		op: OpId,
		observed: ObservedLease,
		commit_id: [u8; 16],
		step_down_ms: u64,
	},
	/// **When**
	///
	/// A stepping-down owner that is not shutting down becomes eligible again, or reads that
	/// the operator's release request was cleared, before its revocation deadline.
	///
	/// **Executor**
	///
	/// Conditionally clears `step_down_ms` with a fresh `commit_id` on the exact observed
	/// version, which need not show the announcement, and returns WithdrawStepDownResponse;
	/// the core always reads back.
	WithdrawStepDownRequest {
		op: OpId,
		observed: ObservedLease,
		commit_id: [u8; 16],
	},
	/// Start all ClusterSingleton services; each service performs its own durable-state recovery.
	StartScope { tenure: CoordinatorTenure },
	/// Fence immediately, then cancel/join and reconcile sent effects before StopScopeResponse.
	StopScopeRequest { tenure: CoordinatorTenure },
}
