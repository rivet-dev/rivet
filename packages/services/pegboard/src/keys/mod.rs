use udb_util::prelude::*;

pub mod actor;
pub mod epoxy;
pub mod ns;
pub mod runner;

pub fn subspace() -> udb_util::Subspace {
	rivet_types::keys::pegboard::subspace()
}

pub fn actor_kv_subspace() -> udb_util::Subspace {
	udb_util::Subspace::new(&(RIVET, PEGBOARD, ACTOR_KV))
}
