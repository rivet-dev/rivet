use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct CreateQuery {
	pub namespace: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsCreateRequest)]
pub struct CreateRequest {
	pub actor_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub input: Option<String>,
	pub runner_name_selector: String,
	pub crash_policy: rivet_types::actors::CrashPolicy,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsCreateResponse)]
pub struct CreateResponse {
	pub actor: rivet_types::actors::Actor,
}
