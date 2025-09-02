use epoxy_protocol::protocol;
use udb_util::FormalKey;
use universaldb::{FdbBindingError, Transaction};

use crate::keys;

/// Get the current ballot for this replica
pub async fn get_ballot(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
) -> Result<protocol::Ballot, FdbBindingError> {
	let ballot_key = keys::replica::CurrentBallotKey;
	let subspace = keys::subspace(replica_id);
	let packed_key = subspace.pack(&ballot_key);

	match tx.get(&packed_key, false).await? {
		Some(bytes) => {
			let ballot = ballot_key
				.deserialize(&bytes)
				.map_err(|e| FdbBindingError::CustomError(e.into()))?;
			Ok(ballot)
		}
		None => {
			// Default ballot for this replica
			Ok(protocol::Ballot {
				epoch: 0,
				ballot: 0,
				replica_id,
			})
		}
	}
}

/// Increment the ballot number and return the new ballot
pub async fn increment_ballot(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
) -> Result<protocol::Ballot, FdbBindingError> {
	let mut current_ballot = get_ballot(tx, replica_id).await?;

	// Increment ballot number
	current_ballot.ballot += 1;

	// Store the new ballot
	let ballot_key = keys::replica::CurrentBallotKey;
	let subspace = keys::subspace(replica_id);
	let packed_key = subspace.pack(&ballot_key);
	let serialized = ballot_key
		.serialize(current_ballot.clone())
		.map_err(|e| FdbBindingError::CustomError(e.into()))?;

	tx.set(&packed_key, &serialized);

	Ok(current_ballot)
}

/// Compare two ballots to determine ordering
pub fn compare_ballots(
	ballot_a: &protocol::Ballot,
	ballot_b: &protocol::Ballot,
) -> std::cmp::Ordering {
	(ballot_a.epoch, ballot_a.ballot, ballot_a.replica_id).cmp(&(
		ballot_b.epoch,
		ballot_b.ballot,
		ballot_b.replica_id,
	))
}

/// Validate that a ballot is the highest seen for the given instance & updates the highest stored
/// ballot if needed.
///
/// Returns true if the ballot is valid (higher than previously seen).
pub async fn validate_and_update_ballot_for_instance(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	ballot: &protocol::Ballot,
	instance: &protocol::Instance,
) -> Result<bool, FdbBindingError> {
	let instance_ballot_key =
		keys::replica::InstanceBallotKey::new(instance.replica_id, instance.slot_id);
	let subspace = keys::subspace(replica_id);
	let packed_key = subspace.pack(&instance_ballot_key);

	// Get the highest ballot seen for this instance
	let highest_ballot = match tx.get(&packed_key, false).await? {
		Some(bytes) => {
			let stored_ballot = instance_ballot_key
				.deserialize(&bytes)
				.map_err(|e| FdbBindingError::CustomError(e.into()))?;
			stored_ballot
		}
		None => {
			// No ballot seen yet for this instance - use default
			protocol::Ballot {
				epoch: 0,
				ballot: 0,
				replica_id: instance.replica_id,
			}
		}
	};

	// Compare incoming ballot with highest seen - only accept if strictly greater
	let is_valid = match compare_ballots(ballot, &highest_ballot) {
		std::cmp::Ordering::Greater => true,
		std::cmp::Ordering::Equal | std::cmp::Ordering::Less => false,
	};

	// If the incoming ballot is higher, update our stored highest
	if compare_ballots(ballot, &highest_ballot) == std::cmp::Ordering::Greater {
		let serialized = instance_ballot_key
			.serialize(ballot.clone())
			.map_err(|e| FdbBindingError::CustomError(e.into()))?;
		tx.set(&packed_key, &serialized);

		tracing::debug!(?ballot, ?instance, "updated highest ballot for instance");
	}

	Ok(is_valid)
}
