use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::Runner;
use crate::pagination::Pagination;

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
	pub namespace: String,
	pub name: Option<String>,
	#[serde(default)]
	pub runner_key: Vec<String>,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnersListResponse)]
pub struct ListResponse {
	pub runners: Vec<Runner>,
	pub pagination: Pagination,
}
