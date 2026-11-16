//! Version envelope for the grant set a JWT carries, which any engine worker may verify.
//!
//! Version 2 adds the fork operation. A grant set is encoded at the lowest version that carries
//! it, so a token without fork stays at version 1, which every engine with JWTs decodes.

mod auth_grants_v1_to_auth_grants_v2;
mod auth_grants_v2_to_auth_grants_v1;

use crate::generated::{auth_grants_v1, auth_grants_v2};

pub enum AuthGrantsData {
	V1(auth_grants_v1::Data),
	V2(auth_grants_v2::Data),
}

impl_versioned_v1_v2!(
	AuthGrantsData,
	AUTH_GRANTS,
	auth_grants_v1::Data,
	auth_grants_v2::Data,
	auth_grants_v1_to_auth_grants_v2::convert_data_auth_grants_v1_to_auth_grants_v2,
	auth_grants_v2_to_auth_grants_v1::convert_data_auth_grants_v2_to_auth_grants_v1
);
