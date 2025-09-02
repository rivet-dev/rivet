use std::collections::HashMap;

use rivet_types::actors::ActorName;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::pagination::Pagination;

#[derive(Debug, Deserialize, Serialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListNamesQuery {
	pub namespace: String,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsListNamesResponse)]
pub struct ListNamesResponse {
	pub names: HashMap<String, ActorName>,
	pub pagination: Pagination,
}
