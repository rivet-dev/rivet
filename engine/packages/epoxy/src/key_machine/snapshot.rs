//! A read-only view of one key's acceptor record.

use epoxy_protocol::protocol;

use super::{state, state::KeyState, types};

/// What readers see of a key: its newest committed value and the open instance's accepted value,
/// which may or may not have been chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeySnapshot {
	pub committed: Option<protocol::CommittedValue>,
	pub pending: Option<protocol::AcceptedValue>,
	pub promised: Option<protocol::Ballot>,
}

pub fn snapshot(state: &KeyState) -> KeySnapshot {
	KeySnapshot {
		committed: state::committed(state).map(types::committed_to_protocol),
		pending: state::accepted(state).map(types::accepted_to_protocol),
		promised: state::promise(state).map(|promise| types::ballot_to_protocol(&promise.ballot)),
	}
}

impl KeySnapshot {
	pub fn read_state(self) -> protocol::KvReadStateResponse {
		protocol::KvReadStateResponse {
			committed: self.committed,
			accepted: self.pending,
			promised: self.promised,
		}
	}
}
