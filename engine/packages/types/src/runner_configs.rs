use std::collections::HashMap;

use gas::prelude::*;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RunnerConfig {
	#[serde(flatten)]
	pub kind: RunnerConfigKind,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
	#[serde(default = "default_drain_on_version_upgrade")]
	pub drain_on_version_upgrade: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerConfigKind {
	Normal {},
	Serverless {
		url: String,
		headers: HashMap<String, String>,
		/// Seconds.
		request_lifespan: u32,
		slots_per_runner: u32,
		min_runners: u32,
		max_runners: u32,
		runners_margin: u32,
	},
}

fn default_drain_on_version_upgrade() -> bool {
	false
}

impl From<RunnerConfig> for rivet_data::generated::namespace_runner_config_v3::RunnerConfig {
	fn from(value: RunnerConfig) -> Self {
		let RunnerConfig {
			kind,
			metadata,
			drain_on_version_upgrade,
		} = value;
		rivet_data::generated::namespace_runner_config_v3::RunnerConfig {
			metadata: metadata.and_then(|value| serde_json::to_string(&value).ok()),
			drain_on_version_upgrade,
			kind: match kind {
				RunnerConfigKind::Normal {} => {
					rivet_data::generated::namespace_runner_config_v3::RunnerConfigKind::Normal
				}
				RunnerConfigKind::Serverless {
					url,
					headers,
					request_lifespan,
					slots_per_runner,
					min_runners,
					max_runners,
					runners_margin,
				} => {
					rivet_data::generated::namespace_runner_config_v3::RunnerConfigKind::Serverless(
						rivet_data::generated::namespace_runner_config_v3::Serverless {
							url,
							headers: headers.into(),
							request_lifespan,
							slots_per_runner,
							min_runners,
							max_runners,
							runners_margin,
						},
					)
				}
			},
		}
	}
}

impl From<rivet_data::generated::namespace_runner_config_v3::RunnerConfig> for RunnerConfig {
	fn from(value: rivet_data::generated::namespace_runner_config_v3::RunnerConfig) -> Self {
		let rivet_data::generated::namespace_runner_config_v3::RunnerConfig {
			metadata,
			kind,
			drain_on_version_upgrade,
		} = value;
		RunnerConfig {
			metadata: metadata.and_then(|raw| serde_json::from_str(&raw).ok()),
			drain_on_version_upgrade,
			kind: match kind {
				rivet_data::generated::namespace_runner_config_v3::RunnerConfigKind::Normal => {
					RunnerConfigKind::Normal {}
				}
				rivet_data::generated::namespace_runner_config_v3::RunnerConfigKind::Serverless(
					o,
				) => RunnerConfigKind::Serverless {
					url: o.url,
					headers: o.headers.into(),
					request_lifespan: o.request_lifespan,
					slots_per_runner: o.slots_per_runner,
					min_runners: o.min_runners,
					max_runners: o.max_runners,
					runners_margin: o.runners_margin,
				},
			},
		}
	}
}

impl RunnerConfig {
	/// If updates to this run config affects the pool.
	pub fn affects_pool(&self) -> bool {
		matches!(self.kind, RunnerConfigKind::Serverless { .. })
	}
}
