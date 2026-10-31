//! Correlated executor completions and lifecycle control for the key-ring state machine.
use super::types::*;
use rivet_object_store_util::conditional_write::WriteResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
	/// **When**
	///
	/// The UPS operator adapter submits one request under its original coordinator tenure.
	///
	/// **State**
	///
	/// Admits only an idle matching tenure/ring version; matching durable receipts replay success.
	/// Busy requests and reused command IDs with different bodies do not mutate the ring.
	///
	/// **Effects**
	///
	/// Requests fresh signing material or replies with the rejection or durable receipt.
	EmergencyRotateRequest {
		call: u64,
		request: EmergencyRotateRequest,
	},
	/// **When**
	///
	/// The singleton runtime revokes the tenure or shuts down.
	///
	/// **State**
	///
	/// Stops this state machine; a sent PUT can still commit under its original condition.
	///
	/// **Effects**
	///
	/// Cancels local I/O and timers without inferring remote rollback.
	StopRequest,
	/// **When**
	///
	/// The store executor completes an exact ring read or publication readback.
	///
	/// **State**
	///
	/// Accepts only the pending operation, then plans maintenance or reconciles publication.
	///
	/// **Effects**
	///
	/// May request entropy, publish an adoption, or arm a lifecycle or retry timer.
	ReadRingResponse { op: OpId, result: ReadResult },
	/// **When**
	///
	/// The key provider completes GenerateKeyRequest.
	///
	/// **State**
	///
	/// The matching generation operation prepares bootstrap or a staged successor.
	///
	/// **Effects**
	///
	/// Publishes the immutable successor or schedules a retry on failure.
	GenerateKeyResponse {
		op: OpId,
		result: Result<Material, String>,
	},
	/// **When**
	///
	/// The store executor completes a conditional ring PUT.
	///
	/// **State**
	///
	/// The publication helper accepts matching attempts and retains uncertain requests.
	///
	/// **Effects**
	///
	/// Schedules readback, an identical retry, or the next lifecycle deadline.
	WriteRingResponse { op: OpId, result: WriteResult },
}
impl rivet_state_machine::EventBody for Event {
	fn name(&self) -> &'static str {
		match self {
			Self::EmergencyRotateRequest { .. } => "emergency_rotate_request",
			Self::StopRequest => "stop_request",
			Self::ReadRingResponse { .. } => "read_ring_response",
			Self::GenerateKeyResponse { .. } => "generate_key_response",
			Self::WriteRingResponse { .. } => "write_ring_response",
		}
	}
}
