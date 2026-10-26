//! The persisted acceptor record, `epoxy.key_state.v1.bare`, and its derived views.
//!
//! **Upgrade contract.** Epoxy runs in a single datacenter today, where every key's scope is the
//! local replica and every proposal commits in its one accept. The record still keeps the
//! multi-region shape: its phase (empty, committed, promised or accepted), the ballots of a
//! promise and an accepted value, a version per value, the key's recorded scope and the
//! configuration epoch. So a multi-region proposer adopts every record written today without a
//! migration: it reads the scope to find the key's quorum and continues from the committed
//! version. Removing or reinterpreting any of these fields needs a new schema version and a
//! converter.

use anyhow::{Result, ensure};
use rivet_data::{generated::epoxy_key_state_v1 as stored, versioned::EpoxyKeyStateData};
use vbare::Versioned;

pub use stored::{
	AcceptedValue, Ballot, CommittedValue, Data as KeyState, KeyAccepted, KeyCommitted, KeyPhase,
	KeyPromised, Promise, Scope,
};

/// A key volume with no record holds an empty acceptor.
pub fn empty() -> KeyState {
	KeyState {
		phase: KeyPhase::KeyEmpty,
		scope: None,
		config_epoch: 0,
	}
}

/// Decodes the key's record. Its embedded vbare header is the record's version mark, so a
/// record written by a newer binary refuses with an explicit error naming the family.
pub fn decode(raw: Option<&[u8]>) -> Result<KeyState> {
	if let Some(raw) = raw {
		rivet_data::families::check_embedded(
			rivet_data::families::VersionFamily::EPOXY_KEY_STATE,
			raw,
		)?;
	}
	Ok(raw
		.map(EpoxyKeyStateData::deserialize_with_embedded_version)
		.transpose()?
		.unwrap_or_else(empty))
}

/// Encodes the record at `version`, the `epoxy.key_state` version the hosting command selected
/// from the read floor, so a rolling deploy never writes a record a live replica cannot read.
pub fn encode(state: &KeyState, version: u16) -> Result<Vec<u8>> {
	EpoxyKeyStateData::serialize_with_embedded_version(state, version)
}

/// Total order on ballots. A higher ballot fences every lower one in the same instance. The
/// incarnation only breaks ties between fast ballots, where no correctness argument relies on
/// which one wins.
pub fn ballot_order(ballot: &Ballot) -> (u64, u64, u64) {
	(
		ballot.counter,
		ballot.replica_id,
		ballot.proposal_incarnation,
	)
}

pub fn committed(state: &KeyState) -> Option<&CommittedValue> {
	match &state.phase {
		KeyPhase::KeyEmpty => None,
		KeyPhase::KeyCommitted(phase) => Some(&phase.committed),
		KeyPhase::KeyPromised(phase) => phase.committed.as_ref(),
		KeyPhase::KeyAccepted(phase) => phase.committed.as_ref(),
	}
}

pub fn promise(state: &KeyState) -> Option<&Promise> {
	match &state.phase {
		KeyPhase::KeyEmpty | KeyPhase::KeyCommitted(_) => None,
		KeyPhase::KeyPromised(phase) => Some(&phase.promise),
		KeyPhase::KeyAccepted(phase) => Some(&phase.promise),
	}
}

pub fn accepted(state: &KeyState) -> Option<&AcceptedValue> {
	match &state.phase {
		KeyPhase::KeyAccepted(phase) => Some(&phase.accepted),
		KeyPhase::KeyEmpty | KeyPhase::KeyCommitted(_) | KeyPhase::KeyPromised(_) => None,
	}
}

/// The newest committed version, or zero when nothing is committed.
pub fn committed_version(state: &KeyState) -> u64 {
	committed(state).map_or(0, |committed| committed.version)
}

/// The version whose instance this acceptor is voting in: the promised one, else the one after
/// the newest committed version.
pub fn open_version(state: &KeyState) -> u64 {
	match promise(state) {
		Some(promise) => promise.version,
		None => committed_version(state) + 1,
	}
}

/// Rebuilds the phase from its parts. Every transition goes through here, so a phase never holds
/// an accepted value without a promise.
pub fn set_phase(
	state: &mut KeyState,
	committed: Option<CommittedValue>,
	promise: Option<Promise>,
	accepted: Option<AcceptedValue>,
) {
	state.phase = match (committed, promise, accepted) {
		(None, None, _) => KeyPhase::KeyEmpty,
		(Some(committed), None, _) => KeyPhase::KeyCommitted(KeyCommitted { committed }),
		(committed, Some(promise), None) => {
			KeyPhase::KeyPromised(KeyPromised { committed, promise })
		}
		(committed, Some(promise), Some(accepted)) => KeyPhase::KeyAccepted(KeyAccepted {
			committed,
			accepted,
			promise,
		}),
	};
}

/// Checks the record's invariants. A violation means a transition is wrong, not that a request
/// was bad.
pub fn validate(state: &KeyState) -> Result<()> {
	let committed_version = committed_version(state);
	if let Some(promise) = promise(state) {
		ensure!(
			promise.version == committed_version + 1,
			"promise is not for the version after the committed one"
		);
	}
	if let (Some(promise), Some(accepted)) = (promise(state), accepted(state)) {
		ensure!(
			accepted.version == promise.version,
			"accepted value is not in the promised instance"
		);
		ensure!(
			ballot_order(&accepted.ballot) <= ballot_order(&promise.ballot),
			"accepted ballot exceeds the promise"
		);
	}
	if let Some(scope) = &state.scope {
		ensure!(!scope.replica_ids.is_empty(), "empty key scope");
		ensure!(
			scope.replica_ids.windows(2).all(|pair| pair[0] < pair[1]),
			"key scope is not sorted and unique"
		);
	}
	Ok(())
}
