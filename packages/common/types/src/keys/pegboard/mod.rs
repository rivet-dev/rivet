use universaldb::prelude::*;

pub mod ns;

pub fn subspace() -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(RIVET, PEGBOARD))
}
