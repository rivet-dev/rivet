use crate::generated::ups_request_v1;

pub enum UpsRequestData {
	V1(ups_request_v1::Data),
}

impl_versioned_v1!(UpsRequestData, UPS_REQUEST, ups_request_v1::Data);
