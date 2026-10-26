//! Read-only executor admission and completion view, derived from the transition state.
use super::{
	state::{Phase, State},
	types::*,
};
use serde::{Deserialize, Serialize};

/// Runtime owners wait on result; executors validate current token and fresh time before dispatch.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Snapshot {
	/// Current operation only; no token is accepted once Finished.
	pub pending: Option<IoCall<OpId>>,
	/// None until completion; the owned runtime joins all tasks before returning it.
	pub result: Option<Result<(), String>>,
	/// ETag successfully applied to the process, even during later refresh failures.
	pub applied_etag: Option<String>,
	/// Last refresh failure available without querying S3 again.
	pub diagnostic: Option<String>,
}
impl From<&State> for Snapshot {
	fn from(state: &State) -> Self {
		Self {
			pending: state.phase.pending().copied().map(|mut call| {
				call.deadline_boot_ms = call
					.deadline_boot_ms
					.min(state.operation_deadline_boot_ms.unwrap_or(u64::MAX));
				call
			}),
			result: match &state.phase {
				Phase::Finished { result, .. } => Some(result.clone()),
				_ => None,
			},
			applied_etag: state.applied_etag.clone(),
			diagnostic: state.diagnostic.clone(),
		}
	}
}
