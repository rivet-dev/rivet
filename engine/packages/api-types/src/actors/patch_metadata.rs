use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};

use rivet_util::Id;

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct PatchMetadataQuery {
	pub namespace: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PatchMetadataPath {
	pub actor_id: Id,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsPatchMetadataRequest)]
pub struct PatchMetadataRequest {
	#[schema(value_type = Object, additional_properties = true)]
	pub metadata: HashMap<String, Option<String>>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsPatchMetadataResponse)]
pub struct PatchMetadataResponse {}
