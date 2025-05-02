use fdb_util::prelude::*;

pub mod actor;
pub mod client;
pub mod datacenter;
pub mod env;
pub mod port;

pub fn subspace() -> fdb_util::Subspace {
	fdb_util::Subspace::new(&(RIVET, PEGBOARD))
}
