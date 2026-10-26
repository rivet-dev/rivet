use crate::generated::ups_packet_v1;

pub enum UpsPacketData {
	V1(ups_packet_v1::Data),
}

impl_versioned_v1!(UpsPacketData, ups_packet_v1::Data);
