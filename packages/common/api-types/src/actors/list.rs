use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
	pub namespace: String,
	pub name: Option<String>,
	pub key: Option<String>,
	pub actor_ids: Option<String>,
	pub include_destroyed: Option<bool>,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsListResponse)]
pub struct ListResponse {
	pub actors: Vec<rivet_types::actors::Actor>,
	pub pagination: crate::pagination::Pagination,
}
