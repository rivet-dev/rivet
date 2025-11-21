use gas::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::pagination::Pagination;

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
	pub namespace: String,
	pub name: Option<String>,
	/// Deprecated.
	#[serde(default)]
	pub runner_ids: Option<String>,
	#[serde(default)]
	pub runner_id: Vec<Id>,
	pub include_stopped: Option<bool>,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnersListResponse)]
pub struct ListResponse {
	pub runners: Vec<rivet_types::runners::Runner>,
	pub pagination: Pagination,
}
