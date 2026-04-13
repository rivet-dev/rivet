use gas::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct KvGetQuery {
	pub namespace: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KvGetPath {
	pub actor_id: Id,
	pub key: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[schema(as = ActorsKvGetResponse)]
#[serde(deny_unknown_fields)]
pub struct KvGetResponse {
	pub value: String,
	pub update_ts: i64,
}
