//! Bounded seq-zero copies and durable release work for complete fork images.
use super::store::{FilesystemStore, LockOperation};
use anyhow::{Result, ensure};
use depot_fs_core::{
	forks::{Materialization, MaterializeProgress},
	keys::slatedb as keys,
	store::{Row, RowKey},
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
	pub async fn materialization(
		&self,
		key: &depot_types::VolumeKey,
		incarnation: [u8; 16],
	) -> Result<Option<Materialization>> {
		self.admit()?;
		let address = keys::PendingMaterializeKey::new(
			depot_types::partition_key_hash(key),
			key.name.clone().map(Into::into),
			Uuid::from_bytes(incarnation),
		);
		let Some(pending) = self
			.db
			.get(address.pack())
			.await?
			.map(|v| address.deserialize(v))
			.transpose()?
		else {
			return Ok(None);
		};
		let record = self
			.incarnation_record(key, incarnation)
			.await?
			.ok_or_else(|| anyhow::anyhow!("materialization lacks incarnation metadata"))?;
		ensure!(
			pending.incarnation == incarnation
				&& !record.base_chain.is_empty()
				&& (pending.cursor.layer as usize) < record.base_chain.len(),
			"invalid materialization progress"
		);
		self.admit()?;
		Ok(Some(Materialization {
			record,
			cursor: pending.cursor,
		}))
	}

	/// The owner enumerates sparse keys in this layer and resolves each through
	/// the complete source image. Sequence zero never supersedes a child's write.
	/// Cursor, rows and final link removal share one durable batch.
	pub async fn copy_materialized_page(
		&self,
		key: &depot_types::VolumeKey,
		incarnation: [u8; 16],
		expected: &FsMaterializeCursor,
		chunks: Vec<FsMaterializedChunk>,
		complete: bool,
	) -> Outcome<MaterializeProgress> {
		let _ordered = self
			.lock(super::locks::LockRequest::volume(key), LockOperation::Other)
			.await;
		self.admit()?;
		let Some(work) = self.materialization(key, incarnation).await? else {
			let record = self.incarnation_record(key, incarnation).await?;
			return Ok(match record {
				Some(record) if record.base_chain.is_empty() => {
					Ok(MaterializeProgress::Complete(record))
				}
				_ => Err(FsForkError::ForkDeleted),
			});
		};
		if work.cursor != *expected {
			return Ok(Ok(MaterializeProgress::More(work.cursor)));
		}
		if chunks.len() > self.batch_rows || chunks.is_empty() && !complete {
			return Ok(Err(FsForkError::Invalid));
		}
		let scope = keys::Scope::new(key, Uuid::from_bytes(incarnation));
		let mut batch = WriteBatch::new();
		let mut last = expected.after.clone();
		let mut bytes = 0usize;
		for chunk in chunks {
			let position = chunk.position;
			if last
				.as_ref()
				.is_some_and(|old| (old.ino, old.index) >= (position.ino, position.index))
				|| position.index.checked_mul(CHUNK_BYTES as u64).is_none()
				|| chunk.bytes.as_ref().is_some_and(|b| b.len() > CHUNK_BYTES)
			{
				return Ok(Err(FsForkError::Invalid));
			}
			let (address, value) = keys::encode_row(
				&scope,
				&RowKey::Chunk(position.ino, position.index),
				0,
				chunk.bytes.map(Row::Bytes),
				self.row_version(),
			)?;
			bytes = bytes
				.saturating_add(address.len())
				.saturating_add(value.len());
			if bytes > self.config.max_read_bytes {
				return Ok(Err(FsForkError::Invalid));
			}
			// Duplicate candidates from different layers must resolve to the
			// same source image; an inconsistent copy must not rewrite history.
			if let Some(old) = self.db.get(&address).await? {
				ensure!(
					old == value,
					"materialization changed a previously copied chunk"
				);
			}
			batch.put(address, value);
			last = Some(position);
		}
		let cursor = if complete {
			FsMaterializeCursor {
				layer: expected.layer + 1,
				complete_image: false,
				after: None,
			}
		} else {
			FsMaterializeCursor {
				layer: expected.layer,
				complete_image: expected.complete_image,
				after: last,
			}
		};
		let pending =
			keys::PendingMaterializeKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		let mut record = work.record;
		let done =
			complete && expected.complete_image || cursor.layer as usize == record.base_chain.len();
		if done {
			let archive =
				keys::IncarnationRecordKey::new(scope.hash, scope.name.clone(), scope.incarnation);
			let mut retained = self
				.db
				.get(archive.pack())
				.await?
				.map(|v| archive.deserialize(v))
				.transpose()?
				.ok_or_else(|| anyhow::anyhow!("materialized fork lacks creation receipt"))?;
			let reference = retained
				.creation
				.clone()
				.ok_or_else(|| anyhow::anyhow!("materialized fork lacks source reference"))?;
			let release =
				keys::PendingForkReleaseKey::new(scope.hash, scope.name.clone(), scope.incarnation);
			record.base_chain.clear();
			retained.record = record.clone();
			batch.put(
				archive.pack(),
				archive.serialize(AtVersion::new(retained, self.row_version()))?,
			);
			// A swept incarnation keeps only its archive; one still holding its own record
			// row gets the cleared chain there, and in the pointer only when that names it.
			if let Some(mut own) = self.incarnation_row(key, incarnation).await? {
				own.base_chain.clear();
				let address = super::store::incarnation_record_key(&own);
				batch.put(
					address.pack(),
					address.serialize(AtVersion::new(own.clone(), self.row_version()))?,
				);
				if self
					.record(key)
					.await?
					.is_some_and(|pointer| pointer.volume_incarnation == incarnation)
				{
					let pointer = super::store::pointer_key(key);
					batch.put(
						pointer.pack(),
						pointer.serialize(AtVersion::new(own, self.row_version()))?,
					);
				}
			}
			batch.put(
				release.pack(),
				release.serialize(AtVersion::new(
					FsPendingForkRelease {
						spec: reference.spec,
					},
					self.row_version(),
				))?,
			);
			batch.delete(pending.pack());
		} else {
			batch.put(
				pending.pack(),
				pending.serialize(AtVersion::new(
					FsPendingMaterialize {
						incarnation,
						cursor: cursor.clone(),
					},
					self.row_version(),
				))?,
			);
		}
		// The page and its cursor ride the tablet's ordinary pass; a crash before it copies
		// the pages again from the cursor that reached cold. The maintainer sends the source's
		// release only once a pass covers the last page.
		self.stage(batch).await?;
		self.admit()?;
		if done {
			self.load_retention(&record).await?;
			Ok(Ok(MaterializeProgress::Complete(record)))
		} else {
			Ok(Ok(MaterializeProgress::More(cursor)))
		}
	}

	/// Old sessions keep their original snapshots and chain. New sessions capture
	/// the complete record, so once the last old session expires the release is safe.
	pub async fn fork_release(
		&self,
		key: &depot_types::VolumeKey,
		incarnation: [u8; 16],
	) -> Result<Option<FsForkSpec>> {
		let _ordered = self
			.lock(super::locks::LockRequest::volume(key), LockOperation::Other)
			.await;
		self.admit()?;
		let scope = keys::Scope::new(key, Uuid::from_bytes(incarnation));
		let now = self.clock.observe()?.boot_ms;
		self.expire_sessions(now);
		if self
			.sessions
			.lock()
			.values()
			.any(|s| s.scope == scope && !s.base_chain.is_empty())
		{
			return Ok(None);
		}
		let address = keys::PendingForkReleaseKey::new(scope.hash, scope.name, scope.incarnation);
		let pending = self
			.db
			.get(address.pack())
			.await?
			.map(|v| address.deserialize(v))
			.transpose()?;
		ensure!(
			pending.as_ref().is_none_or(
				|p| p.spec.destination_volume_key == *key && p.spec.operation_id == incarnation
			),
			"fork release address mismatch"
		);
		self.admit()?;
		Ok(pending.map(|p| p.spec))
	}

	/// Called only after the source confirms the exact terminal release receipt.
	/// A lost confirmation leaves the durable work to retry after regrant.
	pub async fn acknowledge_fork_release(&self, spec: &FsForkSpec) -> Result<()> {
		let _ordered = self
			.lock(super::locks::LockRequest::fork(spec), LockOperation::Other)
			.await;
		self.admit()?;
		let key = &spec.destination_volume_key;
		let address = keys::PendingForkReleaseKey::new(
			depot_types::partition_key_hash(key),
			key.name.clone().map(Into::into),
			Uuid::from_bytes(spec.operation_id),
		);
		if let Some(pending) = self.db.get(address.pack()).await? {
			ensure!(
				address.deserialize(pending)?.spec == *spec,
				"fork release receipt mismatch"
			);
			// A lost removal only sends the release again, which the source answers alike.
			let mut batch = WriteBatch::new();
			batch.delete(address.pack());
			self.stage(batch).await?;
		}
		self.admit()
	}
}

impl FilesystemStore {
	/// Invoked with the final data-sweep batch while holding mutation. Outgoing
	/// cleanup becomes durable before the sweep or logical-delete marker can go.
	pub(super) async fn complete_fork_sweep(
		&self,
		scope: &keys::Scope,
		batch: &mut WriteBatch,
	) -> Result<()> {
		let archive =
			keys::IncarnationRecordKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		if let Some(value) = self.db.get(archive.pack()).await? {
			let archived = archive.deserialize(value)?;
			if !archived.record.base_chain.is_empty()
				&& let Some(reference) = archived.creation
			{
				let release = keys::PendingForkReleaseKey::new(
					scope.hash,
					scope.name.clone(),
					scope.incarnation,
				);
				batch.put(
					release.pack(),
					release.serialize(AtVersion::new(
						FsPendingForkRelease {
							spec: reference.spec,
						},
						self.row_version(),
					))?,
				);
			}
		}
		batch.delete(
			keys::PendingMaterializeKey::new(scope.hash, scope.name.clone(), scope.incarnation)
				.pack(),
		);
		Ok(())
	}
}

impl FilesystemStore {
	/// Recovery starts from the exact pending-work address, then reads the raw
	/// volume key from its archive. Local mutations use the same grant and lock.
	pub async fn materialize_step(
		&self,
		scope: &keys::Scope,
		action: depot_fs_core::forks::materialize::Action,
	) -> Result<depot_fs_core::forks::materialize::Progress> {
		use depot_fs_core::forks::materialize::{Action, Progress};
		self.admit()?;
		let address =
			keys::IncarnationRecordKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		let Some(value) = self.db.get(address.pack()).await? else {
			return Ok(Progress::Idle);
		};
		let archived = address.deserialize(value)?;
		let key = archived.record.volume_key();
		let incarnation = *scope.incarnation.as_bytes();
		ensure!(
			keys::Scope::new(
				&key,
				uuid::Uuid::from_bytes(archived.record.volume_incarnation)
			) == *scope,
			"materialization archive identity mismatch"
		);
		match action {
			Action::Inspect => {}
			Action::Restart { expected, layer } => {
				let _ordered = self
					.lock(
						super::locks::LockRequest::volume(&key),
						LockOperation::Other,
					)
					.await;
				self.admit()?;
				if let Some(work) = self.materialization(&key, incarnation).await? {
					ensure!(
						layer <= expected.layer,
						"materialization restart skips a source layer"
					);
					if work.cursor == expected {
						let pending = keys::PendingMaterializeKey::new(
							scope.hash,
							scope.name.clone(),
							scope.incarnation,
						);
						let mut batch = WriteBatch::new();
						batch.put(
							pending.pack(),
							pending.serialize(AtVersion::new(
								FsPendingMaterialize {
									incarnation,
									cursor: FsMaterializeCursor {
										layer,
										after: None,
										complete_image: true,
									},
								},
								self.row_version(),
							))?,
						);
						self.stage(batch).await?;
					}
				}
			}
			Action::Copy {
				expected,
				chunks,
				complete,
			} => {
				match self
					.copy_materialized_page(&key, incarnation, &expected, chunks, complete)
					.await?
				{
					Ok(MaterializeProgress::Complete(record)) => {
						return Ok(Progress::Complete(record));
					}
					Ok(MaterializeProgress::More(_)) => {}
					Err(FsForkError::ForkDeleted) => return Ok(Progress::Idle),
					Err(e) => anyhow::bail!("materialization rejected: {e:?}"),
				}
			}
		}
		let work = self.materialization(&key, incarnation).await?;
		self.admit()?;
		Ok(work.map_or(Progress::Idle, Progress::Ready))
	}
}
