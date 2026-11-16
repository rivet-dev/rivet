//! Store changes whose durable record is a tablet maintainer log op.
//!
//! Each op runs the same guarded store change live and in replay, writing to the memtable
//! only: the op is durable in the log, and an ordinary pass makes its rows cold.

use super::store::FilesystemStore;
use anyhow::{Context, Result};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_client_protocol_v2::{FsForkReferenceRequest, FsForkReferenceResponse},
	depot_maintainer_log_v2::{MaintainerLogRecord, MaintainerOp},
	shared_depot_fs_v2::FsVolumeRecord,
	shared_depot_types_v1::VolumeLogPosition,
};
use rivet_keys::keys::codec::{FormalKey, KeyAddress};
use std::collections::BTreeSet;

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
	/// `release_fork_reference`. A fork pin op also drops `archived_children` from the volume's
	/// earlier pins; every other op ignores them.
	pub async fn apply_maintainer_op(
		&self,
		op: MaintainerOp,
		record: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		region: u16,
		loaded_parent_incarnations: &[[u8; 16]],
		archived_children: &BTreeSet<[u8; 16]>,
	) -> Result<crate::tablet_maintainer_machine::types::ReplayOutcome> {
		use crate::tablet_maintainer_machine::types::ReplayOutcome;
		match op {
			MaintainerOp::ForkPinOp(op) => {
				// The op is the pin row: live and in replay it writes the same row at the same
				// cut, so a receipt that names the pin never points at a row replay would not
				// rebuild. The drop of archived children is the cache's, not the op's, so a
				// replay with less known drops less and the next pin or load makes up for it.
				let key = VolumeLogKey(op.volume_log_key);
				// A claim log's rows are its base volume's, so the pin is the base's.
				let (scope, _) = depot_fs_core::keys::slatedb::Scope::of_log(
					&key.0.volume_key,
					uuid::Uuid::from_bytes(key.0.volume_incarnation),
				);
				// A pin of a volume whose apply this maintainer does not hold registered, such
				// as an idle parent's, which is logged at its stored cut, is covered by the pass
				// that moved the stored cold past its cut: that pass held the pin's row, and a
				// later pin pass may have deleted it since, which a replay must not undo. An
				// incarnation with no stored record counts as covered too: a live idle pin always
				// has one, so the volume was deleted and swept, and a pin row written for it
				// would hold its catalog cut forever, since only a load's pass removes it.
				//
				// The exception is a volume the maintainer holds loaded whose record never
				// reached cold, which it sends with the row its replicas describe (cold 0): its
				// pin row was in the memtable a crash lost, so it is written again, unless the
				// store does hold a record past the cut after all.
				let root = op
					.cut
					.root_position()
					.context("a fork pin's cut names no root claim log")?
					.volume_seq;
				let covered = match &record {
					Some(record) if record.cold.volume_seq > 0 => false,
					Some(_) => self
						.incarnation_row(&key.0.volume_key, key.0.volume_incarnation)
						.await?
						.is_some_and(|stored| stored.cold.volume_seq > root),
					None => self
						.incarnation_row(&key.0.volume_key, key.0.volume_incarnation)
						.await?
						.is_none_or(|stored| stored.cold.volume_seq > root),
				};
				if !covered {
					let pin = depot_fs_core::FsForkPin {
						created_at: op.created_at_unix_ms,
						last_fork: op.last_fork,
						cut: op.cut,
						waiting: op.waiting,
					};
					self.apply_fork_pin(&scope, &pin, op.first_fork, archived_children)
						.await?;
				}
				Ok(ReplayOutcome::ForkPinned {
					key,
					last_fork: op.last_fork,
				})
			}
			MaintainerOp::CheckpointOp(op) => {
				// The outcome was the live caller's; replay only repeats the change.
				self.checkpoint_logged(op.request, record, applied, op.created_at_unix_ms)
					.await?;
				Ok(ReplayOutcome::Applied)
			}
			MaintainerOp::RetentionCheckpointOp(op) => {
				// A retention checkpoint targets the volume's one claim log.
				let Ok([target]) = <[_; 1]>::try_from(op.targets) else {
					anyhow::bail!("a retention checkpoint op targets one claim log");
				};
				let key = VolumeLogKey(target.volume_log_key);
				let incarnation = key.0.volume_incarnation;
				let record = match record {
					Some(record) => Some(record),
					None => self.incarnation_row(&key.0.volume_key, incarnation).await?,
				};
				// An incarnation without a record of its own is replaced or never stored:
				// its retention is moot, and nothing is known of its checkpoints.
				let checkpoint_seq =
					match record.filter(|record| record.volume_incarnation == incarnation) {
						Some(record) => {
							self.retention_checkpoint(&record, &target.after, op.created_at_unix_ms)
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
					.targets
					.first()
					.map(|target| target.after.clone())
					.or(applied);
				let group_acquire = match &op.request {
					FsForkReferenceRequest::FsForkAcquire(acquire) => {
						match &acquire.spec.selector {
							depot_fs_core::FsForkSelector::FsForkGroup(group) => {
								Some((acquire.spec.parent_volume_key.clone(), group.fork_number))
							}
							depot_fs_core::FsForkSelector::FsForkLatest
							| depot_fs_core::FsForkSelector::FsForkNamed(_)
							| depot_fs_core::FsForkSelector::FsForkAtTime(_) => None,
						}
					}
					FsForkReferenceRequest::FsForkRelease(_)
					| FsForkReferenceRequest::FsForkCheck(_) => None,
				};
				// A group acquire pins nothing: every pin, an idle parent's included, is its
				// own op, logged and replayed before any acquire that names it.
				let result = self
					.fork_reference_request(
						op.request,
						region,
						depot_fs_core::forks::ReferenceBarrier::Passed,
						record,
						applied,
						Some(op.created_at_unix_ms),
						loaded_parent_incarnations,
						None,
					)
					.await?;
				// The live maintainer logs an acquire only once a pin covers its fork, so a
				// replay that finds none means the pin's op is missing from the log.
				if let Some((parent_volume_key, fork_number)) = group_acquire
					&& let depot_fs_core::forks::ReferenceResult::Complete { response, .. } =
						&result && matches!(
					response.as_ref(),
					FsForkReferenceResponse::FsForkError(depot_fs_core::FsForkError::Busy)
				) {
					tracing::warn!(
						?parent_volume_key,
						fork_number,
						"a replayed group fork acquire found no pin covering its fork; the pin op it followed is missing, so the acquire's receipt is not rebuilt and its fork job asks again"
					);
				}
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
					.resume_published_fork(
						&op.spec,
						op.image,
						op.incarnation_epoch,
						op.created_at_fork,
					)
					.await?
				{
					true => ReplayOutcome::Applied,
					false => ReplayOutcome::ResumeFork(op.spec),
				},
			),
		}
	}
}
