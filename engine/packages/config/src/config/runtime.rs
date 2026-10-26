use anyhow::{Result, bail};
use std::collections::HashMap;
use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
	/// Adjusts worker curve around this value (in millecores, i.e. 1000 = 1 core). Is not a hard limit. When
	/// unset, uses /sys/fs/cgroup/cpu.max, and if that is unset uses total host cpu.
	pub worker_cpu_max: Option<usize>,
	/// Time (in seconds) to allow for the gasoline worker engine to stop gracefully after receiving SIGTERM.
	/// Defaults to 30 seconds.
	worker_shutdown_duration: Option<u32>,
	/// Poll interval for pulling workflows and deciding whether a short sleep stays in memory.
	/// Unit is in milliseconds. Defaults to 16000.
	pub(crate) worker_poll_interval_ms: Option<u64>,
	/// Maximum wake condition keys read per workflow name in a single pull. Defaults to 20000, or
	/// 5000 for the storage compaction workflows. Set the "default" key to apply to all workflow
	/// names.
	pub(crate) worker_max_wake_keys_per_workflow_name_per_pull: Option<HashMap<String, usize>>,
	/// Maximum deduplicated workflows considered in a single pull. Defaults to 10000.
	pub(crate) worker_max_deduped_workflows_per_pull: Option<usize>,
	/// Maximum wake condition keys a worker clears when leasing workflows in a single pull.
	/// Defaults to 40000.
	/// Maximum workflows a worker attempts to lease in a single pull. Defaults to 1000.
	pub(crate) worker_max_workflows_per_pull: Option<usize>,
	pub(crate) worker_max_wake_condition_clears_per_pull: Option<usize>,
	/// Maximum concurrently running workflows of a given workflow name for the **entire cluster**.
	pub(crate) worker_max_concurrent_workflows: Option<HashMap<String, usize>>,
	/// Time (in seconds) to allow for guard to wait for pending requests after receiving SIGTERM. Defaults
	/// to nine minutes.
	guard_shutdown_duration: Option<u32>,
	/// Time (in seconds) after which the engine process will forcibly exit after receiving SIGTERM.
	/// Must be greater than or equal to both worker_shutdown_duration and guard_shutdown_duration.
	/// Defaults to nine minutes and 50 seconds, allowing the storage stop wait, singleton handover
	/// and teardown reserve.
	force_shutdown_duration: Option<u32>,
}

impl Runtime {
	pub fn validate(&self) -> Result<()> {
		if self.worker_poll_interval_ms == Some(0) {
			bail!("runtime.worker_poll_interval_ms must be greater than 0");
		}
		if let Some(max_wake_keys) = &self.worker_max_wake_keys_per_workflow_name_per_pull {
			for (workflow_name, max) in max_wake_keys {
				if *max == 0 {
					bail!(
						"runtime.worker_max_wake_keys_per_workflow_name_per_pull.{workflow_name} must be greater than 0"
					);
				}
			}
		}
		if self.worker_max_deduped_workflows_per_pull == Some(0) {
			bail!("runtime.worker_max_deduped_workflows_per_pull must be greater than 0");
		}
		if self.worker_max_workflows_per_pull == Some(0) {
			bail!("runtime.worker_max_workflows_per_pull must be greater than 0");
		}
		if self.worker_max_wake_condition_clears_per_pull == Some(0) {
			bail!("runtime.worker_max_wake_condition_clears_per_pull must be greater than 0");
		}

		Ok(())
	}

	pub fn worker_shutdown_duration(&self) -> Duration {
		Duration::from_secs(self.worker_shutdown_duration.unwrap_or(30) as u64)
	}

	pub fn worker_poll_interval(&self) -> Duration {
		Duration::from_millis(self.worker_poll_interval_ms.unwrap_or(16_000))
	}

	pub fn worker_max_wake_keys_per_workflow_name_per_pull(&self, workflow_name: &str) -> usize {
		if let Some(map) = &self.worker_max_wake_keys_per_workflow_name_per_pull {
			if let Some(max) = map.get(workflow_name).or_else(|| map.get("default")) {
				return *max;
			}
		}

		// Compaction workflows wake far more often than they can be leased, so a smaller read keeps
		// their wake subspace from crowding out the rest of the pull.
		match workflow_name {
			"depot_db_manager3"
			| "depot_db_hot_compactor3"
			| "depot_db_cold_compactor3"
			| "depot_db_reclaimer3" => 5_000,
			_ => 20_000,
		}
	}

	pub fn worker_max_deduped_workflows_per_pull(&self) -> usize {
		self.worker_max_deduped_workflows_per_pull.unwrap_or(10_000)
	}

	pub fn worker_max_workflows_per_pull(&self) -> usize {
		self.worker_max_workflows_per_pull.unwrap_or(1_000)
	}

	pub fn worker_max_wake_condition_clears_per_pull(&self) -> usize {
		// Signal wake clears use the largest wake condition key:
		// roughly 100 bytes for `(RIVET, GASOLINE, KV, WAKE, WORKFLOW, workflow_name, ts,
		// workflow_id, SIGNAL, signal_id)`. A 10 MiB transaction fits about
		// 10 * 1024 * 1024 / 100 = 104857 clears.
		// Default pulled wake conditions roughly maxes out at 40k (5k x 4 compaction workflows + 20k actor
		// wfs, the rest are negligible)
		self.worker_max_wake_condition_clears_per_pull
			.unwrap_or(40_000)
	}

	pub fn worker_max_concurrent_workflows(&self) -> HashMap<String, usize> {
		let mut map = HashMap::from([
			("depot_db_manager3".to_string(), 100),
			("depot_db_hot_compactor3".to_string(), 100),
			("depot_db_cold_compactor3".to_string(), 100),
			("depot_db_reclaimer3".to_string(), 100),
		]);

		if let Some(worker_max_concurrent_workflows) = &self.worker_max_concurrent_workflows {
			map.extend(worker_max_concurrent_workflows.clone());
		}

		map
	}

	pub fn guard_shutdown_duration(&self) -> Duration {
		Duration::from_secs(self.guard_shutdown_duration.unwrap_or(9 * 60) as u64)
	}

	/// Returns the force shutdown duration, defaulting to nine minutes and 50 seconds.
	pub fn force_shutdown_duration(&self) -> Duration {
		Duration::from_secs(self.force_shutdown_duration.unwrap_or(9 * 60 + 50) as u64)
	}
}
