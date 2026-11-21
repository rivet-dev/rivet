//! Versioned wrapper for the one persisted type, the parked record.

use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2};

pub enum ParkedRecord {
	V1(v1::ParkedRecord),
	V2(v2::ParkedRecord),
}

impl OwnedVersionedData for ParkedRecord {
	type Latest = v2::ParkedRecord;

	fn wrap_latest(latest: v2::ParkedRecord) -> Self {
		ParkedRecord::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			ParkedRecord::V2(data) => Ok(data),
			ParkedRecord::V1(_) => bail!("version not latest"),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ParkedRecord::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ParkedRecord::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid parked record version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ParkedRecord::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ParkedRecord::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl ParkedRecord {
	/// A record parked before deletes existed has no delete in progress.
	fn v1_to_v2(self) -> Result<Self> {
		match self {
			ParkedRecord::V1(record) => Ok(ParkedRecord::V2(v2::ParkedRecord {
				generation: record.generation,
				cutover_ts: record.cutover_ts,
				parked_at: record.parked_at,
				v3_actor_id: record.v3_actor_id,
				workflow_id: record.workflow_id,
				workflow_name: record.workflow_name,
				wake_deadline_ts: record.wake_deadline_ts,
				wake_signals: record.wake_signals,
				delete_requested_at: None,
				deleted_at: None,
			})),
			ParkedRecord::V2(_) => bail!("unexpected parked record version"),
		}
	}

	/// A v1 record has no delete fields, so a record with a delete in progress cannot be written
	/// as v1.
	fn v2_to_v1(self) -> Result<Self> {
		match self {
			ParkedRecord::V2(record) => {
				if record.delete_requested_at.is_some() || record.deleted_at.is_some() {
					bail!("a parked record with a delete in progress has no v1 form");
				}
				Ok(ParkedRecord::V1(v1::ParkedRecord {
					generation: record.generation,
					cutover_ts: record.cutover_ts,
					parked_at: record.parked_at,
					v3_actor_id: record.v3_actor_id,
					workflow_id: record.workflow_id,
					workflow_name: record.workflow_name,
					wake_deadline_ts: record.wake_deadline_ts,
					wake_signals: record.wake_signals,
				}))
			}
			ParkedRecord::V1(_) => bail!("unexpected parked record version"),
		}
	}
}
