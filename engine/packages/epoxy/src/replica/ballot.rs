use anyhow::Result;
use epoxy_protocol::protocol;
use universaldb::Transaction;
use universaldb::utils::{FormalKey, IsolationLevel::*};

use crate::keys;

/// Get the current ballot for this replica
#[tracing::instrument(skip_all)]
pub async fn get_ballot(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
) -> Result<protocol::Ballot> {
	let ballot_key = keys::replica::CurrentBallotKey;
	let subspace = keys::subspace(replica_id);
	let packed_key = subspace.pack(&ballot_key);

	match tx.get(&packed_key, Serializable).await? {
		Some(bytes) => {
			let ballot = ballot_key.deserialize(&bytes)?;
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
#[tracing::instrument(skip_all)]
pub async fn increment_ballot(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
) -> Result<protocol::Ballot> {
	let mut current_ballot = get_ballot(tx, replica_id).await?;

	// Increment ballot number
	current_ballot.ballot += 1;

	// Store the new ballot
	let ballot_key = keys::replica::CurrentBallotKey;
	let subspace = keys::subspace(replica_id);
	let packed_key = subspace.pack(&ballot_key);
	let serialized = ballot_key.serialize(current_ballot.clone())?;

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

/// Result of ballot validation with detailed context for error reporting
#[derive(Debug)]
pub struct BallotValidationResult {
	pub is_valid: bool,
	pub incoming_ballot: protocol::Ballot,
	pub stored_ballot: protocol::Ballot,
	pub comparison: std::cmp::Ordering,
}

/// Validate that a ballot is the highest seen for the given instance & updates the highest stored
/// ballot if needed.
///
/// Returns detailed validation result including comparison information.
#[tracing::instrument(skip_all)]
pub async fn validate_and_update_ballot_for_instance(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	ballot: &protocol::Ballot,
	instance: &protocol::Instance,
) -> Result<BallotValidationResult> {
	let instance_ballot_key =
		keys::replica::InstanceBallotKey::new(instance.replica_id, instance.slot_id);
	let subspace = keys::subspace(replica_id);
	let packed_key = subspace.pack(&instance_ballot_key);

	// Get the highest ballot seen for this instance
	let highest_ballot = match tx.get(&packed_key, Serializable).await? {
		Some(bytes) => {
			let stored_ballot = instance_ballot_key.deserialize(&bytes)?;
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
	let comparison = compare_ballots(ballot, &highest_ballot);
	let is_valid = match comparison {
		std::cmp::Ordering::Greater => true,
		std::cmp::Ordering::Equal | std::cmp::Ordering::Less => false,
	};

	// If the incoming ballot is higher, update our stored highest
	if comparison == std::cmp::Ordering::Greater {
		let serialized = instance_ballot_key.serialize(ballot.clone())?;
		tx.set(&packed_key, &serialized);

		tracing::debug!(?ballot, ?instance, "updated highest ballot for instance");
	}

	Ok(BallotValidationResult {
		is_valid,
		incoming_ballot: ballot.clone(),
		stored_ballot: highest_ballot,
		comparison,
	})
}
