use gas::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
	pub limit: Option<usize>,
	pub cursor: Option<String>,
	pub name: Option<String>,
	/// Deprecated.
	#[serde(default)]
	pub namespace_ids: Option<String>,
	#[serde(default)]
	pub namespace_id: Vec<Id>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamespaceListResponse)]
pub struct ListResponse {
	pub namespaces: Vec<rivet_types::namespaces::Namespace>,
	pub pagination: crate::pagination::Pagination,
}
