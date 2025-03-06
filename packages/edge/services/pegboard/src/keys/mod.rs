use fdb_util::prelude::*;
use foundationdb as fdb;

pub mod actor;
pub mod client;
pub mod datacenter;
pub mod env;
pub mod port;

pub fn subspace() -> fdb::tuple::Subspace {
	fdb::tuple::Subspace::all().subspace(&(RIVET, PEGBOARD))
}
