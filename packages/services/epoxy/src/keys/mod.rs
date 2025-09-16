use epoxy_protocol::protocol::ReplicaId;
use universaldb::prelude::*;

pub mod keys;
pub mod replica;

pub fn subspace(replica_id: ReplicaId) -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(RIVET, EPOXY, REPLICA, replica_id))
}
