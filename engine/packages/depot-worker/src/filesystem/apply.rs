//! Bounded transaction replay. All parts write at the commit sequence; no read
//! session is published there until the ordered store operation completes.
//!
//! An apply holds the tablet lock shared and its volume stripe exclusively, and releases them while a transaction page is
//! fetched from a replica. Applies of other volumes and read-session opens may hold it
//! meanwhile; the maintainer never runs either beside this volume's apply, and sessions
//! are bounded below the commit sequence that the partial replay stages. Mutations
//! that span volumes hold the lock exclusively.
use super::{
	session::ReadSession,
	store::{FilesystemStore, LockOperation},
};
use crate::tablet_maintainer_machine::types::ApplyAuthority;
use anyhow::{Result, ensure};
use depot_fs_core::{
	keys::slatedb as keys,
	store::{Batch, Row, RowKey},
	txn::{Resolution, Resolver},
	*,
};
use depot_types::VolumeLogKey;
use futures_util::future::BoxFuture;
use rivet_data::{
	generated::{
		depot_log_shard_v1::{VolumeLogEntry, VolumeLogEntryBody},
		shared_depot_types_v1::VolumeLogPosition,
	},
	versioned::DepotFsEntryData,
};
use std::collections::BTreeMap;
use uuid::Uuid;
use vbare::Versioned;

/// Re-reads one bounded page from an authoritative replica. Implementations
/// validate the original replica proof and grant on admission and completion.
pub trait TransactionReader: Send + Sync {
	fn read(&self, first: u64) -> BoxFuture<'_, Result<Vec<VolumeLogEntry>>>;
}

/// What applying one entry or transaction part leaves for after its rows are staged.
struct Applied {
	checkpoints: BTreeMap<(u64, Vec<u8>), FsCheckpoint>,
	admin: Vec<(u64, FsOp)>,
}

impl Applied {
	/// The checkpoints of a transaction part, which `txn::validate` keeps free of
	/// administrative ops.
	fn into_rows(self) -> Result<BTreeMap<(u64, Vec<u8>), FsCheckpoint>> {
		ensure!(
			self.admin.is_empty(),
			"administrative op inside a transaction"
		);
		Ok(self.checkpoints)
	}
}

impl FilesystemStore {
	/// Convenience for complete contiguous images, including a transaction wholly
	/// contained in this slice. Continuous maintainers retain their Resolver.
	pub async fn apply(
		&self,
		key: &VolumeLogKey,
		previous: &VolumeLogPosition,
		entries: &[VolumeLogEntry],
	) -> Result<VolumeLogPosition> {
		struct Slice<'a>(&'a [VolumeLogEntry]);
		impl TransactionReader for Slice<'_> {
			fn read(&self, first: u64) -> BoxFuture<'_, Result<Vec<VolumeLogEntry>>> {
				Box::pin(async move {
					Ok(self
						.0
						.iter()
						.filter(|e| e.volume_seq >= first)
						.cloned()
						.collect())
				})
			}
		}
		let (_, claim_log) = keys::Scope::of_log(
			&key.0.volume_key,
			Uuid::from_bytes(key.0.volume_incarnation),
		);
		let mut resolver = Resolver::new(claim_log, previous.clone());
		self.apply_resolved(key, &mut resolver, entries, &Slice(entries))
			.await?;
		ensure!(
			resolver.open().is_empty(),
			"continuous transaction apply requires a retained resolver"
		);
		Ok(resolver.position().clone())
	}

	pub async fn apply_resolved(
		&self,
		key: &VolumeLogKey,
		resolver: &mut Resolver,
		entries: &[VolumeLogEntry],
		reader: &dyn TransactionReader,
	) -> Result<()> {
		self.apply_resolved_under(
			key,
			ApplyAuthority::Current,
			None,
			resolver,
			entries,
			reader,
		)
		.await
	}

	/// `apply_resolved` under an explicit authority over the incarnation's record; see
	/// [`ApplyAuthority`]. `loaded` is the record as the maintainer holds it, which a logged
	/// retention policy applies to in the stored record's place; only a page carrying one
	/// needs it.
	pub async fn apply_resolved_under(
		&self,
		key: &VolumeLogKey,
		authority: ApplyAuthority,
		loaded: Option<&FsVolumeRecord>,
		resolver: &mut Resolver,
		entries: &[VolumeLogEntry],
		reader: &dyn TransactionReader,
	) -> Result<()> {
		self.apply_resolved_tracking_alarm(key, authority, loaded, resolver, entries, reader)
			.await
			.map(|_| ())
	}

	/// Tracks alarm index changes while preserving the admitted authority over the record.
	pub(crate) async fn apply_resolved_tracking_alarm(
		&self,
		key: &VolumeLogKey,
		authority: ApplyAuthority,
		loaded: Option<&FsVolumeRecord>,
		resolver: &mut Resolver,
		entries: &[VolumeLogEntry],
		reader: &dyn TransactionReader,
	) -> Result<Option<rivet_data::generated::shared_depot_alarms_v1::AlarmSummary>> {
		let mut alarm_changed = false;
		let mut next_resolver = resolver.clone();
		// The core runs no other apply, lookup or writer of this volume beside it, so in
		// `core` mode it takes only the tablet lock.
		let request = self.locks.core_ordered(
			super::locks::LockRequest::shared().write(super::locks::stripe(&key.0.volume_key)),
		);
		let _running = self.enter_volumes([(
			super::locks::StoreVolume::from(&key.0.volume_key),
			super::locks::VolumeAccess::Keyed,
		)]);
		let mut ordered = Some(self.lock(request.clone(), LockOperation::Apply).await);
		self.admit()?;
		// A claim log writes its base volume's rows, stamped with its own claim log.
		let (scope, claim_log) = keys::Scope::of_log(
			&key.0.volume_key,
			Uuid::from_bytes(key.0.volume_incarnation),
		);
		if let Some(retention) = self.catalog.lock().get(&scope) {
			ensure!(
				resolver.cold().volume_seq >= retention.cold_of(claim_log),
				"filesystem replay precedes persisted cold position"
			);
		}
		let record = self
			.incarnation_row(&key.0.volume_key, key.0.volume_incarnation)
			.await?;
		// A volume that reads its metadata through its base keeps the range whiteouts its
		// range deletes leave; one that holds its own metadata reads nothing through them.
		let reads_through = record
			.as_ref()
			.is_some_and(|record| !record.base_chain.is_empty() && !record.metadata_local);
		if let Some(record) = &record {
			let admitted = match authority {
				ApplyAuthority::Current => record.status != FsVolumeStatus::Deleted,
				// A recovered delete applies the tail its tombstones kept under the
				// incarnation's own keys, whatever its record says.
				ApplyAuthority::RetiredTail => true,
			};
			ensure!(
				admitted,
				"cannot apply to a retired filesystem incarnation {}",
				Uuid::from_bytes(key.0.volume_incarnation),
			);
		}
		for entry in entries {
			let previous = next_resolver.position().clone();
			match next_resolver.push(entry)? {
				Resolution::Standalone(decoded) => {
					alarm_changed |= decoded.ops.iter().any(|op| {
						matches!(op, FsOp::FsKvPut(v) if v.ino == 0
							&& v.key.as_ref() == depot_types::alarm::COLD_ALARM_KEY)
					});
					let applied = self
						.apply_part(
							&scope,
							claim_log,
							entry.volume_seq,
							entry.client_epoch,
							previous.volume_seq,
							&decoded,
							reads_through,
						)
						.await?;
					self.finish_checkpoints(&scope, entry.client_epoch, applied.checkpoints)
						.await?;
					self.apply_admin(&scope, &key.0.volume_key, loaded, applied.admin)
						.await?;
				}
				Resolution::Commit {
					id,
					open,
					entry: commit,
				} => {
					alarm_changed = true;
					self.apply_transaction(
						&scope,
						claim_log,
						&open,
						&id,
						entry.volume_seq,
						entry.client_epoch,
						&commit,
						reads_through,
						reader,
						&request,
						&mut ordered,
					)
					.await?;
				}
				Resolution::Pending | Resolution::Discarded => {}
			}
		}
		self.admit()?;
		self.catalog
			.lock()
			.entry(scope)
			.or_default()
			.apply_cold
			.insert(claim_log, next_resolver.cold().clone());
		// The index reads the volume's own alarm row only, never through a lazy fork's lineage,
		// so this holds no apply lock across another volume's reads. A lazily forked volume's
		// inherited alarm is indexed once the child writes the row: actor and filesystem state
		// volumes, the only volumes with alarms, are written solely by their durable state
		// machine, whose first commit in a forked incarnation arms the alarm again.
		let alarm = if alarm_changed {
			self.update_alarm_index(key, next_resolver.position().clone())
				.await?
		} else {
			None
		};
		*resolver = next_resolver;
		Ok(alarm)
	}

	/// Streams the logical concatenation through the existing per-entry plan.
	/// Scans at C include preceding parts at C, exactly like preceding puts in a
	/// merged entry. Staging never publishes C, including after partial replay.
	///
	/// `ordered` holds the tablet lock shared and this volume's stripe on entry and on
	/// return. Both are released while a page is in flight, so a slow replica never
	/// blocks read-session opens, and reacquired tablet first.
	#[allow(clippy::too_many_arguments)]
	async fn apply_transaction<'a>(
		&'a self,
		scope: &keys::Scope,
		claim_log: u16,
		open: &txn::OpenTxn,
		id: &[u8],
		seq: u64,
		epoch: u64,
		commit: &FsEntry,
		reads_through: bool,
		reader: &dyn TransactionReader,
		request: &super::locks::LockRequest,
		ordered: &mut Option<super::locks::StoreGuard<'a>>,
	) -> Result<()> {
		let mut next = open.first.volume_seq;
		let mut checkpoints = BTreeMap::new();
		while next < seq {
			self.admit()?;
			let start = next;
			drop(ordered.take());
			let page = reader.read(next).await;
			*ordered = Some(self.lock(request.clone(), LockOperation::Apply).await);
			self.admit()?;
			let page = page?;
			ensure!(!page.is_empty(), "missing pending transaction entries");
			for entry in page.into_iter().take_while(|e| e.volume_seq < seq) {
				ensure!(
					entry.volume_seq == next && entry.client_epoch == epoch,
					"transaction replay changed history"
				);
				next += 1;
				let VolumeLogEntryBody::PayloadWrite(payload) = entry.body else {
					anyhow::bail!("transaction crossed a new claim");
				};
				let raw = depot_types::payload_codec::decode(&payload)?;
				let part = DepotFsEntryData::deserialize_with_embedded_version(&raw)?;
				txn::validate(&part)?;
				if part.txn.as_ref().is_some_and(|v| txn::id(v) == id) {
					ensure!(
						matches!(part.txn, Some(FsTxn::FsTxnPending(_))),
						"transaction already closed"
					);
					checkpoints.extend(
						self.apply_part(scope, claim_log, seq, epoch, seq, &part, reads_through)
							.await?
							.into_rows()?,
					);
				}
			}
			ensure!(next > start, "transaction replay did not advance");
		}
		checkpoints.extend(
			self.apply_part(scope, claim_log, seq, epoch, seq, commit, reads_through)
				.await?
				.into_rows()?,
		);
		self.finish_checkpoints(scope, epoch, checkpoints).await
	}

	async fn apply_part(
		&self,
		scope: &keys::Scope,
		claim_log: u16,
		seq: u64,
		epoch: u64,
		before_seq: u64,
		entry: &FsEntry,
		reads_through: bool,
	) -> Result<Applied> {
		let mut plan = apply::plan(claim_log, seq, entry)?;
		let before = ReadSession::internal(
			self.db.snapshot().await?,
			scope.clone(),
			VolumeLogPosition {
				volume_seq: before_seq,
				client_epoch: epoch,
			},
			self.clock.clone(),
			self.guard.clone(),
			self.config.clone(),
		);
		self.plan_range_whiteouts(&before, &mut plan, reads_through)
			.await?;
		// The final rows are encoded before any row of the entry is staged, so a row the store
		// cannot encode yet refuses the entry with nothing of it written. A range delete's
		// tombstones are encoded page by page as they are read; they carry no value, so only a
		// claim log that the current version cannot name refuses one, and the pages staged
		// before it stay hidden at the entry's sequence until the replay stages them again.
		let checkpoints = std::mem::take(&mut plan.final_rows.checkpoints);
		let admin = std::mem::take(&mut plan.final_rows.admin);
		let final_rows = self.encode_batch(scope, claim_log, seq, plan.final_rows)?;
		for range in plan.ranges {
			let mut cursor = None;
			loop {
				let rows = before
					.scan_for_apply(
						&range,
						cursor.as_ref(),
						self.batch_rows.min(self.config.max_rows),
					)
					.await?;
				if rows.is_empty() {
					break;
				}
				cursor = rows.last().map(|(key, _, _)| key.clone());
				let batch = Batch {
					rows: rows.into_iter().map(|(key, _, _)| (key, None)).collect(),
					checkpoints: BTreeMap::new(),
					admin: Vec::new(),
				};
				self.stage(self.encode_batch(scope, claim_log, seq, batch)?)
					.await?;
			}
		}
		self.stage(final_rows).await?;
		Ok(Applied { checkpoints, admin })
	}

	/// Adds the range whiteouts the plan's KV range deletes leave on a volume with a base,
	/// which hide the keys of each range that a base link holds; a volume without a base reads
	/// nothing through a whiteout and writes none. Each is coalesced with the file's existing
	/// whiteouts it overlaps or touches, so a file keeps one row per disjoint deleted range. Only
	/// those whiteouts are read, never every whiteout of the file, so no count of earlier range
	/// deletes can fail a committed one. The existing whiteouts are read at the sequence before
	/// the entry, so a replay settles them the same.
	async fn plan_range_whiteouts(
		&self,
		before: &ReadSession,
		plan: &mut apply::ApplyPlan,
		reads_through: bool,
	) -> Result<()> {
		use depot_fs_core::{overlay, store::RowRange};
		if !reads_through {
			return Ok(());
		}

		// The entry's own ranges merged first, so each merged range is disjoint from the others
		// and does not touch them.
		let mut added: Vec<overlay::RangeWhiteout> = Vec::new();
		for range in &plan.ranges {
			if let RowRange::Kv { ino, start, end } = range {
				added = overlay::coalesce(
					added,
					overlay::RangeWhiteout {
						ino: *ino,
						start: start.clone(),
						end: end.clone(),
					},
				);
			}
		}
		if added.is_empty() {
			return Ok(());
		}

		// Each range takes in the whiteout at or before its start when that one reaches it, then
		// every whiteout starting inside it or where it ends, growing as they extend it. A range
		// that touches the one written before it in the same file joins it instead.
		let page = self.batch_rows.min(self.config.max_rows).max(1);
		let mut merged: Vec<overlay::RangeWhiteout> = Vec::new();
		let mut removed: BTreeMap<(u64, Vec<u8>), overlay::RangeWhiteout> = BTreeMap::new();
		for range in added {
			let mut whiteout = match merged.pop() {
				Some(mut previous)
					if previous.ino == range.ino && reaches(&previous, &range.start) =>
				{
					previous.end = wider_end(previous.end.take(), range.end.clone());
					previous
				}
				previous => {
					merged.extend(previous);
					let mut whiteout = range.clone();
					let before_start = before
						.own_whiteout_at_or_before(range.ino, &range.start)
						.await?;
					if let Some(existing) = before_start
						&& reaches(&existing, &range.start)
					{
						whiteout.start = existing.start.clone();
						whiteout.end = wider_end(whiteout.end.take(), existing.end.clone());
						removed.insert((existing.ino, existing.start.clone()), existing);
					}
					whiteout
				}
			};

			// Whiteouts starting after the range's start, up to where the merged range ends.
			let mut cursor = range.start.clone();
			loop {
				let next = before
					.own_whiteouts_after(whiteout.ino, &cursor, whiteout.end.as_deref(), page)
					.await?;
				let Some(last) = next.last() else {
					break;
				};
				cursor = last.start.clone();
				for existing in next {
					whiteout.end = wider_end(whiteout.end.take(), existing.end.clone());
					removed.insert((existing.ino, existing.start.clone()), existing);
				}
			}
			merged.push(whiteout);
		}

		// Only the whiteouts the delete changed are written: the ones it took in as tombstones,
		// unless a merged whiteout keeps the same start, and new or widened ones as rows.
		let batch = &mut plan.final_rows;
		for ((ino, start), _) in &removed {
			let kept = merged
				.iter()
				.any(|whiteout| whiteout.ino == *ino && &whiteout.start == start);
			if !kept {
				batch
					.rows
					.insert(RowKey::KvRangeWhiteout(*ino, start.clone()), None);
			}
		}
		for whiteout in merged {
			let unchanged = removed
				.get(&(whiteout.ino, whiteout.start.clone()))
				.is_some_and(|existing| existing == &whiteout);
			if unchanged {
				continue;
			}
			batch.rows.insert(
				RowKey::KvRangeWhiteout(whiteout.ino, whiteout.start),
				Some(Row::KvRangeWhiteout(whiteout.end)),
			);
		}
		Ok(())
	}

	/// Applies the administrative ops of one standalone entry after its rows. Each is
	/// idempotent, so a replay from an older cold position converges on the same rows. None
	/// flushes: the op's own committed log entry is its durable record until the pass that
	/// moves cold past it.
	async fn apply_admin(
		&self,
		scope: &keys::Scope,
		volume_key: &depot_types::VolumeKey,
		loaded: Option<&FsVolumeRecord>,
		admin: Vec<(u64, FsOp)>,
	) -> Result<()> {
		for (seq, op) in admin {
			self.admit()?;
			match op {
				FsOp::FsCheckpointRelease(release) => {
					self.release_logged_checkpoint(scope, volume_key, seq, &release)
						.await?;
				}
				FsOp::FsForkHoldRelease(release) => {
					ensure!(
						release.spec.parent_volume_key == *volume_key,
						"fork hold release logged in another volume"
					);
					// The incarnation applying this release is live, or its delete's drain
					// applies it and that delete owns its sweep.
					let applying = [*scope.incarnation.as_bytes()];
					if let Err(error) = self
						.release_fork_reference_unlocked(release.spec, false, &applying)
						.await?
					{
						// A conflicting reference belongs to another fork with this id;
						// the logged release has nothing of its own to remove.
						tracing::warn!(
							?error,
							"logged fork hold release found no hold of its fork"
						);
					}
				}
				FsOp::FsForkCheckDone(done) => {
					ensure!(
						done.spec.parent_volume_key == *volume_key,
						"fork check logged in another volume"
					);
					if let Err(error) = self.finish_fork_check_unlocked(done.spec, false).await? {
						tracing::warn!(?error, "logged fork check found no reference of its fork");
					}
				}
				// The maintainer core owns the record, so it takes the policy when this apply
				// succeeds, and the next pass writes it.
				FsOp::FsRetentionSet(set) => {
					self.apply_logged_retention(scope, volume_key, &set, loaded)
						.await?;
				}
				FsOp::FsRoot(_)
				| FsOp::FsCreate(_)
				| FsOp::FsMkdir(_)
				| FsOp::FsSymlink(_)
				| FsOp::FsMknod(_)
				| FsOp::FsLink(_)
				| FsOp::FsUnlink(_)
				| FsOp::FsRename(_)
				| FsOp::FsSetAttr(_)
				| FsOp::FsWrite(_)
				| FsOp::FsSetXattr(_)
				| FsOp::FsRemoveXattr(_)
				| FsOp::FsReclaim(_)
				| FsOp::FsKvPut(_)
				| FsOp::FsKvDelete(_)
				| FsOp::FsKvDeleteRange(_)
				| FsOp::FsCheckpoint(_) => anyhow::bail!("row op planned as administrative"),
			}
		}
		Ok(())
	}
	async fn finish_checkpoints(
		&self,
		scope: &keys::Scope,
		epoch: u64,
		checkpoints: BTreeMap<(u64, Vec<u8>), FsCheckpoint>,
	) -> Result<()> {
		let mut batch = slatedb::WriteBatch::new();
		let written = self
			.checkpoint_entries(scope, epoch, checkpoints, &mut batch)
			.await?;
		self.stage(batch).await?;
		// A flush retention publish of this scope may run beside the apply and may have
		// replaced the pins added before the write; restore them and count the write.
		self.pin_written_checkpoints(scope, &written);
		Ok(())
	}
}

pub(crate) struct ReplicaTransactionReader<'a> {
	pub clock: std::sync::Arc<dyn rivet_env::Clock>,
	pub read_timeout_ms: u64,
	pub read_deadline_boot_ms: u64,
	pub replicas: &'a dyn crate::tablet_maintainer_machine::runner::ReplicaTransport,
	pub source: Option<(
		rivet_topology::worker::WorkerId,
		rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState,
	)>,
	pub key: VolumeLogKey,
	pub through: u64,
	pub guard: std::sync::Arc<dyn rivet_env::MutationGuard>,
	pub max_bytes: u32,
	/// The final tail a delete's drain fixed, when the entries are read for one: a tombstone
	/// holding it is a source, and the tail counts as committed on it.
	pub drain_through: Option<VolumeLogPosition>,
}
impl TransactionReader for ReplicaTransactionReader<'_> {
	fn read(&self, first: u64) -> BoxFuture<'_, Result<Vec<VolumeLogEntry>>> {
		Box::pin(async move {
			use rivet_data::generated::{
				depot_storage_worker_protocol_v1 as wire,
				shared_depot_types_v1::VolumeLogReplicaStatus,
			};
			ensure!(self.guard.may_send(), "transaction replay grant revoked");
			let (worker, proof) = self
				.source
				.as_ref()
				.ok_or_else(|| anyhow::anyhow!("transaction source unavailable"))?;
			let unavailable = |message: String| {
				crate::tablet_maintainer_machine::types::FilesystemApplyError::ReplicaRead {
					worker: worker.clone(),
					message,
				}
			};
			let deadline = self
				.clock
				.observe()?
				.boot_ms
				.saturating_add(self.read_timeout_ms)
				.min(self.read_deadline_boot_ms);
			let reply = rivet_env::timeout_at(
				self.clock.as_ref(),
				deadline,
				self.replicas.request(
					worker.clone(),
					wire::StorageWorkerRequest::ReadVolumeLogEntriesRequest(
						wire::ReadVolumeLogEntriesRequest {
							include_deleted: self.drain_through.is_some(),
							volume_log_key: self.key.0.clone(),
							first_volume_seq: first,
							max_bytes: self.max_bytes,
						},
					),
				),
			)
			.await;
			// Authority loss dominates even a transport failure: a retry must not
			// turn a revoked grant into permission to keep using this writer.
			ensure!(self.guard.may_send(), "transaction replay grant revoked");
			let reply = reply
				.map_err(|e| unavailable(e.to_string()))?
				.map_err(unavailable)?;
			let wire::StorageWorkerResponse::ReadVolumeLogEntriesResponse(page) = reply else {
				return Err(unavailable("transaction source rejected read".into()).into());
			};
			// A delete's drain reads its final tail from a tombstone, which keeps that tail.
			let readable =
				|state: &rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState| {
					state.status == VolumeLogReplicaStatus::InSync
						|| (state.status == VolumeLogReplicaStatus::Deleted
							&& self.drain_through.is_some())
				};
			// The entries through `self.through` were proven committed when the apply
			// started, and a committed entry is never cut from a replica, so the source
			// still holds them while it keeps its incarnation and a tail at or past them.
			if !readable(&page.state)
				|| page.state.replica_incarnation != proof.replica_incarnation
				|| page.state.appended.volume_seq < self.through
				|| proof.appended.volume_seq < self.through
				|| page.state.cold.volume_seq >= first
			{
				return Err(unavailable("transaction source changed".into()).into());
			}
			ensure!(
				page.entries
					.iter()
					.map(depot_types::log_shard::entry_bytes)
					.sum::<u64>() <= u64::from(self.max_bytes),
				"transaction source exceeded read bound"
			);

			Ok(page.entries)
		})
	}
}

/// Whether a range starting at `start` overlaps or touches `whiteout`, so the two coalesce.
fn reaches(whiteout: &depot_fs_core::overlay::RangeWhiteout, start: &[u8]) -> bool {
	whiteout.start.as_slice() <= start && whiteout.end.as_deref().is_none_or(|end| start <= end)
}

/// The later of two exclusive range ends, where `None` is unbounded.
fn wider_end(a: Option<Vec<u8>>, b: Option<Vec<u8>>) -> Option<Vec<u8>> {
	match (a, b) {
		(Some(a), Some(b)) => Some(a.max(b)),
		(None, _) | (_, None) => None,
	}
}
