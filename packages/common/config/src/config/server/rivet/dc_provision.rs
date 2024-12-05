use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DatacenterProvision {
	pub provider: Provider,
	pub provider_datacenter_id: String,
	pub pools: HashMap<PoolType, Pool>,
	pub prebakes_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Provider {
	Linode,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Pool {
	pub hardware: Vec<Hardware>,
	pub desired_count: u32,
	pub min_count: u32,
	pub max_count: u32,
	pub drain_timeout: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PoolType {
	Job,
	Gg,
	Ats,
	Pegboard,
	PegboardIsolate,
	Fdb,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Hardware {
	pub name: String,
}
