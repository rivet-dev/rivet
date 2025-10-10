use std::collections::HashMap;

use gas::prelude::*;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerConfig {
	Normal {},
	Serverless {
		url: String,
		headers: Option<HashMap<String, String>>,
		/// Seconds.
		request_lifespan: u32,
		slots_per_runner: u32,
		min_runners: Option<u32>,
		max_runners: u32,
		runners_margin: Option<u32>,
	},
}

impl Into<rivet_types::runner_configs::RunnerConfig> for RunnerConfig {
	fn into(self) -> rivet_types::runner_configs::RunnerConfig {
		match self {
			RunnerConfig::Normal {} => rivet_types::runner_configs::RunnerConfig::Normal {},
			RunnerConfig::Serverless {
				url,
				headers,
				request_lifespan,
				slots_per_runner,
				min_runners,
				max_runners,
				runners_margin,
			} => rivet_types::runner_configs::RunnerConfig::Serverless {
				url,
				headers: headers.unwrap_or_default(),
				request_lifespan,
				slots_per_runner,
				min_runners: min_runners.unwrap_or_default(),
				max_runners,
				runners_margin: runners_margin.unwrap_or_default(),
			},
		}
	}
}
