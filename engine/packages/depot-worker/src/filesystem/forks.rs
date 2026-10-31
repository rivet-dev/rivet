//! Durable fork preparation and publication share the maintainer's storage owner.
use super::store::{FilesystemStore, LockOperation};
use anyhow::{Result, ensure};
use depot_fs_core::{
	forks::{Preparation, valid_spec},
	keys::slatedb as keys,
	*,
};
use rivet_keys::keys::{
	KeyAddress,
	codec::{AtVersion, FormalKey},
};
use slatedb::WriteBatch;
use uuid::Uuid;

type Outcome<T> = Result<std::result::Result<T, FsForkError>>;



impl FilesystemStore {
	/// Reads the pending fork row and then the volume record, as two separate reads that
	/// must not interleave with a writer of either row. The core must reserve the name
	/// before preparing a fork.
	///
	/// A shared hold is enough. Every store writer of those rows (fork preparation and
	/// publication, fork references, pending work, checkpoints, flush staging and finish,
	/// materialize and sweeps) holds the lock exclusively. The other shared holders,
	/// session opens and applies, only read the volume record and write rows of their own
	/// scope. The maintainer core also orders the rest: it runs exclusive store work
	/// beside no ordered read, holds a lookup back while an upload or cold pass writes
	/// that volume's record, and never overlaps the lookup with keyed work of the same
	/// volume. Deleted-volume sweeps delete the record outside this lock, ordered by the
	/// core alone.
	///
	/// Without `incarnation` the row is the name's pointer, the newest stored incarnation,
	/// which an open of an unloaded name reads; a pointer naming another volume key leaves
	/// `key` without a row. With it the row is that incarnation's own.
	pub async fn lifecycle_lookup(
		&self,
		key: &depot_types::VolumeKey,
		incarnation: Option<[u8; 16]>,
	) -> Result<depot_fs_core::forks::LifecycleLookup> {
		let _running = self.enter_volumes([(
			super::locks::StoreVolume::from(key),
			super::locks::VolumeAccess::Lookup,
		)]);
		let _ordered = self
			.lock_core_ordered(
				super::locks::LockRequest::shared().read(super::locks::stripe(key)),
				LockOperation::LifecycleLookup,
			)
			.await;
		self.admit()?;
		let pending = self.pending_fork(key).await?.map(Box::new);
		let pointer = self.pointer(key).await?;
		let (row, superseded) = match incarnation {
			None => (super::store::names_key(pointer, key), false),
			// The pointer is a copy of the row of the incarnation it names, so only a lookup
			// of another incarnation reads its own row. Only a stored own record can show the
			// pointer's incarnation is newer: without one this incarnation's epoch is unknown.
			// The pointer may name another volume key sharing this name, such as a recreated
			// actor's database; it is then another incarnation of the name, and epochs order
			// every incarnation of a name whatever its volume key.
			Some(incarnation) => match super::store::names_key(pointer.clone(), key) {
				Some(own) if own.volume_incarnation == incarnation => (Some(own), false),
				_ => {
					let own = self.incarnation_row(key, incarnation).await?;
					let superseded = pointer.zip(own.as_ref()).is_some_and(|(pointer, own)| {
						pointer.incarnation_epoch > own.incarnation_epoch
					});
					(own, superseded)
				}
			},
		};
		self.admit()?;
		Ok(depot_fs_core::forks::LifecycleLookup {
			row,
			pending,
			superseded,
		})
	}
	/// Reads what an operator inspection of `key` shows from the store: the pointer row, the
	/// pending fork row and the own record row of each of `incarnations`. It takes no lock and
	/// is not ordered against store writes, so it may observe a record a change in flight is
	/// about to replace; an inspection only informs, and every repair call is fenced on its own.
	pub async fn repair_records(
		&self,
		key: &depot_types::VolumeKey,
		incarnations: &[[u8; 16]],
	) -> Result<crate::tablet_maintainer_machine::types::RepairRecords> {
		self.admit()?;
		let pointer = self.record(key).await?;
		let pending_fork = self.pending_fork(key).await?;
		let mut rows = Vec::with_capacity(incarnations.len());
		for incarnation in incarnations {
			rows.push((*incarnation, self.incarnation_row(key, *incarnation).await?));
		}
		self.admit()?;
		Ok(crate::tablet_maintainer_machine::types::RepairRecords {
			pointer,
			pending_fork,
			incarnations: rows,
		})
	}

	/// Exact lookup for lifecycle admission before background recovery has
	/// reached the pending-work page containing this logical volume.
	pub async fn pending_fork(
		&self,
		key: &depot_types::VolumeKey,
	) -> Result<Option<FsPendingFork>> {
		self.admit()?;
		let address = keys::PendingForkKey::new(
			depot_types::partition_key_hash(key),
			key.name.clone().map(Into::into),
		);
		let value = self
			.db
			.get(address.pack())
			.await?
			.map(|v| address.deserialize(v))
			.transpose()?;
		ensure!(
			value
				.as_ref()
				.is_none_or(|v| v.spec.destination_volume_key == *key),
			"fork preparation address mismatch"
		);
		self.admit()?;
		Ok(value)
	}

	async fn completed_fork(
		&self,
		spec: &FsForkSpec,
	) -> Result<Option<std::result::Result<FsVolumeRecord, FsForkError>>> {
		let destination = &spec.destination_volume_key;
		let hash = depot_types::partition_key_hash(destination);
		let failure = keys::ForkFailureKey::new(
			hash,
			destination.name.clone().map(Into::into),
			Uuid::from_bytes(spec.operation_id),
		);
		if let Some(value) = self.db.get(failure.pack()).await? {
			let failed = failure.deserialize(value)?;
			self.admit()?;
			return Ok(Some(Err(if failed.spec == *spec {
				failed.error
			} else {
				FsForkError::OperationConflict
			})));
		}
		let archive = keys::IncarnationRecordKey::new(
			hash,
			destination.name.clone().map(Into::into),
			Uuid::from_bytes(spec.operation_id),
		);
		if let Some(value) = self.db.get(archive.pack()).await? {
			let archived = archive.deserialize(value)?;
			ensure!(
				archived.record.volume_key() == *destination
					&& archived.record.volume_incarnation == spec.operation_id,
				"fork archive address mismatch"
			);
			let Some(receipt) = archived.creation else {
				return Ok(Some(Err(FsForkError::OperationConflict)));
			};
			if receipt.spec != *spec {
				return Ok(Some(Err(FsForkError::OperationConflict)));
			}
			let own = self.incarnation_row(destination, spec.operation_id).await?;
			self.admit()?;
			return Ok(Some(match own {
				Some(row) if row.status != FsVolumeStatus::Deleted => Ok(row),
				_ => Err(FsForkError::ForkDeleted),
			}));
		}
		Ok(None)
	}

	/// Persist a terminal negative outcome before releasing its name or source.
	/// Already published operations keep their success; request expiry alone
	/// never calls this operation.
	pub async fn fail_fork(&self, spec: &FsForkSpec, error: FsForkError) -> Outcome<Preparation> {
		let _ordered = self
			.lock(super::locks::LockRequest::fork(spec), LockOperation::Other)
			.await;
		self.admit()?;
		if let Some(completed) = self.completed_fork(spec).await? {
			return Ok(completed.map(Preparation::Complete));
		}
		let Some(pending) = self.pending_fork(&spec.destination_volume_key).await? else {
			return Ok(Err(FsForkError::ForkDeleted));
		};
		if pending.spec != *spec {
			return Ok(Err(FsForkError::OperationConflict));
		}
		self.write_fork_failure(spec, error, true).await
	}

	/// Replays a logged fork failure. The client was told the fork failed, so the failure is
	/// written even when the pending row was lost with an earlier lifetime's memtable;
	/// otherwise a retry of the same operation could prepare and publish it.
	pub async fn replay_fork_failure(
		&self,
		spec: &FsForkSpec,
		error: FsForkError,
	) -> Outcome<Preparation> {
		let _ordered = self
			.lock(super::locks::LockRequest::fork(spec), LockOperation::Other)
			.await;
		self.admit()?;
		if let Some(completed) = self.completed_fork(spec).await? {
			return Ok(completed.map(Preparation::Complete));
		}
		let pending = self.pending_fork(&spec.destination_volume_key).await?;
		let owns_pending = match pending {
			Some(pending) if pending.spec != *spec => false,
			Some(_) => true,
			None => false,
		};
		self.write_fork_failure(spec, error, owns_pending).await
	}

	/// Writes `spec`'s failure with the release of its source hold and the sweep of its rows,
	/// deleting the destination's pending fork row when it belongs to `spec`.
	async fn write_fork_failure(
		&self,
		spec: &FsForkSpec,
		error: FsForkError,
		delete_pending: bool,
	) -> Outcome<Preparation> {
		let scope = keys::Scope::new(
			&spec.destination_volume_key,
			Uuid::from_bytes(spec.operation_id),
		);
		let failure = keys::ForkFailureKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		let release =
			keys::PendingForkReleaseKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		let sweep = keys::PendingSweepKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		let mut batch = WriteBatch::new();
		batch.put(
			failure.pack(),
			failure.serialize(AtVersion::new(
				FsForkFailure {
					spec: spec.clone(),
					error: error.clone(),
				},
				self.row_version(),
			))?,
		);
		batch.put(
			release.pack(),
			release.serialize(AtVersion::new(
				FsPendingForkRelease { spec: spec.clone() },
				self.row_version(),
			))?,
		);
		batch.put(
			sweep.pack(),
			sweep.serialize(AtVersion::new(
				FsPendingSweep {
					incarnation: spec.operation_id,
					cursor: None,
				},
				self.row_version(),
			))?,
		);
		if delete_pending {
			batch.delete(keys::PendingForkKey::new(scope.hash, scope.name).pack());
		}
		self.stage(batch).await?;
		// The preparation rides the tablet's ordinary pass. A crash before it loses only
		// an unpublished fork's progress: the caller retries the same operation, and the
		// source's pending check releases a hold nobody claims.
		self.admit()?;
		Ok(Err(error))
	}

	/// A source hold is acquired only after this identity is durable. Losing the
	/// reply leaves the identical operation available to retry or background recovery.
	pub async fn prepare_fork(&self, spec: FsForkSpec, region: u16) -> Outcome<Preparation> {
		self.prepare_fork_over(spec, region, None).await
	}

	/// The record a restore replaces: the restored incarnation's own record, or else the
	/// record the maintainer registered for an incarnation whose first record has not reached
	/// the store yet. That record rides the tablet's ordinary pass, and a restore never asks
	/// for one. A pointer naming a newer incarnation is returned instead, which the caller
	/// refuses as a changed incarnation. A fork that is not a restore gets the pointer.
	async fn replaced_record(
		&self,
		spec: &FsForkSpec,
		replaced: Option<&FsVolumeRecord>,
	) -> Result<Option<FsVolumeRecord>> {
		let key = &spec.destination_volume_key;
		let pointer = self.record(key).await?;
		let Some(restore) = spec.restore else {
			return Ok(pointer);
		};
		let own = match &pointer {
			Some(pointer) if pointer.volume_incarnation == restore => Some(pointer.clone()),
			Some(_) | None => self.incarnation_row(key, restore).await?,
		};
		let target_epoch = own
			.as_ref()
			.or(replaced)
			.map(|record| record.incarnation_epoch);
		if let Some(pointer) = pointer
			&& pointer.volume_incarnation != restore
			&& target_epoch.is_none_or(|epoch| pointer.incarnation_epoch > epoch)
		{
			return Ok(Some(pointer));
		}
		if own.is_some() {
			return Ok(own);
		}
		Ok(replaced
			.filter(|record| {
				record.volume_key() == *key
					&& Some(record.volume_incarnation) == spec.restore
					&& record.status == FsVolumeStatus::Live
			})
			.cloned())
	}

	async fn prepare_fork_over(
		&self,
		spec: FsForkSpec,
		region: u16,
		replaced: Option<FsVolumeRecord>,
	) -> Outcome<Preparation> {
		let _ordered = self
			.lock(super::locks::LockRequest::fork(&spec), LockOperation::Other)
			.await;
		self.admit()?;
		if !valid_spec(&spec, region) {
			return Ok(Err(FsForkError::Invalid));
		}
		let destination = &spec.destination_volume_key;
		let hash = depot_types::partition_key_hash(destination);
		if let Some(completed) = self.completed_fork(&spec).await? {
			return Ok(completed.map(Preparation::Complete));
		}
		if let Some(pending) = self.pending_fork(destination).await? {
			self.admit()?;
			return Ok(if pending.spec == spec {
				Ok(Preparation::Pending(Box::new(pending)))
			} else if pending.spec.operation_id == spec.operation_id {
				Err(FsForkError::OperationConflict)
			} else {
				Err(FsForkError::Busy)
			});
		}
		let current = self.replaced_record(&spec, replaced.as_ref()).await?;
		match (spec.restore, current) {
			(None, Some(_)) => return Ok(Err(FsForkError::DestinationExists)),
			(Some(expected), Some(row))
				if row.volume_incarnation == expected && row.status == FsVolumeStatus::Live => {}
			(Some(_), _) => return Ok(Err(FsForkError::IncarnationChanged)),
			(None, None) => {}
		}
		let pending = FsPendingFork {
			spec,
			preparation: FsForkPreparation::FsForkAcquirePending,
		};
		let address = keys::PendingForkKey::new(
			hash,
			pending
				.spec
				.destination_volume_key
				.name
				.clone()
				.map(Into::into),
		);
		let mut batch = WriteBatch::new();
		batch.put(
			address.pack(),
			address.serialize(AtVersion::new(pending.clone(), self.row_version()))?,
		);
		self.stage(batch).await?;
		// The preparation rides the tablet's ordinary pass. A crash before it loses only
		// an unpublished fork's progress: the caller retries the same operation, and the
		// source's pending check releases a hold nobody claims.
		self.admit()?;
		Ok(Ok(Preparation::Pending(Box::new(pending))))
	}
}

impl FilesystemStore {
	/// Persist the exact source result before reading any metadata pages. A
	/// recovery resumes this image rather than resolving Latest again.
	pub async fn remember_fork_reference(&self, receipt: FsForkReceipt) -> Outcome<Preparation> {
		self.remember_fork_reference_over(receipt, None).await
	}

	async fn remember_fork_reference_over(
		&self,
		receipt: FsForkReceipt,
		replaced: Option<FsVolumeRecord>,
	) -> Outcome<Preparation> {
		let _ordered = self
			.lock(
				super::locks::LockRequest::fork(&receipt.spec),
				LockOperation::Other,
			)
			.await;
		self.admit()?;
		let spec = &receipt.spec;
		if let Some(completed) = self.completed_fork(spec).await? {
			return Ok(completed.map(Preparation::Complete));
		}
		let Some(mut pending) = self.pending_fork(&spec.destination_volume_key).await? else {
			return Ok(Err(FsForkError::ForkDeleted));
		};
		if pending.spec != *spec {
			return Ok(Err(FsForkError::OperationConflict));
		}
		if receipt.image.link.volume_key != spec.parent_volume_key
			|| receipt.image.link.region != Some(spec.parent_region)
			|| receipt.image.parent_base.len() >= MAX_FORK_CHAIN_DEPTH
		{
			return Ok(Err(FsForkError::Invalid));
		}
		let mut identities = std::collections::BTreeSet::new();
		for link in std::iter::once(&receipt.image.link).chain(&receipt.image.parent_base) {
			let Some(region) = link.region else {
				return Ok(Err(FsForkError::Invalid));
			};
			if (region == spec.destination_region
				&& link.volume_key == spec.destination_volume_key
				&& link.incarnation == spec.operation_id)
				|| !identities.insert((region, link.volume_key.clone(), link.incarnation))
			{
				return Ok(Err(FsForkError::Invalid));
			}
		}
		if let Some(expected) = spec.restore {
			let Some(current) = self.replaced_record(spec, replaced.as_ref()).await? else {
				return Ok(Err(FsForkError::IncarnationChanged));
			};
			if current.volume_incarnation != expected || current.status != FsVolumeStatus::Live {
				return Ok(Err(FsForkError::IncarnationChanged));
			}
			let target = &receipt.image.link;
			let own = target.region == Some(spec.destination_region)
				&& target.volume_key == spec.destination_volume_key;
			let reachable = own
				|| current.base_chain.iter().any(|link| {
					link.volume_key == target.volume_key
						&& link.incarnation == target.incarnation
						&& link.region.unwrap_or(spec.destination_region) == spec.parent_region
						&& target.position.volume_seq <= link.position.volume_seq
				});
			if !reachable {
				return Ok(Err(FsForkError::CheckpointNotReachable));
			}
		}
		match &pending.preparation {
			FsForkPreparation::FsForkAcquirePending => {}
			FsForkPreparation::FsForkCopying(copy) if copy.image == receipt.image => {
				return Ok(Ok(Preparation::Pending(Box::new(pending))));
			}
			FsForkPreparation::FsForkReady(ready) if ready.image == receipt.image => {
				return Ok(Ok(Preparation::Pending(Box::new(pending))));
			}
			_ => return Ok(Err(FsForkError::OperationConflict)),
		}
		pending.preparation = FsForkPreparation::FsForkCopying(FsForkCopying {
			image: receipt.image,
			cursor: None,
		});
		self.write_fork_preparation(&pending).await?;
		Ok(Ok(Preparation::Pending(Box::new(pending))))
	}

	async fn write_fork_preparation(&self, pending: &FsPendingFork) -> Result<()> {
		let key = &pending.spec.destination_volume_key;
		let address = keys::PendingForkKey::new(
			depot_types::partition_key_hash(key),
			key.name.clone().map(Into::into),
		);
		let mut batch = WriteBatch::new();
		batch.put(
			address.pack(),
			address.serialize(AtVersion::new(pending.clone(), self.row_version()))?,
		);
		self.stage(batch).await?;
		// The preparation rides the tablet's ordinary pass. A crash before it loses only
		// an unpublished fork's progress: the caller retries the same operation, and the
		// source's pending check releases a hold nobody claims.
		self.admit()
	}

	/// Metadata pages are invisible until publication. The cursor and rows are
	/// one durable batch; a duplicate page observes the already advanced cursor.
	pub async fn copy_fork_metadata(
		&self,
		spec: &FsForkSpec,
		after: Option<Vec<u8>>,
		rows: Vec<(depot_fs_core::store::RowKey, depot_fs_core::store::Row)>,
		complete: bool,
	) -> Outcome<Preparation> {
		use depot_fs_core::store::{Row, RowKey};
		let _ordered = self
			.lock(super::locks::LockRequest::fork(spec), LockOperation::Other)
			.await;
		self.admit()?;
		let Some(mut pending) = self.pending_fork(&spec.destination_volume_key).await? else {
			return Ok(Err(FsForkError::ForkDeleted));
		};
		if pending.spec != *spec {
			return Ok(Err(FsForkError::OperationConflict));
		}
		let copy = match &pending.preparation {
			FsForkPreparation::FsForkCopying(copy) => copy.clone(),
			FsForkPreparation::FsForkReady(_) => {
				return Ok(Ok(Preparation::Pending(Box::new(pending))));
			}
			_ => return Ok(Err(FsForkError::Busy)),
		};
		if copy.cursor.as_deref() != after.as_deref() {
			return Ok(Ok(Preparation::Pending(Box::new(pending))));
		}
		if rows.len() > self.batch_rows || (rows.is_empty() && !complete) {
			return Ok(Err(FsForkError::Invalid));
		}
		let source = keys::Scope::new(
			&spec.parent_volume_key,
			Uuid::from_bytes(copy.image.link.incarnation),
		);
		let target = keys::Scope::new(
			&spec.destination_volume_key,
			Uuid::from_bytes(spec.operation_id),
		);
		let mut previous = after;
		let mut batch = WriteBatch::new();
		let mut bytes = 0usize;
		for (key, mut row) in rows {
			if matches!(key, RowKey::Chunk(..)) {
				return Ok(Err(FsForkError::Invalid));
			}
			let cursor = keys::encode_row(&source, &key, 0, None, self.row_version())?
				.0
				.to_vec();
			if previous.as_ref().is_some_and(|old| old >= &cursor) {
				return Ok(Err(FsForkError::Invalid));
			}
			if let Row::Inode(inode) = &mut row {
				*inode = depot_fs_core::forks::copy_inode(inode.clone());
			}
			let (address, value) =
				keys::encode_row(&target, &key, 0, Some(row), self.row_version())?;
			bytes = bytes
				.saturating_add(address.len())
				.saturating_add(value.len());
			if bytes > self.config.max_read_bytes {
				return Ok(Err(FsForkError::Invalid));
			}
			batch.put(address, value);
			previous = Some(cursor);
		}
		pending.preparation = if complete {
			FsForkPreparation::FsForkReady(FsForkReady { image: copy.image })
		} else {
			FsForkPreparation::FsForkCopying(FsForkCopying {
				image: copy.image,
				cursor: previous.map(Into::into),
			})
		};
		let address = keys::PendingForkKey::new(target.hash, target.name.clone());
		batch.put(
			address.pack(),
			address.serialize(AtVersion::new(pending.clone(), self.row_version()))?,
		);
		self.stage(batch).await?;
		// The preparation rides the tablet's ordinary pass. A crash before it loses only
		// an unpublished fork's progress: the caller retries the same operation, and the
		// source's pending check releases a hold nobody claims.
		self.admit()?;
		Ok(Ok(Preparation::Pending(Box::new(pending))))
	}

	/// The owning core supplies the retired incarnation only after its old log
	/// writer has been claimed. A recovered Ready preparation must claim again.
	pub async fn publish_fork(
		&self,
		spec: &FsForkSpec,
		claimed: Option<[u8; 16]>,
		flush: bool,
		incarnation_epoch: u64,
	) -> Outcome<FsVolumeRecord> {
		self.publish_fork_over(spec, claimed, flush, None, incarnation_epoch)
			.await
	}

	/// Publishes like `publish_fork`. A restore of an incarnation whose first record has not
	/// reached the store yet replaces `replaced`, the record the maintainer registered for it.
	async fn publish_fork_over(
		&self,
		spec: &FsForkSpec,
		claimed: Option<[u8; 16]>,
		flush: bool,
		replaced: Option<FsVolumeRecord>,
		incarnation_epoch: u64,
	) -> Outcome<FsVolumeRecord> {
		let _ordered = self
			.lock(super::locks::LockRequest::fork(spec), LockOperation::Other)
			.await;
		self.admit()?;
		if let Some(completed) = self.completed_fork(spec).await? {
			return Ok(completed);
		}
		if claimed != spec.restore {
			return Ok(Err(FsForkError::Busy));
		}
		let Some(pending) = self.pending_fork(&spec.destination_volume_key).await? else {
			return Ok(Err(FsForkError::ForkDeleted));
		};
		if pending.spec != *spec {
			return Ok(Err(FsForkError::OperationConflict));
		}
		let FsForkPreparation::FsForkReady(ready) = pending.preparation else {
			return Ok(Err(FsForkError::Busy));
		};
		let old = self.replaced_record(spec, replaced.as_ref()).await?;
		match (&old, spec.restore) {
			(None, None) => {}
			(Some(row), Some(inc))
				if row.volume_incarnation == inc && row.status == FsVolumeStatus::Live => {}
			_ => return Ok(Err(FsForkError::IncarnationChanged)),
		}
		let hash = depot_types::partition_key_hash(&spec.destination_volume_key);
		let name = spec.destination_volume_key.name.clone();
		let mut base_chain = vec![ready.image.link.clone()];
		base_chain.extend(ready.image.parent_base.clone());
		let record = FsVolumeRecord {
			incarnation_epoch,
			recent_writes: Vec::new(),
			partition_key: spec.destination_volume_key.partition_key.clone(),
			name: name.clone(),
			volume_incarnation: spec.operation_id,
			status: FsVolumeStatus::Live,
			cold: depot_fs_core::forks::initial_position(&ready.image.link.position)?,
			base_chain,
			retention: old.as_ref().and_then(|r| r.retention.clone()),
		};
		let receipt = FsForkReceipt {
			spec: spec.clone(),
			image: ready.image,
		};
		let pointer = self.pointer(&spec.destination_volume_key).await?;
		let mut batch = WriteBatch::new();
		if let Some(old) = old {
			ensure!(
				old.incarnation_epoch < incarnation_epoch,
				"a restore's incarnation epoch must exceed the one it replaces"
			);
			let mut retired = old.clone();
			retired.status = FsVolumeStatus::Deleted;
			let mut projection = super::store::PlannedWrites::default();
			self.plan_alarm_index(&retired, &mut projection).await?;
			projection.apply(&mut batch);
			// The replaced incarnation's own row says Deleted, so it is retired however its
			// cleanup later finds it; the pointer moves to the new incarnation below.
			let retired_row = super::store::incarnation_record_key(&retired);
			batch.put(
				retired_row.pack(),
				retired_row.serialize(AtVersion::new(retired.clone(), self.row_version()))?,
			);
			let incarnation = Uuid::from_bytes(old.volume_incarnation);
			let archive =
				keys::IncarnationRecordKey::new(hash, name.clone().map(Into::into), incarnation);
			let creation = self
				.db
				.get(archive.pack())
				.await?
				.map(|v| archive.deserialize(v))
				.transpose()?
				.and_then(|v| v.creation);
			batch.put(
				archive.pack(),
				archive.serialize(AtVersion::new(
					FsIncarnationRecord {
						record: old.clone(),
						creation,
					},
					self.row_version(),
				))?,
			);
			let sweep = keys::PendingSweepKey::new(hash, name.clone().map(Into::into), incarnation);
			batch.put(
				sweep.pack(),
				sweep.serialize(AtVersion::new(
					FsPendingSweep {
						incarnation: old.volume_incarnation,
						cursor: None,
					},
					self.row_version(),
				))?,
			);
		}
		let incarnation = Uuid::from_bytes(spec.operation_id);
		let archive =
			keys::IncarnationRecordKey::new(hash, name.clone().map(Into::into), incarnation);
		batch.put(
			archive.pack(),
			archive.serialize(AtVersion::new(
				FsIncarnationRecord {
					record: record.clone(),
					creation: Some(receipt),
				},
				self.row_version(),
			))?,
		);
		let alarm_key = depot_types::VolumeLogKey::from(
			rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
				volume_key: spec.destination_volume_key.clone(),
				volume_incarnation: spec.operation_id,
			},
		);
		let summary = self.alarm_at(&alarm_key, record.cold.clone()).await?;
		let mut projection = super::store::PlannedWrites::default();
		self.put_alarm_projection(
			&alarm_key,
			None,
			&rivet_data::generated::depot_alarm_v1::AlarmCurrent {
				summary,
				retry: None,
			},
			&mut projection,
		)?;
		projection.apply(&mut batch);
		super::store::put_record(&mut batch, pointer.as_ref(), &record, self.row_version())?;
		batch.delete(keys::PendingForkKey::new(hash, name.clone().map(Into::into)).pack());
		if spec.parent_region != spec.destination_region {
			let pending = keys::PendingMaterializeKey::new(hash, name.map(Into::into), incarnation);
			batch.put(
				pending.pack(),
				pending.serialize(AtVersion::new(
					FsPendingMaterialize {
						incarnation: spec.operation_id,
						cursor: FsMaterializeCursor {
							complete_image: false,
							layer: 0,
							after: None,
						},
					},
					self.row_version(),
				))?,
			);
		}
		self.stage(batch).await?;
		// A logged publication's op is its durable record. Only a publication whose tablet
		// never started its maintainer log, which state-machine tests run, flushes.
		if flush {
			self.shared_flush
				.flush(&self.db, super::shared_flush::FlushCause::Fork)
				.await?;
		}
		self.admit()?;
		self.load_retention(&record).await?;
		Ok(Ok(record))
	}
}

impl FilesystemStore {
	/// Replays a logged publication. Returns whether the destination is published: either the
	/// publication reached the store before the move, or the preparation it finds is ready,
	/// which a durable prefix of the store only holds with every copied page before it. Any
	/// other state may lack pages, so the fork is prepared again with the logged image, from
	/// its durable copy cursor if one survived, and the caller copies and publishes again.
	/// A publication whose fork was failed after it was logged is not rebuilt: the client
	/// was told it failed, so the op is done.
	///
	/// A restore is never published here: its job drains the replaced incarnation again, so
	/// every entry that incarnation's claimed log holds is applied before it is replaced.
	pub async fn resume_published_fork(
		&self,
		spec: &FsForkSpec,
		image: FsForkImage,
		incarnation_epoch: u64,
	) -> Result<bool> {
		let _ordered = self
			.lock(super::locks::LockRequest::fork(spec), LockOperation::Other)
			.await;
		self.admit()?;
		if self.completed_fork(spec).await?.is_some() {
			return Ok(true);
		}
		let scope = keys::Scope::new(
			&spec.destination_volume_key,
			Uuid::from_bytes(spec.operation_id),
		);
		let failure = keys::ForkFailureKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		if let Some(value) = self.db.get(failure.pack()).await? {
			let failure = failure.deserialize(value)?;
			if failure.spec == *spec {
				return Ok(true);
			}
		}
		let pending = self.pending_fork(&spec.destination_volume_key).await?;
		let preparation = match pending {
			Some(pending) if pending.spec != *spec => {
				anyhow::bail!("a logged fork publication found another fork preparing its name")
			}
			Some(FsPendingFork {
				preparation: FsForkPreparation::FsForkReady(ready),
				..
			}) if ready.image == image => {
				if spec.restore.is_some() {
					return Ok(false);
				}
				drop(_ordered);
				return Ok(self
					.publish_fork(spec, None, false, incarnation_epoch)
					.await?
					.is_ok());
			}
			Some(FsPendingFork {
				preparation: FsForkPreparation::FsForkCopying(copy),
				..
			}) if copy.image == image => return Ok(false),
			Some(_) | None => FsForkPreparation::FsForkCopying(FsForkCopying {
				image,
				cursor: None,
			}),
		};
		self.write_fork_preparation(&FsPendingFork {
			spec: spec.clone(),
			preparation,
		})
		.await?;
		Ok(false)
	}
}

impl FilesystemStore {
	/// A source check cannot confuse an unfinished durable create with absence.
	/// Retired incarnations remain present until their sweep/release work ends.
	pub async fn check_fork(&self, spec: &FsForkSpec, region: u16) -> Outcome<FsForkPresence> {
		let _ordered = self
			.lock(super::locks::LockRequest::fork(spec), LockOperation::Other)
			.await;
		self.admit()?;
		if !valid_spec(spec, region) {
			return Ok(Err(FsForkError::Invalid));
		}
		let key = &spec.destination_volume_key;
		if let Some(pending) = self.pending_fork(key).await?
			&& pending.spec.operation_id == spec.operation_id
		{
			return Ok(if pending.spec == *spec {
				Ok(FsForkPresence::Preparing)
			} else {
				Err(FsForkError::OperationConflict)
			});
		}
		let scope = keys::Scope::new(key, Uuid::from_bytes(spec.operation_id));
		let failure = keys::ForkFailureKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		if let Some(value) = self.db.get(failure.pack()).await? {
			let failure = failure.deserialize(value)?;
			self.admit()?;
			return Ok(if failure.spec == *spec {
				Ok(FsForkPresence::Absent)
			} else {
				Err(FsForkError::OperationConflict)
			});
		}
		let archive =
			keys::IncarnationRecordKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		let Some(value) = self.db.get(archive.pack()).await? else {
			self.admit()?;
			return Ok(Ok(FsForkPresence::Absent));
		};
		let record = archive.deserialize(value)?;
		if record.creation.as_ref().is_none_or(|r| r.spec != *spec) {
			return Ok(Err(FsForkError::OperationConflict));
		}
		// The incarnation's own record row stays until its data is swept, so it holds the
		// source reference until then, deleted or not.
		let present =
			self.incarnation_row(key, spec.operation_id)
				.await?
				.is_some() || self
				.db
				.get(
					keys::PendingSweepKey::new(scope.hash, scope.name.clone(), scope.incarnation)
						.pack(),
				)
				.await?
				.is_some() || self
				.db
				.get(
					keys::PendingForkReleaseKey::new(scope.hash, scope.name, scope.incarnation)
						.pack(),
				)
				.await?
				.is_some();
		self.admit()?;
		Ok(Ok(if present {
			FsForkPresence::Present
		} else {
			FsForkPresence::Absent
		}))
	}
}

impl FilesystemStore {
	/// Each call is bounded and locally ordered. The deterministic owner supplies
	/// immutable identity and chooses retries after interpreting this result.
	///
	/// `replaced` is the record a restore replaces when its first record has not reached the
	/// store yet, as the maintainer registered it.
	pub async fn fork_store_action(
		&self,
		spec: FsForkSpec,
		region: u16,
		action: depot_fs_core::forks::StoreAction,
		replaced: Option<FsVolumeRecord>,
	) -> Result<depot_fs_core::forks::StoreResult> {
		use depot_fs_core::forks::{StoreAction, StoreResult};
		let outcome = match action {
			StoreAction::Inspect => {
				let _ordered = self
					.lock(super::locks::LockRequest::fork(&spec), LockOperation::Other)
					.await;
				self.admit()?;
				if !valid_spec(&spec, region) {
					return Ok(StoreResult::Rejected(FsForkError::Invalid));
				}
				if let Some(completed) = self.completed_fork(&spec).await? {
					completed.map(Preparation::Complete)
				} else if let Some(pending) =
					self.pending_fork(&spec.destination_volume_key).await?
				{
					if pending.spec == spec {
						Ok(Preparation::Pending(Box::new(pending)))
					} else if pending.spec.operation_id == spec.operation_id {
						Err(FsForkError::OperationConflict)
					} else {
						Err(FsForkError::Busy)
					}
				} else {
					return Ok(StoreResult::Unprepared);
				}
			}
			StoreAction::Prepare => self.prepare_fork_over(spec, region, replaced).await?,
			StoreAction::RestoreTarget => {
				let _ordered = self
					.lock(super::locks::LockRequest::fork(&spec), LockOperation::Other)
					.await;
				self.admit()?;
				let pending = self.pending_fork(&spec.destination_volume_key).await?;
				if !pending.is_some_and(|pending| {
					pending.spec == spec
						&& matches!(pending.preparation, FsForkPreparation::FsForkReady(_))
				}) {
					return Ok(StoreResult::Rejected(FsForkError::OperationConflict));
				}
				let row = self.replaced_record(&spec, replaced.as_ref()).await?;
				self.admit()?;
				return Ok(match row {
					Some(row)
						if Some(row.volume_incarnation) == spec.restore
							&& row.status == FsVolumeStatus::Live =>
					{
						StoreResult::RestoreTarget(row)
					}
					_ => StoreResult::Rejected(FsForkError::IncarnationChanged),
				});
			}
			StoreAction::Remember(image) => {
				self.remember_fork_reference_over(FsForkReceipt { spec, image }, replaced)
					.await?
			}
			StoreAction::Copy { after, page } => {
				let Ok(rows) = page
					.rows
					.into_iter()
					.map(depot_fs_core::wire::decode_row)
					.collect::<depot_fs_core::Result<Vec<_>>>()
				else {
					return Ok(StoreResult::Rejected(FsForkError::Invalid));
				};
				self.copy_fork_metadata(&spec, after, rows, page.complete)
					.await?
			}
			StoreAction::Publish {
				claimed,
				logged,
				incarnation_epoch,
			} => self
				.publish_fork_over(&spec, claimed, !logged, replaced, incarnation_epoch)
				.await?
				.map(Preparation::Complete),
			StoreAction::Fail(error) => self.fail_fork(&spec, error).await?,
		};
		self.admit()?;
		Ok(match outcome {
			Ok(progress) => StoreResult::Progress(progress),
			Err(error) => StoreResult::Rejected(error),
		})
	}
}
