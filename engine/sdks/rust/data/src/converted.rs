use anyhow::*;
use gas::prelude::*;

use crate::generated::*;

pub struct RunnerAllocIdxKeyData {
	pub workflow_id: Id,
	pub remaining_slots: u32,
	pub total_slots: u32,
}

impl TryFrom<pegboard_namespace_runner_alloc_idx_v2::Data> for RunnerAllocIdxKeyData {
	type Error = anyhow::Error;

	fn try_from(value: pegboard_namespace_runner_alloc_idx_v2::Data) -> Result<Self> {
		Ok(RunnerAllocIdxKeyData {
			workflow_id: Id::from_slice(&value.workflow_id)?,
			remaining_slots: value.remaining_slots,
			total_slots: value.total_slots,
			protocol_version: value.protocol_version,
		})
	}
}

impl TryFrom<RunnerAllocIdxKeyData> for pegboard_namespace_runner_alloc_idx_v2::Data {
	type Error = anyhow::Error;

	fn try_from(value: RunnerAllocIdxKeyData) -> Result<Self> {
		Ok(pegboard_namespace_runner_alloc_idx_v2::Data {
			workflow_id: value.workflow_id.as_bytes(),
			remaining_slots: value.remaining_slots,
			total_slots: value.total_slots,
			protocol_version: value.protocol_version,
		})
	}
}

pub struct MetadataKeyData {
	pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl TryFrom<pegboard_runner_metadata_v1::Data> for MetadataKeyData {
	type Error = anyhow::Error;

	fn try_from(value: pegboard_runner_metadata_v1::Data) -> Result<Self> {
		Ok(MetadataKeyData {
			metadata: serde_json::from_str(&value.metadata)?,
		})
	}
}

impl TryFrom<MetadataKeyData> for pegboard_runner_metadata_v1::Data {
	type Error = anyhow::Error;

	fn try_from(value: MetadataKeyData) -> Result<Self> {
		Ok(pegboard_runner_metadata_v1::Data {
			metadata: serde_json::to_string(&value.metadata)?,
		})
	}
}

pub struct ActorByKeyKeyData {
	pub workflow_id: Id,
	pub is_destroyed: bool,
}

impl TryFrom<pegboard_namespace_actor_by_key_v1::Data> for ActorByKeyKeyData {
	type Error = anyhow::Error;

	fn try_from(value: pegboard_namespace_actor_by_key_v1::Data) -> Result<Self> {
		Ok(ActorByKeyKeyData {
			workflow_id: Id::from_slice(&value.workflow_id)?,
			is_destroyed: value.is_destroyed,
		})
	}
}

impl TryFrom<ActorByKeyKeyData> for pegboard_namespace_actor_by_key_v1::Data {
	type Error = anyhow::Error;

	fn try_from(value: ActorByKeyKeyData) -> Result<Self> {
		Ok(pegboard_namespace_actor_by_key_v1::Data {
			workflow_id: value.workflow_id.as_bytes(),
			is_destroyed: value.is_destroyed,
		})
	}
}

pub struct RunnerByKeyKeyData {
	pub runner_id: Id,
	pub workflow_id: Id,
}

impl TryFrom<pegboard_namespace_runner_by_key_v1::Data> for RunnerByKeyKeyData {
	type Error = anyhow::Error;

	fn try_from(value: pegboard_namespace_runner_by_key_v1::Data) -> Result<Self> {
		Ok(RunnerByKeyKeyData {
			runner_id: Id::from_slice(&value.runner_id)?,
			workflow_id: Id::from_slice(&value.workflow_id)?,
		})
	}
}

impl TryFrom<RunnerByKeyKeyData> for pegboard_namespace_runner_by_key_v1::Data {
	type Error = anyhow::Error;

	fn try_from(value: RunnerByKeyKeyData) -> Result<Self> {
		Ok(pegboard_namespace_runner_by_key_v1::Data {
			runner_id: value.runner_id.as_bytes(),
			workflow_id: value.workflow_id.as_bytes(),
		})
	}
}

#[derive(Debug)]
pub struct ActorNameKeyData {
	pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl TryFrom<pegboard_namespace_actor_name_v1::Data> for ActorNameKeyData {
	type Error = anyhow::Error;

	fn try_from(value: pegboard_namespace_actor_name_v1::Data) -> Result<Self> {
		Ok(ActorNameKeyData {
			metadata: serde_json::from_str(&value.metadata)?,
		})
	}
}

impl TryFrom<ActorNameKeyData> for pegboard_namespace_actor_name_v1::Data {
	type Error = anyhow::Error;

	fn try_from(value: ActorNameKeyData) -> Result<Self> {
		Ok(pegboard_namespace_actor_name_v1::Data {
			metadata: serde_json::to_string(&value.metadata)?,
		})
	}
}
