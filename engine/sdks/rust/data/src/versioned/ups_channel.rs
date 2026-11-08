use crate::generated::ups_channel_v1;

pub enum UpsChannelData {
	V1(ups_channel_v1::ChannelMessage),
}

impl_versioned_v1!(UpsChannelData, UPS_CHANNEL, ups_channel_v1::ChannelMessage);
