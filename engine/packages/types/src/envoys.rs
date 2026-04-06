use gas::prelude::*;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Envoy {
	pub envoy_key: String,
	pub namespace_id: Id,
	pub datacenter: String,
	pub pool_name: String,
	pub version: u32,
	pub slots: u64,
	pub create_ts: i64,
	pub stop_ts: Option<i64>,
	pub last_ping_ts: i64,
	pub last_connected_ts: Option<i64>,
	pub last_rtt: u32,
	pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}
