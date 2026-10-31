//! Conversions between the stored acceptor record and the peer protocol.

use epoxy_protocol::protocol;

use super::state::{AcceptedValue, Ballot, CommittedValue, Promise};

/// Counter of a key's fast round. A fast ballot needs no prepare and no ballot write; an acceptor
/// takes it only for an instance with no other promise or accepted value.
pub const FAST_COUNTER: u64 = 1;

/// Identifies the replica whose key volume runs the transition.
#[derive(Clone, Copy, Debug)]
pub struct KeyContext {
	pub replica_id: protocol::ReplicaId,
}

pub fn ballot_to_stored(ballot: &protocol::Ballot) -> Ballot {
	Ballot {
		counter: ballot.counter,
		replica_id: ballot.replica_id,
		proposal_incarnation: ballot.proposal_incarnation,
	}
}

pub fn ballot_to_protocol(ballot: &Ballot) -> protocol::Ballot {
	protocol::Ballot {
		counter: ballot.counter,
		replica_id: ballot.replica_id,
		proposal_incarnation: ballot.proposal_incarnation,
	}
}

pub fn protocol_ballot_order(ballot: &protocol::Ballot) -> (u64, u64, u64) {
	(
		ballot.counter,
		ballot.replica_id,
		ballot.proposal_incarnation,
	)
}

pub fn committed_to_stored(value: &protocol::CommittedValue) -> CommittedValue {
	CommittedValue {
		value: value.value.clone(),
		version: value.version,
		mutable: value.mutable,
	}
}

pub fn promise_for(ballot: &protocol::Ballot, version: u64) -> Promise {
	Promise {
		ballot: ballot_to_stored(ballot),
		version,
	}
}

pub fn committed_to_protocol(value: &CommittedValue) -> protocol::CommittedValue {
	protocol::CommittedValue {
		value: value.value.clone(),
		version: value.version,
		mutable: value.mutable,
	}
}

pub fn accepted_to_protocol(value: &AcceptedValue) -> protocol::AcceptedValue {
	protocol::AcceptedValue {
		value: value.value.clone(),
		ballot: ballot_to_protocol(&value.ballot),
		version: value.version,
		mutable: value.mutable,
	}
}
