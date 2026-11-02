use crate::generated::epoxy_key_state_v1;

pub enum EpoxyKeyStateData {
	V1(epoxy_key_state_v1::Data),
}

impl_versioned_v1!(EpoxyKeyStateData, epoxy_key_state_v1::Data);
