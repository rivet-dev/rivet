//! Checkpoint metadata shares the filesystem store's ordered mutation boundary.
//! Log intents are replayed with their original epoch. A holder logs its releases and
//! retention changes too, and they are confirmed once applied. A change made without a
//! holder flushes an applied floor before replying, so replay cannot resurrect removed
//! holds; such changes share one flush.

use super::store::{FilesystemStore, LockOperation, PlannedWrites};
use crate::tablet_maintainer_machine::state::checkpoints::logged_minimum;
use anyhow::{Result, ensure};
use bytes::Bytes;
use depot_fs_core::{keys::slatedb as keys, *};
use rivet_data::generated::{
	depot_client_protocol_v1::*, shared_depot_types_v1::VolumeLogPosition,
};
use rivet_keys::keys::{
	KeyAddress, PrefixAddress,
	codec::{AtVersion, FormalKey},
};
use slatedb::WriteBatch;
use std::{collections::BTreeMap, ops::Bound};
use uuid::Uuid;

use depot_fs_core::checkpoints::*;

fn rejected(error: CheckpointError) -> CheckpointResult {
	CheckpointResult {
		response: VolumeCheckpointResponse::CheckpointError(error),
		record: None,
	}
}
pub(super) fn address(scope: &keys::Scope, seq: u64, name: Vec<u8>) -> keys::VolumeCheckpointKey {
	keys::VolumeCheckpointKey::new(
		scope.hash,
		scope.name.clone(),
		scope.incarnation,
		0,
		seq,
		name.into(),
	)
}

fn info(key: &keys::VolumeCheckpointKey, checkpoint: FsVolumeCheckpoint) -> FsCheckpointInfo {
	FsCheckpointInfo {
		name: key.checkpoint_name().to_vec().into(),
		volume_incarnation: *key.incarnation().as_bytes(),
		position: VolumeLogPosition {
			volume_seq: key.sequence(),
			client_epoch: checkpoint.client_epoch,
		},
		checkpoint,
	}
}

impl FilesystemStore {
	pub fn with_checkpoints(mut self, config: CheckpointConfig) -> Result<Self> {
		config.validate()?;
		self.checkpoint_config = config;
		Ok(self)
	}

	async fn checkpoint_has_fork(
		&self,
		scope: &keys::Scope,
		key: &keys::VolumeCheckpointKey,
	) -> Result<bool> {
		let prefix = keys::CheckpointForkHoldsPrefix::new(
			scope.hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			key.sequence(),
			Bytes::copy_from_slice(key.checkpoint_name()),
		)
		.pack();
		Ok(self
			.db
			.scan_prefix(prefix, ..)
			.await?
			.next()
			.await?
			.is_some())
	}
	/// Streams the compact checkpoint prefix; fork-only checkpoints do not need
	/// to fit in the named cap or in a caller's memory budget.
	pub(super) async fn inspect_checkpoint(
		&self,
		scope: &keys::Scope,
		name: &[u8],
	) -> Result<CheckpointInspection> {
		let prefix =
			keys::CheckpointRowsPrefix::new(scope.hash, scope.name.clone(), scope.incarnation, 0)
				.pack();
		let mut rows = self.db.scan_prefix(prefix, ..).await?;
		let mut named_count = 0u32;
		let mut checkpoint = None;
		while let Some(row) = rows.next().await? {
			self.admit()?;
			let key = keys::VolumeCheckpointKey::unpack(&row.key)?;
			let value = key.deserialize(row.value)?;
			named_count = named_count.saturating_add(u32::from(value.named));
			if key.checkpoint_name() == name
				&& (value.named || value.retention || self.checkpoint_has_fork(scope, &key).await?)
			{
				ensure!(
					checkpoint.is_none(),
					"duplicate checkpoint name in one incarnation"
				);
				checkpoint = Some(info(&key, value));
			}
		}
		Ok(CheckpointInspection {
			volume_incarnation: *scope.incarnation.as_bytes(),
			checkpoint,
			named_count,
			max_named: self.checkpoint_config.max_named,
		})
	}

	/// A committed entry can lose a concurrent name/cap race. Confirmation
	/// resolves the authoritative checkpoint instead of acknowledging a wrong S.
	pub(super) async fn checkpoint_entries(
		&self,
		scope: &keys::Scope,
		epoch: u64,
		entries: BTreeMap<(u64, Vec<u8>), FsCheckpoint>,
		batch: &mut WriteBatch,
	) -> Result<std::collections::BTreeSet<u64>> {
		let mut added = 0;
		let mut written = std::collections::BTreeSet::new();
		let mut names = std::collections::BTreeSet::new();
		for ((seq, name), intent) in entries {
			ensure!(
				valid_name(&name) && intent.name == name,
				"invalid checkpoint log intent"
			);
			// An inactive exact-sequence row records a deletion while an older
			// open transaction still prevents the cold position passing this intent.
			if self
				.db
				.get(address(scope, seq, name.clone()).pack())
				.await?
				.is_some()
			{
				continue;
			}
			let inspection = self.inspect_checkpoint(scope, &name).await?;
			if inspection.checkpoint.is_some()
				|| !names.insert(name.clone())
				|| inspection.named_count.saturating_add(added) >= inspection.max_named
			{
				continue;
			}
			added += 1;
			let key = address(scope, seq, name);
			batch.put(
				key.pack(),
				key.serialize(AtVersion::new(
					FsVolumeCheckpoint {
						named: true,
						retention: false,
						created_at: intent.created_at,
						client_epoch: epoch,
					},
					self.row_version(),
				))?,
			);
			// Adding before the batch is conservative. A failed write fences this
			// store; no compaction can lose a version needed by an admitted pin.
			if let Some(retention) = self.catalog.lock().get_mut(scope) {
				retention.checkpoints.insert(seq);
			}
			written.insert(seq);
		}
		Ok(written)
	}

	/// A bounded metadata page includes earlier incarnations. Formal prefix seeks
	/// skip ordinary rows; the cost follows checkpoints and incarnations, not file size.
	pub(super) async fn checkpoint_page(
		&self,
		key: &depot_types::VolumeKey,
		request: &CheckpointList,
	) -> Result<VolumeCheckpointResponse> {
		let prefix = keys::VolumePrefix::new(
			depot_types::partition_key_hash(key),
			key.name.clone().map(Into::into),
		)
		.pack();
		let after = request.after.as_ref().map(|v| Bytes::copy_from_slice(v));
		if request.limit == 0
			|| request.limit > 256
			|| after.as_ref().is_some_and(|a| !a.starts_with(&prefix))
		{
			return Ok(VolumeCheckpointResponse::CheckpointError(
				CheckpointError::Invalid,
			));
		}
		let lower = after.as_ref().map_or(Bound::Unbounded, |a| {
			Bound::Excluded(a.slice(prefix.len()..))
		});
		let mut scan = self
			.db
			.scan_prefix(prefix, (lower, Bound::<Bytes>::Unbounded))
			.await?;
		let mut checkpoints = Vec::new();
		let mut after = None;
		for _ in 0..self.batch_rows {
			self.admit()?;
			let Some(row) = scan.next().await? else {
				return Ok(VolumeCheckpointResponse::CheckpointPage(CheckpointPage {
					checkpoints,
					after: None,
				}));
			};
			after = Some(row.key.to_vec());
			if let Ok(key) = keys::VolumeCheckpointKey::unpack(&row.key) {
				let value = key.deserialize(row.value)?;
				if value.named || value.retention {
					checkpoints.push(info(&key, value));
				}
				if checkpoints.len() == request.limit as usize {
					break;
				}
			} else if keys::ForkReferenceKey::unpack(&row.key).is_ok() {
				let mut end = keys::ForkReferencesPrefix::new(
					depot_types::partition_key_hash(key),
					key.name.clone().map(Into::into),
				)
				.pack()
				.to_vec();
				end.push(0xff);
				scan.seek(&end).await?;
				after = Some(end);
			} else if keys::ForkFailureKey::unpack(&row.key).is_ok() {
				let mut end = keys::ForkFailuresPrefix::new(
					depot_types::partition_key_hash(key),
					key.name.clone().map(Into::into),
				)
				.pack()
				.to_vec();
				end.push(0xff);
				scan.seek(&end).await?;
				after = Some(end);
			} else if let Some(scope) = keys::decode_scope(&row.key) {
				let checkpoint_prefix = keys::CheckpointRowsPrefix::new(
					scope.hash,
					scope.name.clone(),
					scope.incarnation,
					0,
				)
				.pack();
				let next = if row.key < checkpoint_prefix {
					checkpoint_prefix
				} else {
					let mut end =
						keys::IncarnationPrefix::new(scope.hash, scope.name, scope.incarnation)
							.pack()
							.to_vec();
					// Every owned tuple component sorts before this prefix end.
					end.push(0xff);
					Bytes::from(end)
				};
				scan.seek(&next).await?;
				after = Some(next.to_vec());
			}
		}
		Ok(VolumeCheckpointResponse::CheckpointPage(CheckpointPage {
			checkpoints,
			after: after.map(Into::into),
		}))
	}

	/// Removes only an explicitly selected flag. The last hold and its sweep
	/// marker change in the same batch; a fork hold always keeps the row alive.
	async fn update_hold(
		&self,
		scope: &keys::Scope,
		checkpoint: &FsCheckpointInfo,
		named: bool,
		retention: bool,
		current: Option<&FsVolumeRecord>,
		batch: &mut PlannedWrites,
	) -> Result<()> {
		// Only a stored record proves the incarnation retired. Without one it may be live with
		// its first record still riding the tiering pass; a deleted incarnation whose record
		// was removed already has its sweep, written in the same batch as the removal.
		let retired = current.is_some_and(|current| {
			current.volume_incarnation != *scope.incarnation.as_bytes()
				|| current.status == FsVolumeStatus::Deleted
		});
		self.update_hold_in(scope, checkpoint, named, retention, current, retired, batch)
			.await
	}

	/// `update_hold` for a caller that knows whether `scope` is retired. Only a retired
	/// incarnation is marked for sweeping when its last hold goes.
	#[allow(clippy::too_many_arguments)]
	async fn update_hold_in(
		&self,
		scope: &keys::Scope,
		checkpoint: &FsCheckpointInfo,
		named: bool,
		retention: bool,
		current: Option<&FsVolumeRecord>,
		retired: bool,
		batch: &mut PlannedWrites,
	) -> Result<()> {
		let key = address(
			scope,
			checkpoint.position.volume_seq,
			checkpoint.name.to_vec(),
		);
		let mut value = checkpoint.checkpoint.clone();
		value.named = named;
		value.retention = retention;
		let prefix = keys::CheckpointForkHoldsPrefix::new(
			scope.hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			key.sequence(),
			Bytes::copy_from_slice(key.checkpoint_name()),
		)
		.pack();
		let mut holds = self.db.scan_prefix(prefix, ..).await?;
		let held = named || retention || holds.next().await?.is_some();
		let replayable = current.is_some_and(|record| {
			let mut record = record.clone();
			self.limit_cold(scope, &mut record);
			record.volume_incarnation == *scope.incarnation.as_bytes()
				&& record.status != FsVolumeStatus::Deleted
				&& key.sequence() > record.cold.volume_seq
		});
		if held || replayable {
			batch.put(
				key.pack(),
				key.serialize(AtVersion::new(value, self.row_version()))?,
			);
		} else {
			batch.delete(key.pack());
			if retired {
				// A conservative marker is safe while other checkpoints remain:
				// sweep checks every hold again before deleting any version.
				let pending =
					keys::PendingSweepKey::new(scope.hash, scope.name.clone(), scope.incarnation);
				batch.put(
					pending.pack(),
					pending.serialize(AtVersion::new(
						FsPendingSweep {
							incarnation: *scope.incarnation.as_bytes(),
							cursor: None,
						},
						self.row_version(),
					))?,
				);
			}
		}
		Ok(())
	}

	/// Flag maintenance belongs to the same batch as the cold record. The
	/// bounded retention set is independent of arbitrarily many fork-only rows.
	pub(super) async fn retention_updates(
		&self,
		record: &FsVolumeRecord,
		batch: &mut PlannedWrites,
	) -> Result<()> {
		self.retention_updates_with(record, batch, true).await
	}

	/// `retention_updates`, adding the automatic checkpoint that falls due only with
	/// `create_due`. The apply of a logged policy passes false: automatic checkpoints are
	/// taken at the pass, at its cold position.
	async fn retention_updates_with(
		&self,
		record: &FsVolumeRecord,
		batch: &mut PlannedWrites,
		create_due: bool,
	) -> Result<()> {
		let key = depot_types::VolumeKey {
			partition_key: record.partition_key.clone(),
			name: record.name.clone(),
		};
		let scope = keys::Scope::new(&key, Uuid::from_bytes(record.volume_incarnation));
		let prefix =
			keys::CheckpointRowsPrefix::new(scope.hash, scope.name.clone(), scope.incarnation, 0)
				.pack();
		let mut rows = self.db.scan_prefix(prefix, ..).await?;
		let mut retained = BTreeMap::new();
		while let Some(row) = rows.next().await? {
			self.admit()?;
			let address = keys::VolumeCheckpointKey::unpack(&row.key)?;
			let checkpoint = info(&address, address.deserialize(row.value)?);
			if !checkpoint.checkpoint.named
				&& !checkpoint.checkpoint.retention
				&& checkpoint.position.volume_seq <= record.cold.volume_seq
			{
				self.update_hold(&scope, &checkpoint, false, false, Some(record), batch)
					.await?;
			}
			if record.status == FsVolumeStatus::Deleted {
				// A retired incarnation's named checkpoints outlive it: only a checkpoint
				// delete removes the named flag, so a restore or a delete leaves them listed,
				// forkable and restorable. Its policy stops with it, so the retention flag goes
				// and an incarnation held only by automatic checkpoints is swept.
				if checkpoint.checkpoint.retention {
					self.update_hold(
						&scope,
						&checkpoint,
						checkpoint.checkpoint.named,
						false,
						Some(record),
						batch,
					)
					.await?;
				}
			} else if checkpoint.checkpoint.retention {
				let order = (
					checkpoint.checkpoint.created_at,
					checkpoint.position.volume_seq,
					checkpoint.name.clone(),
				);
				retained.insert(order, checkpoint);
				ensure!(
					retained.len() <= MAX_RETENTION_COUNT as usize,
					"retention set exceeds its configured bound"
				);
			}
		}
		if record.status == FsVolumeStatus::Deleted {
			return Ok(());
		}
		let policy = record
			.retention
			.as_ref()
			.or(self.checkpoint_config.retention.as_ref());
		let now = self.clock.observe()?.unix_ms;
		let newest = retained
			.last_key_value()
			.map(|((created, seq, _), _)| (*created, *seq));
		// A volume unchanged since its newest retention checkpoint takes no copy of it: the
		// copy would only push an older distinct restore point out of the policy's count.
		let unchanged = newest.is_some_and(|(_, seq)| seq == record.cold.volume_seq);
		if create_due
			&& !unchanged
			&& retention_due(policy, newest.map(|(created, _)| created), now)
		{
			let name = automatic_name(now, record.cold.volume_seq);
			let address = address(&scope, record.cold.volume_seq, name.clone());
			let checkpoint = FsVolumeCheckpoint {
				named: false,
				retention: true,
				created_at: now,
				client_epoch: record.cold.client_epoch,
			};
			batch.put(
				address.pack(),
				address.serialize(AtVersion::new(checkpoint.clone(), self.row_version()))?,
			);
			retained.insert(
				(now, record.cold.volume_seq, name.into()),
				info(&address, checkpoint),
			);
		}
		let count = policy.map_or(0, |p| p.count) as usize;
		while retained.len() > count {
			let (_, checkpoint) = retained.pop_first().expect("nonempty retention set");
			self.update_hold(
				&scope,
				&checkpoint,
				checkpoint.checkpoint.named,
				false,
				Some(record),
				batch,
			)
			.await?;
		}
		Ok(())
	}

	/// Applies a logged `FsCheckpointRelease` of the volume in `scope`: clears the `named`
	/// flag of `release.name` in the incarnation it names. Replay is a no-op once the flag is
	/// clear. Only an older incarnation can be retired, because the apply admits only the
	/// live one, so only its last hold marks a sweep.
	///
	/// In the live incarnation the release at `seq` only acts on a checkpoint logged before
	/// it. A replay after a crash can see rows applied after the replayed position, and a
	/// later checkpoint of the same name must survive the replay of an earlier release.
	pub(super) async fn release_logged_checkpoint(
		&self,
		scope: &keys::Scope,
		volume_key: &depot_types::VolumeKey,
		seq: u64,
		release: &FsCheckpointRelease,
	) -> Result<()> {
		let target = keys::Scope::new(volume_key, Uuid::from_bytes(release.incarnation));
		let Some(checkpoint) = self
			.inspect_checkpoint(&target, &release.name)
			.await?
			.checkpoint
		else {
			return Ok(());
		};
		let retired = target.incarnation != scope.incarnation;
		if !checkpoint.checkpoint.named || (!retired && checkpoint.position.volume_seq >= seq) {
			return Ok(());
		}
		let current = self.record(volume_key).await?;
		let mut planned = PlannedWrites::default();
		self.update_hold_in(
			&target,
			&checkpoint,
			false,
			checkpoint.checkpoint.retention,
			current.as_ref(),
			retired,
			&mut planned,
		)
		.await?;
		let mut batch = WriteBatch::new();
		planned.apply(&mut batch);
		self.stage(batch).await
	}

	/// Applies a logged `FsRetentionSet` to the retention rows of the live incarnation in
	/// `scope`, so a listing after the apply shows the new policy's set. The record itself
	/// takes the policy in the maintainer core, and the next pass writes it. Before that first
	/// pass the store has no record of this incarnation, while a logged retention checkpoint
	/// may already have written retention rows
	/// from the maintainer's loaded record; `loaded` is that record, and the prune uses it in
	/// the stored one's place.
	pub(super) async fn apply_logged_retention(
		&self,
		scope: &keys::Scope,
		volume_key: &depot_types::VolumeKey,
		set: &FsRetentionSet,
		loaded: Option<&FsVolumeRecord>,
	) -> Result<()> {
		let incarnation = *scope.incarnation.as_bytes();
		let stored = self.incarnation_row(volume_key, incarnation).await?;
		let mut record = match stored {
			Some(stored) => stored,
			None => match loaded {
				Some(loaded) if loaded.volume_incarnation == incarnation => loaded.clone(),
				Some(_) | None => return Ok(()),
			},
		};
		if record.status == FsVolumeStatus::Deleted {
			return Ok(());
		}
		record.retention = set.policy.clone();
		let mut planned = PlannedWrites::default();
		self.retention_updates_with(&record, &mut planned, false)
			.await?;
		let mut batch = WriteBatch::new();
		planned.apply(&mut batch);
		self.stage(batch).await
	}

	/// Whether `record`'s incarnation of `key` has a pending sweep, which only a deleted or
	/// replaced incarnation has.
	async fn sweeping(
		&self,
		key: &depot_types::VolumeKey,
		record: &FsVolumeRecord,
	) -> Result<bool> {
		let scope = keys::Scope::new(key, Uuid::from_bytes(record.volume_incarnation));
		let pending = keys::PendingSweepKey::new(scope.hash, scope.name, scope.incarnation);
		Ok(self.db.get(pending.pack()).await?.is_some())
	}

	/// Called by the maintainer's one FIFO store owner with a coherent applied
	/// position. No exclusive writer is created, claimed or recovered here.
	pub async fn checkpoint_request(
		&self,
		request: VolumeCheckpointRequest,
		loaded: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
	) -> Result<CheckpointResult> {
		self.checkpoint_change(request, loaded, applied, None).await
	}

	/// `checkpoint_request` for a change whose durable record is a maintainer log op made at
	/// `created_at_unix_ms`. It writes the change to the memtable only: the op is durable in
	/// the log, and replay runs this same change at the same applied position. The volume
	/// record is not written at the applied position, because only a pass may record cold
	/// positions, and the catalog keeps the record's durable cold.
	pub async fn checkpoint_logged(
		&self,
		request: VolumeCheckpointRequest,
		loaded: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		created_at_unix_ms: i64,
	) -> Result<CheckpointResult> {
		self.checkpoint_change(request, loaded, applied, Some(created_at_unix_ms))
			.await
	}

	async fn checkpoint_change(
		&self,
		request: VolumeCheckpointRequest,
		loaded: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		logged: Option<i64>,
	) -> Result<CheckpointResult> {
		let _ordered = self
			.lock(
				super::locks::LockRequest::volume(&request.volume_key),
				LockOperation::Other,
			)
			.await;
		self.admit()?;
		if !valid_operation(&request.operation) {
			return Ok(rejected(CheckpointError::Invalid));
		}
		// Restore preserves older incarnations. Their names remain manageable
		// even after the current logical volume has completed deletion.
		if let CheckpointOperation::CheckpointList(op) = &request.operation {
			let response = self.checkpoint_page(&request.volume_key, op).await?;
			self.admit()?;
			return Ok(CheckpointResult {
				response,
				record: None,
			});
		}
		let pointer = self.pointer(&request.volume_key).await?;
		let stored = super::store::names_key(pointer.clone(), &request.volume_key);
		// A deletion's sweep removes the record and writes the incarnation's pending sweep in one
		// batch, under this volume's stripe. The maintainer may still hold that incarnation's last
		// loaded record; it must not be written back as the current one.
		let loaded = match (&stored, loaded) {
			(None, Some(loaded)) if self.sweeping(&request.volume_key, &loaded).await? => None,
			(_, loaded) => loaded,
		};
		if stored
			.as_ref()
			.or(loaded.as_ref())
			.is_none_or(|r| r.status == FsVolumeStatus::Deleted)
			&& let CheckpointOperation::CheckpointDelete(op) = &request.operation
		{
			let scope =
				keys::Scope::new(&request.volume_key, Uuid::from_bytes(op.volume_incarnation));
			if let Some(checkpoint) = self.inspect_checkpoint(&scope, &op.name).await?.checkpoint {
				// A loaded record of the target incarnation outranks a stored record of an
				// older one, which would otherwise prove the target retired.
				let proving = match (&stored, &loaded) {
					(_, Some(loaded)) if loaded.volume_incarnation == op.volume_incarnation => {
						Some(loaded)
					}
					(stored, loaded) => stored.as_ref().or(loaded.as_ref()),
				};
				let mut planned = PlannedWrites::default();
				self.update_hold(
					&scope,
					&checkpoint,
					false,
					checkpoint.checkpoint.retention,
					proving,
					&mut planned,
				)
				.await?;
				let mut batch = WriteBatch::new();
				planned.apply(&mut batch);
				self.stage(batch).await?;
				// A logged change's op is its durable record, and a holder-confirmed release
				// is durable in the holder's volume log. Only a maintainer-made change of a
				// tablet that never started its maintainer log, which state-machine tests run,
				// flushes.
				if logged.is_none() && op.minimum.is_none() {
					self.shared_flush
						.flush(&self.db, super::shared_flush::FlushCause::Checkpoint)
						.await?;
				}
			}
			self.admit()?;
			return Ok(CheckpointResult {
				response: VolumeCheckpointResponse::CheckpointUpdated,
				record: None,
			});
		}
		if let (Some(stored), Some(loaded)) = (&stored, &loaded) {
			if stored.volume_incarnation != loaded.volume_incarnation {
				return Ok(rejected(CheckpointError::IncarnationChanged));
			}
			ensure!(
				stored.cold.volume_seq
					<= applied
						.as_ref()
						.map_or(loaded.cold.volume_seq, |p| p.volume_seq),
				"checkpoint applied floor is stale"
			);
		}
		// Loaded metadata includes retry receipts for every applied entry.
		let Some(mut record) = loaded.or(stored) else {
			return Ok(rejected(CheckpointError::VolumeNotFound));
		};
		// The record as the store or the last pass holds it, before the applied position.
		let durable = record.clone();
		if record.status == FsVolumeStatus::Deleted {
			return Ok(rejected(CheckpointError::VolumeNotFound));
		}
		if let Some(applied) = applied {
			record.cold = applied;
		}
		let scope = keys::Scope::new(
			&request.volume_key,
			Uuid::from_bytes(record.volume_incarnation),
		);
		// The holder logged this change; it is done once the apply passed its position. The
		// entry stays in the log until the pass that moves cold past it, so nothing flushes.
		if let Some(minimum) = logged_minimum(&request.operation) {
			let response = if minimum.volume_incarnation != record.volume_incarnation {
				rejected(CheckpointError::IncarnationChanged)
			} else if minimum.position.volume_seq > record.cold.volume_seq {
				rejected(CheckpointError::Lagging)
			} else {
				CheckpointResult {
					response: VolumeCheckpointResponse::CheckpointUpdated,
					record: None,
				}
			};
			self.admit()?;
			return Ok(response);
		}
		let mut batch = WriteBatch::new();
		let response = match request.operation {
			CheckpointOperation::CheckpointInspect(op) => {
				if !valid_name(&op.name) {
					return Ok(rejected(CheckpointError::Invalid));
				}
				let response = self.inspect_checkpoint(&scope, &op.name).await?;
				self.admit()?;
				return Ok(CheckpointResult {
					response: VolumeCheckpointResponse::CheckpointInspection(response),
					record: None,
				});
			}
			CheckpointOperation::CheckpointList(_) => {
				unreachable!("listing handled before current-record admission")
			}
			CheckpointOperation::CheckpointCreate(op) => {
				let confirmation = op.minimum.is_some();
				let inspection = self.inspect_checkpoint(&scope, &op.name).await?;
				let now = match logged {
					Some(created_at) => created_at,
					None => self.clock.observe()?.unix_ms,
				};
				let checkpoint = match named_checkpoint(inspection, &record.cold, &op, now) {
					Ok(info) => info,
					Err(error) => return Ok(rejected(error)),
				};
				if confirmation {
					// The log quorum already makes this intent durable. Its
					// canonical name can be confirmed from completed apply
					// without forcing an SST/manifest write per checkpoint.
					self.admit()?;
					return Ok(CheckpointResult {
						response: VolumeCheckpointResponse::FsCheckpointInfo(checkpoint),
						record: None,
					});
				}
				let key = address(
					&scope,
					checkpoint.position.volume_seq,
					checkpoint.name.to_vec(),
				);
				batch.put(
					key.pack(),
					key.serialize(AtVersion::new(
						checkpoint.checkpoint.clone(),
						self.row_version(),
					))?,
				);
				VolumeCheckpointResponse::FsCheckpointInfo(checkpoint)
			}
			CheckpointOperation::CheckpointDelete(op) => {
				if op.name.is_empty() || op.name.len() > MAX_NAME_BYTES {
					return Ok(rejected(CheckpointError::Invalid));
				}
				let target =
					keys::Scope::new(&request.volume_key, Uuid::from_bytes(op.volume_incarnation));
				if let Some(checkpoint) =
					self.inspect_checkpoint(&target, &op.name).await?.checkpoint
				{
					let mut planned = PlannedWrites::default();
					self.update_hold(
						&target,
						&checkpoint,
						false,
						checkpoint.checkpoint.retention,
						Some(&record),
						&mut planned,
					)
					.await?;
					planned.apply(&mut batch);
				}
				VolumeCheckpointResponse::CheckpointUpdated
			}
			CheckpointOperation::CheckpointSetRetention(op) => {
				if !valid_policy(op.retention.as_ref()) {
					return Ok(rejected(CheckpointError::Invalid));
				}
				record.retention = op.retention;
				let mut planned = PlannedWrites::default();
				// A logged change replays at its logged position, so it takes no automatic
				// checkpoint by the wall clock; the scheduled retention op takes those.
				self.retention_updates_with(&record, &mut planned, logged.is_none())
					.await?;
				planned.apply(&mut batch);
				VolumeCheckpointResponse::CheckpointUpdated
			}
		};
		if logged.is_some() {
			// The log op orders this change against the volume's own log on replay, so the
			// record keeps its durable cold, and the catalog pins the rows it now holds.
			self.stage(batch).await?;
			self.admit()?;
			self.load_retention(&durable).await?;
			return Ok(CheckpointResult {
				response,
				record: None,
			});
		}
		// Flush the applied record with metadata changes: replay from an older
		// cold floor must never recreate an already deleted checkpoint intent.
		self.limit_cold(&scope, &mut record);
		super::store::put_record(&mut batch, pointer.as_ref(), &record, self.row_version())?;
		self.stage(batch).await?;
		// No op logged this change: its tablet never started its maintainer log, which only
		// state-machine tests run, so the reply waits for a flush.
		self.shared_flush
			.flush(&self.db, super::shared_flush::FlushCause::Checkpoint)
			.await?;
		self.admit()?;
		self.load_retention(&record).await?;
		Ok(CheckpointResult {
			response,
			record: Some(record),
		})
	}

	/// Takes a volume's due automatic checkpoint at `position`, its applied position when the
	/// maintainer decided it, and drops retention checkpoints beyond the policy's count. It
	/// writes to the memtable only; the maintainer log op that asked for it is its durable
	/// record. Nothing is taken when the newest retention checkpoint is younger than the
	/// policy's interval at `created_at_unix_ms`, so a replay takes the same one or none, and
	/// nothing is taken when the newest already stands at `position`: the volume has not
	/// changed since, and a copy would push an older distinct restore point out of the
	/// policy's count. Returns the volume seq of the newest retention checkpoint after the
	/// pass, or `None` when the pass was moot or none exists, so the maintainer can skip the
	/// next check of a volume that stays at that position.
	pub async fn retention_checkpoint(
		&self,
		record: &FsVolumeRecord,
		position: &VolumeLogPosition,
		created_at_unix_ms: i64,
	) -> Result<Option<u64>> {
		let _ordered = self
			.lock(
				super::locks::LockRequest::volume(&record.volume_key()),
				LockOperation::Other,
			)
			.await;
		self.admit()?;
		if record.status == FsVolumeStatus::Deleted {
			return Ok(None);
		}
		let key = record.volume_key();
		// A pointer naming another volume key sharing this name also replaced the incarnation.
		let stored = self.pointer(&key).await?;
		if stored
			.as_ref()
			.is_some_and(|stored| stored.volume_incarnation != record.volume_incarnation)
			|| self.sweeping(&key, record).await?
		{
			// The incarnation was replaced or is being swept; its retention is moot.
			return Ok(None);
		}
		let scope = keys::Scope::new(&key, Uuid::from_bytes(record.volume_incarnation));
		let prefix =
			keys::CheckpointRowsPrefix::new(scope.hash, scope.name.clone(), scope.incarnation, 0)
				.pack();
		let mut rows = self.db.scan_prefix(prefix, ..).await?;
		let mut retained = BTreeMap::new();
		while let Some(row) = rows.next().await? {
			self.admit()?;
			let address = keys::VolumeCheckpointKey::unpack(&row.key)?;
			let checkpoint = info(&address, address.deserialize(row.value)?);
			if checkpoint.checkpoint.retention {
				let order = (
					checkpoint.checkpoint.created_at,
					checkpoint.position.volume_seq,
					checkpoint.name.clone(),
				);
				retained.insert(order, checkpoint);
			}
		}
		let policy = record
			.retention
			.as_ref()
			.or(self.checkpoint_config.retention.as_ref());
		let newest = retained
			.last_key_value()
			.map(|((created, seq, _), _)| (*created, *seq));
		let newest_seq = newest.map(|(_, seq)| seq);
		if !retention_due(
			policy,
			newest.map(|(created, _)| created),
			created_at_unix_ms,
		) {
			return Ok(newest_seq);
		}
		if newest_seq == Some(position.volume_seq) {
			return Ok(newest_seq);
		}
		let mut planned = PlannedWrites::default();
		let name = automatic_name(created_at_unix_ms, position.volume_seq);
		let address = address(&scope, position.volume_seq, name.clone());
		let checkpoint = FsVolumeCheckpoint {
			named: false,
			retention: true,
			created_at: created_at_unix_ms,
			client_epoch: position.client_epoch,
		};
		planned.put(
			address.pack(),
			address.serialize(AtVersion::new(checkpoint.clone(), self.row_version()))?,
		);
		retained.insert(
			(created_at_unix_ms, position.volume_seq, name.into()),
			info(&address, checkpoint),
		);
		let count = policy.map_or(0, |p| p.count) as usize;
		while retained.len() > count {
			let (_, checkpoint) = retained.pop_first().expect("nonempty retention set");
			self.update_hold(
				&scope,
				&checkpoint,
				checkpoint.checkpoint.named,
				false,
				stored.as_ref().or(Some(record)),
				&mut planned,
			)
			.await?;
		}
		// Pin the new checkpoint before the write, as a logged checkpoint does.
		if let Some(retention) = self.catalog.lock().get_mut(&scope) {
			retention.checkpoints.insert(position.volume_seq);
		}
		let mut batch = WriteBatch::new();
		planned.apply(&mut batch);
		self.stage(batch).await?;
		self.admit()?;
		Ok(Some(position.volume_seq))
	}
}


