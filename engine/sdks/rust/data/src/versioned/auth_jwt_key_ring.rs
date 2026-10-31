use crate::generated::auth_jwt_key_ring_v1;

pub enum AuthJwtKeyRingData {
	V1(auth_jwt_key_ring_v1::Data),
}

impl_versioned_v1!(AuthJwtKeyRingData, auth_jwt_key_ring_v1::Data);
