use gas::prelude::*;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Actor {
	pub actor_id: Id,
	pub name: String,
	pub key: Option<String>,

	pub namespace_id: Id,
	pub datacenter: String,
	pub runner_name_selector: String,
	pub crash_policy: CrashPolicy,

	pub create_ts: i64,
	pub start_ts: Option<i64>,
	pub pending_allocation_ts: Option<i64>,
	pub connectable_ts: Option<i64>,
	pub sleep_ts: Option<i64>,
	pub destroy_ts: Option<i64>,
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CrashPolicy {
	Restart,
	Sleep,
	#[default]
	Destroy,
}

#[derive(Debug, Deserialize, Serialize, Hash, ToSchema)]
pub struct ActorName {
	pub metadata: serde_json::Map<String, serde_json::Value>,
}

// HACK: We can't define ToSchema on HashableMap directly, so we have to define concrete types that
// we want to be supported in OpenAPI
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StringHashableMap(pub util::serde::HashableMap<String, String>);

impl From<util::serde::HashableMap<String, String>> for StringHashableMap {
	fn from(value: util::serde::HashableMap<String, String>) -> Self {
		Self(value)
	}
}

impl Deref for StringHashableMap {
	type Target = util::serde::HashableMap<String, String>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl utoipa::ToSchema for StringHashableMap {}

impl utoipa::PartialSchema for StringHashableMap {
	fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
		utoipa::openapi::ObjectBuilder::new()
			.additional_properties(Some(
				utoipa::openapi::ObjectBuilder::new()
					.schema_type(utoipa::openapi::schema::Type::String),
			))
			.into()
	}
}
