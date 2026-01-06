use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod list;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RunnerConfigResponse {
	#[serde(flatten)]
	pub config: rivet_types::runner_configs::RunnerConfig,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub runner_pool_error: Option<rivet_types::actor::RunnerPoolError>,
}
