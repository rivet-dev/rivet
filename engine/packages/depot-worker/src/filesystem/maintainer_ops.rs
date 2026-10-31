//! Store changes whose durable record is a tablet maintainer log op.
//!
//! Each op runs the same guarded store change live and in replay, writing to the memtable
//! only: the op is durable in the log, and an ordinary pass makes its rows cold.

use super::store::FilesystemStore;
use anyhow::Result;
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_maintainer_log_v1::{MaintainerLogRecord, MaintainerOp},
	shared_depot_fs_v1::FsVolumeRecord,
	shared_depot_types_v1::VolumeLogPosition,
};
use rivet_keys::keys::codec::{FormalKey, KeyAddress};

impl FilesystemStore {
	/// The tablet's record of its maintainer log, stored at the record row of the log
	/// volume `volume_key`.
	pub async fn maintainer_log_record(
		&self,
		volume_key: &depot_types::VolumeKey,
	) -> Result<Option<MaintainerLogRecord>> {
		self.admit()?;
		let key = crate::tablet_maintainer_machine::state::maintainer_log::record_key(volume_key);
		let record = self
			.db
			.get(key.pack())
			.await?
			.map(|value| key.deserialize(value))
			.transpose()?;
		self.admit()?;
		Ok(record)
	}

	/// Applies one recovered op. `record` and `applied` describe the op's volume when its
	/// apply reached the op's position. `loaded_parent_incarnations` are as for
	/// `release_fork_reference`.
	pub async fn apply_maintainer_op(
		&self,
		op: MaintainerOp,
		record: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		region: u16,
		loaded_parent_incarnations: &[[u8; 16]],
	) -> Result<crate::tablet_maintainer_machine::types::ReplayOutcome> {
		use crate::tablet_maintainer_machine::types::ReplayOutcome;
		match op {
			MaintainerOp::CheckpointOp(op) => {
				// The outcome was the live caller's; replay only repeats the change.
				self.checkpoint_logged(op.request, record, applied, op.created_at_unix_ms)
					.await?;
				Ok(ReplayOutcome::Applied)
			}
			MaintainerOp::RetentionCheckpointOp(op) => {
				let key = VolumeLogKey(op.target.volume_log_key);
				let incarnation = key.0.volume_incarnation;
				let record = match record {
					Some(record) => Some(record),
					None => self.incarnation_row(&key.0.volume_key, incarnation).await?,
				};
				// An incarnation without a record of its own is replaced or never stored:
				// its retention is moot, and nothing is known of its checkpoints.
				let checkpoint_seq = match record
					.filter(|record| record.volume_incarnation == incarnation)
				{
					Some(record) => {
						self.retention_checkpoint(&record, &op.target.after, op.created_at_unix_ms)
							.await?
					}
					None => None,
				};
				Ok(ReplayOutcome::RetentionChecked {
					key,
					checkpoint_seq,
				})
			}
			MaintainerOp::ForkReferenceOp(op) => {
				// The acquire pins the position the live op followed. A replay that finds
				// the reference already held or released answers alike and changes nothing.
				let applied = op
					.target
					.as_ref()
					.map(|target| target.after.clone())
					.or(applied);
				self.fork_reference_request(
					op.request,
					region,
					depot_fs_core::forks::ReferenceBarrier::Passed,
					record,
					applied,
					Some(op.created_at_unix_ms),
					loaded_parent_incarnations,
				)
				.await?;
				Ok(ReplayOutcome::Applied)
			}
			MaintainerOp::ForkFailOp(op) => {
				// Repeats the failure the client was told; a fork already published or
				// failed keeps its outcome.
				let _outcome = self.replay_fork_failure(&op.spec, op.error).await?;
				Ok(ReplayOutcome::Applied)
			}
			MaintainerOp::RepairAuditOp(op) => {
				// Live and in replay the row takes the op's state of the call; replaying an
				// older op of the call first is harmless, since the newer op follows it.
				self.write_repair_audit(op).await?;
				Ok(ReplayOutcome::Applied)
			}
			MaintainerOp::ForkPublishOp(op) => Ok(
				match self
					.resume_published_fork(&op.spec, op.image, op.incarnation_epoch)
					.await?
				{
					true => ReplayOutcome::Applied,
					false => ReplayOutcome::ResumeFork(op.spec),
				},
			),
		}
	}
}
