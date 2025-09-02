use epoxy_protocol::protocol::ReplicaId;
use udb_util::prelude::*;

pub mod keys;
pub mod replica;

pub fn subspace(replica_id: ReplicaId) -> udb_util::Subspace {
	udb_util::Subspace::new(&(RIVET, EPOXY, REPLICA, replica_id))
}
