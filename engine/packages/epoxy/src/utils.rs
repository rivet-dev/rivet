use anyhow::*;
use epoxy_protocol::protocol::{self, ReplicaId};
use std::fmt;
use universaldb::{Transaction, utils::IsolationLevel::*};

#[derive(Clone, Copy, Debug)]
pub enum QuorumType {
	Fast,
	Slow,
	All,
	Any,
}

impl fmt::Display for QuorumType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			QuorumType::Fast => write!(f, "fast"),
			QuorumType::Slow => write!(f, "slow"),
			QuorumType::All => write!(f, "all"),
			QuorumType::Any => write!(f, "any"),
		}
	}
}

pub enum ReplicaFilter {
	All,
	Active,
}

/// Use this replica list for any action that requires a quorum.
pub fn get_quorum_members(config: &protocol::ClusterConfig) -> Vec<ReplicaId> {
	// Only active nodes can participate in quorums
	config
		.replicas
		.iter()
		.filter(|r| matches!(r.status, protocol::ReplicaStatus::Active))
		.map(|r| r.replica_id)
		.collect()
}

/// Use this replica list for any action that should still be sent to joining replicas.
pub fn get_all_replicas(config: &protocol::ClusterConfig) -> Vec<ReplicaId> {
	config.replicas.iter().map(|r| r.replica_id).collect()
}

// See EPaxos 4.3
pub fn calculate_quorum(n: usize, q: QuorumType) -> usize {
	match n {
		// Nonsensical
		0 => 0,
		1 => 1,
		// EPaxos does not apply to clusters with N < 3 because you cannot tolerate any faults. However we can
		// still get correctness invariants to hold by requiring both nodes to agree on everything (quorum
		// size is always 2)
		2 => match q {
			QuorumType::Fast => 2,
			QuorumType::Slow => 2,
			QuorumType::All => 2,
			QuorumType::Any => 1,
		},
		// Note that for even N's we don't gain any extra fault tolerance but we get potentially better read
		// latency. N=4 acts like N=3 in terms of fault tolerance.
		n => {
			let f = (n - 1) / 2;

			match q {
				QuorumType::Fast => f + (f + 1) / 2,
				QuorumType::Slow => f + 1,
				QuorumType::All => n,
				QuorumType::Any => 1,
			}
		}
	}
}

/// Calculates quorum size assuming the sender is excluded.
pub fn calculate_fanout_quorum(n: usize, q: QuorumType) -> usize {
	match n {
		// Nonsensical
		0 => 0,
		1 => 0,
		// NOTE: See comments in `calculate_quorum`
		2 => 1,
		n => {
			let f = (n - 1) / 2;

			match q {
				QuorumType::Fast => (f + (f + 1) / 2) - 1,
				QuorumType::Slow => f,
				QuorumType::All => n - 1,
				QuorumType::Any => 1,
			}
		}
	}
}

pub async fn read_config(
	tx: &Transaction,
	replica_id: ReplicaId,
) -> Result<protocol::ClusterConfig> {
	use universaldb::utils::FormalKey;

	let config_key = crate::keys::replica::ConfigKey;
	let subspace = crate::keys::subspace(replica_id);
	let packed_key = subspace.pack(&config_key);

	match tx.get(&packed_key, Serializable).await? {
		Some(value) => {
			let config = config_key.deserialize(&value)?;
			Ok(config)
		}
		None => {
			bail!(
				"replica {} has not been configured yet, verify that the coordinator has reconfigured the cluster for this replica successfully",
				replica_id
			)
		}
	}
}

pub async fn read_instance_number(tx: &Transaction, replica_id: ReplicaId) -> Result<u64> {
	use universaldb::prelude::*;

	let subspace = crate::keys::subspace(replica_id);
	let instance_number_key = crate::keys::replica::InstanceNumberKey;
	let packed_key = subspace.pack(&instance_number_key);

	match tx.get(&packed_key, Serializable).await? {
		Some(bytes) => Ok(instance_number_key.deserialize(&bytes)?),
		None => Ok(0),
	}
}

/// Reads all instances that have touched a specific key.
pub async fn read_key_instances(
	tx: &Transaction,
	replica_id: ReplicaId,
	key: Vec<u8>,
) -> Result<Vec<protocol::Instance>> {
	use universaldb::RangeOption;
	use universaldb::prelude::*;

	let subspace = crate::keys::subspace(replica_id);
	let key_subspace = subspace.subspace(&crate::keys::replica::KeyInstanceKey::subspace(key));

	let range_opt: RangeOption = (&key_subspace).into();
	let entries = tx.get_range(&range_opt, 1, Serializable).await?;

	let mut instances = Vec::new();
	for kv in entries.into_iter() {
		// Unpack the key to get instance info
		let (instance_replica_id, instance_slot_id): (ReplicaId, protocol::SlotId) =
			key_subspace.unpack(kv.key())?;
		instances.push(protocol::Instance {
			replica_id: instance_replica_id,
			slot_id: instance_slot_id,
		});
	}

	Ok(instances)
}

/// Reads a log entry for a specific instance.
pub async fn read_log_entry(
	tx: &Transaction,
	replica_id: ReplicaId,
	instance: &protocol::Instance,
) -> Result<Option<protocol::LogEntry>> {
	use universaldb::prelude::*;

	let subspace = crate::keys::subspace(replica_id);
	let log_key = crate::keys::replica::LogEntryKey::new(instance.replica_id, instance.slot_id);
	let packed_key = subspace.pack(&log_key);

	match tx.get(&packed_key, Serializable).await? {
		Some(bytes) => Ok(Some(log_key.deserialize(&bytes)?)),
		None => Ok(None),
	}
}
