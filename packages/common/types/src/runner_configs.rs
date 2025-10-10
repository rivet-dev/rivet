use std::collections::HashMap;

use gas::prelude::*;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerConfig {
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

impl From<RunnerConfig> for rivet_data::generated::namespace_runner_config_v1::Data {
	fn from(value: RunnerConfig) -> Self {
		match value {
			RunnerConfig::Normal {} => {
				rivet_data::generated::namespace_runner_config_v1::Data::Normal(
					rivet_data::generated::namespace_runner_config_v1::Normal {},
				)
			}
			RunnerConfig::Serverless {
				url,
				headers,
				request_lifespan,
				slots_per_runner,
				min_runners,
				max_runners,
				runners_margin,
			} => rivet_data::generated::namespace_runner_config_v1::Data::Serverless(
				rivet_data::generated::namespace_runner_config_v1::Serverless {
					url,
					headers: headers.into(),
					request_lifespan,
					slots_per_runner,
					min_runners,
					max_runners,
					runners_margin,
				},
			),
		}
	}
}

impl From<rivet_data::generated::namespace_runner_config_v1::Data> for RunnerConfig {
	fn from(value: rivet_data::generated::namespace_runner_config_v1::Data) -> Self {
		match value {
			rivet_data::generated::namespace_runner_config_v1::Data::Normal(_) => {
				RunnerConfig::Normal {}
			}
			rivet_data::generated::namespace_runner_config_v1::Data::Serverless(o) => {
				RunnerConfig::Serverless {
					url: o.url,
					headers: o.headers.into(),
					request_lifespan: o.request_lifespan,
					slots_per_runner: o.slots_per_runner,
					min_runners: o.min_runners,
					max_runners: o.max_runners,
					runners_margin: o.runners_margin,
				}
			}
		}
	}
}

impl RunnerConfig {
	/// If updates to this run config affects the autoscaler.
	pub fn affects_autoscaler(&self) -> bool {
		match self {
			Self::Serverless { .. } => true,
			Self::Normal { .. } => false,
		}
	}
}
