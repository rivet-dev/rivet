use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct RescheduleQuery {
	pub namespace: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReschedulePath {
	pub actor_id: Id,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[schema(as = ActorsRescheduleResponse)]
pub struct RescheduleResponse {}
