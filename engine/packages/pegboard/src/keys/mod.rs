use universaldb::prelude::*;

pub mod actor;
pub mod epoxy;
pub mod ns;
pub mod runner;
pub mod runner_config;

pub fn subspace() -> universaldb::utils::Subspace {
	rivet_types::keys::pegboard::subspace()
}

pub fn actor_kv_subspace() -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(RIVET, PEGBOARD, ACTOR_KV))
}
