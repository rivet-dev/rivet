use anyhow::Result;
use epoxy_protocol::protocol;
use universaldb::{Transaction, utils::IsolationLevel::Serializable};

use crate::{
	keys::{self, KvAccepted2Key, KvAcceptedKey, KvBallotKey, KvValueKey},
	replica::ballot::Ballot,
};

#[tracing::instrument(skip_all, fields(%replica_id, key = ?accept_req.key))]
pub async fn accept(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	accept_req: protocol::AcceptRequest,
) -> Result<protocol::AcceptResponse> {
	let tx = tx.with_subspace(keys::subspace(replica_id));
	let protocol::AcceptRequest {
		key,
		value,
		ballot,
		mutable,
		version,
	} = accept_req;

	let value_key = KvValueKey::new(key.clone());
	let ballot_key = KvBallotKey::new(key.clone());
	let accepted_key = KvAcceptedKey::new(key.clone());
	let accepted2_key = KvAccepted2Key::new(key);
	let request_ballot = Ballot::from(ballot.clone());

	let (committed_value, current_ballot, accepted_value, accepted2_value) = tokio::try_join!(
		tx.read_opt(&value_key, Serializable),
		tx.read_opt(&ballot_key, Serializable),
		tx.read_opt(&accepted_key, Serializable),
		tx.read_opt(&accepted2_key, Serializable),
	)?;

	if let Some(committed_value) = committed_value {
		if !committed_value.mutable || !mutable || version <= committed_value.version {
			return Ok(protocol::AcceptResponse::AcceptResponseAlreadyCommitted(
				protocol::AcceptResponseAlreadyCommitted {
					value: committed_value.value,
				},
			));
		}
	}

	if let Some(current_ballot) = current_ballot.map(Ballot::from) {
		if current_ballot > request_ballot {
			return Ok(protocol::AcceptResponse::AcceptResponseHigherBallot(
				protocol::AcceptResponseHigherBallot {
					ballot: current_ballot.into(),
				},
			));
		}
	}

	if let Some(existing_accepted) = accepted2_value {
		let existing_ballot = Ballot::from(existing_accepted.ballot.clone());
		if existing_ballot == request_ballot {
			let same_value = existing_accepted.value == value;
			let same_version = existing_accepted.version == version;
			let same_mutability = existing_accepted.mutable == mutable;
			if same_value && same_version && same_mutability {
				tx.write(&ballot_key, ballot.clone())?;
				return Ok(protocol::AcceptResponse::AcceptResponseOk(
					protocol::AcceptResponseOk { ballot },
				));
			}

			return Ok(protocol::AcceptResponse::AcceptResponseHigherBallot(
				protocol::AcceptResponseHigherBallot {
					ballot: existing_ballot.into(),
				},
			));
		}
	}

	// Legacy value
	if let Some(existing_accepted) = accepted_value {
		let existing_ballot = Ballot::from(existing_accepted.ballot.clone());
		if existing_ballot == request_ballot {
			let same_value = Some(existing_accepted.value) == value;
			let same_version = existing_accepted.version == version;
			let same_mutability = existing_accepted.mutable == mutable;
			if same_value && same_version && same_mutability {
				tx.write(&ballot_key, ballot.clone())?;
				return Ok(protocol::AcceptResponse::AcceptResponseOk(
					protocol::AcceptResponseOk { ballot },
				));
			}

			return Ok(protocol::AcceptResponse::AcceptResponseHigherBallot(
				protocol::AcceptResponseHigherBallot {
					ballot: existing_ballot.into(),
				},
			));
		}
	}

	tx.write(&ballot_key, ballot.clone())?;
	tx.write(
		&accepted2_key,
		protocol::AcceptedValue {
			value,
			ballot: ballot.clone(),
			version,
			mutable,
		},
	)?;

	Ok(protocol::AcceptResponse::AcceptResponseOk(
		protocol::AcceptResponseOk { ballot },
	))
}
