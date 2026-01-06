use universaldb::prelude::*;

pub fn subspace() -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(RIVET, BACKFILL))
}
