//! Serving authority published before executor effects are dispatched.
use super::{
	state::{ServicesState, State},
	types::{CoordinatorTenure, OpId},
};
use rivet_topology::worker::WorkerIdentity;

/// Singleton consumers re-read this and the current Topology snapshot at every authority boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
	/// Permanent local worker fencing; a new heartbeat notification cannot restore this candidate.
	pub worker_fenced: bool,
	/// Only this operation may dispatch a conditional write; late executor work is discarded.
	pub pending_operation: Option<OpId>,
	/// No executor mutation may be sent after its operation deadline.
	pub operation_deadline_boot_ms: Option<u64>,
	/// None immediately on coordinator-leadership loss, independently of storage worker authority.
	pub tenure: Option<CoordinatorTenure>,
	/// While stepping down, services are revoked at this instant even if the alarm is late.
	pub step_down_boot_ms: Option<u64>,
	/// A stale notification can never extend this worker lease deadline.
	pub worker_deadline_boot_ms: Option<u64>,
	/// Candidate shutdown/fencing is complete; the runtime owner may drop/join its runner.
	pub stopped: bool,
	/// The unreleased manifest owner this candidate waits on. The runtime reports when a
	/// storage listing no longer shows it; None for the owner itself and after shutdown.
	pub watched_owner: Option<WorkerIdentity>,
}

impl Snapshot {
	/// Local permission is necessary, but cannot fence an already-sent remote mutation by itself.
	pub fn permits(&self, tenure: &CoordinatorTenure, now_boot_ms: u64) -> bool {
		self.tenure.as_ref() == Some(tenure)
			&& self
				.serving_deadline_boot_ms()
				.is_some_and(|end| now_boot_ms < end)
	}

	/// The earlier of the worker lease deadline and an announced step-down revocation.
	pub fn serving_deadline_boot_ms(&self) -> Option<u64> {
		match (self.worker_deadline_boot_ms, self.step_down_boot_ms) {
			(Some(worker), Some(step_down)) => Some(worker.min(step_down)),
			(Some(deadline), None) | (None, Some(deadline)) => Some(deadline),
			(None, None) => None,
		}
	}

	pub(super) fn from_state(state: &State) -> Self {
		Self {
			worker_fenced: state.lease.fenced(),
			pending_operation: state.control.pending().map(|call| call.op),
			operation_deadline_boot_ms: state.control.pending().map(|call| call.deadline_boot_ms),
			tenure: if state.lease.fenced() {
				None
			} else {
				match &state.services {
					ServicesState::Running(tenure) | ServicesState::SteppingDown { tenure, .. } => {
						Some(tenure.clone())
					}
					ServicesState::Absent
					| ServicesState::Stopping { .. }
					| ServicesState::SafeToRelease(_) => None,
				}
			},
			step_down_boot_ms: match &state.services {
				ServicesState::SteppingDown {
					revoke_at_boot_ms, ..
				} => Some(*revoke_at_boot_ms),
				ServicesState::Running(_)
				| ServicesState::Absent
				| ServicesState::Stopping { .. }
				| ServicesState::SafeToRelease(_) => None,
			},
			worker_deadline_boot_ms: state.lease.deadline_boot_ms(),
			stopped: (state.shutdown || state.lease.fenced())
				&& state.services == ServicesState::Absent
				&& (matches!(state.control, super::state::ControlState::Idle)
					|| state.control.suspended()),
			watched_owner: if state.lease.fenced() {
				None
			} else {
				super::watched_owner(state).map(|lease| lease.tenure.owner)
			},
		}
	}
}
