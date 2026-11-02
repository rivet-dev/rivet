//! Requests that drive a key's acceptor record. Each event is one `ReplicaCommand` on the key
//! volume, and the command answers from the event's own transition.

use epoxy_protocol::protocol;

#[derive(Clone, Debug)]
pub enum KeyEvent {
	/// **When**
	///
	/// A local or latest-reachable read asks for the committed value.
	///
	/// **State**
	///
	/// Unchanged.
	///
	/// **Effects**
	///
	/// None. The reply carries the newest committed value.
	Get,

	/// **When**
	///
	/// A diagnostic read asks for the whole record.
	///
	/// **State**
	///
	/// Unchanged.
	///
	/// **Effects**
	///
	/// None. The reply carries committed, accepted and promised state.
	Inspect,

	/// **When**
	///
	/// A proposer runs a prepare round for one version.
	///
	/// **State**
	///
	/// A predecessor newer than the committed value is adopted first. A decided version is
	/// answered with the committed value; a lower ballot than the promise is rejected. Otherwise
	/// the instance promises the ballot, keeping any accepted value.
	///
	/// **Effects**
	///
	/// None. The reply carries the accepted value, if any.
	Prepare(protocol::PrepareRequest),

	/// **When**
	///
	/// A proposer asks the acceptor to accept a value for one version.
	///
	/// **State**
	///
	/// A predecessor newer than the committed value is adopted first. A decided version is
	/// answered with the committed value, and an expectation that the committed value fails is a
	/// mismatch. A fast ballot is accepted only into an instance holding no other promise or
	/// value; a prepared ballot only at or above the promise. An acceptor that is the key's whole
	/// scope commits what it accepts in the same transition, and then also serves a stale version
	/// by evaluating the expectation against its newest value.
	///
	/// **Effects**
	///
	/// None. Accepting the same ballot and value again writes nothing.
	Accept(protocol::AcceptRequest),

	/// **When**
	///
	/// A proposer, or a reader that finished a value, records a chosen value.
	///
	/// **State**
	///
	/// The version becomes the newest committed value. An older or duplicate commit writes
	/// nothing.
	///
	/// **Effects**
	///
	/// None.
	Commit(protocol::CommitRequest),

	/// **When**
	///
	/// A linearizable or owner read asks for the record, optionally with a read promise.
	///
	/// **State**
	///
	/// The configuration epoch rises to the request's; a newer stored epoch is an error. A read
	/// ballot at least the promise becomes the open instance's promise.
	///
	/// **Effects**
	///
	/// None.
	ReadState(protocol::KvReadStateRequest),
}

impl KeyEvent {
	/// Bounded metrics label.
	pub fn name(&self) -> &'static str {
		match self {
			Self::Get => "get",
			Self::Inspect => "inspect",
			Self::Prepare(_) => "prepare",
			Self::Accept(_) => "accept",
			Self::Commit(_) => "commit",
			Self::ReadState(_) => "read_state",
		}
	}
}

impl From<protocol::ReplicaOperation> for KeyEvent {
	fn from(operation: protocol::ReplicaOperation) -> Self {
		use protocol::ReplicaOperation as Operation;
		match operation {
			Operation::GetReplica => Self::Get,
			Operation::InspectReplica => Self::Inspect,
			Operation::PrepareRequest(input) => Self::Prepare(input),
			Operation::AcceptRequest(input) => Self::Accept(input),
			Operation::CommitRequest(input) => Self::Commit(input),
			Operation::KvReadStateRequest(input) => Self::ReadState(input),
		}
	}
}
