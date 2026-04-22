use epoxy_protocol::protocol::ReplicaId;
use universaldb::prelude::*;

pub mod keys;
pub mod replica;

pub use self::keys::{
	ChangelogKey, KvAccepted2Key, KvAcceptedKey, KvAcceptedValue, KvBallotKey,
	KvOptimisticCacheKey, KvValueKey, LegacyCommittedValueKey,
};
pub use self::replica::ConfigKey;

pub fn subspace(replica_id: ReplicaId) -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(RIVET, EPOXY_V2, REPLICA, replica_id))
}

pub fn legacy_subspace(replica_id: ReplicaId) -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(RIVET, EPOXY_V1, REPLICA, replica_id))
}
