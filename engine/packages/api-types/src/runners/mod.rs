use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod list;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Runner {
	pub runner_key: String,
	pub namespace_id: rivet_util::Id,
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

impl From<rivet_types::envoys::Envoy> for Runner {
	fn from(envoy: rivet_types::envoys::Envoy) -> Self {
		Self {
			runner_key: envoy.envoy_key,
			namespace_id: envoy.namespace_id,
			datacenter: envoy.datacenter,
			pool_name: envoy.pool_name,
			version: envoy.version,
			slots: envoy.slots,
			create_ts: envoy.create_ts,
			stop_ts: envoy.stop_ts,
			last_ping_ts: envoy.last_ping_ts,
			last_connected_ts: envoy.last_connected_ts,
			last_rtt: envoy.last_rtt,
			metadata: envoy.metadata,
		}
	}
}
