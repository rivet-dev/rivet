use foundationdb as fdb;

pub mod client;
pub mod datacenter;

pub fn subspace() -> fdb::tuple::Subspace {
	fdb::tuple::Subspace::all().subspace(&("pegboard"))
}
