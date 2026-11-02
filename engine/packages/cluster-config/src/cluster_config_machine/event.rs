//! Caller hints and correlated executor completions enter the deterministic core.
use super::types::*;
use rivet_object_store_util::conditional_write::WriteResult;
use serde::{Deserialize, Serialize};

/// Inputs contain data only; runtime alarms arrive separately through RuntimeEvent.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
	/// UPS change/gap hint; coalesced while another config operation is pending.
	RefreshRequest,
	/// Runtime owner is stopping; revoke local work without claiming remote PUT rollback.
	StopRequest,
	/// Result of ReadConfigRequest, also used for uncertain-write readback.
	ReadConfigResponse { op: OpId, result: ReadResult },
	/// Result of PrepareConfigRequest; validation failures prohibit publication.
	PrepareConfigResponse {
		op: OpId,
		result: Result<PreparedConfig, String>,
	},
	/// Result of WriteConfigRequest; unknown/conflicting outcomes require helper readback.
	WriteConfigResponse { op: OpId, result: WriteResult },
	/// Result of ApplyConfigRequest; stale completions cannot mark a newer snapshot applied.
	ApplyConfigResponse {
		op: OpId,
		result: Result<(), String>,
	},
}
