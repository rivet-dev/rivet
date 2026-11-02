use crate::generated::auth_grants_v1;

pub enum AuthGrantsData {
	V1(auth_grants_v1::Data),
}

impl_versioned_v1!(AuthGrantsData, auth_grants_v1::Data);
