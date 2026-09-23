use rivet_auth_jwt::Id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug)]
pub enum AccessNamespaceScope {
	Any,
	Id(Id),
	Name(String),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[schema(as = AuthTargetScope)]
pub enum TargetScope {
	Any,
	Id(Id),
}
