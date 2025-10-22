use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("kv")]
pub enum Kv {
	#[error("no_leader_elected", "No leader has been elected yet.")]
	NoLeaderElected,

	#[error("leader_forwarding_failed", "Failed to forward request to leader.")]
	LeaderForwardingFailed,

	#[error(
		"response_channel_closed",
		"Failed to receive KV response, channel closed."
	)]
	ResponseChannelClosed,

	#[error("not_leader", "Current node is not the leader.")]
	NotLeader,
}
