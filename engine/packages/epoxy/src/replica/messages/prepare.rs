use anyhow::Result;
use epoxy_protocol::protocol;
use universaldb::{Transaction, utils::IsolationLevel::Serializable};

use crate::{
	keys::{self, KvAccepted2Key, KvAcceptedKey, KvBallotKey, KvValueKey},
	replica::ballot::Ballot,
};

#[tracing::instrument(skip_all, fields(%replica_id, key = ?prepare_req.key))]
pub async fn prepare(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	prepare_req: protocol::PrepareRequest,
) -> Result<protocol::PrepareResponse> {
	let tx = tx.with_subspace(keys::subspace(replica_id));
	let protocol::PrepareRequest {
		key,
		ballot,
		mutable,
		version,
	} = prepare_req;

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
			return Ok(protocol::PrepareResponse::PrepareResponseAlreadyCommitted(
				protocol::PrepareResponseAlreadyCommitted {
					value: committed_value.value,
				},
			));
		}
	}

	if let Some(current_ballot) = current_ballot.map(Ballot::from) {
		// Equal-ballot Prepare is idempotent. This allows a proposer that has already
		// reserved a ballot locally to include itself in the prepare quorum.
		if request_ballot < current_ballot {
			return Ok(protocol::PrepareResponse::PrepareResponseHigherBallot(
				protocol::PrepareResponseHigherBallot {
					ballot: current_ballot.into(),
				},
			));
		}
	}

	// Promise not to accept lower ballots by persisting the request ballot.
	// After this write, highest_ballot == request.ballot by construction.
	tx.write(&ballot_key, ballot.clone())?;

	let (accepted_value, accepted_ballot) = match accepted2_value {
		Some(accepted_value) => {
			let protocol::AcceptedValue {
				value,
				ballot,
				version,
				mutable,
			} = accepted_value;
			(
				Some(protocol::CommittedValue {
					value,
					version,
					mutable,
				}),
				Some(ballot),
			)
		}
		None => match accepted_value {
			Some(accepted_value) => {
				let crate::keys::KvAcceptedValue {
					value,
					ballot,
					version,
					mutable,
				} = accepted_value;
				(
					Some(protocol::CommittedValue {
						value: Some(value),
						version,
						mutable,
					}),
					Some(ballot),
				)
			}
			None => (None, None),
		},
	};

	Ok(protocol::PrepareResponse::PrepareResponseOk(
		protocol::PrepareResponseOk {
			highest_ballot: ballot,
			accepted_value,
			accepted_ballot,
		},
	))
}
