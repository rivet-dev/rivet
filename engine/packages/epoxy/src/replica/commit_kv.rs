use anyhow::Result;
use epoxy_protocol::protocol;
use universaldb::{
	Transaction,
	utils::IsolationLevel::Serializable,
};

use crate::{
	keys::{self, CommittedValue, KvAcceptedKey, KvBallotKey, KvOptimisticCacheKey, KvValueKey},
	replica::ballot::Ballot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitKvOutcome {
	Committed,
	AlreadyCommitted { value: Vec<u8>, version: u64 },
	StaleBallot { current_ballot: protocol::Ballot },
}

#[tracing::instrument(skip_all, fields(%replica_id, key = ?key))]
pub async fn commit_kv(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	key: Vec<u8>,
	value: Vec<u8>,
	ballot: protocol::Ballot,
	mutable: bool,
	version: u64,
) -> Result<CommitKvOutcome> {
	let tx = tx.with_subspace(keys::subspace(replica_id));
	let value_key = KvValueKey::new(key.clone());
	let ballot_key = KvBallotKey::new(key.clone());
	let accepted_key = KvAcceptedKey::new(key.clone());
	let cache_key = KvOptimisticCacheKey::new(key.clone());
	let request_ballot = Ballot::from(ballot.clone());

	let (committed_value, current_ballot, accepted_value) = tokio::try_join!(
		tx.read_opt(&value_key, Serializable),
		tx.read_opt(&ballot_key, Serializable),
		tx.read_opt(&accepted_key, Serializable),
	)?;

	if let Some(committed_value) = committed_value {
		if !committed_value.mutable || !mutable || version <= committed_value.version {
			return Ok(CommitKvOutcome::AlreadyCommitted {
				value: committed_value.value,
				version: committed_value.version,
			});
		}
	}

	let accepted_matches_request = accepted_value.as_ref().map_or(false, |accepted_value| {
		accepted_value.ballot == ballot
			&& accepted_value.value == value
			&& accepted_value.version == version
			&& accepted_value.mutable == mutable
	});

	if let Some(current_ballot) = current_ballot.map(Ballot::from) {
		// Once a replica has accepted this exact value at this ballot, a later prepare can raise
		// the promise without invalidating the already-chosen value. Allow the commit to finish in
		// that case so quorum acceptance can still become learned state.
		if request_ballot < current_ballot && !accepted_matches_request {
			return Ok(CommitKvOutcome::StaleBallot {
				current_ballot: current_ballot.into(),
			});
		}
	}

	tx.write(
		&value_key,
		CommittedValue {
			value: value.clone(),
			version,
			mutable,
		},
	)?;
	tx.delete(&accepted_key);
	if mutable {
		tx.delete(&ballot_key);
		tx.delete(&cache_key);
	}
	crate::replica::changelog::append(replica_id, &tx, key, value, version, mutable)?;

	Ok(CommitKvOutcome::Committed)
}
