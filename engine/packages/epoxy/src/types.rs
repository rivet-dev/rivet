use epoxy_protocol::protocol;
use serde::{Deserialize, Serialize};

// IMPORTANT: We cannot use the protocol types in the workflow engine because generated BARE codes
// does now allow us to have backwards compatibility.

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct Instance {
	pub replica_id: protocol::ReplicaId,
	pub slot_id: protocol::SlotId,
}

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

impl Into<protocol::Instance> for Instance {
	fn into(self) -> protocol::Instance {
		protocol::Instance {
			replica_id: self.replica_id,
			slot_id: self.slot_id,
		}
	}
}

impl From<protocol::Instance> for Instance {
	fn from(instance: protocol::Instance) -> Self {
		Instance {
			replica_id: instance.replica_id,
			slot_id: instance.slot_id,
		}
	}
}

impl Into<protocol::ClusterConfig> for ClusterConfig {
	fn into(self) -> protocol::ClusterConfig {
		protocol::ClusterConfig {
			coordinator_replica_id: self.coordinator_replica_id,
			epoch: self.epoch,
			replicas: self.replicas.into_iter().map(|r| r.into()).collect(),
		}
	}
}

impl From<protocol::ClusterConfig> for ClusterConfig {
	fn from(config: protocol::ClusterConfig) -> Self {
		ClusterConfig {
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

impl From<protocol::ReplicaConfig> for ReplicaConfig {
	fn from(config: protocol::ReplicaConfig) -> Self {
		ReplicaConfig {
			replica_id: config.replica_id,
			status: config.status.into(),
			api_peer_url: config.api_peer_url,
			guard_url: config.guard_url,
		}
	}
}

impl Into<protocol::ReplicaConfig> for ReplicaConfig {
	fn into(self) -> protocol::ReplicaConfig {
		protocol::ReplicaConfig {
			replica_id: self.replica_id,
			status: self.status.into(),
			api_peer_url: self.api_peer_url,
			guard_url: self.guard_url,
		}
	}
}

impl From<protocol::ReplicaStatus> for ReplicaStatus {
	fn from(status: protocol::ReplicaStatus) -> Self {
		match status {
			protocol::ReplicaStatus::Joining => ReplicaStatus::Joining,
			protocol::ReplicaStatus::Learning => ReplicaStatus::Learning,
			protocol::ReplicaStatus::Active => ReplicaStatus::Active,
		}
	}
}

impl Into<protocol::ReplicaStatus> for ReplicaStatus {
	fn into(self) -> protocol::ReplicaStatus {
		match self {
			ReplicaStatus::Joining => protocol::ReplicaStatus::Joining,
			ReplicaStatus::Learning => protocol::ReplicaStatus::Learning,
			ReplicaStatus::Active => protocol::ReplicaStatus::Active,
		}
	}
}
