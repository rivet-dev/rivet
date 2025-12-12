use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct GetOrCreateQuery {
	pub namespace: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsGetOrCreateRequest)]
pub struct GetOrCreateRequest {
	// Ignored in api-peer
	pub datacenter: Option<String>,
	pub name: String,
	pub key: String,
	pub input: Option<String>,
	pub runner_name_selector: String,
	pub crash_policy: rivet_types::actors::CrashPolicy,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsGetOrCreateResponse)]
pub struct GetOrCreateResponse {
	pub actor: rivet_types::actors::Actor,
	pub created: bool,
}
