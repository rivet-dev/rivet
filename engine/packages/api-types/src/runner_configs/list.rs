use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

use crate::pagination::Pagination;

use super::RunnerConfigResponse;

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
	pub namespace: String,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
	pub variant: Option<rivet_types::keys::namespace::runner_config::RunnerConfigVariant>,
	/// Deprecated.
	#[serde(default)]
	pub runner_names: Option<String>,
	#[serde(default)]
	pub runner_name: Vec<String>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ListPath {}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListResponse {
	pub runner_configs: HashMap<String, RunnerConfigResponse>,
	pub pagination: Pagination,
}
