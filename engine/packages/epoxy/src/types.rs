use epoxy_protocol::protocol;
use serde::{Deserialize, Serialize};

// IMPORTANT: We cannot use the protocol types in the workflow engine because generated BARE code
// does not allow us to preserve backwards-compatible workflow state.

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct ClusterConfig {
	pub coordinator_replica_id: protocol::ReplicaId,
	pub epoch: u64,
	pub replicas: Vec<ReplicaConfig>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct ReplicaConfig {
	pub replica_id: protocol::ReplicaId,
	pub status: ReplicaStatus,
	pub api_peer_url: String,
	pub guard_url: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, PartialOrd, Ord, Hash, Clone)]
pub enum ReplicaStatus {
	Joining,
	Learning,
	Active,
}

impl From<protocol::ClusterConfig> for ClusterConfig {
	fn from(config: protocol::ClusterConfig) -> Self {
		Self {
			coordinator_replica_id: config.coordinator_replica_id,
			epoch: config.epoch,
			replicas: config
				.replicas
				.into_iter()
				.map(ReplicaConfig::from)
				.collect(),
		}
	}
}

impl From<ClusterConfig> for protocol::ClusterConfig {
	fn from(config: ClusterConfig) -> Self {
		Self {
			coordinator_replica_id: config.coordinator_replica_id,
			epoch: config.epoch,
			replicas: config.replicas.into_iter().map(Into::into).collect(),
		}
	}
}

impl From<protocol::ReplicaConfig> for ReplicaConfig {
	fn from(config: protocol::ReplicaConfig) -> Self {
		Self {
			replica_id: config.replica_id,
			status: config.status.into(),
			api_peer_url: config.api_peer_url,
			guard_url: config.guard_url,
		}
	}
}

impl From<ReplicaConfig> for protocol::ReplicaConfig {
	fn from(config: ReplicaConfig) -> Self {
		Self {
			replica_id: config.replica_id,
			status: config.status.into(),
			api_peer_url: config.api_peer_url,
			guard_url: config.guard_url,
		}
	}
}

impl From<protocol::ReplicaStatus> for ReplicaStatus {
	fn from(status: protocol::ReplicaStatus) -> Self {
		match status {
			protocol::ReplicaStatus::Joining => Self::Joining,
			protocol::ReplicaStatus::Learning => Self::Learning,
			protocol::ReplicaStatus::Active => Self::Active,
		}
	}
}

impl From<ReplicaStatus> for protocol::ReplicaStatus {
	fn from(status: ReplicaStatus) -> Self {
		match status {
			ReplicaStatus::Joining => Self::Joining,
			ReplicaStatus::Learning => Self::Learning,
			ReplicaStatus::Active => Self::Active,
		}
	}
}
