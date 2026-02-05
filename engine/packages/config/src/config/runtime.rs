use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
	#[serde(default)]
	pub worker: Worker,
	/// Time (in seconds) to allow for guard to wait for pending requests after receiving SIGTERM. Defaults
	/// to 1 hour.
	guard_shutdown_duration: Option<u32>,
	/// Time (in seconds) after which the engine process will forcibly exit after receiving SIGTERM.
	/// Must be greater than both worker_shutdown_duration and guard_shutdown_duration.
	/// Defaults to guard_shutdown_duration + 30 seconds.
	force_shutdown_duration: Option<u32>,
	/// Whether or not to allow running the engine when the previous version that was run is higher than
	/// the current version.
	allow_version_rollback: Option<bool>,
}

impl Runtime {
	pub fn guard_shutdown_duration(&self) -> Duration {
		Duration::from_secs(self.guard_shutdown_duration.unwrap_or(60 * 60) as u64)
	}

	/// Returns the force shutdown duration, defaulting to guard_shutdown_duration + 30 seconds.
	pub fn force_shutdown_duration(&self) -> Duration {
		self.force_shutdown_duration.map_or_else(
			|| self.guard_shutdown_duration() + Duration::from_secs(30),
			|secs| Duration::from_secs(secs as u64),
		)
	}

	pub fn allow_version_rollback(&self) -> bool {
		self.allow_version_rollback.unwrap_or_default()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Worker {
	/// Adjusts worker curve around this value (in millicores, i.e. 1000 = 1 core). Is not a hard limit. When
	/// unset, uses /sys/fs/cgroup/cpu.max, and if that is unset uses total host cpu.
	pub cpu_max: Option<usize>,
	/// Determine load shedding ratio based on linear mapping on cpu usage. We will gradually
	/// pull less workflows as the cpu usage increases. Units are in (permilli overall cpu usage, permilli)
	/// Default:
	///       |     .   .
	///  100% | _____   .
	///       |     .\  .
	/// % wfs |     . \ .
	///       |     .  \.
	///    5% |     .   \_____
	///       |_____.___.______
	///       0    60% 80%
	///         avg cpu usage
	load_shedding_curve: Option<[(u64, u64); 2]>,
	/// Time (in seconds) to allow for the gasoline worker engine to stop gracefully after receiving SIGTERM.
	/// Defaults to 30 seconds.
	shutdown_duration: Option<u32>,
}

impl Worker {
	pub fn load_shedding_curve(&self) -> [(u64, u64); 2] {
		self.load_shedding_curve.unwrap_or([(600, 1000), (800, 50)])
	}

	pub fn shutdown_duration(&self) -> Duration {
		Duration::from_secs(self.shutdown_duration.unwrap_or(30) as u64)
	}
}
