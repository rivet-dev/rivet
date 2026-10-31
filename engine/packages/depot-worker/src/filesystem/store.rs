//! Grant-owned SlateDB work. The maintainer supplies authoritative applied
//! positions; only completed entries may be published in its read-session view.

use super::{
	filter::{Catalog, Retention},
	session::SessionConfig,
};
use anyhow::{Result, ensure};
use bytes::Bytes;
use depot_fs_core::{keys::slatedb as keys, store::Batch, work::SweepHold, *};
use futures_util::{StreamExt, stream::FuturesOrdered};
use rivet_env::{Clock, MutationGuard};
use rivet_keys::keys::{
	KeyAddress, PrefixAddress,
	codec::{AtVersion, FormalKey},
};
use slatedb::{Db, WriteBatch};
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};
use uuid::Uuid;

/// Physical cleanup waits for checkpoint, fork and live read-session pins.
/// `More` is an exclusive cursor after a completed flush.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SweepOutcome {
	Held(SweepHold),
	More(Bytes),
	Complete,
}

/// Writes planned by a read phase and appended to one batch afterwards, so reads of
/// separate scopes can run concurrently without sharing a `WriteBatch`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlannedWrites(Vec<PlannedWrite>, Option<PointerWrite>);

/// A planned write of a name's pointer row. Records of one name planned separately may each
/// want the pointer; `plan_flush` keeps the one with the highest incarnation epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PointerWrite {
	key: Bytes,
	incarnation_epoch: u64,
	value: Bytes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlannedWrite {
	Put { key: Bytes, value: Bytes },
	Delete { key: Bytes },
}

impl PlannedWrites {
	pub(crate) fn put(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
		self.0.push(PlannedWrite::Put {
			key: Bytes::copy_from_slice(key.as_ref()),
			value: Bytes::copy_from_slice(value.as_ref()),
		});
	}

	pub(crate) fn delete(&mut self, key: impl AsRef<[u8]>) {
		self.0.push(PlannedWrite::Delete {
			key: Bytes::copy_from_slice(key.as_ref()),
		});
	}

	/// Appends the writes in the order they were planned, then the pointer write, if any.
	pub(crate) fn apply(self, batch: &mut WriteBatch) {
		for write in self.0 {
			match write {
				PlannedWrite::Put { key, value } => batch.put_bytes(key, value),
				PlannedWrite::Delete { key } => batch.delete(key),
			}
		}
		if let Some(pointer) = self.1 {
			batch.put_bytes(pointer.key, pointer.value);
		}
	}

	/// Plans `record`'s write at `version`, the store's `depot.fs_rows` version: its
	/// incarnation row, and the name's pointer when the record takes it from `pointer`, the
	/// pointer row as stored.
	pub(crate) fn record(
		&mut self,
		pointer: Option<&FsVolumeRecord>,
		record: &FsVolumeRecord,
		version: u16,
	) -> Result<()> {
		let own = incarnation_record_key(record);
		self.put(
			own.pack(),
			own.serialize(AtVersion::new(record.clone(), version))?,
		);
		if takes_pointer(pointer, record)? {
			let address = pointer_key(&record_key(record));
			self.1 = Some(PointerWrite {
				key: address.pack(),
				incarnation_epoch: record.incarnation_epoch,
				value: address.serialize(AtVersion::new(record.clone(), version))?,
			});
		}
		Ok(())
	}
}

/// Writes `record` into `batch` the way `PlannedWrites::record` plans it, for writers that
/// build their batch directly.
pub(crate) fn put_record(
	batch: &mut WriteBatch,
	pointer: Option<&FsVolumeRecord>,
	record: &FsVolumeRecord,
	version: u16,
) -> Result<()> {
	let mut planned = PlannedWrites::default();
	planned.record(pointer, record, version)?;
	planned.apply(batch);
	Ok(())
}

/// Whether a write of `record` also replaces the name's pointer row `pointer`: the pointer
/// always holds the stored record with the highest incarnation epoch. Epochs come from one
/// counter per tablet, so two incarnations of a name never share one; a tie is corruption.
/// Epoch zero marks a record whose epoch its writer never learned, such as the Deleted
/// record of an incarnation recovered from tombstones alone; it is the oldest of its name and
/// never takes a pointer another incarnation holds. Only a live record takes an empty pointer:
/// a Deleted record there would outlive the sweep that already released the pointer.
pub(crate) fn takes_pointer(
	pointer: Option<&FsVolumeRecord>,
	record: &FsVolumeRecord,
) -> Result<bool> {
	let Some(pointer) = pointer else {
		return Ok(record.status == FsVolumeStatus::Live);
	};
	if pointer.volume_incarnation == record.volume_incarnation {
		return Ok(true);
	}
	if record.incarnation_epoch == 0 {
		return Ok(false);
	}
	ensure!(
		pointer.incarnation_epoch != record.incarnation_epoch,
		"two incarnations of one volume share incarnation epoch {}",
		record.incarnation_epoch
	);
	Ok(pointer.incarnation_epoch < record.incarnation_epoch)
}

/// The volumes `records` belong to, each once: one pass may carry several incarnations of a
/// name, which are one volume to the store's footprint.
fn record_volumes(records: &[FsVolumeRecord]) -> BTreeSet<super::locks::StoreVolume> {
	records
		.iter()
		.map(super::locks::StoreVolume::from)
		.collect()
}

/// The pointer row `pointer` when it names `key`, which is then `key`'s current record.
pub(crate) fn names_key(
	pointer: Option<FsVolumeRecord>,
	key: &depot_types::VolumeKey,
) -> Option<FsVolumeRecord> {
	pointer.filter(|pointer| pointer.partition_key == key.partition_key && pointer.name == key.name)
}

/// The name's pointer row address.
pub(crate) fn pointer_key(key: &depot_types::VolumeKey) -> keys::VolumeRecordKey {
	keys::VolumeRecordKey::new(
		depot_types::partition_key_hash(key),
		key.name.clone().map(Into::into),
	)
}

/// Rejects a stale write of one incarnation's record over `old`, the record stored for it:
/// the identity, ordering epoch and base chain never change, a deleted record stays deleted,
/// and the cold position only advances.
pub(crate) fn ensure_record_follows(old: &FsVolumeRecord, record: &FsVolumeRecord) -> Result<()> {
	ensure!(
		old.volume_incarnation == record.volume_incarnation
			&& old.incarnation_epoch == record.incarnation_epoch
			&& old.base_chain == record.base_chain
			&& (old.status != FsVolumeStatus::Deleted || record.status == FsVolumeStatus::Deleted)
			&& (old.cold.volume_seq < record.cold.volume_seq
				&& old.cold.client_epoch <= record.cold.client_epoch
				|| old.cold == record.cold),
		"stale filesystem flush"
	);
	Ok(())
}

/// The row address of one incarnation's own record.
pub(crate) fn incarnation_record_key(record: &FsVolumeRecord) -> keys::VolumeIncarnationRecordKey {
	let scope = record_scope(record);
	keys::VolumeIncarnationRecordKey::new(scope.hash, scope.name, scope.incarnation)
}

fn record_scope(record: &FsVolumeRecord) -> keys::Scope {
	keys::Scope::new(
		&record_key(record),
		Uuid::from_bytes(record.volume_incarnation),
	)
}

fn record_key(record: &FsVolumeRecord) -> depot_types::VolumeKey {
	depot_types::VolumeKey {
		partition_key: record.partition_key.clone(),
		name: record.name.clone(),
	}
}

/// The store operation waiting for a store lock. It is a bounded metric label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LockOperation {
	LifecycleLookup,
	StageFlush,
	FinishFlush,
	OpenSession,
	Apply,
	Sweep,
	Other,
}

impl LockOperation {
	pub(crate) fn label(self) -> &'static str {
		match self {
			LockOperation::LifecycleLookup => "lifecycle_lookup",
			LockOperation::StageFlush => "stage_flush",
			LockOperation::FinishFlush => "finish_flush",
			LockOperation::OpenSession => "open_session",
			LockOperation::Apply => "apply",
			LockOperation::Sweep => "sweep",
			LockOperation::Other => "other",
		}
	}
}







/// Snapshots one retention load reads before it gives up on a scope whose concurrent
/// loaders keep inserting first.
const MAX_RETENTION_LOAD_ATTEMPTS: usize = 16;

/// Records whose old rows one flush stage reads at once by default.
pub const DEFAULT_FLUSH_STAGE_CONCURRENCY: usize = 16;

/// Cloning this executor does not acquire a grant or start tasks. Its containing
/// cold-store owner closes the database and joins SlateDB's tasks on shutdown.
/// The `depot.fs_rows` version a tablet's store encodes rows at: the version the grant
/// selected from the topology's read floor, capped by the `depot.fs_rows` mark already durable
/// in the tablet's maintainer log record.
///
/// The cap is what lets any pass make rows durable, including one that writes no record, such
/// as a write-to-cold pass or a pass during a split: a row is never encoded above a mark that
/// is already durable, so no reader the coordinator may grant the tablet to finds a row it
/// cannot read. The mark rises only through a record pass, which raises it to the selected
/// version, so rows follow one pass after it.
pub(crate) struct RowVersion {
	selected: std::sync::atomic::AtomicU16,
	marked: std::sync::atomic::AtomicU16,
}

impl RowVersion {
	/// Until the grant opens the store, rows encode at version 1, which every binary that
	/// records marks reads and which a store without marks holds.
	fn new() -> Self {
		Self {
			selected: std::sync::atomic::AtomicU16::new(1),
			marked: std::sync::atomic::AtomicU16::new(1),
		}
	}

	fn current(&self) -> u16 {
		let selected = self.selected.load(std::sync::atomic::Ordering::Acquire);
		selected.min(self.marked.load(std::sync::atomic::Ordering::Acquire))
	}

	fn open(
		&self,
		selected: u16,
		marks: &[rivet_data::generated::shared_depot_types_v1::VersionMark],
	) {
		self.selected
			.store(selected, std::sync::atomic::Ordering::Release);
		self.marked
			.store(Self::rows_mark(marks), std::sync::atomic::Ordering::Release);
	}

	fn marked_durable(&self, marks: &[rivet_data::generated::shared_depot_types_v1::VersionMark]) {
		self.marked
			.fetch_max(Self::rows_mark(marks), std::sync::atomic::Ordering::AcqRel);
	}

	/// The `depot.fs_rows` mark in `marks`. A store without one holds version 1 rows.
	fn rows_mark(marks: &[rivet_data::generated::shared_depot_types_v1::VersionMark]) -> u16 {
		rivet_data::families::mark_of(marks, rivet_data::families::VersionFamily::DEPOT_FS_ROWS)
			.unwrap_or(1)
	}
}

pub struct FilesystemStore {
	pub(crate) db: Db,
	/// Immutable ownership of this grant; shared split snapshots also contain foreign rows.
	pub(crate) hash_range: depot_types::PartitionKeyHashRange,
	/// See [`RowVersion`].
	pub(crate) rows: RowVersion,
	pub(crate) clock: Arc<dyn Clock>,
	pub(crate) guard: Arc<dyn MutationGuard>,
	pub(crate) catalog: Catalog,
	pub(crate) config: SessionConfig,
	pub(super) base_reader: Option<Arc<dyn super::base::BaseChunkReader>>,
	pub(crate) sessions: parking_lot::Mutex<super::session::SessionTable>,
	pub(crate) rng: rivet_env::RngStream,
	pub(crate) session_slots: Arc<tokio::sync::Semaphore>,
	/// Whether the near-capacity warning for read sessions fired and has not re-armed.
	pub(crate) session_warned: std::sync::atomic::AtomicBool,
	/// This store's share of the read session capacity gauge.
	pub(crate) _session_capacity: rivet_metrics::IntGaugeGuard,
	/// The tablet lock and volume stripes. See `locks.rs` for the rules every caller
	/// follows. They also protect direct injected callers that bypass the core.
	pub(crate) locks: super::locks::StoreLocks,
	/// Store work running per volume, checked against the footprint matrix in debug
	/// builds of `core` mode, where that work takes no stripe.
	pub(crate) accesses: super::locks::ActiveAccesses,
	pub batch_rows: usize,
	/// Records whose old rows `stage_flush` reads at once.
	pub(super) flush_stage_concurrency: usize,
	pub(super) checkpoint_config: depot_fs_core::checkpoints::CheckpointConfig,
	/// The owning cold store's manifest observer, sampled after every flush so the L0 and
	/// sorted run gauges follow tablet flushes and not only lifecycle passes.
	pub(crate) observer: Option<Arc<parking_lot::Mutex<crate::metrics::ColdStoreObserver>>>,
	/// The memtable flush the tablet's passes and split imports share.
	pub(crate) shared_flush: super::shared_flush::SharedFlush,
}

impl FilesystemStore {
	/// The `depot.fs_rows` version every row this store writes now is encoded at.
	pub(crate) fn row_version(&self) -> u16 {
		self.rows.current()
	}

	/// The grant opened the store: rows are encoded at `selected`, the version the grant
	/// chose from the read floor, but never above the mark in `marks`, the store's durable
	/// maintainer log record. Called once, before the grant writes anything.
	pub(crate) fn open_row_version(
		&self,
		selected: u16,
		marks: &[rivet_data::generated::shared_depot_types_v1::VersionMark],
	) {
		self.rows.open(selected, marks);
	}

	/// A flush made a maintainer log record with `marks` durable, so rows may now be
	/// encoded up to its `depot.fs_rows` mark.
	pub(crate) fn rows_marked_durable(
		&self,
		marks: &[rivet_data::generated::shared_depot_types_v1::VersionMark],
	) {
		self.rows.marked_durable(marks);
	}

	pub fn new(
		db: Db,
		clock: Arc<dyn Clock>,
		guard: Arc<dyn MutationGuard>,
		catalog: Catalog,
		config: SessionConfig,
		batch_rows: usize,
		rng: rivet_env::RngStream,
	) -> Result<Self> {
		ensure!(
			batch_rows > 0
				&& config.max_rows > 0
				&& config.max_sessions > 0
				&& config.max_read_session_ms > 0
				&& config.read_session_idle_ms > 0
				&& config.latest_session_idle_ms > 0
				&& config.max_read_bytes > 0,
			"invalid filesystem store limits"
		);
		Ok(Self {
			db,
			hash_range: depot_types::PartitionKeyHashRange {
				start: [0; 16],
				end: None,
			},
			rows: RowVersion::new(),
			clock,
			guard,
			catalog,
			session_slots: Arc::new(tokio::sync::Semaphore::new(config.max_sessions)),
			session_warned: std::sync::atomic::AtomicBool::new(false),
			_session_capacity: {
				use rivet_metrics::GaugeGuardExt;
				crate::metrics::READ_SESSION_CAPACITY.add_guard(config.max_sessions as i64)
			},
			config,
			sessions: parking_lot::Mutex::new(Default::default()),
			rng,
			locks: Default::default(),
			accesses: Default::default(),
			batch_rows,
			flush_stage_concurrency: DEFAULT_FLUSH_STAGE_CONCURRENCY,
			checkpoint_config: Default::default(),
			base_reader: None,
			observer: None,
			shared_flush: Default::default(),
		})
	}

	/// Set before publishing this store to its grant's operations.
	pub(crate) fn with_tablet_range(
		mut self,
		range: depot_types::PartitionKeyHashRange,
	) -> Result<Self> {
		ensure!(range.is_valid(), "invalid filesystem tablet range");
		self.hash_range = range;
		Ok(self)
	}

	pub fn with_flush_stage_concurrency(mut self, concurrency: usize) -> Result<Self> {
		ensure!(concurrency > 0, "flush stage concurrency must be positive");
		self.flush_stage_concurrency = concurrency;
		Ok(self)
	}

	/// Replaces the volume lock table with one of `stripes` stripes. Call it before the
	/// store serves work: locks held on the old table do not carry over.
	pub fn with_volume_lock_stripes(mut self, stripes: usize) -> Result<Self> {
		ensure!(stripes > 0, "volume lock stripes must be positive");
		self.locks = super::locks::StoreLocks::new(stripes).with_mode(self.locks.mode());
		Ok(self)
	}

	/// Sets which store operations take volume stripes.
	pub fn with_volume_locks(mut self, mode: super::locks::VolumeLockMode) -> Self {
		self.locks = std::mem::take(&mut self.locks).with_mode(mode);
		self
	}

	pub fn with_base_reader(mut self, reader: Arc<dyn super::base::BaseChunkReader>) -> Self {
		self.base_reader = Some(reader);
		self
	}

	pub(crate) fn with_observer(
		mut self,
		observer: Arc<parking_lot::Mutex<crate::metrics::ColdStoreObserver>>,
	) -> Self {
		self.observer = Some(observer);
		self
	}

	/// Takes the locks `request` names, recording how long `operation` waited.
	pub(crate) async fn lock(
		&self,
		request: super::locks::LockRequest,
		operation: LockOperation,
	) -> super::locks::StoreGuard<'_> {
		self.locks.lock(request, operation).await
	}

	/// Takes the locks of work the maintainer core orders: `request` in `stripes` mode,
	/// only its tablet lock in `core` mode.
	pub(crate) async fn lock_core_ordered(
		&self,
		request: super::locks::LockRequest,
		operation: LockOperation,
	) -> super::locks::StoreGuard<'_> {
		self.locks
			.lock(self.locks.core_ordered(request), operation)
			.await
	}

	/// Records core-ordered work on its volumes for the debug footprint check. It records
	/// nothing in `stripes` mode, where the stripes order the same work.
	pub(crate) fn enter_volumes(
		&self,
		accesses: impl IntoIterator<Item = (super::locks::StoreVolume, super::locks::VolumeAccess)>,
	) -> super::locks::AccessGuard<'_> {
		match self.locks.mode() {
			super::locks::VolumeLockMode::Core => self.accesses.enter(accesses),
			super::locks::VolumeLockMode::Stripes => self.accesses.enter([]),
		}
	}

	pub(crate) fn admit(&self) -> Result<()> {
		ensure!(self.guard.may_send(), "filesystem store grant revoked");
		self.clock.observe()?;
		Ok(())
	}
	/// The current record of `key`: the name's pointer when it names `key`. A pointer naming
	/// another volume key that shares this one's placement and name means `key` has no current
	/// incarnation.
	pub async fn record(&self, key: &depot_types::VolumeKey) -> Result<Option<FsVolumeRecord>> {
		Ok(names_key(self.pointer(key).await?, key))
	}

	/// The name's pointer row, whichever volume key holds it. Different volume keys can share
	/// one placement and name, such as one actor's database across a destroy and recreate, and
	/// the pointer then holds their newest stored incarnation. Decisions about which
	/// incarnation takes or releases the pointer use this row; reads of one key's current
	/// record use `record`.
	pub async fn pointer(&self, key: &depot_types::VolumeKey) -> Result<Option<FsVolumeRecord>> {
		self.admit()?;
		let address = pointer_key(key);
		let record = self
			.db
			.get(address.pack())
			.await?
			.map(|bytes| keys::decode_volume_record(&bytes))
			.transpose()?;
		ensure!(
			record.as_ref().is_none_or(|r| {
				depot_types::partition_key_hash(&r.volume_key())
					== depot_types::partition_key_hash(key)
					&& r.name == key.name
			}),
			"filesystem record identity mismatch"
		);
		self.admit()?;
		Ok(record)
	}

	/// The record of one incarnation from its own row, whatever the name's pointer names. The
	/// row outlives the incarnation's removal from the pointer until its data is swept.
	pub async fn incarnation_row(
		&self,
		key: &depot_types::VolumeKey,
		incarnation: [u8; 16],
	) -> Result<Option<FsVolumeRecord>> {
		self.admit()?;
		let scope = keys::Scope::new(key, Uuid::from_bytes(incarnation));
		let address =
			keys::VolumeIncarnationRecordKey::new(scope.hash, scope.name, scope.incarnation);
		let record = self
			.db
			.get(address.pack())
			.await?
			.map(|bytes| keys::decode_volume_record(&bytes))
			.transpose()?;
		ensure!(
			record
				.as_ref()
				.is_none_or(|r| r.partition_key == key.partition_key
					&& r.name == key.name
					&& r.volume_incarnation == incarnation),
			"filesystem incarnation record identity mismatch"
		);
		self.admit()?;
		Ok(record)
	}

	/// Loads one record's checkpoint and fork-hold pins into the retention catalog.
	///
	/// No lock orders the loaders of a scope. Each one notes the scope's load count before
	/// its snapshot and inserts only if the count is unchanged, comparing and inserting
	/// under one catalog lock; a loader that lost the race reads again. The last insert
	/// therefore always comes from a snapshot taken after every earlier insert.
	pub async fn load_retention(&self, record: &FsVolumeRecord) -> Result<()> {
		let scope = record_scope(record);
		for _ in 0..MAX_RETENTION_LOAD_ATTEMPTS {
			self.admit()?;
			let loaded = self.retention_loads(&scope);
			let snapshot = self.db.snapshot().await?;
			let checkpoints = Self::retained_checkpoints(&snapshot, &scope).await?;
			self.admit()?;
			if self.insert_retention_if(scope.clone(), record, checkpoints, loaded) {
				return Ok(());
			}
		}
		anyhow::bail!(
			"retention load of a scope kept losing to concurrent loaders; the caller retries"
		)
	}

	/// The checkpoint sequences a scope's named, retention and fork-hold rows pin, as of
	/// `snapshot`.
	async fn retained_checkpoints(
		snapshot: &slatedb::DbSnapshot,
		scope: &keys::Scope,
	) -> Result<BTreeSet<u64>> {
		let mut checkpoints = BTreeSet::new();
		let prefix =
			keys::CheckpointRowsPrefix::new(scope.hash, scope.name.clone(), scope.incarnation, 0)
				.pack();
		let mut rows = snapshot.scan_prefix(prefix, ..).await?;
		while let Some(row) = rows.next().await? {
			let address = keys::VolumeCheckpointKey::unpack(&row.key)?;
			let value = address.deserialize(row.value)?;
			if value.named || value.retention {
				checkpoints.insert(address.sequence());
			}
		}
		let prefix =
			keys::ForkHoldRowsPrefix::new(scope.hash, scope.name.clone(), scope.incarnation, 0)
				.pack();
		let mut rows = snapshot.scan_prefix(prefix, ..).await?;
		while let Some(row) = rows.next().await? {
			checkpoints.insert(keys::ForkHoldKey::unpack(&row.key)?.sequence());
		}
		Ok(checkpoints)
	}

	/// Replaces a scope's catalog entry if its load count is still `loaded`, keeping its
	/// apply boundary and sessions. Returns whether it replaced the entry.
	fn insert_retention_if(
		&self,
		scope: keys::Scope,
		record: &FsVolumeRecord,
		checkpoints: BTreeSet<u64>,
		loaded: u64,
	) -> bool {
		let mut catalog = self.catalog.lock();
		if catalog.get(&scope).map_or(0, |retention| retention.loads) != loaded {
			return false;
		}
		let old = catalog.remove(&scope);
		let apply_cold = old.as_ref().and_then(|old| old.apply_cold.clone());
		let loads = old.as_ref().map_or(0, |old| old.loads).wrapping_add(1);
		let sessions = old.map_or_else(BTreeMap::new, |old| old.sessions);
		catalog.insert(
			scope,
			Retention {
				apply_cold,
				cold: record.cold.volume_seq,
				cold_barrier: Some(self.db.manifest().last_l0_seq()),
				checkpoints,
				has_base: !record.base_chain.is_empty(),
				sessions,
				loads,
			},
		);
		true
	}

	/// Adds checkpoints a writer of `scope` just wrote to its catalog entry and counts it as
	/// a load, so a loader whose snapshot predates the write reads again instead of
	/// replacing these pins.
	pub(crate) fn pin_written_checkpoints(&self, scope: &keys::Scope, written: &BTreeSet<u64>) {
		if written.is_empty() {
			return;
		}
		if let Some(retention) = self.catalog.lock().get_mut(scope) {
			retention.checkpoints.extend(written.iter().copied());
			retention.loads = retention.loads.wrapping_add(1);
		}
	}

	/// How many times a scope's retention has been loaded.
	fn retention_loads(&self, scope: &keys::Scope) -> u64 {
		self.catalog
			.lock()
			.get(scope)
			.map_or(0, |retention| retention.loads)
	}

	/// Apply progress is mirrored from the resolver only after all row work completes.
	/// Administrative flushes must use the same boundary as normal background flushes.
	pub(super) fn limit_cold(&self, scope: &keys::Scope, record: &mut FsVolumeRecord) {
		if let Some(limit) = self
			.catalog
			.lock()
			.get(scope)
			.and_then(|v| v.apply_cold.as_ref())
		{
			if limit.volume_seq < record.cold.volume_seq {
				record.cold = limit.clone();
			}
		}
		record
			.recent_writes
			.retain(|r| r.volume_seq <= record.cold.volume_seq);
	}
	pub(super) async fn stage(&self, batch: WriteBatch) -> Result<()> {
		self.admit()?;
		if !batch.is_empty() {
			self.db.write(batch).await?;
		}
		self.admit()?;
		Ok(())
	}
	pub(super) fn encode_batch(
		scope: &keys::Scope,
		seq: u64,
		rows: Batch,
		version: u16,
	) -> Result<WriteBatch> {
		let mut batch = WriteBatch::new();
		for (key, value) in rows.rows {
			let (key, value) = keys::encode_row(scope, &key, seq, value, version)?;
			batch.put(key, value);
		}
		Ok(batch)
	}
	/// Stage all due records together, then flush once. The maintainer may issue
	/// Trim only after success. A lost reply requires replay/readback, never trim.
	pub async fn flush(&self, records: &[FsVolumeRecord]) -> Result<()> {
		self.stage_flush(records).await?;
		self.finish_flush(records).await?;
		Ok(())
	}

	/// Writes the records after every earlier mutation without waiting for
	/// durability. Nothing they describe may be trimmed until `finish_flush` succeeds.
	///
	/// The plan is read without any volume stripe: the core admitted the stage as a
	/// writer of every record, so no core-admitted operation of those volumes runs until
	/// it completes. Only the in-memory recheck and the write hold the tablet lock shared
	/// and every record's stripe exclusively, which still orders the stage's hold
	/// releases against checkpoint-session opens, the one pair the core does not order.
	pub async fn stage_flush(&self, records: &[FsVolumeRecord]) -> Result<()> {
		self.stage_flush_with_log(records, None).await
	}

	/// `stage_flush`, staging the tablet's maintainer log record in the same batch, so the
	/// cold position the log may trim to is durable exactly when the flush is.
	pub async fn stage_flush_with_log(
		&self,
		records: &[FsVolumeRecord],
		maintainer_log: Option<
			&crate::tablet_maintainer_machine::state::maintainer_log::RecordWrite,
		>,
	) -> Result<()> {
		let _running = self.enter_volumes(
			record_volumes(records)
				.into_iter()
				.map(|volume| (volume, super::locks::VolumeAccess::Write)),
		);
		self.admit()?;
		let plans = self
			.plan_flush(records, self.flush_stage_concurrency)
			.await?;
		let request = records
			.iter()
			.fold(super::locks::LockRequest::shared(), |request, record| {
				request.write(super::locks::stripe(&record_key(record)))
			});
		let _ordered = self.lock(request, LockOperation::StageFlush).await;
		self.admit()?;
		for record in records {
			self.recheck_flush_record(record)?;
		}
		let mut batch = WriteBatch::new();
		for plan in plans {
			plan.apply(&mut batch);
		}
		if let Some(write) = maintainer_log {
			let key = crate::tablet_maintainer_machine::state::maintainer_log::record_key(
				&write.volume_key,
			);
			batch.put(
				key.pack(),
				key.serialize(AtVersion::new(write.record.clone(), write.version))?,
			);
		}
		self.stage(batch).await?;
		Ok(())
	}

	/// Repeats the in-memory part of a record's flush plan under its stripe. The plan's
	/// SlateDB reads ran earlier without a lock; the apply boundary in the catalog is the
	/// only planned input that can move without a store write, so a transaction opened
	/// since the plan fails the stage instead of being crossed. No planned deletion
	/// consults a scope's read sessions: a checkpoint-session open registers its pins in
	/// the catalog before it releases its shared stripe, and the compaction filter keeps
	/// what those pins name even after the stage deletes the checkpoint row.
	fn recheck_flush_record(&self, record: &FsVolumeRecord) -> Result<()> {
		let key = record_key(record);
		let scope = keys::Scope::new(&key, Uuid::from_bytes(record.volume_incarnation));
		let mut bounded = record.clone();
		self.limit_cold(&scope, &mut bounded);
		ensure!(
			bounded.cold == record.cold,
			"flush crosses an open transaction"
		);
		Ok(())
	}

	/// Reads what each record's flush must write, `concurrency` records at a time, and
	/// returns the writes in input order. Each record reads and writes only its own scope,
	/// and the duplicate check before any read makes the scopes distinct, so the result
	/// does not depend on how the reads interleave. It takes no lock: the core orders the
	/// stage against every other writer of its records.
	pub(crate) async fn plan_flush(
		&self,
		records: &[FsVolumeRecord],
		concurrency: usize,
	) -> Result<Vec<PlannedWrites>> {
		let mut unique = BTreeSet::new();
		for record in records {
			ensure!(
				unique.insert(incarnation_record_key(record).pack()),
				"duplicate filesystem flush record"
			);
		}
		let concurrency = concurrency.max(1);
		let mut remaining = records.iter();
		let mut reading = FuturesOrdered::new();
		let mut plans = Vec::with_capacity(records.len());
		loop {
			while reading.len() < concurrency {
				let Some(record) = remaining.next() else {
					break;
				};
				reading.push_back(self.plan_flush_record(record));
			}
			let Some(plan) = reading.next().await else {
				break;
			};
			plans.push(plan?);
		}

		// Records of one name may each take the pointer; the highest incarnation epoch wins,
		// so the pointer names the newest stored incarnation whatever the input order. The
		// winner keeps its pointer write in its own plan, so there is one plan per record.
		let mut winners = BTreeMap::<Bytes, (usize, u64)>::new();
		for (index, plan) in plans.iter().enumerate() {
			let Some(pointer) = &plan.1 else {
				continue;
			};
			match winners.get(&pointer.key) {
				Some((_, kept)) if *kept > pointer.incarnation_epoch => {}
				Some(_) | None => {
					winners.insert(pointer.key.clone(), (index, pointer.incarnation_epoch));
				}
			}
		}
		for (index, plan) in plans.iter_mut().enumerate() {
			let loses = plan.1.as_ref().is_some_and(|pointer| {
				winners.get(&pointer.key).map(|(winner, _)| *winner) != Some(index)
			});
			if loses {
				plan.1 = None;
			}
		}
		Ok(plans)
	}

	async fn plan_flush_record(&self, record: &FsVolumeRecord) -> Result<PlannedWrites> {
		let key = record_key(record);
		let pointer = self.pointer(&key).await?;
		// The pointer is a copy of the row of the incarnation it names, so only a record of
		// another incarnation reads its own row.
		let stored = match names_key(pointer.clone(), &key) {
			Some(pointer) if pointer.volume_incarnation == record.volume_incarnation => {
				Some(pointer)
			}
			Some(_) | None => {
				self.incarnation_row(&key, record.volume_incarnation)
					.await?
			}
		};
		if let Some(old) = &stored {
			ensure_record_follows(old, record)?;
		}
		let scope = keys::Scope::new(&key, Uuid::from_bytes(record.volume_incarnation));
		let mut bounded = record.clone();
		self.limit_cold(&scope, &mut bounded);
		ensure!(
			bounded.cold == record.cold,
			"flush crosses an open transaction"
		);
		let mut planned = PlannedWrites::default();
		self.plan_alarm_index(record, &mut planned).await?;
		self.retention_updates(record, &mut planned).await?;
		planned.record(pointer.as_ref(), record, self.row_version())?;
		if record.status == FsVolumeStatus::Deleted {
			let marker =
				keys::PendingDeleteKey::new(scope.hash, scope.name.clone(), scope.incarnation);
			planned.put(marker.pack(), marker.serialize(())?);
		}
		Ok(planned)
	}

	/// Makes everything written before the call durable, including records staged
	/// by `stage_flush`, then loads their retention. The upload runs without any
	/// store lock, so later applies continue and land in a newer memtable that
	/// this flush may or may not include. Including them is harmless: replay from
	/// the published cold floor rewrites the same immutable rows.
	///
	/// Each phase is timed separately: the memtable upload, the wait for the lock and
	/// the retention loop. Returns the phase times so the caller can split real upload
	/// work from time held by a full level 0.
	pub(crate) async fn finish_flush(
		&self,
		records: &[FsVolumeRecord],
	) -> Result<crate::metrics::FlushTimings> {
		let _running = self.enter_volumes(
			record_volumes(records)
				.into_iter()
				.map(|volume| (volume, super::locks::VolumeAccess::Unpublished)),
		);
		let phase = |name: &str| {
			crate::metrics::TABLET_STORE_OPERATION_DURATION
				.with_label_values(&[name])
				.start_timer()
		};
		let l0_full = self
			.observer
			.as_ref()
			.is_some_and(|observer| observer.lock().l0_full());
		let memtable = phase("filesystem_flush_memtable");
		let split = crate::metrics::TABLET_FLUSH_MEMTABLE_DURATION
			.with_label_values(&[if l0_full { "full" } else { "open" }])
			.start_timer();
		self.shared_flush
			.flush(&self.db, super::shared_flush::FlushCause::Pass)
			.await?;
		drop(split);
		let memtable = std::time::Duration::from_secs_f64(memtable.stop_and_record());
		if let Some(observer) = &self.observer {
			observer.lock().observe(&self.db);
		}
		let retention = phase("filesystem_flush_retention");
		let plan = self.plan_retention(records).await?;
		let lock_wait = self.publish_retention(records, plan).await?;
		let retention = std::time::Duration::from_secs_f64(retention.stop_and_record());
		crate::metrics::TABLET_STORE_OPERATION_DURATION
			.with_label_values(&["filesystem_flush_finish_lock"])
			.observe(lock_wait.as_secs_f64());
		Ok(crate::metrics::FlushTimings {
			l0_full,
			memtable,
			retention,
		})
	}

	/// Reads every record's pins from one snapshot, concurrently and without a lock,
	/// with each scope's load count from before the snapshot.
	pub(crate) async fn plan_retention(
		&self,
		records: &[FsVolumeRecord],
	) -> Result<Vec<(keys::Scope, u64, BTreeSet<u64>)>> {
		self.admit()?;
		let scopes = records.iter().map(record_scope).collect::<Vec<_>>();
		let loads = scopes
			.iter()
			.map(|scope| self.retention_loads(scope))
			.collect::<Vec<_>>();
		let snapshot = self.db.snapshot().await?;
		let concurrency = self.flush_stage_concurrency.max(1);
		let mut remaining = scopes.iter();
		let mut reading = FuturesOrdered::new();
		let mut pins = Vec::with_capacity(scopes.len());
		loop {
			while reading.len() < concurrency {
				let Some(scope) = remaining.next() else {
					break;
				};
				reading.push_back(Self::retained_checkpoints(&snapshot, scope));
			}
			let Some(pinned) = reading.next().await else {
				break;
			};
			pins.push(pinned?);
		}
		drop(reading);
		self.admit()?;
		Ok(scopes
			.into_iter()
			.zip(loads)
			.zip(pins)
			.map(|((scope, loaded), pinned)| (scope, loaded, pinned))
			.collect())
	}

	/// Inserts planned pins into the catalog. A checkpoint writer of the scope reloads or
	/// counts its pins after its write, so if one ran since the snapshot (the scope's load
	/// count moved), the record is loaded again instead of replacing the writer's newer
	/// pins. The compare and insert happen under one catalog lock, so in `core` mode this
	/// takes only the tablet lock. Returns the time spent waiting for locks.
	pub(crate) async fn publish_retention(
		&self,
		records: &[FsVolumeRecord],
		plan: Vec<(keys::Scope, u64, BTreeSet<u64>)>,
	) -> Result<std::time::Duration> {
		let mut lock_wait = std::time::Duration::ZERO;
		for (record, (scope, loaded, pinned)) in records.iter().zip(plan) {
			#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
			let waiting = std::time::Instant::now();
			let ordered = self
				.lock_core_ordered(
					super::locks::LockRequest::shared()
						.write(super::locks::stripe(&record_key(record))),
					LockOperation::FinishFlush,
				)
				.await;
			lock_wait += waiting.elapsed();
			if !self.insert_retention_if(scope, record, pinned, loaded) {
				// The reload reads SlateDB, so it runs without the guard.
				drop(ordered);
				self.load_retention(record).await?;
			}
		}
		Ok(lock_wait)
	}

	/// Bounded physical sweep of a retired incarnation. The owner persists the
	/// exclusive cursor with PendingSweep; replay after a lost reply is harmless.
	/// Existing checkpoint, fork and session holds always prevent reclamation.
	pub async fn sweep(&self, scope: &keys::Scope, after: Option<Bytes>) -> Result<SweepOutcome> {
		self.sweep_bounded(scope, after, self.batch_rows as u32)
			.await
	}
	/// Bounded by both the store limit and its owner's current cleanup budget.
	pub async fn sweep_bounded(
		&self,
		scope: &keys::Scope,
		after: Option<Bytes>,
		max_rows: u32,
	) -> Result<SweepOutcome> {
		let _ordered = self
			.lock(
				super::locks::LockRequest::scope(scope),
				LockOperation::Sweep,
			)
			.await;
		self.sweep_locked(scope, after, max_rows).await
	}
	/// `sweep_bounded` for a caller that already holds the scope's stripe.
	pub(crate) async fn sweep_locked(
		&self,
		scope: &keys::Scope,
		after: Option<Bytes>,
		max_rows: u32,
	) -> Result<SweepOutcome> {
		ensure!(max_rows > 0, "sweep requires a nonzero row budget");
		let limit = self.batch_rows.min(max_rows as usize);
		self.admit()?;
		let record_key = keys::VolumeIncarnationRecordKey::new(
			scope.hash,
			scope.name.clone(),
			scope.incarnation,
		);
		let record = self
			.db
			.get(record_key.pack())
			.await?
			.map(|v| record_key.deserialize(v))
			.transpose()?;
		let pending = keys::PendingSweepKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		ensure!(
			record.is_some() || self.db.get(pending.pack()).await?.is_some(),
			"sweep has neither its incarnation's record nor durable intent"
		);
		// A marker beside a live record of its own incarnation is stale: nothing retired it.
		// Dropping the marker keeps the rows, where failing would fence the maintainer again
		// each time discovery found it. Every retirement, a delete's or a restore's, writes the
		// incarnation's own row Deleted before it schedules the sweep.
		if record
			.as_ref()
			.is_some_and(|record| record.status != FsVolumeStatus::Deleted)
		{
			tracing::warn!(
				?scope,
				"dropping a pending sweep of a live incarnation; a writer scheduled it without proof the incarnation was retired"
			);
			let mut batch = WriteBatch::new();
			batch.delete(pending.pack());
			self.stage(batch).await?;
			return Ok(SweepOutcome::Complete);
		}
		let now = self.clock.observe()?.boot_ms;
		if self
			.catalog
			.lock()
			.get(scope)
			.is_some_and(|r| r.sessions.values().any(|(_, expires)| now < *expires))
		{
			return Ok(SweepOutcome::Held(SweepHold::ReadSession));
		}
		let held = self.db.snapshot().await?;
		let mut checkpoints = held
			.scan_prefix(
				keys::CheckpointRowsPrefix::new(
					scope.hash,
					scope.name.clone(),
					scope.incarnation,
					0,
				)
				.pack(),
				..,
			)
			.await?;
		while let Some(row) = checkpoints.next().await? {
			let key = keys::VolumeCheckpointKey::unpack(&row.key)?;
			let checkpoint = key.deserialize(row.value)?;
			if checkpoint.named || checkpoint.retention {
				return Ok(SweepOutcome::Held(SweepHold::Checkpoint));
			}
		}
		let mut forks = held
			.scan_prefix(
				keys::ForkHoldRowsPrefix::new(scope.hash, scope.name.clone(), scope.incarnation, 0)
					.pack(),
				..,
			)
			.await?;
		if forks.next().await?.is_some() {
			return Ok(SweepOutcome::Held(SweepHold::Checkpoint));
		}
		let prefix =
			keys::IncarnationPrefix::new(scope.hash, scope.name.clone(), scope.incarnation).pack();
		let snapshot = self.db.snapshot().await?;
		if let Some(after) = &after {
			ensure!(
				after.starts_with(&prefix),
				"sweep cursor belongs to another incarnation"
			);
		}
		let lower = after.as_ref().map_or(std::ops::Bound::Unbounded, |cursor| {
			std::ops::Bound::Excluded(cursor.slice(prefix.len()..))
		});
		let mut scan = snapshot
			.scan_prefix(prefix, (lower, std::ops::Bound::<Bytes>::Unbounded))
			.await?;
		let mut batch = WriteBatch::new();
		let mut last = None;
		let mut count = 0;
		let mut deleted = 0;
		while count < limit {
			let Some(row) = scan.next().await? else { break };
			// Checkpoint and fork holds survive data sweeping until their owner
			// explicitly removes them. They are never interpreted as row versions.
			if keys::decode_key(&row.key).is_ok() {
				batch.delete(&row.key);
				deleted += 1;
			}
			last = Some(row.key);
			count += 1;
		}
		if count < limit {
			self.complete_fork_sweep(scope, &mut batch).await?;
			// The incarnation's own record is its last row: it proves the incarnation's final
			// position to fork base reads for as long as any of its data remains. A pointer
			// that still names the incarnation goes with it.
			batch.delete(record_key.pack());
			let pointer = keys::VolumeRecordKey::new(scope.hash, scope.name.clone());
			if let Some(value) = self.db.get(pointer.pack()).await?
				&& pointer.deserialize(value)?.volume_incarnation == *scope.incarnation.as_bytes()
			{
				batch.delete(pointer.pack());
			}
			batch.delete(
				keys::PendingDeleteKey::new(scope.hash, scope.name.clone(), scope.incarnation)
					.pack(),
			);
		}
		// Deleting rows is idempotent and the cursor that records progress is staged after
		// them, so any durable prefix of the store that holds the cursor holds the deletes.
		// The tablet's ordinary pass makes both durable; a crash before it repeats pages.
		self.stage(batch).await?;
		self.admit()?;
		if count < limit {
			// The scope stripe excludes new pins and apply. Retirement and hold checks
			// above proved that no reader can need the reclaimed incarnation again.
			self.catalog.lock().remove(scope);
		}
		crate::metrics::DELETE_ROWS.inc_by(deleted);
		Ok(if count < limit {
			SweepOutcome::Complete
		} else {
			SweepOutcome::More(last.expect("nonempty sweep page"))
		})
	}
}
