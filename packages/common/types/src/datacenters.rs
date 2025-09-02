use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Datacenter {
	pub datacenter_label: u16,
	pub name: String,
	pub url: String,
}
