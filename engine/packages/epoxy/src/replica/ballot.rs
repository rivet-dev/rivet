use anyhow::{Context, Result};
use epoxy_protocol::protocol;
use std::cmp::Ordering;
use universaldb::Transaction;
use universaldb::utils::{FormalKey, IsolationLevel::Serializable};

use crate::keys::{self, CommittedValue, KvBallotKey, KvValueKey, LegacyCommittedValueKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ballot {
	pub counter: u64,
	pub replica_id: protocol::ReplicaId,
}

impl Ballot {
	pub const fn new(counter: u64, replica_id: protocol::ReplicaId) -> Self {
		Self {
			counter,
			replica_id,
		}
	}

	pub const fn zero(replica_id: protocol::ReplicaId) -> Self {
		Self::new(0, replica_id)
	}

	pub const fn is_zero(self) -> bool {
		self.counter == 0
	}
}

impl Ord for Ballot {
	fn cmp(&self, other: &Self) -> Ordering {
		self.counter
			.cmp(&other.counter)
			.then_with(|| self.replica_id.cmp(&other.replica_id))
	}
}

impl PartialOrd for Ballot {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl From<protocol::Ballot> for Ballot {
	fn from(ballot: protocol::Ballot) -> Self {
		Self::new(ballot.counter, ballot.replica_id)
	}
}

impl From<Ballot> for protocol::Ballot {
	fn from(ballot: Ballot) -> Self {
		protocol::Ballot {
			counter: ballot.counter,
			replica_id: ballot.replica_id,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BallotSelection {
	AlreadyCommitted(Vec<u8>),
	AlreadyCommittedMutable {
		value: CommittedValue,
		ballot: Ballot,
	},
	NeedsPrepare {
		ballot: Ballot,
	},
	FreshBallot(Ballot),
}

#[tracing::instrument(skip_all, fields(%replica_id))]
pub async fn ballot_selection(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	key: Vec<u8>,
	mutable: bool,
) -> Result<BallotSelection> {
	let subspace = keys::subspace(replica_id);
	let legacy_subspace = keys::legacy_subspace(replica_id);
	let value_key = KvValueKey::new(key.clone());
	let legacy_value_key = LegacyCommittedValueKey::new(key.clone());
	let legacy_v2_value_key = KvValueKey::new(key.clone());
	let ballot_key = KvBallotKey::new(key);

	let packed_value_key = subspace.pack(&value_key);
	let packed_legacy_value_key = legacy_subspace.pack(&legacy_value_key);
	let packed_legacy_v2_value_key = legacy_subspace.pack(&legacy_v2_value_key);
	let packed_ballot_key = subspace.pack(&ballot_key);

	let (committed_value, legacy_committed_value, legacy_v2_committed_value, current_ballot) = tokio::try_join!(
		async {
			let value = tx.get(&packed_value_key, Serializable).await?;
			if let Some(bytes) = value {
				Ok::<_, anyhow::Error>(Some(value_key.deserialize(&bytes)?))
			} else {
				Ok::<_, anyhow::Error>(None)
			}
		},
		async {
			let value = tx.get(&packed_legacy_value_key, Serializable).await?;
			if let Some(bytes) = value {
				Ok::<_, anyhow::Error>(Some(legacy_value_key.deserialize(&bytes)?))
			} else {
				Ok::<_, anyhow::Error>(None)
			}
		},
		async {
			let value = tx.get(&packed_legacy_v2_value_key, Serializable).await?;
			if let Some(bytes) = value {
				Ok::<_, anyhow::Error>(Some(legacy_v2_value_key.deserialize(&bytes)?))
			} else {
				Ok::<_, anyhow::Error>(None)
			}
		},
		async {
			let ballot = tx.get(&packed_ballot_key, Serializable).await?;
			if let Some(bytes) = ballot {
				Ok::<_, anyhow::Error>(Some(Ballot::from(ballot_key.deserialize(&bytes)?)))
			} else {
				Ok::<_, anyhow::Error>(None)
			}
		}
	)?;

	if let Some(value) = committed_value
		.or_else(|| {
			legacy_committed_value.map(|value| CommittedValue {
				value,
				version: 0,
				mutable: false,
			})
		})
		.or_else(|| legacy_v2_committed_value)
	{
		if !value.mutable || !mutable {
			return Ok(BallotSelection::AlreadyCommitted(value.value));
		}

		let ballot = reserve_next_ballot(
			tx,
			replica_id,
			ballot_key,
			current_ballot.unwrap_or_else(|| Ballot::zero(replica_id)),
		)?;
		return Ok(BallotSelection::AlreadyCommittedMutable { value, ballot });
	}

	let current_ballot = current_ballot.unwrap_or_else(|| Ballot::zero(replica_id));
	let ballot = reserve_next_ballot(tx, replica_id, ballot_key, current_ballot)?;
	if current_ballot.is_zero() {
		Ok(BallotSelection::FreshBallot(ballot))
	} else {
		Ok(BallotSelection::NeedsPrepare { ballot })
	}
}

pub fn store_ballot(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	key: Vec<u8>,
	ballot: Ballot,
) -> Result<()> {
	let ballot_key = KvBallotKey::new(key);
	let subspace = keys::subspace(replica_id);
	let packed_ballot_key = subspace.pack(&ballot_key);
	let serialized = ballot_key.serialize(ballot.into())?;
	tx.set(&packed_ballot_key, &serialized);
	Ok(())
}

fn reserve_next_ballot(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	ballot_key: KvBallotKey,
	current_ballot: Ballot,
) -> Result<Ballot> {
	let next_counter = current_ballot
		.counter
		.checked_add(1)
		.context("ballot counter overflow")?;
	let ballot = Ballot::new(next_counter, replica_id);
	let subspace = keys::subspace(replica_id);
	let packed_ballot_key = subspace.pack(&ballot_key);
	let serialized = ballot_key.serialize(ballot.into())?;
	tx.set(&packed_ballot_key, &serialized);
	Ok(ballot)
}
