//! Read-only dispatch view. Secret ring bytes are never published in runtime snapshots.
use super::{
	state::{Phase, State},
	types::OpId,
};
use rivet_state_machine_util::IoCall;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
	pub pending: Option<IoCall<OpId>>,
	pub stopped: bool,
	pub diagnostic: Option<String>,
}
impl From<&State> for Snapshot {
	fn from(state: &State) -> Self {
		Self {
			pending: state.pending().copied(),
			stopped: matches!(state.phase, Phase::Stopped),
			diagnostic: state.diagnostic.clone(),
		}
	}
}
impl rivet_state_machine::Snapshot for Snapshot {}
