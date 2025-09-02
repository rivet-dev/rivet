use anyhow::*;
use epoxy_protocol::protocol::{self, ReplicaId};
use universaldb::Transaction;

#[derive(Clone, Copy)]
pub enum QuorumType {
	Fast,
	Slow,
	All,
	Any,
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

pub fn calculate_quorum(n: usize, q: QuorumType) -> usize {
	match q {
		QuorumType::Fast => (n * 3) / 4 + 1,
		QuorumType::Slow => n / 2 + 1,
		QuorumType::All => n,
		QuorumType::Any => 1,
	}
}

pub async fn read_config(
	tx: &Transaction,
	replica_id: ReplicaId,
) -> Result<protocol::ClusterConfig> {
	use udb_util::FormalKey;

	let config_key = crate::keys::replica::ConfigKey;
	let subspace = crate::keys::subspace(replica_id);
	let packed_key = subspace.pack(&config_key);

	match tx.get(&packed_key, false).await? {
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
