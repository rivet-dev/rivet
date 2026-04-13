use gas::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportRequest {
	pub namespace: String,
	pub selector: ExportSelector,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportSelector {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub all: Option<bool>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub actor_names: Option<ExportActorNamesSelector>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub actor_ids: Option<ExportActorIdsSelector>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportActorNamesSelector {
	pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportActorIdsSelector {
	pub ids: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportResponse {
	pub archive_path: String,
	pub actor_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportRequest {
	pub target_namespace: String,
	pub archive_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportResponse {
	pub imported_actors: usize,
	pub skipped_actors: usize,
	pub warnings: Vec<String>,
}
