use gas::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::pagination::Pagination;

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams, Default)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
	pub namespace: String,
	pub name: Option<String>,
	pub key: Option<String>,
	/// Deprecated.
	#[serde(default)]
	pub actor_ids: Option<String>,
	#[serde(default)]
	pub actor_id: Vec<Id>,
	pub include_destroyed: Option<bool>,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsListResponse)]
pub struct ListResponse {
	pub actors: Vec<rivet_types::actors::Actor>,
	pub pagination: Pagination,
}
