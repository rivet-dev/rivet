use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

/// A structured runtime config change that every process in the cluster picks up on refresh.
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
	pub tracing_filter: Option<Option<String>>,
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub tracing_sampler_ratio: Option<Option<f64>>,
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
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub sqlite_server_prefetch: Option<Option<bool>>,
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
	/// Whether the tablet coordinator chooses discretionary placement actions: bootstrapping
	/// tablets, splits, rebalancing replacements and grant moves. The coordinator picks a
	/// change up at its next drive. Off, reconfigurations already written to the manifest
	/// finish, and repair keeps running: dead or stopping replicas are replaced and short
	/// replica sets grown. Operator commands and missing-owner failover are unaffected.
	#[serde(
		default,
		deserialize_with = "double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub depot_balancing_enabled: Option<Option<bool>>,
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
