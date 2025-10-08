use gas::prelude::*;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Namespace {
	pub namespace_id: Id,
	pub name: String,
	pub display_name: String,
	pub create_ts: i64,
}
