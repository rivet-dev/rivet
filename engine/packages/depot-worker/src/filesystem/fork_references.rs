//! A source reference fixes its checkpoint and keeps an immutable retry receipt.
use super::{
	checkpoints::address,
	store::{FilesystemStore, LockOperation},
};
use anyhow::{Result, ensure};
use depot_fs_core::{keys::slatedb as keys, *};
use rivet_data::generated::{
	depot_client_protocol_v1::{CheckpointList, VolumeCheckpointResponse},
	shared_depot_types_v1::VolumeLogPosition,
};
use rivet_keys::keys::{
	KeyAddress,
	codec::{AtVersion, FormalKey},
};
use slatedb::WriteBatch;
use uuid::Uuid;

type Outcome<T> = Result<std::result::Result<T, FsForkError>>;
impl FilesystemStore {
	/// An incarnation's own record row, or, once its data is swept, the archive a fork or
	/// restore kept, so a restore never needs to reconstruct an old chain by guesswork.
	pub(super) async fn incarnation_record(
		&self,
		volume: &depot_types::VolumeKey,
		incarnation: [u8; 16],
	) -> Result<Option<FsVolumeRecord>> {
		if let Some(own) = self.incarnation_row(volume, incarnation).await? {
			return Ok(Some(own));
		}
		let key = keys::IncarnationRecordKey::new(
			depot_types::partition_key_hash(volume),
			volume.name.clone().map(Into::into),
			Uuid::from_bytes(incarnation),
		);
		let record = self
			.db
			.get(key.pack())
			.await?
			.map(|v| key.deserialize(v))
			.transpose()?
			.map(|v| v.record);
		ensure!(
			record
				.as_ref()
				.is_none_or(|r| r.volume_key() == *volume && r.volume_incarnation == incarnation),
			"retained incarnation address mismatch"
		);
		self.admit()?;
		Ok(record)
	}

	async fn select_fork_point(
		&self,
		spec: &FsForkSpec,
		current: &FsVolumeRecord,
	) -> Outcome<FsCheckpointInfo> {
		let scope = keys::Scope::new(
			&spec.parent_volume_key,
			Uuid::from_bytes(current.volume_incarnation),
		);
		match &spec.selector {
			FsForkSelector::FsForkLatest => {
				if let Some(minimum) = &spec.minimum {
					if minimum.incarnation != current.volume_incarnation {
						return Ok(Err(FsForkError::IncarnationChanged));
					}
					if minimum.position.volume_seq > current.cold.volume_seq {
						return Ok(Err(FsForkError::Lagging));
					}
					if minimum.position.client_epoch > current.cold.client_epoch {
						return Ok(Err(FsForkError::Invalid));
					}
				}
				let mut name = b"\0fork/".to_vec();
				name.extend_from_slice(&spec.operation_id);
				return Ok(Ok(FsCheckpointInfo {
					name: name.into(),
					volume_incarnation: current.volume_incarnation,
					position: current.cold.clone(),
					checkpoint: FsVolumeCheckpoint {
						named: false,
						retention: false,
						created_at: self.clock.observe()?.unix_ms,
						client_epoch: current.cold.client_epoch,
					},
				}));
			}
			FsForkSelector::FsForkNamed(named) => {
				if named.name.is_empty() || named.name.len() > checkpoints::MAX_NAME_BYTES {
					return Ok(Err(FsForkError::Invalid));
				}
				let scope = keys::Scope {
					incarnation: Uuid::from_bytes(
						named.incarnation.unwrap_or(current.volume_incarnation),
					),
					..scope
				};
				if let Some(point) = self
					.inspect_checkpoint(&scope, &named.name)
					.await?
					.checkpoint && (point.checkpoint.named || point.checkpoint.retention)
				{
					return Ok(Ok(point));
				}
				if named.incarnation.is_some() {
					return Ok(Err(FsForkError::CheckpointNotFound));
				}
			}
			FsForkSelector::FsForkAtTime(_) => {}
		}
		let mut after = None;
		let mut selected: Option<FsCheckpointInfo> = None;
		loop {
			let VolumeCheckpointResponse::CheckpointPage(page) = self
				.checkpoint_page(
					&spec.parent_volume_key,
					&CheckpointList { after, limit: 256 },
				)
				.await?
			else {
				anyhow::bail!("internal checkpoint cursor rejected");
			};
			for point in page.checkpoints {
				match &spec.selector {
					FsForkSelector::FsForkNamed(named) if point.name == named.name => {
						if selected.is_some() {
							return Ok(Err(FsForkError::CheckpointAmbiguous));
						}
						selected = Some(point);
					}
					FsForkSelector::FsForkAtTime(time)
						if point.checkpoint.created_at <= time.unix_ms =>
					{
						let order = |p: &FsCheckpointInfo| {
							(
								p.checkpoint.created_at,
								p.volume_incarnation,
								p.position.volume_seq,
								p.name.clone(),
							)
						};
						if selected
							.as_ref()
							.is_none_or(|old| order(&point) > order(old))
						{
							selected = Some(point);
						}
					}
					_ => {}
				}
			}
			after = page.after;
			if after.is_none() {
				break;
			}
		}
		Ok(selected.ok_or(
			if matches!(spec.selector, FsForkSelector::FsForkAtTime(_)) {
				FsForkError::ForkOutOfRetention
			} else {
				FsForkError::CheckpointNotFound
			},
		))
	}

	/// Selector resolution and the hold are one ordered mutation. Without `logged`, the
	/// returned record is a durability proof for the source core's cold/trim view. With
	/// `logged`, the acquire is the apply of a maintainer log op created at that time: the
	/// op is its durable record, so nothing is flushed, the volume record keeps its stored
	/// cold position, and no record is returned.
	pub async fn acquire_fork_reference(
		&self,
		spec: FsForkSpec,
		region: u16,
		max_depth: usize,
		loaded: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		logged: Option<i64>,
	) -> Outcome<(FsForkReceipt, Option<FsVolumeRecord>)> {
		let _ordered = self
			.lock(super::locks::LockRequest::fork(&spec), LockOperation::Other)
			.await;
		self.admit()?;
		if spec.parent_region != region
			|| !forks::valid_spec(&spec, spec.destination_region)
			|| max_depth == 0
			|| max_depth > MAX_FORK_CHAIN_DEPTH
		{
			return Ok(Err(FsForkError::Invalid));
		}
		let hash = depot_types::partition_key_hash(&spec.parent_volume_key);
		let fork = Uuid::from_bytes(spec.operation_id);
		let reference = keys::ForkReferenceKey::new(
			hash,
			spec.parent_volume_key.name.clone().map(Into::into),
			fork,
		);
		if let Some(result) = self.lookup_fork_reference(&spec).await? {
			return Ok(result.map(|receipt| (receipt, None)));
		}
		let stored_pointer = self.pointer(&spec.parent_volume_key).await?;
		let stored = super::store::names_key(stored_pointer.clone(), &spec.parent_volume_key);
		if stored
			.as_ref()
			.zip(loaded.as_ref())
			.is_some_and(|(a, b)| a.volume_incarnation != b.volume_incarnation)
		{
			return Ok(Err(FsForkError::IncarnationChanged));
		}
		let Some(mut current) = stored.or(loaded) else {
			return Ok(Err(FsForkError::VolumeNotFound));
		};
		let durable = current.clone();
		ensure!(
			current.volume_key() == spec.parent_volume_key,
			"fork source record identity mismatch"
		);
		if current.status == FsVolumeStatus::Deleted {
			return Ok(Err(FsForkError::VolumeDeleted));
		}
		if let Some(applied) = applied {
			ensure!(
				applied.volume_seq >= current.cold.volume_seq
					&& applied.client_epoch >= current.cold.client_epoch
					&& (applied.volume_seq != current.cold.volume_seq
						|| applied.client_epoch == current.cold.client_epoch),
				"fork applied floor moved backwards"
			);
			current.cold = applied;
		}
		let point = match self.select_fork_point(&spec, &current).await? {
			Ok(p) => p,
			Err(e) => return Ok(Err(e)),
		};
		if forks::initial_position(&point.position).is_err() {
			return Ok(Err(FsForkError::Invalid));
		}
		let Some(parent) = self
			.incarnation_record(&spec.parent_volume_key, point.volume_incarnation)
			.await?
			.or_else(|| {
				(current.volume_incarnation == point.volume_incarnation).then(|| current.clone())
			})
		else {
			anyhow::bail!("checkpoint lacks retained incarnation metadata");
		};
		if parent.base_chain.len() >= max_depth {
			return Ok(Err(FsForkError::ForkChainTooDeep));
		}
		let mut parent_base = parent.base_chain.clone();
		for link in &mut parent_base {
			link.region.get_or_insert(region);
		}
		let receipt = FsForkReceipt {
			image: FsForkImage {
				link: FsBaseLink {
					volume_key: spec.parent_volume_key.clone(),
					incarnation: point.volume_incarnation,
					checkpoint_name: point.name.clone(),
					position: point.position.clone(),
					region: Some(region),
				},
				parent_base,
			},
			spec,
		};
		let scope = keys::Scope::new(
			&receipt.spec.parent_volume_key,
			Uuid::from_bytes(point.volume_incarnation),
		);
		let checkpoint = address(&scope, point.position.volume_seq, point.name.to_vec());
		let hold = keys::ForkHoldKey::new(
			hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			point.position.volume_seq,
			point.name.into(),
			fork,
		);
		let pending = keys::PendingForkCheckKey::new(hash, scope.name.clone(), fork);
		let now = match logged {
			Some(created_at) => created_at,
			None => self.clock.observe()?.unix_ms,
		};
		let mut batch = WriteBatch::new();
		batch.put(
			checkpoint.pack(),
			checkpoint.serialize(AtVersion::new(point.checkpoint.clone(), self.row_version()))?,
		);
		batch.put(
			hold.pack(),
			hold.serialize(AtVersion::new(
				FsForkHold {
					fork_volume_key: receipt.spec.destination_volume_key.clone(),
					fork_region: receipt.spec.destination_region,
					created_at: now,
				},
				self.row_version(),
			))?,
		);
		batch.put(
			reference.pack(),
			reference.serialize(AtVersion::new(
				FsForkReference::FsForkReferenceHeld(FsForkReferenceHeld {
					receipt: receipt.clone(),
					created_at: now,
				}),
				self.row_version(),
			))?,
		);
		batch.put(
			pending.pack(),
			pending.serialize(AtVersion::new(
				FsPendingForkCheck {
					fork_incarnation: receipt.spec.operation_id,
					created_at: now,
				},
				self.row_version(),
			))?,
		);
		let archive = keys::IncarnationRecordKey::new(hash, scope.name.clone(), scope.incarnation);
		let creation = self
			.db
			.get(archive.pack())
			.await?
			.map(|v| archive.deserialize(v))
			.transpose()?
			.and_then(|v| v.creation);
		let current_scope = keys::Scope::new(
			&current.volume_key(),
			Uuid::from_bytes(current.volume_incarnation),
		);
		self.limit_cold(&current_scope, &mut current);
		// A logged acquire leaves every record at its stored cold position: only an
		// ordinary pass advances cold. Reads at the pinned position above it are served
		// from the applied store; see `open_checkpoint_session`.
		let current = if logged.is_some() { durable } else { current };
		let retained = if current.volume_incarnation == point.volume_incarnation {
			current.clone()
		} else {
			parent
		};
		batch.put(
			archive.pack(),
			archive.serialize(AtVersion::new(
				FsIncarnationRecord {
					record: retained,
					creation,
				},
				self.row_version(),
			))?,
		);
		if logged.is_none() {
			super::store::put_record(
				&mut batch,
				stored_pointer.as_ref(),
				&current,
				self.row_version(),
			)?;
		}
		if let Some(retention) = self.catalog.lock().get_mut(&scope) {
			retention.checkpoints.insert(point.position.volume_seq);
		}
		self.stage(batch).await?;
		if logged.is_none() {
			self.shared_flush
				.flush(&self.db, super::shared_flush::FlushCause::ForkReference)
				.await?;
		}
		self.admit()?;
		self.load_retention(&current).await?;
		Ok(Ok((receipt, logged.is_none().then_some(current))))
	}
}

impl FilesystemStore {
	/// Release is itself idempotent. Keep a terminal receipt so a delayed acquire
	/// cannot recreate this source reference after destination cleanup.
	/// `loaded_parent_incarnations` are the parent incarnations the maintainer holds loaded.
	/// The release never schedules a sweep of one of them: a missing record does not prove an
	/// incarnation is gone, because a new volume's first record rides the tiering pass.
	pub async fn release_fork_reference(
		&self,
		spec: FsForkSpec,
		region: u16,
		flush: bool,
		loaded_parent_incarnations: &[[u8; 16]],
	) -> Outcome<()> {
		let _ordered = self
			.lock(super::locks::LockRequest::fork(&spec), LockOperation::Other)
			.await;
		self.admit()?;
		if spec.parent_region != region || !forks::valid_spec(&spec, spec.destination_region) {
			return Ok(Err(FsForkError::Invalid));
		}
		self.release_fork_reference_unlocked(spec, flush, loaded_parent_incarnations)
			.await
	}

	/// The release itself, for a caller that already orders it against every other change of
	/// the parent. With `flush`, its rows are durable before it returns; the log apply passes
	/// false, because the op's own log entry is its durable record until the next pass.
	pub(crate) async fn release_fork_reference_unlocked(
		&self,
		spec: FsForkSpec,
		flush: bool,
		loaded_parent_incarnations: &[[u8; 16]],
	) -> Outcome<()> {
		use rivet_keys::keys::PrefixAddress;
		let hash = depot_types::partition_key_hash(&spec.parent_volume_key);
		let fork = Uuid::from_bytes(spec.operation_id);
		let reference = keys::ForkReferenceKey::new(
			hash,
			spec.parent_volume_key.name.clone().map(Into::into),
			fork,
		);
		let receipt = match self
			.db
			.get(reference.pack())
			.await?
			.map(|v| reference.deserialize(v))
			.transpose()?
		{
			Some(FsForkReference::FsForkReferenceReleased(done)) if done.spec == spec => {
				self.admit()?;
				return Ok(Ok(()));
			}
			Some(FsForkReference::FsForkReferenceHeld(held)) if held.receipt.spec == spec => {
				held.receipt
			}
			Some(_) => return Ok(Err(FsForkError::OperationConflict)),
			None => {
				// A cancelled destination can race an acquire still in transit.
				// This terminal receipt fences that future acquire too.
				let mut batch = WriteBatch::new();
				batch.put(
					reference.pack(),
					reference.serialize(AtVersion::new(
						FsForkReference::FsForkReferenceReleased(FsForkReferenceReleased { spec }),
						self.row_version(),
					))?,
				);
				self.stage(batch).await?;
				if flush {
					self.shared_flush
						.flush(&self.db, super::shared_flush::FlushCause::ForkReference)
						.await?;
				}
				self.admit()?;
				return Ok(Ok(()));
			}
		};
		let link = &receipt.image.link;
		let scope = keys::Scope::new(&link.volume_key, Uuid::from_bytes(link.incarnation));
		let checkpoint = address(
			&scope,
			link.position.volume_seq,
			link.checkpoint_name.to_vec(),
		);
		let value = checkpoint.deserialize(
			self.db
				.get(checkpoint.pack())
				.await?
				.ok_or_else(|| anyhow::anyhow!("held checkpoint missing"))?,
		)?;
		ensure!(
			value.client_epoch == link.position.client_epoch,
			"held checkpoint position mismatch"
		);
		let hold = keys::ForkHoldKey::new(
			hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			link.position.volume_seq,
			link.checkpoint_name.clone().into(),
			fork,
		);
		ensure!(
			self.db.get(hold.pack()).await?.is_some(),
			"reference lacks its fork hold"
		);
		let prefix = keys::CheckpointForkHoldsPrefix::new(
			hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			link.position.volume_seq,
			link.checkpoint_name.clone().into(),
		)
		.pack();
		let mut rows = self.db.scan_prefix(prefix, ..).await?;
		let mut other = false;
		while let Some(row) = rows.next().await? {
			if row.key != hold.pack() {
				other = true;
				break;
			}
		}
		let mut batch = WriteBatch::new();
		batch.delete(hold.pack());
		let pending = keys::PendingForkCheckKey::new(hash, scope.name.clone(), fork);
		batch.delete(pending.pack());
		batch.put(
			reference.pack(),
			reference.serialize(AtVersion::new(
				FsForkReference::FsForkReferenceReleased(FsForkReferenceReleased {
					spec: receipt.spec.clone(),
				}),
				self.row_version(),
			))?,
		);
		let own = self
			.incarnation_row(&link.volume_key, link.incarnation)
			.await?;
		if !value.named && !value.retention && !other {
			batch.delete(checkpoint.pack());
			// The maintainer's loaded incarnation is live or owns its own deletion sweep, even
			// before its first record is stored.
			let loaded = loaded_parent_incarnations.contains(&link.incarnation);
			let retired = own
				.as_ref()
				.is_none_or(|r| r.status == FsVolumeStatus::Deleted);
			if retired && !loaded {
				let sweep = keys::PendingSweepKey::new(hash, scope.name.clone(), scope.incarnation);
				batch.put(
					sweep.pack(),
					sweep.serialize(AtVersion::new(
						FsPendingSweep {
							incarnation: link.incarnation,
							cursor: None,
						},
						self.row_version(),
					))?,
				);
			}
		}
		self.stage(batch).await?;
		if flush {
			self.shared_flush
				.flush(&self.db, super::shared_flush::FlushCause::ForkReference)
				.await?;
		}
		self.admit()?;
		if let Some(record) = self
			.incarnation_record(&link.volume_key, link.incarnation)
			.await?
		{
			self.load_retention(&record).await?;
		}
		Ok(Ok(()))
	}
}

impl FilesystemStore {
	/// An absent destination has a durable terminal outcome; a preparing one
	/// must keep both its reference and discovery check. A published destination
	/// owns eventual release, so only its discovery index can be removed.
	pub async fn complete_fork_check(
		&self,
		spec: FsForkSpec,
		region: u16,
		presence: FsForkPresence,
		loaded_parent_incarnations: &[[u8; 16]],
	) -> Outcome<()> {
		// A lost release of an absent destination's hold leaves its pending check behind,
		// which checks again and releases again, so the release rides the ordinary pass.
		if presence == FsForkPresence::Absent {
			return self
				.release_fork_reference(spec, region, false, loaded_parent_incarnations)
				.await;
		}
		let _ordered = self
			.lock(super::locks::LockRequest::fork(&spec), LockOperation::Other)
			.await;
		self.admit()?;
		if spec.parent_region != region {
			return Ok(Err(FsForkError::Invalid));
		}
		// A lost removal of the pending check only checks again later.
		if presence == FsForkPresence::Present {
			return self.finish_fork_check_unlocked(spec, false).await;
		}
		self.check_fork_reference_unlocked(&spec).await
	}

	/// Whether the parent still holds `spec`'s reference: `Ok` while it is held or after its
	/// release, an error for a missing or different reference.
	async fn check_fork_reference_unlocked(&self, spec: &FsForkSpec) -> Outcome<()> {
		let hash = depot_types::partition_key_hash(&spec.parent_volume_key);
		let incarnation = Uuid::from_bytes(spec.operation_id);
		let reference = keys::ForkReferenceKey::new(
			hash,
			spec.parent_volume_key.name.clone().map(Into::into),
			incarnation,
		);
		let Some(value) = self.db.get(reference.pack()).await? else {
			return Ok(Err(FsForkError::CheckpointNotFound));
		};
		match reference.deserialize(value)? {
			FsForkReference::FsForkReferenceHeld(held) if held.receipt.spec == *spec => {}
			FsForkReference::FsForkReferenceReleased(done) if done.spec == *spec => {}
			_ => return Ok(Err(FsForkError::OperationConflict)),
		}
		self.admit()?;
		Ok(Ok(()))
	}

	/// Removes the pending check of a fork found present, for a caller that orders it against
	/// every other change of the parent. `flush` is as for `release_fork_reference_unlocked`.
	pub(crate) async fn finish_fork_check_unlocked(
		&self,
		spec: FsForkSpec,
		flush: bool,
	) -> Outcome<()> {
		let hash = depot_types::partition_key_hash(&spec.parent_volume_key);
		let incarnation = Uuid::from_bytes(spec.operation_id);
		let reference = keys::ForkReferenceKey::new(
			hash,
			spec.parent_volume_key.name.clone().map(Into::into),
			incarnation,
		);
		let Some(value) = self.db.get(reference.pack()).await? else {
			return Ok(Err(FsForkError::CheckpointNotFound));
		};
		match reference.deserialize(value)? {
			FsForkReference::FsForkReferenceHeld(held) if held.receipt.spec == spec => {}
			FsForkReference::FsForkReferenceReleased(done) if done.spec == spec => {
				self.admit()?;
				return Ok(Ok(()));
			}
			_ => return Ok(Err(FsForkError::OperationConflict)),
		}
		let mut batch = WriteBatch::new();
		batch.delete(
			keys::PendingForkCheckKey::new(
				hash,
				spec.parent_volume_key.name.map(Into::into),
				incarnation,
			)
			.pack(),
		);
		self.stage(batch).await?;
		if flush {
			self.shared_flush
				.flush(&self.db, super::shared_flush::FlushCause::ForkReference)
				.await?;
		}
		self.admit()?;
		Ok(Ok(()))
	}
}

impl FilesystemStore {
	async fn lookup_fork_reference(
		&self,
		spec: &FsForkSpec,
	) -> anyhow::Result<Option<std::result::Result<FsForkReceipt, FsForkError>>> {
		self.admit()?;
		let key = keys::ForkReferenceKey::new(
			depot_types::partition_key_hash(&spec.parent_volume_key),
			spec.parent_volume_key.name.clone().map(Into::into),
			Uuid::from_bytes(spec.operation_id),
		);
		let value = self
			.db
			.get(key.pack())
			.await?
			.map(|bytes| key.deserialize(bytes))
			.transpose()?;
		self.admit()?;
		Ok(value.map(|value| match value {
			FsForkReference::FsForkReferenceHeld(held) if held.receipt.spec == *spec => {
				Ok(held.receipt)
			}
			FsForkReference::FsForkReferenceReleased(released) if released.spec == *spec => {
				Err(FsForkError::ForkDeleted)
			}
			_ => Err(FsForkError::OperationConflict),
		}))
	}
	/// Ordered source/destination reference administration. Lookup precedes
	/// current-incarnation validation inside acquisition, preserving exact retries.
	///
	/// `logged` carries the creation time of the maintainer log op an acquire or release
	/// applies; such a change is not flushed, because the op is its durable record.
	pub async fn fork_reference_request(
		&self,
		request: rivet_data::generated::depot_client_protocol_v1::FsForkReferenceRequest,
		region: u16,
		barrier: depot_fs_core::forks::ReferenceBarrier,
		record: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		logged: Option<i64>,
		loaded_parent_incarnations: &[[u8; 16]],
	) -> anyhow::Result<depot_fs_core::forks::ReferenceResult> {
		use depot_fs_core::forks::{ReferenceBarrier, ReferenceResult};
		use rivet_data::generated::depot_client_protocol_v1::{
			FsForkReferenceRequest as Request, FsForkReferenceResponse as Response,
		};
		if request.region() != region
			|| !forks::valid_spec(request.spec(), request.spec().destination_region)
		{
			return Ok(ReferenceResult::Complete {
				response: Box::new(Response::FsForkError(FsForkError::Invalid)),
				record: None,
			});
		}
		if let Request::FsForkAcquire(v) = &request
			&& barrier == ReferenceBarrier::Lookup
		{
			return Ok(match self.lookup_fork_reference(&v.spec).await? {
				Some(result) => ReferenceResult::Complete {
					response: Box::new(match result {
						Ok(receipt) => Response::FsForkReceipt(receipt),
						Err(error) => Response::FsForkError(error),
					}),
					record: None,
				},
				None => ReferenceResult::SourceProgressRequired,
			});
		}
		let (response, record) = match request {
			Request::FsForkAcquire(v) => match self
				.acquire_fork_reference(
					v.spec,
					region,
					MAX_FORK_CHAIN_DEPTH,
					record,
					applied,
					logged,
				)
				.await?
			{
				Ok((receipt, record)) => (Response::FsForkReceipt(receipt), record),
				Err(error) => (Response::FsForkError(error), None),
			},
			Request::FsForkRelease(v) => (
				match self
					.release_fork_reference(
						v.spec,
						region,
						logged.is_none(),
						loaded_parent_incarnations,
					)
					.await?
				{
					Ok(()) => Response::FsForkReleased,
					Err(error) => Response::FsForkError(error),
				},
				None,
			),
			Request::FsForkCheck(v) => (
				match self.check_fork(&v.spec, region).await? {
					Ok(presence) => Response::FsForkPresence(presence),
					Err(error) => Response::FsForkError(error),
				},
				None,
			),
		};
		Ok(depot_fs_core::forks::ReferenceResult::Complete {
			response: Box::new(response),
			record,
		})
	}
}
