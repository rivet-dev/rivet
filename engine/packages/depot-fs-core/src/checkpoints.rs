//! Checkpoint identities, bounded policy and durable mutation results.
use crate::*;
use anyhow::{Result, ensure};
use rivet_data::generated::depot_client_protocol_v1::VolumeCheckpointResponse;

/// Values are bounded before they enter either the log or the store queue.
pub const MAX_NAME_BYTES: usize = 255;
pub const MAX_RETENTION_COUNT: u32 = 1024;
const AUTOMATIC_PREFIX: &[u8] = b"\0retention/";

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CheckpointConfig {
	pub max_named: u32,
	pub retention: Option<FsRetentionPolicy>,
}
impl Default for CheckpointConfig {
	fn default() -> Self {
		Self {
			max_named: 64,
			retention: None,
		}
	}
}
impl CheckpointConfig {
	pub fn validate(&self) -> Result<()> {
		ensure!(
			self.max_named > 0 && self.max_named <= 65536,
			"invalid named checkpoint limit"
		);
		ensure!(
			valid_policy(self.retention.as_ref()),
			"invalid checkpoint retention policy"
		);
		Ok(())
	}
}
pub fn valid_policy(policy: Option<&FsRetentionPolicy>) -> bool {
	policy.is_none_or(|p| p.count <= MAX_RETENTION_COUNT && (p.count == 0 || p.interval_ms > 0))
}
pub fn valid_name(name: &[u8]) -> bool {
	!name.is_empty()
		&& name.len() <= MAX_NAME_BYTES
		&& !name.starts_with(AUTOMATIC_PREFIX)
		&& !name.starts_with(b"\0fork/")
}

/// Durable record returned to the core only after a successful flush. The core
/// updates its loaded record from this proof before it admits another store job.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CheckpointResult {
	pub response: VolumeCheckpointResponse,
	pub record: Option<FsVolumeRecord>,
}

/// Automatic names are deterministic across a lost flush reply. The prefix is
/// reserved at the human-named API boundary, avoiding accidental name collisions.
pub fn automatic_name(now_unix_ms: i64, seq: u64) -> Vec<u8> {
	let mut name = AUTOMATIC_PREFIX.to_vec();
	name.extend_from_slice(&now_unix_ms.to_be_bytes());
	name.extend_from_slice(&seq.to_be_bytes());
	name
}

/// Wall-clock rollback never advances retention. The next interval is measured
/// from the most recent durable automatic checkpoint, including across re-grant.
pub fn retention_due(
	policy: Option<&FsRetentionPolicy>,
	newest_unix_ms: Option<i64>,
	now_unix_ms: i64,
) -> bool {
	policy.is_some_and(|p| {
		p.count > 0
			&& p.interval_ms > 0
			&& newest_unix_ms.is_none_or(|last| {
				now_unix_ms >= last
					&& (now_unix_ms as i128 - last as i128) >= i128::from(p.interval_ms)
			})
	})
}

/// Authoritative name/cap decision shared by physical and simulated storage.
/// An existing named point wins unchanged, while confirmation cannot create a
/// replacement for an intent removed after it was applied.
pub fn named_checkpoint(
	inspection: rivet_data::generated::depot_client_protocol_v1::CheckpointInspection,
	position: &rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
	request: &rivet_data::generated::depot_client_protocol_v1::CheckpointCreate,
	now_unix_ms: i64,
) -> std::result::Result<
	FsCheckpointInfo,
	rivet_data::generated::depot_client_protocol_v1::CheckpointError,
> {
	use rivet_data::generated::depot_client_protocol_v1::CheckpointError as E;
	if !valid_name(&request.name) {
		return Err(E::Invalid);
	}
	if let Some(minimum) = &request.minimum {
		if minimum.volume_incarnation != inspection.volume_incarnation {
			return Err(E::IncarnationChanged);
		}
		if minimum.position.volume_seq > position.volume_seq {
			return Err(E::Lagging);
		}
		if minimum.position.client_epoch > position.client_epoch {
			return Err(E::Invalid);
		}
	}
	match inspection.checkpoint {
		Some(info) if info.checkpoint.named => Ok(info),
		Some(_) => Err(E::NameInUse),
		None if inspection.named_count >= inspection.max_named => Err(E::TooManyNamed),
		None if request.minimum.is_some() => Err(E::NotFound),
		None => Ok(FsCheckpointInfo {
			name: request.name.clone(),
			volume_incarnation: inspection.volume_incarnation,
			position: position.clone(),
			checkpoint: FsVolumeCheckpoint {
				named: true,
				retention: false,
				created_at: now_unix_ms,
				client_epoch: position.client_epoch,
			},
		}),
	}
}

pub fn valid_operation(
	operation: &rivet_data::generated::depot_client_protocol_v1::CheckpointOperation,
) -> bool {
	use rivet_data::generated::depot_client_protocol_v1::CheckpointOperation as Op;
	match operation {
		Op::CheckpointCreate(op) => valid_name(&op.name),
		Op::CheckpointInspect(op) => valid_name(&op.name),
		Op::CheckpointDelete(op) => !op.name.is_empty() && op.name.len() <= MAX_NAME_BYTES,
		Op::CheckpointList(op) => {
			(1..=256).contains(&op.limit) && op.after.as_ref().is_none_or(|a| a.len() <= 16 * 1024)
		}
		Op::CheckpointSetRetention(op) => valid_policy(op.retention.as_ref()),
	}
}
