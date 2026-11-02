//! One-shot adapter work; every retry and deadline belongs to the pure core.
use super::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
	/// **When**
	///
	/// Admission rejects a command, or publication/readback confirms its committed receipt.
	///
	/// **Executor**
	///
	/// Rechecks the original live tenure and delivers the matching local reply channel.
	/// The UPS adapter checks the same authority and request deadline before replying remotely.
	EmergencyRotateResponse {
		call: u64,
		response: EmergencyRotateResponse,
	},
	/// **When**
	///
	/// Startup, recovery or publication reconciliation needs the current ring.
	///
	/// **Executor**
	///
	/// Reads one typed object and returns ReadRingResponse, distinguishing absent and invalid.
	ReadRingRequest { op: OpId },
	/// **When**
	///
	/// Bootstrap or normal rotation needs fresh signing material.
	///
	/// **Executor**
	///
	/// Samples secure randomness at the production boundary and returns GenerateKeyResponse.
	/// Simulations provide fixed material through the same event.
	GenerateKeyRequest { op: OpId },
	/// **When**
	///
	/// A transition or publication retry dispatches its immutable conditional mutation.
	///
	/// **Executor**
	///
	/// Rechecks tenure authority, sends exactly one PUT and returns WriteRingResponse.
	WriteRingRequest { op: OpId, request: Mutation },
	/// **When**
	///
	/// An operation completes, expires or is stopped.
	///
	/// **Executor**
	///
	/// Cancels the matching local task. There is no response and no remote rollback guarantee.
	CancelOperation { op: OpId },
}
impl rivet_state_machine::Effect for Effect {
	fn name(&self) -> &'static str {
		match self {
			Self::EmergencyRotateResponse { .. } => "emergency_rotate_response",
			Self::ReadRingRequest { .. } => "read_ring_request",
			Self::GenerateKeyRequest { .. } => "generate_key_request",
			Self::WriteRingRequest { .. } => "write_ring_request",
			Self::CancelOperation { .. } => "cancel_operation",
		}
	}
}
