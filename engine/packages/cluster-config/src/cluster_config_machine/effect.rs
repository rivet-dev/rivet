//! Executor work uses injected storage and the existing process Config handle.
use super::types::*;
use serde::{Deserialize, Serialize};

/// The executor returns matching response variants; only cancellation is one-way.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
	/// Read the fixed ClusterConfigKey and return ReadConfigResponse.
	ReadConfigRequest { op: OpId },
	/// Merge and validate an update, or prepare the empty bootstrap object.
	PrepareConfigRequest { op: OpId, preparation: Preparation },
	/// Send the exact conditional request and return WriteConfigResponse.
	WriteConfigRequest { op: OpId, request: PreparedConfig },
	/// Apply an accepted config snapshot and return ApplyConfigResponse; performed synchronously
	/// at dispatch to prevent an older apply effect overtaking a newer one.
	ApplyConfigRequest { op: OpId, snapshot: StoredConfig },
	/// Cancel a local executor future after its alarm/response; does not retract remote writes.
	CancelOperation { op: OpId },
}
