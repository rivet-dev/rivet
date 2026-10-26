//! The durable state machine of one Epoxy key in one region: its acceptor record.
//!
//! Every version of a key is one single-decree Paxos instance. Versions up to the newest
//! committed one are decided, and only the instance after it is open. The record owns the
//! acceptor's invariants: at most one value is committed per version, and a promise fences
//! every lower ballot in its instance.
//!
//! A key's fast round is ballot counter 1. An acceptor takes a fast ballot first come: only
//! into an instance that holds no other promise or accepted value. A fast value is chosen only
//! when every member of the key's scope accepted it, so two fast values can never both be
//! chosen, and a later prepare majority always sees a chosen one. An acceptor that is the key's
//! whole scope is that whole quorum, so it commits what it accepts in the same transition.
//!
//! The machine has no effects and no timers. Each event is one command on the key's volume,
//! the transition rewrites only the volume's own record, and the command answers from the
//! transition. That is why the first command on a new key can ride its volume's creating write.

pub mod event;
pub mod snapshot;
pub mod state;
pub mod types;

use anyhow::{Result, bail, ensure};
use epoxy_protocol::protocol;

use event::KeyEvent;
use state::{KeyState, ballot_order};
use types::{
	FAST_COUNTER, KeyContext, accepted_to_protocol, ballot_to_protocol, ballot_to_stored,
	committed_to_protocol, committed_to_stored, promise_for, protocol_ballot_order,
};

/// Applies one event to the record and returns the command's reply. The caller persists the
/// record only when it changed. An error means the request was malformed; the record is then
/// left as it was.
pub fn on_event(
	state: &mut KeyState,
	ctx: KeyContext,
	event: KeyEvent,
) -> Result<protocol::ReplicaCommandResponse> {
	use protocol::ReplicaCommandResponse as Response;
	let response = match event {
		KeyEvent::Get => Response::ReplicaValue(protocol::ReplicaValue {
			value: state::committed(state).map(committed_to_protocol),
		}),
		KeyEvent::Inspect => Response::ReplicaInspection(protocol::ReplicaInspection {
			state: snapshot::snapshot(state).read_state(),
		}),
		KeyEvent::Prepare(request) => Response::PrepareResponse(prepare(state, &request)?),
		KeyEvent::Accept(request) => Response::AcceptResponse(accept(state, ctx, &request)?),
		KeyEvent::Commit(request) => Response::CommitResponse(commit(state, &request)),
		KeyEvent::ReadState(request) => Response::KvReadStateResponse(read_state(state, &request)?),
	};
	debug_validate(state)?;
	Ok(response)
}

/// Bounded metrics label of a transition's reply.
pub fn reply_name(reply: &protocol::ReplicaCommandResponse) -> &'static str {
	use protocol::{
		AcceptResponse as A, CommitResponse as C, PrepareResponse as P, ReplicaCommandResponse as R,
	};
	match reply {
		R::ReplicaValue(_) => "value",
		R::ReplicaInspection(_) => "inspection",
		R::KvReadStateResponse(_) => "read_state",
		R::PrepareResponse(P::PrepareResponseOk(_)) => "promised",
		R::PrepareResponse(P::PrepareResponseAlreadyCommitted(_)) => "already_committed",
		R::PrepareResponse(P::PrepareResponseHigherBallot(_)) => "higher_ballot",
		R::PrepareResponse(P::PrepareResponseScopeMismatch(_)) => "scope_mismatch",
		R::AcceptResponse(A::AcceptResponseOk(_)) => "accepted",
		R::AcceptResponse(A::AcceptResponseCommitted(_)) => "committed",
		R::AcceptResponse(A::AcceptResponseAlreadyCommitted(_)) => "already_committed",
		R::AcceptResponse(A::AcceptResponseHigherBallot(_)) => "higher_ballot",
		R::AcceptResponse(A::AcceptResponseContended(_)) => "contended",
		R::AcceptResponse(A::AcceptResponseMismatch(_)) => "mismatch",
		R::AcceptResponse(A::AcceptResponseScopeMismatch(_)) => "scope_mismatch",
		R::CommitResponse(C::CommitResponseOk) => "committed",
		R::CommitResponse(C::CommitResponseAlreadyCommitted(_)) => "already_committed",
	}
}

/// Checks the record's invariants after a transition in debug builds.
pub fn debug_validate(state: &KeyState) -> Result<()> {
	if cfg!(debug_assertions) {
		state::validate(state)?;
	}
	Ok(())
}

/// The scope a key records when `scope` differs from it. A request without a scope claim is
/// never refused: it can only use the classic majority rounds. A key records its scope with its
/// first accept or prepare and keeps it, so a region added later never joins an existing key's
/// quorum and a proposal from it must go through a scope region.
fn scope_mismatch(state: &KeyState, scope: &[protocol::ReplicaId]) -> Result<Option<Vec<u64>>> {
	if scope.is_empty() {
		return Ok(None);
	}
	ensure!(
		scope.windows(2).all(|pair| pair[0] < pair[1]),
		"Epoxy key scope is not sorted and unique"
	);
	Ok(match &state.scope {
		Some(recorded) if recorded.replica_ids != scope => Some(recorded.replica_ids.clone()),
		Some(_) | None => None,
	})
}

fn record_scope(state: &mut KeyState, scope: &[protocol::ReplicaId]) {
	if state.scope.is_none() && !scope.is_empty() {
		state.scope = Some(state::Scope {
			replica_ids: scope.to_vec(),
		});
	}
}

/// The committed value's place in a request for one version.
enum Instance {
	/// The version is committed, or the committed value can never be replaced.
	Decided(protocol::CommittedValue),
	/// The version is the open instance.
	Open,
}

/// Adopts a request's predecessor when this acceptor lags, then places `version`. A
/// predecessor is the chosen value of `version - 1`, so recording it as committed only replaces
/// open-instance state for versions that are already decided. A version past the open instance
/// with no predecessor to reach it is malformed.
fn instance(
	state: &mut KeyState,
	version: u64,
	mutable: bool,
	predecessor: Option<&protocol::CommittedValue>,
) -> Result<Instance> {
	if let Some(predecessor) = predecessor
		&& predecessor.version > state::committed_version(state)
		&& state::committed(state).is_none_or(|committed| committed.mutable)
	{
		ensure!(
			predecessor.version + 1 == version,
			"Epoxy predecessor is not the previous version"
		);
		state::set_phase(state, Some(committed_to_stored(predecessor)), None, None);
	}

	if let Some(committed) = state::committed(state)
		&& (!committed.mutable || !mutable || version <= committed.version)
	{
		return Ok(Instance::Decided(committed_to_protocol(committed)));
	}
	if version != state::open_version(state) {
		bail!("Epoxy request is ahead of the acceptor and carries no predecessor");
	}
	Ok(Instance::Open)
}

/// Makes `ballot` the open instance's promise, keeping its accepted value.
fn raise_open_promise(state: &mut KeyState, ballot: &protocol::Ballot) {
	let version = state::open_version(state);
	let committed = state::committed(state).cloned();
	let accepted = state::accepted(state).cloned();
	state::set_phase(
		state,
		committed,
		Some(promise_for(ballot, version)),
		accepted,
	);
}

fn prepare(
	state: &mut KeyState,
	request: &protocol::PrepareRequest,
) -> Result<protocol::PrepareResponse> {
	use protocol::PrepareResponse as Response;
	if let Some(scope) = scope_mismatch(state, &request.scope)? {
		return Ok(Response::PrepareResponseScopeMismatch(
			protocol::PrepareResponseScopeMismatch { scope },
		));
	}
	if let Instance::Decided(committed) = instance(
		state,
		request.version,
		request.mutable,
		request.predecessor.as_ref(),
	)? {
		return Ok(Response::PrepareResponseAlreadyCommitted(
			protocol::PrepareResponseAlreadyCommitted { committed },
		));
	}
	if let Some(promise) = state::promise(state)
		&& ballot_order(&promise.ballot) > protocol_ballot_order(&request.ballot)
	{
		return Ok(Response::PrepareResponseHigherBallot(
			protocol::PrepareResponseHigherBallot {
				ballot: ballot_to_protocol(&promise.ballot),
			},
		));
	}

	let accepted = state::accepted(state).cloned();
	let committed = state::committed(state).cloned();
	record_scope(state, &request.scope);
	state::set_phase(
		state,
		committed,
		Some(promise_for(&request.ballot, request.version)),
		accepted.clone(),
	);
	Ok(Response::PrepareResponseOk(protocol::PrepareResponseOk {
		highest_ballot: request.ballot.clone(),
		accepted_value: accepted.as_ref().map(|accepted| protocol::CommittedValue {
			value: accepted.value.clone(),
			version: accepted.version,
			mutable: accepted.mutable,
		}),
		accepted_ballot: accepted
			.as_ref()
			.map(|accepted| ballot_to_protocol(&accepted.ballot)),
	}))
}

fn accept(
	state: &mut KeyState,
	ctx: KeyContext,
	request: &protocol::AcceptRequest,
) -> Result<protocol::AcceptResponse> {
	use protocol::AcceptResponse as Response;
	// The scope is checked first: an acceptor commits alone only under the key's own scope.
	if let Some(scope) = scope_mismatch(state, &request.scope)? {
		return Ok(Response::AcceptResponseScopeMismatch(
			protocol::AcceptResponseScopeMismatch { scope },
		));
	}
	let whole_scope = request.scope.as_slice() == [ctx.replica_id];

	// An acceptor that is the whole scope decides alone, so it serves a stale version by
	// moving the request to its open instance and checking the expectation there.
	let mut version = request.version;
	if whole_scope
		&& let Some(committed) = state::committed(state)
		&& committed.mutable
		&& request.mutable
		&& version <= committed.version
	{
		version = committed.version + 1;
	}

	if let Instance::Decided(committed) = instance(
		state,
		version,
		request.mutable,
		request.predecessor.as_ref(),
	)? {
		return Ok(Response::AcceptResponseAlreadyCommitted(
			protocol::AcceptResponseAlreadyCommitted { committed },
		));
	}

	// The expectation is checked against the value this version would replace. Every acceptor
	// of the instance holds the same chosen predecessor, so they all agree. A value that is
	// already the committed one is answered as committed and writes nothing. Either answer
	// describes only this acceptor's newest value, which a proposer of a mutable value treats as
	// final only once a prepared majority shows it is the key's newest.
	let committed = state::committed(state).cloned();
	let previous = committed
		.as_ref()
		.and_then(|committed| committed.value.as_deref());
	if let Some(committed) = &committed
		&& previous == request.value.as_deref()
	{
		return Ok(Response::AcceptResponseAlreadyCommitted(
			protocol::AcceptResponseAlreadyCommitted {
				committed: committed_to_protocol(committed),
			},
		));
	}
	if let Some(expected) = &request.expect_one_of
		&& !expected
			.iter()
			.any(|expected| expected.as_deref() == previous)
	{
		return Ok(Response::AcceptResponseMismatch(
			protocol::AcceptResponseMismatch {
				committed: committed.as_ref().map(committed_to_protocol),
			},
		));
	}

	let promise = state::promise(state).cloned();
	let accepted = state::accepted(state).cloned();
	let same_ballot =
		|ballot: &state::Ballot| ballot_order(ballot) == protocol_ballot_order(&request.ballot);
	let same_value = |accepted: &state::AcceptedValue| {
		accepted.value.as_deref() == request.value.as_deref() && accepted.mutable == request.mutable
	};
	if request.ballot.counter == FAST_COUNTER {
		// First come: a fast ballot only enters an instance no other proposal has touched.
		let promise_is_ours = promise
			.as_ref()
			.is_none_or(|promise| same_ballot(&promise.ballot));
		let accepted_is_ours = accepted
			.as_ref()
			.is_none_or(|accepted| same_ballot(&accepted.ballot) && same_value(accepted));
		if !promise_is_ours || !accepted_is_ours {
			return Ok(Response::AcceptResponseContended(
				protocol::AcceptResponseContended {
					promise: promise
						.as_ref()
						.map(|promise| ballot_to_protocol(&promise.ballot)),
					accepted: accepted.as_ref().map(accepted_to_protocol),
				},
			));
		}
	} else {
		if let Some(promise) = &promise
			&& ballot_order(&promise.ballot) > protocol_ballot_order(&request.ballot)
		{
			return Ok(Response::AcceptResponseHigherBallot(
				protocol::AcceptResponseHigherBallot {
					ballot: ballot_to_protocol(&promise.ballot),
				},
			));
		}
		// A ballot carries one value; the same ballot with another value is refused.
		if let Some(accepted) = &accepted
			&& same_ballot(&accepted.ballot)
			&& !same_value(accepted)
		{
			return Ok(Response::AcceptResponseHigherBallot(
				protocol::AcceptResponseHigherBallot {
					ballot: ballot_to_protocol(&accepted.ballot),
				},
			));
		}
	}

	record_scope(state, &request.scope);
	if whole_scope {
		let chosen = state::CommittedValue {
			value: request.value.clone(),
			version,
			mutable: request.mutable,
		};
		state::set_phase(state, Some(chosen.clone()), None, None);
		return Ok(Response::AcceptResponseCommitted(
			protocol::AcceptResponseCommitted {
				committed: committed_to_protocol(&chosen),
			},
		));
	}

	let accepted = state::AcceptedValue {
		value: request.value.clone(),
		ballot: ballot_to_stored(&request.ballot),
		version,
		mutable: request.mutable,
	};
	state::set_phase(
		state,
		committed,
		Some(promise_for(&request.ballot, version)),
		Some(accepted),
	);
	Ok(Response::AcceptResponseOk(protocol::AcceptResponseOk {
		ballot: request.ballot.clone(),
	}))
}

fn commit(state: &mut KeyState, request: &protocol::CommitRequest) -> protocol::CommitResponse {
	use protocol::CommitResponse as Response;
	if let Some(committed) = state::committed(state)
		&& (!committed.mutable || !request.mutable || request.version <= committed.version)
	{
		return Response::CommitResponseAlreadyCommitted(
			protocol::CommitResponseAlreadyCommitted {
				value: committed.value.clone(),
			},
		);
	}

	// A commit only ever carries a chosen value, and a chosen value is final for its version, so
	// it is recorded whatever the open instance promised. Every open-instance promise is for a
	// version at or below this one, so recording it retires them.
	let committed = state::CommittedValue {
		value: request.value.clone(),
		version: request.version,
		mutable: request.mutable,
	};
	state::set_phase(state, Some(committed), None, None);
	Response::CommitResponseOk
}

fn read_state(
	state: &mut KeyState,
	request: &protocol::KvReadStateRequest,
) -> Result<protocol::KvReadStateResponse> {
	ensure!(
		state.config_epoch <= request.epoch,
		"Epoxy key belongs to a newer configuration"
	);
	// The epoch is recorded only with a read promise, so a plain read writes nothing.
	if let Some(ballot) = &request.ballot
		&& state::promise(state)
			.is_none_or(|promise| ballot_order(&promise.ballot) <= protocol_ballot_order(ballot))
	{
		state.config_epoch = request.epoch;
		raise_open_promise(state, ballot);
	}
	Ok(snapshot::snapshot(state).read_state())
}
