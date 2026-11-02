use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

use crate::pagination::Pagination;

use super::RunnerPoolResponse;

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
	pub namespace: String,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
	pub variant: Option<rivet_types::keys::namespace::envoy_pool::EnvoyPoolVariant>,
	/// Deprecated.
	#[serde(default)]
	pub pool_names: Option<String>,
	#[serde(default)]
	pub pool_name: Vec<String>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ListPath {}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListResponse {
	pub runner_pools: HashMap<String, RunnerPoolResponse>,
	pub pagination: Pagination,
}
