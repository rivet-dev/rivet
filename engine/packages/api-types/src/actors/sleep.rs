use gas::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct SleepQuery {
	pub namespace: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SleepPath {
	pub actor_id: Id,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsSleepRequest)]
pub struct SleepRequest {}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsSleepResponse)]
#[serde(deny_unknown_fields)]
pub struct SleepResponse {}
