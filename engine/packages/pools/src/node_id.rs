use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NodeId(Uuid);

impl NodeId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}

	pub fn as_uuid(&self) -> Uuid {
		self.0
	}
}

impl fmt::Display for NodeId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl From<Uuid> for NodeId {
	fn from(value: Uuid) -> Self {
		Self(value)
	}
}

impl From<NodeId> for Uuid {
	fn from(value: NodeId) -> Self {
		value.0
	}
}
