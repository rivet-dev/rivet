use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

/// A structured runtime config change, broadcast to every process in the cluster.
///
/// Runtime reconfiguration is deliberately structured rather than an arbitrary patch over the
/// config root: only the properties listed here can be changed, so an operator cannot reshape config
/// into a state that was never validated at load time. Add a field here to make one more property
/// runtime-configurable.
///
/// Each field is doubly optional: absent leaves the current value alone, `Some(None)` clears the
/// override so the value loaded from the config file and environment applies again, and
/// `Some(Some(value))` overrides it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicConfigUpdate {
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub max_storage_bytes: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_admission_percent: Option<Option<f64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_write_bytes_per_second: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub actor_write_bytes_per_second: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub actor_read_bytes_per_second: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_read_bytes_per_second: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_hot_fold_direct_to_shard: Option<Option<bool>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_max_hot_drain_span_txids: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_stage_throttle_budget_multiplier: Option<Option<f64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_stage_throttle_admit_soft_util: Option<Option<f64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_stage_throttle_backoff_ms: Option<Option<i64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_install_throttle_budget_multiplier: Option<Option<f64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_install_throttle_admit_soft_util: Option<Option<f64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_reclaim_throttle_budget_multiplier: Option<Option<f64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub compaction_reclaim_throttle_admit_soft_util: Option<Option<f64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub worker_poll_interval_ms: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub worker_max_deduped_workflows_per_pull: Option<Option<usize>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub worker_max_workflows_per_pull: Option<Option<usize>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub worker_max_wake_condition_clears_per_pull: Option<Option<usize>>,
	/// Per workflow name wake key limits. Unlike the other properties this is keyed, so an absent
	/// key leaves that workflow name alone, `Some(max)` overrides it, and `None` clears the
	/// override for that one name. The "default" key applies to all workflow names.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub worker_max_wake_keys_per_workflow_name_per_pull: Option<HashMap<String, Option<usize>>>,
	/// Per workflow name concurrency limits. Unlike the other properties this is keyed, so an
	/// absent key leaves that workflow name alone, `Some(max)` overrides it, and `None` clears the
	/// override for that one name.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub worker_max_concurrent_workflows: Option<HashMap<String, Option<usize>>>,
}

/// Deserializes a present `null` as `Some(None)` rather than `None`, so clearing a property stays
/// distinct from leaving it alone. Without this the two collapse into the same value on the wire.
pub fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
	T: Deserialize<'de>,
	D: Deserializer<'de>,
{
	Deserialize::deserialize(deserializer).map(Some)
}
