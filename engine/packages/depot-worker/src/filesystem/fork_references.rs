//! A source reference fixes its checkpoint and keeps an immutable retry receipt.
use super::{
	checkpoints::address,
	store::{FilesystemStore, LockOperation},
};
use anyhow::{Result, ensure};
use depot_fs_core::{keys::slatedb as keys, *};
use rivet_data::generated::{
	depot_client_protocol_v2::{CheckpointList, VolumeCheckpointResponse},
	shared_depot_types_v1::VolumeLogPosition,
};
use rivet_keys::keys::{
	KeyAddress, PrefixAddress,
	codec::{AtVersion, FormalKey},
};
use slatedb::WriteBatch;
use uuid::Uuid;

pub use crate::tablet_maintainer_machine::types::GroupChildren;

type Outcome<T> = Result<std::result::Result<T, FsForkError>>;

/// What a group fork acquire found on the parent, besides a refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupAcquire {
	/// An incarnation created before the fork, live or deleted at or after it, pinned at the
	/// fork: the receipt names the pin, and carries the metadata when it is small enough.
	Receipt {
		receipt: FsForkReceipt,
		metadata: Option<rivet_data::generated::shared_depot_fs_read_v2::FsForkMetadataPage>,
	},
	/// Every incarnation created before the fork was deleted before it.
	Absent,
	/// No incarnation was created before the fork.
	Continue,
	/// The incarnation stands but no pin covers the fork yet; the maintainer logs one and the
	/// acquire is tried again.
	NeedsPin,
}

/// What planning an idle parent's fork pin found, without writing anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdlePinPlan {
	/// A pin covers the fork, or the fork needs none because the name's incarnation was
	/// created at or after it, deleted before it, or never existed; the acquire goes on.
	Pinned,
	/// The pin to log and apply before the acquire: the parent at its stored cut for the
	/// forks after its last pin through the acquired one, waiting on their live children.
	Pin(rivet_data::generated::depot_maintainer_log_v2::ForkPinOp),
	/// The children the maintainer gave do not cover every fork from `first_fork`, the one
	/// after the parent's last pin, through the acquired one; the maintainer reads those and
	/// plans again.
	NeedsChildren { first_fork: u64 },
	/// No pin can stand for the fork yet: every child it would wait on is archived, so the
	/// pin would wait on no one, or the store cannot write the pin's rows yet. The acquire is
	/// answered busy and tried again.
	Busy,
}

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
			// A group fork acquire is answered from the volume's fork pins by
			// `acquire_group_fork_reference`, never from a checkpoint.
			FsForkSelector::FsForkGroup(_) => return Ok(Err(FsForkError::Invalid)),
			FsForkSelector::FsForkLatest => {
				if let Some(minimum) = &spec.minimum {
					if minimum.incarnation != current.volume_incarnation {
						return Ok(Err(FsForkError::IncarnationChanged));
					}
					// The parent has one claim log, so the minimum names claim log 0 alone.
					let Some(position) = minimum.position.root_position() else {
						return Ok(Err(FsForkError::Invalid));
					};
					if position.volume_seq > current.cold.volume_seq {
						return Ok(Err(FsForkError::Lagging));
					}
					if position.client_epoch > current.cold.client_epoch {
						return Ok(Err(FsForkError::Invalid));
					}
				}
				let mut name = b"\0fork/".to_vec();
				name.extend_from_slice(&spec.operation_id);
				return Ok(Ok(FsCheckpointInfo {
					name: name.into(),
					volume_incarnation: current.volume_incarnation,
					position: FsCut::single(current.cold.clone()),
					checkpoint: FsVolumeCheckpoint {
						named: false,
						retention: false,
						created_at: self.clock.observe()?.unix_ms,
						client_epoch: current.cold.client_epoch,
						cut: Some(FsCut::single(current.cold.clone())),
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
								p.position.root_position().map(|p| p.volume_seq),
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

	/// A group fork acquire: the parent's state as of fork `fork_number` of its group. The
	/// name's current incarnation decides the answer by the fork it was created at and, once
	/// deleted, the fork it was deleted at; an incarnation created before the fork and alive
	/// at it answers with the pin covering the fork. No hold row is written: the pin's own
	/// `waiting` list is what keeps it. The receipt row is written as for any acquire, so a
	/// retry finds the same pin, and the incarnation's record is retained for the chain.
	///
	/// The acquire never pins: a pin is durable only as its own maintainer log op, so an
	/// uncovered fork answers `NeedsPin`, whether the parent is loaded, whose apply path pins
	/// it, or idle, which [`Self::plan_idle_fork_pin`] plans for the maintainer to log.
	pub async fn acquire_group_fork_reference(
		&self,
		spec: FsForkSpec,
		region: u16,
		max_depth: usize,
		loaded: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		logged: Option<i64>,
	) -> Outcome<GroupAcquire> {
		let FsForkSelector::FsForkGroup(group) = &spec.selector else {
			return Ok(Err(FsForkError::Invalid));
		};
		let fork_number = group.fork_number;
		// A group fork's rows exist only from `depot.fs_rows` version 2. A fresh grant writes
		// below it until its first pass makes the raised mark durable, so the acquire is
		// refused for a retry until then rather than failing to encode.
		if self.row_version() < super::fork_pins::FORK_PIN_ROW_VERSION {
			return Ok(Err(FsForkError::Busy));
		}
		let _ordered = self
			.lock(super::locks::LockRequest::fork(&spec), LockOperation::Other)
			.await;
		self.admit()?;
		if !valid_group_spec(&spec, region, max_depth, fork_number) {
			return Ok(Err(FsForkError::Invalid));
		}
		let hash = depot_types::partition_key_hash(&spec.parent_volume_key);
		if let Some(result) = self.lookup_fork_reference(&spec).await? {
			return Ok(result.map(|receipt| GroupAcquire::Receipt {
				receipt,
				metadata: None,
			}));
		}
		let mut current = match self
			.group_fork_source(&spec, fork_number, loaded, applied)
			.await?
		{
			Ok(GroupSource::Current(current)) => current,
			Ok(GroupSource::Answer(answer)) => return Ok(Ok(answer)),
			Err(error) => return Ok(Err(error)),
		};
		let scope = keys::Scope::new(
			&spec.parent_volume_key,
			Uuid::from_bytes(current.volume_incarnation),
		);
		// The pin covering the fork: pins cover contiguous ranges from the fork the
		// incarnation was created at, so at most one does.
		let Some((pin_key, pin)) = self.covering_fork_pin(&scope, fork_number).await? else {
			// A delete pins its incarnation before it drains, so a deleted incarnation with no
			// pin covering the fork was deleted before deletes pinned, and its data may be
			// partly swept: the fork is refused rather than read from what is left.
			if current.status == FsVolumeStatus::Deleted {
				tracing::warn!(
					?scope,
					fork_number,
					"a group fork acquire found the parent deleted with no fork pin covering the fork; the child cannot read it"
				);
				return Ok(Err(FsForkError::BaseUnavailable));
			}
			return Ok(Ok(GroupAcquire::NeedsPin));
		};
		if forks::initial_position(&pin.cut).is_err() {
			return Ok(Err(FsForkError::Invalid));
		}
		if current.base_chain.len() >= max_depth {
			return Ok(Err(FsForkError::ForkChainTooDeep));
		}
		let mut parent_base = current.base_chain.clone();
		for link in &mut parent_base {
			link.region.get_or_insert(region);
		}
		let receipt = FsForkReceipt {
			image: FsForkImage {
				link: FsBaseLink {
					volume_key: spec.parent_volume_key.clone(),
					incarnation: current.volume_incarnation,
					checkpoint_name: Vec::new().into(),
					position: pin.cut.clone(),
					region: Some(region),
					fork_pin: Some(pin_key.first_fork()),
				},
				parent_base,
			},
			spec,
		};
		let reference = keys::ForkReferenceKey::new(
			hash,
			receipt.spec.parent_volume_key.name.clone().map(Into::into),
			Uuid::from_bytes(receipt.spec.operation_id),
		);
		let now = match logged {
			Some(created_at) => created_at,
			None => self.clock.observe()?.unix_ms,
		};
		let mut batch = WriteBatch::new();
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
		let archive = keys::IncarnationRecordKey::new(hash, scope.name.clone(), scope.incarnation);
		let creation = self
			.db
			.get(archive.pack())
			.await?
			.map(|v| archive.deserialize(v))
			.transpose()?
			.and_then(|v| v.creation);
		self.limit_cold(&mut current);
		batch.put(
			archive.pack(),
			archive.serialize(AtVersion::new(
				FsIncarnationRecord {
					record: current.clone(),
					creation,
				},
				self.row_version(),
			))?,
		);
		self.stage(batch).await?;
		self.admit()?;
		// The fork lock writes the parent's stripe, which the session the metadata is read
		// through takes shared; the receipt is staged, so the read runs after the lock.
		drop(_ordered);
		let metadata = self.inline_metadata(&current, &receipt.image.link).await?;
		Ok(Ok(GroupAcquire::Receipt { receipt, metadata }))
	}

	/// The incarnation a group fork acquire of fork `fork_number` answers from, or the answer
	/// when the name's incarnations decide it without a pin. `loaded` is the record of a parent
	/// the tablet holds loaded and `applied` its applied position; an idle parent holds all it
	/// applied in its stored record.
	async fn group_fork_source(
		&self,
		spec: &FsForkSpec,
		fork_number: u64,
		loaded: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
	) -> Outcome<GroupSource> {
		let stored_pointer = self.pointer(&spec.parent_volume_key).await?;
		let stored = super::store::names_key(stored_pointer, &spec.parent_volume_key);
		if stored
			.as_ref()
			.zip(loaded.as_ref())
			.is_some_and(|(a, b)| a.volume_incarnation != b.volume_incarnation)
		{
			return Ok(Err(FsForkError::IncarnationChanged));
		}
		// A key with no current incarnation, or one created at or after the fork, may still have
		// had a deleted incarnation at the fork, whose pointer a newer incarnation of the name
		// took or whose sweep released it. That incarnation answers; with none, the key held
		// nothing at the fork, and the walk goes on to the next ancestor.
		let Some(mut current) = stored.or(loaded) else {
			return self.deleted_at_fork_or_continue(spec, fork_number).await;
		};
		ensure!(
			current.volume_key() == spec.parent_volume_key,
			"fork source record identity mismatch"
		);
		if current.created_at_fork.unwrap_or(0) >= fork_number {
			return self.deleted_at_fork_or_continue(spec, fork_number).await;
		}
		// Deleted before the fork: the volume did not exist at it, whatever the ancestors hold.
		if current.status == FsVolumeStatus::Deleted
			&& current.deleted_at_fork.unwrap_or(0) < fork_number
		{
			return Ok(Ok(GroupSource::Answer(GroupAcquire::Absent)));
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
		Ok(Ok(GroupSource::Current(current)))
	}

	/// The deleted incarnation of the parent key that existed at fork `fork_number`, as its
	/// fork index row names it, or `Continue`. One seek reads the key's first incarnation deleted
	/// at or after the fork, and one point read its record: the incarnations of one key follow
	/// one another, so that is the only one that can span the fork, and it does when it was
	/// created before it.
	async fn deleted_at_fork_or_continue(
		&self,
		spec: &FsForkSpec,
		fork_number: u64,
	) -> Outcome<GroupSource> {
		let key = &spec.parent_volume_key;
		let hash = depot_types::partition_key_hash(key);
		let name: Option<Vec<u8>> = key.name.clone().map(Into::into);
		let partition_key: bytes::Bytes = key.partition_key.clone().into();
		let prefix =
			keys::DeletedIncarnationPrefix::new(hash, name.clone(), partition_key.clone()).pack();
		let from =
			keys::DeletedIncarnationKey::new(hash, name, partition_key, fork_number, Uuid::nil())
				.pack();
		let mut rows = self
			.db
			.scan_prefix(
				prefix.clone(),
				(
					std::ops::Bound::Included(from.slice(prefix.len()..)),
					std::ops::Bound::<bytes::Bytes>::Unbounded,
				),
			)
			.await?;
		let first = rows.next().await?;
		self.admit()?;
		let Some(row) = first else {
			return Ok(Ok(GroupSource::Answer(GroupAcquire::Continue)));
		};
		let index = keys::DeletedIncarnationKey::unpack(&row.key)?;
		let Some(record) = self
			.incarnation_record(key, *index.incarnation.as_bytes())
			.await?
		else {
			return Ok(Ok(GroupSource::Answer(GroupAcquire::Continue)));
		};
		if record.created_at_fork.unwrap_or(0) >= fork_number {
			return Ok(Ok(GroupSource::Answer(GroupAcquire::Continue)));
		}
		Ok(Ok(GroupSource::Current(record)))
	}

	/// Plans the fork pin of a parent volume its tablet does not hold loaded, for a group
	/// fork acquire of fork `fork_number`, and writes nothing. The pin stands at the parent's
	/// stored cut, which holds everything it applied, for the forks after its last pin through
	/// `fork_number`, waiting on their children that are not archived. The maintainer logs the
	/// returned pin as a `ForkPinOp` and applies it before the acquire is tried again, so the
	/// pin is durable in the log before any receipt or reply names it, and a replay rebuilds
	/// it. `children` are the group's fork children the maintainer's cache knows.
	pub async fn plan_idle_fork_pin(
		&self,
		spec: &FsForkSpec,
		region: u16,
		children: &GroupChildren,
	) -> Result<IdlePinPlan> {
		let FsForkSelector::FsForkGroup(group) = &spec.selector else {
			return Ok(IdlePinPlan::Pinned);
		};
		let fork_number = group.fork_number;
		if self.row_version() < super::fork_pins::FORK_PIN_ROW_VERSION {
			return Ok(IdlePinPlan::Busy);
		}
		let _ordered = self
			.lock(super::locks::LockRequest::fork(spec), LockOperation::Other)
			.await;
		self.admit()?;
		if !valid_group_spec(spec, region, MAX_FORK_CHAIN_DEPTH, fork_number) {
			return Ok(IdlePinPlan::Pinned);
		}
		let current = match self
			.group_fork_source(spec, fork_number, None, None)
			.await?
		{
			Ok(GroupSource::Current(current)) => current,
			// The acquire answers these itself.
			Ok(GroupSource::Answer(_)) | Err(_) => return Ok(IdlePinPlan::Pinned),
		};
		let scope = keys::Scope::new(
			&spec.parent_volume_key,
			Uuid::from_bytes(current.volume_incarnation),
		);
		let pins = self.fork_pins(&scope).await?;
		if pins.iter().any(|(key, pin)| covers(key, pin, fork_number)) {
			return Ok(IdlePinPlan::Pinned);
		}
		// A deleted incarnation's delete logged its pin before it drained; a pin planned now
		// could stand over data its sweep already removed, so the acquire answers instead.
		if current.status == FsVolumeStatus::Deleted {
			return Ok(IdlePinPlan::Pinned);
		}
		// The stored value covers forks whose pin waited on no one or was deleted by a pin
		// pass, as at load; the rows cover the rest.
		let first_fork = super::fork_pins::pinned_through(current.created_at_fork, &pins)
			.max(current.fork_pinned_through)
			.unwrap_or(0)
			+ 1;
		if first_fork > fork_number {
			return Ok(IdlePinPlan::Busy);
		}
		if !(first_fork..=fork_number).all(|fork| children.children.contains_key(&fork)) {
			return Ok(IdlePinPlan::NeedsChildren { first_fork });
		}
		let waiting: Vec<[u8; 16]> = children
			.children
			.range(first_fork..=fork_number)
			.filter(|(_, (_, archived))| !archived)
			.map(|(_, (child, _))| *child)
			.collect();
		if waiting.is_empty() {
			return Ok(IdlePinPlan::Busy);
		}
		// An idle volume holds all it applied at its stored cold position.
		let cut = FsCut::single(current.cold.clone());
		Ok(IdlePinPlan::Pin(
			rivet_data::generated::depot_maintainer_log_v2::ForkPinOp {
				volume_log_key: rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
					volume_key: spec.parent_volume_key.clone(),
					volume_incarnation: current.volume_incarnation,
				},
				cut,
				first_fork,
				last_fork: fork_number,
				waiting,
				created_at_unix_ms: self.clock.observe().map_err(anyhow::Error::from)?.unix_ms,
			},
		))
	}

	/// The volume's metadata rows at `link`'s cut when they fit the inline budget in one page,
	/// else `None`: the fork then reads them through the chain. One bounded scan; the budget
	/// is counted as the rows come, so a large volume costs one page and nothing more.
	async fn inline_metadata(
		&self,
		record: &FsVolumeRecord,
		link: &FsBaseLink,
	) -> Result<Option<rivet_data::generated::shared_depot_fs_read_v2::FsForkMetadataPage>> {
		use depot_fs_core::store::RowRange;
		use rivet_data::generated::shared_depot_fs_read_v2::{FsForkMetadataPage, FsReadRow};
		let limit = self.batch_rows.min(self.config.max_rows);
		if limit == 0 || self.config.inline_metadata_bytes == 0 {
			return Ok(None);
		}
		let session = match self.open_checkpoint_session(link).await {
			Ok(session) => session,
			Err(error) => {
				tracing::debug!(
					?error,
					"a group acquire could not read the pinned metadata inline"
				);
				return Ok(None);
			}
		};
		let result = session.scan(&RowRange::MetadataAll, None, limit).await;
		self.close_session(session.id);
		self.admit()?;
		let rows = match result {
			Ok(rows) => rows,
			Err(error) => {
				tracing::debug!(
					?error,
					"a group acquire could not read the pinned metadata inline"
				);
				return Ok(None);
			}
		};
		if rows.len() >= limit {
			return Ok(None);
		}
		let mut bytes = 0usize;
		let mut page = Vec::with_capacity(rows.len());
		for (key, claim_log, value) in rows {
			bytes = bytes.saturating_add(inline_row_bytes(&key, &value));
			if bytes > self.config.inline_metadata_bytes {
				return Ok(None);
			}
			page.push(FsReadRow {
				change_seq: None,
				key: key.into(),
				value: value.into(),
				claim_log,
			});
		}
		tracing::debug!(
			volume_incarnation = ?Uuid::from_bytes(record.volume_incarnation),
			rows = page.len(),
			bytes,
			"a group acquire carries the pinned metadata inline"
		);
		Ok(Some(FsForkMetadataPage {
			rows: page,
			complete: true,
		}))
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
					fork_pin: None,
				},
				parent_base,
			},
			spec,
		};
		let scope = keys::Scope::new(
			&receipt.spec.parent_volume_key,
			Uuid::from_bytes(point.volume_incarnation),
		);
		// The parent has one claim log, so the point's cut names claim log 0 alone.
		let point_seq = point.position.single_position()?.volume_seq;
		let checkpoint = address(&scope, point_seq, point.name.to_vec());
		let hold = keys::ForkHoldKey::new(
			hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			point_seq,
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
					cut: Some(point.position.clone()),
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
		self.limit_cold(&mut current);
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
			retention
				.pins
				.insert(super::filter::PinnedCut::single(point_seq));
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
		let link_position = link.position.single_position()?;
		let checkpoint = address(
			&scope,
			link_position.volume_seq,
			link.checkpoint_name.to_vec(),
		);
		let value = checkpoint.deserialize(
			self.db
				.get(checkpoint.pack())
				.await?
				.ok_or_else(|| anyhow::anyhow!("held checkpoint missing"))?,
		)?;
		ensure!(
			value.client_epoch == link_position.client_epoch,
			"held checkpoint position mismatch"
		);
		let hold = keys::ForkHoldKey::new(
			hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			link_position.volume_seq,
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
			link_position.volume_seq,
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
	/// `children` is given only to a group acquire's lookup of an idle parent, which then
	/// plans the parent's pin when no receipt exists.
	pub async fn fork_reference_request(
		&self,
		request: rivet_data::generated::depot_client_protocol_v2::FsForkReferenceRequest,
		region: u16,
		barrier: depot_fs_core::forks::ReferenceBarrier,
		record: Option<FsVolumeRecord>,
		applied: Option<VolumeLogPosition>,
		logged: Option<i64>,
		loaded_parent_incarnations: &[[u8; 16]],
		children: Option<&GroupChildren>,
	) -> anyhow::Result<depot_fs_core::forks::ReferenceResult> {
		use depot_fs_core::forks::{ReferenceBarrier, ReferenceResult};
		use rivet_data::generated::depot_client_protocol_v2::{
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
			if let Some(result) = self.lookup_fork_reference(&v.spec).await? {
				return Ok(ReferenceResult::Complete {
					response: Box::new(match result {
						Ok(receipt) => Response::FsForkReceipt(receipt),
						Err(error) => Response::FsForkError(error),
					}),
					record: None,
				});
			}
			// With the group's children, the lookup also plans an idle parent's pin.
			let Some(children) = children else {
				return Ok(ReferenceResult::SourceProgressRequired);
			};
			return Ok(
				match self.plan_idle_fork_pin(&v.spec, region, children).await? {
					IdlePinPlan::Pinned => ReferenceResult::SourceProgressRequired,
					IdlePinPlan::Pin(pin) => ReferenceResult::NeedsIdlePin(Box::new(pin)),
					IdlePinPlan::NeedsChildren { first_fork } => {
						ReferenceResult::NeedsGroupChildren { first_fork }
					}
					IdlePinPlan::Busy => ReferenceResult::Complete {
						response: Box::new(Response::FsForkError(FsForkError::Busy)),
						record: None,
					},
				},
			);
		}
		let (response, record) = match request {
			Request::FsForkAcquire(v)
				if matches!(v.spec.selector, FsForkSelector::FsForkGroup(_)) =>
			{
				match self
					.acquire_group_fork_reference(
						v.spec,
						region,
						MAX_FORK_CHAIN_DEPTH,
						record,
						applied,
						logged,
					)
					.await?
				{
					Ok(GroupAcquire::Receipt { receipt, metadata }) => (
						Response::FsForkGroupReceipt(
							rivet_data::generated::depot_client_protocol_v2::FsForkGroupReceipt {
								receipt,
								metadata,
							},
						),
						None,
					),
					Ok(GroupAcquire::Absent) => (Response::FsForkGroupAbsent, None),
					Ok(GroupAcquire::Continue) => (Response::FsForkGroupContinue, None),
					Ok(GroupAcquire::NeedsPin) => (Response::FsForkError(FsForkError::Busy), None),
					Err(error) => (Response::FsForkError(error), None),
				}
			}
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

/// The incarnation a group fork acquire answers from, or the answer its name gives.
enum GroupSource {
	Current(FsVolumeRecord),
	Answer(GroupAcquire),
}

/// Whether the pin at `key` covers fork `fork_number`. Pins cover contiguous ranges from the
/// fork their incarnation was created at, so at most one does.
fn covers(key: &keys::ForkPinKey, pin: &FsForkPin, fork_number: u64) -> bool {
	key.first_fork() <= fork_number && fork_number <= pin.last_fork
}

/// Whether a group fork spec may be answered in `region` with a chain of at most `max_depth`.
fn valid_group_spec(spec: &FsForkSpec, region: u16, max_depth: usize, fork_number: u64) -> bool {
	spec.parent_region == region
		&& forks::valid_spec(spec, spec.destination_region)
		&& max_depth != 0
		&& max_depth <= MAX_FORK_CHAIN_DEPTH
		&& fork_number != 0
}

/// The bytes a metadata row costs the inline budget: its variable parts, names and values,
/// over a fixed share for the rest of its encoding.
fn inline_row_bytes(
	key: &depot_fs_core::store::RowKey,
	value: &depot_fs_core::store::Row,
) -> usize {
	use depot_fs_core::store::{Row, RowKey};
	const FIXED: usize = 64;
	let key_bytes = match key {
		RowKey::Dentry(_, name) | RowKey::Xattr(_, name) | RowKey::Kv(_, name) => name.len(),
		RowKey::Link(_, _, name) => name.len(),
		RowKey::Format | RowKey::Inode(_) | RowKey::Orphan(_) | RowKey::Chunk(..) => 0,
		RowKey::KvRangeWhiteout(_, start) => start.len(),
	};
	let value_bytes = match value {
		Row::Bytes(bytes) => bytes.len(),
		Row::Format(_) | Row::Inode(_) | Row::Dentry(_) | Row::Unit => 0,
		Row::KvRangeWhiteout(end) => end.as_ref().map_or(0, Vec::len),
	};
	FIXED.saturating_add(key_bytes).saturating_add(value_bytes)
}
