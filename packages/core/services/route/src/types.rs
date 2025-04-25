use std::collections::HashMap;

use chirp_workflow::prelude::*;
use strum::FromRepr;

/// Enum representing the route target type
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum RouteTargetType {
	Actors = 0,
}

impl RouteTargetType {
	pub fn from_i64(value: i64) -> Option<Self> {
		Self::from_repr(value as usize)
	}

	pub fn as_i64(&self) -> i64 {
		*self as i64
	}
}

/// ADT enum for route target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteTarget {
	Actors {
		selector_tags: HashMap<String, String>,
	},
}

/// Database representation of a route
#[derive(Debug, Serialize, Deserialize)]
pub struct Route {
	pub route_id: Uuid,
	pub namespace_id: Uuid,
	pub name_id: String,
	pub hostname: String,
	pub path: String,
	pub route_subpaths: bool,
	pub strip_prefix: bool,
	pub target: RouteTarget,
	pub create_ts: i64,
	pub update_ts: i64,
	pub delete_ts: Option<i64>,
}
