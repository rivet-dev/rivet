use gas::prelude::*;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Runner {
	pub runner_id: Id,
	pub namespace_id: Id,
	pub datacenter: String,
	pub name: String,
	pub key: String,
	pub version: u32,
	pub total_slots: u32,
	pub remaining_slots: u32,
	pub create_ts: i64,
	pub drain_ts: Option<i64>,
	pub stop_ts: Option<i64>,
	pub last_ping_ts: i64,
	pub last_connected_ts: Option<i64>,
	pub last_rtt: u32,
	pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}
