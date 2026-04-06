use gas::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct RescheduleQuery {
	pub namespace: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReschedulePath {
	pub actor_id: Id,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsRescheduleRequest)]
pub struct RescheduleRequest {}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsRescheduleResponse)]
#[serde(deny_unknown_fields)]
pub struct RescheduleResponse {}
