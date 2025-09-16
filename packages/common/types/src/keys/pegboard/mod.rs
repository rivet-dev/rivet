use udb_util::prelude::*;

pub mod ns;

pub fn subspace() -> udb_util::Subspace {
	udb_util::Subspace::new(&(RIVET, PEGBOARD))
}
