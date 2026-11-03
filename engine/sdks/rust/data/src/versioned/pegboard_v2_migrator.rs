use crate::generated::pegboard_v2_migrator_v1 as wire;

pub enum MigrateFromV2InFlight {
	V1(wire::MigrateFromV2InFlight),
}

impl_versioned_v1!(MigrateFromV2InFlight, wire::MigrateFromV2InFlight);

pub enum MigrateFromV2Pass {
	V1(wire::MigrateFromV2Pass),
}

impl_versioned_v1!(MigrateFromV2Pass, wire::MigrateFromV2Pass);

pub enum MigrateFromV2Fence {
	V1(wire::MigrateFromV2Fence),
}

impl_versioned_v1!(MigrateFromV2Fence, wire::MigrateFromV2Fence);

pub enum MigrateFromV2StateWriteRequest {
	V1(wire::MigrateFromV2StateWriteRequest),
}

impl_versioned_v1!(
	MigrateFromV2StateWriteRequest,
	wire::MigrateFromV2StateWriteRequest
);

pub enum MigrateFromV2StateWriteResponse {
	V1(wire::MigrateFromV2StateWriteResponse),
}

impl_versioned_v1!(
	MigrateFromV2StateWriteResponse,
	wire::MigrateFromV2StateWriteResponse
);

pub enum MigrateFromV2StateLoadRequest {
	V1(wire::MigrateFromV2StateLoadRequest),
}

impl_versioned_v1!(
	MigrateFromV2StateLoadRequest,
	wire::MigrateFromV2StateLoadRequest
);

pub enum MigrateFromV2StateLoadResponse {
	V1(wire::MigrateFromV2StateLoadResponse),
}

impl_versioned_v1!(
	MigrateFromV2StateLoadResponse,
	wire::MigrateFromV2StateLoadResponse
);

pub enum MigrateFromV2StateReadRequest {
	V1(wire::MigrateFromV2StateReadRequest),
}

impl_versioned_v1!(
	MigrateFromV2StateReadRequest,
	wire::MigrateFromV2StateReadRequest
);

pub enum MigrateFromV2ActorRequest {
	V1(wire::MigrateFromV2ActorRequest),
}

impl_versioned_v1!(MigrateFromV2ActorRequest, wire::MigrateFromV2ActorRequest);

pub enum MigrateFromV2ActorResponse {
	V1(wire::MigrateFromV2ActorResponse),
}

impl_versioned_v1!(MigrateFromV2ActorResponse, wire::MigrateFromV2ActorResponse);
