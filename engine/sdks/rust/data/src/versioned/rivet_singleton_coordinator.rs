use crate::generated::rivet_singleton_coordinator_v1;

pub enum RivetSingletonCoordinatorData {
	V1(rivet_singleton_coordinator_v1::Data),
}

impl_versioned_v1!(
	RivetSingletonCoordinatorData,
	rivet_singleton_coordinator_v1::Data
);
