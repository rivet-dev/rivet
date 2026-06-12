use std::collections::HashMap;

use gas::prelude::*;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RunnerConfig {
	#[serde(flatten)]
	pub kind: RunnerConfigKind,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
	/// Deprecated.
	pub drain_on_version_upgrade: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerConfigKind {
	Normal {
		drain_on_version_upgrade: Option<bool>,
		/// Seconds.
		actor_eviction_delay: Option<u32>,
		/// Seconds.
		actor_eviction_period: Option<u32>,
		/// Actors per second.
		actor_eviction_rate: Option<f32>,
	},
	Serverless {
		url: String,
		headers: Option<HashMap<String, String>>,
		/// Seconds.
		request_lifespan: u32,
		max_concurrent_actors: Option<u64>,
		/// Seconds.
		drain_grace_period: Option<u32>,
		/// Deprecated.
		slots_per_runner: Option<u32>,
		/// Deprecated.
		min_runners: Option<u32>,
		/// Deprecated.
		max_runners: Option<u32>,
		/// Deprecated.
		runners_margin: Option<u32>,
		/// Milliseconds between metadata polling. If not set, uses the global default.
		metadata_poll_interval: Option<u64>,
		drain_on_version_upgrade: Option<bool>,
		/// Seconds.
		actor_eviction_delay: Option<u32>,
		/// Seconds.
		actor_eviction_period: Option<u32>,
		/// Actors per second.
		actor_eviction_rate: Option<f32>,
	},
}

fn default_drain_on_version_upgrade() -> bool {
	true
}

impl Into<rivet_types::runner_configs::RunnerConfig> for RunnerConfig {
	fn into(self) -> rivet_types::runner_configs::RunnerConfig {
		let RunnerConfig {
			kind,
			metadata,
			drain_on_version_upgrade: root_drain_on_version_upgrade,
		} = self;
		let kind = match kind {
			RunnerConfigKind::Normal {
				drain_on_version_upgrade,
				actor_eviction_delay,
				actor_eviction_period,
				actor_eviction_rate,
			} => rivet_types::runner_configs::RunnerConfigKind::Normal {
				drain_on_version_upgrade: root_drain_on_version_upgrade
					.or(drain_on_version_upgrade)
					.unwrap_or_else(default_drain_on_version_upgrade),
				actor_eviction_delay: actor_eviction_delay.unwrap_or(0),
				actor_eviction_period: actor_eviction_period.unwrap_or(0),
				actor_eviction_rate: actor_eviction_rate.unwrap_or(1.0),
			},
			RunnerConfigKind::Serverless {
				url,
				headers,
				request_lifespan,
				max_concurrent_actors,
				drain_grace_period,
				slots_per_runner,
				min_runners,
				max_runners,
				runners_margin,
				metadata_poll_interval,
				drain_on_version_upgrade,
				actor_eviction_delay,
				actor_eviction_period,
				actor_eviction_rate,
			} => rivet_types::runner_configs::RunnerConfigKind::Serverless {
				url,
				headers: headers.unwrap_or_default(),
				request_lifespan,
				max_concurrent_actors: max_concurrent_actors
					.unwrap_or(max_runners.unwrap_or(1000) as u64),
				// Default to the runner stop window.
				drain_grace_period: drain_grace_period.unwrap_or(30 * 60),
				slots_per_runner: slots_per_runner.unwrap_or(1),
				min_runners: min_runners.unwrap_or_default(),
				max_runners: max_runners.unwrap_or(1000),
				runners_margin: runners_margin.unwrap_or_default(),
				metadata_poll_interval,
				drain_on_version_upgrade: root_drain_on_version_upgrade
					.or(drain_on_version_upgrade)
					.unwrap_or_else(default_drain_on_version_upgrade),
				actor_eviction_delay: actor_eviction_delay.unwrap_or(0),
				actor_eviction_period: actor_eviction_period.unwrap_or(0),
				actor_eviction_rate: actor_eviction_rate.unwrap_or(1.0),
			},
		};
		rivet_types::runner_configs::RunnerConfig { kind, metadata }
	}
}
