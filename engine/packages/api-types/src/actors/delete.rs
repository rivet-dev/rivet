use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct DeleteQuery {
	pub namespace: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeletePath {
	pub actor_id: Id,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[schema(as = ActorsDeleteResponse)]
pub struct DeleteResponse {}
