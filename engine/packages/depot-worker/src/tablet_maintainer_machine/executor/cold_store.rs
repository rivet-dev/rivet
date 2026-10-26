//! SlateDB handle owned by one tablet grant. Filesystem apply and lifecycle
//! metadata share the same ordered storage owner and durability barrier.

mod deletion;

pub use deletion::DeleteSweepResult;

use anyhow::{Context, Result, ensure};
use depot_types::{
	VolumeLogKey,
	keys::{s3::TabletVolumePrefix, slatedb::VolumeRecordKey},
};
use rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord;
use rivet_env::capabilities::slatedb::{DatabaseCreation, SlateDbFactory};
use rivet_keys::keys::codec::{FormalKey, KeyAddress};
use slatedb::{
	Db, WriteBatch,
	config::{CompactorOptions, CompressionCodec, Settings},
};
use std::{num::NonZeroU64, time::Duration};
use uuid::Uuid;
use vbare::Message;

/// Bounds the upstream memtable and compaction work for a single tablet.
#[derive(Clone, Debug)]
pub struct ColdStoreConfig {
	pub checkpoints: depot_fs_core::checkpoints::CheckpointConfig,
	pub l0_sst_size_bytes: usize,
	pub l0_max_ssts: usize,
	/// Largest SST a compaction writes. A read that misses the metadata cache decodes the
	/// whole index of the SST it probes, so this bounds the cost of each miss once the
	/// indexes of a tablet outgrow the shared metadata cache.
	pub compacted_sst_size_bytes: usize,
	/// Upper bound for one database's unflushed bytes. [`ColdStoreConfig::for_maintained_tablets`]
	/// lowers it to this database's share of `unflushed_budget_bytes`.
	pub max_unflushed_bytes: usize,
	/// Unflushed memtable bytes for every database this process maintains together.
	pub unflushed_budget_bytes: usize,
	/// Cadence for the compaction scheduler and the embedded worker's polling. The writer,
	/// scheduler and worker share one process and wake each other on every manifest and
	/// `.compactions` write, so L0 drains at compaction speed; the polls only bound how long a
	/// change made elsewhere goes unseen. Each poll reads object storage even when nothing
	/// changed, so on an idle tablet these polls are most of its object-store requests.
	pub compactor_poll_interval_ms: u64,
	/// Cadence for the writer's manifest poll, the fallback for compaction results it was not
	/// woken for. The tablet grant epoch, not this poll, fences an old writer.
	pub manifest_poll_interval_ms: u64,
	/// Cadence for publishing finished compactions to the manifest. SlateDB reads object
	/// storage on this tick only while a compaction is running.
	pub compactor_commit_interval_ms: u64,
	pub max_concurrent_compactions: usize,
	pub min_compaction_sources: usize,
	/// At least `l0_max_ssts`, so one compaction drains a full L0 instead of leaving
	/// flushes stalled behind a second compaction cycle.
	pub max_compaction_sources: usize,
	/// Open client read sessions this tablet's filesystem store admits at once.
	pub max_read_sessions: usize,
	/// How long a read session stays registered after its last read.
	pub read_session_idle_ms: u64,
	/// The idle timeout of a session opened at the latest position, which HTTP directory
	/// cursors resume between pages.
	pub latest_session_idle_ms: u64,
	/// Records whose old rows one flush stage reads at once.
	pub flush_stage_concurrency: usize,
	/// Volume lock stripes of the tablet's filesystem store.
	pub volume_lock_stripes: usize,
	/// Which store operations take those stripes.
	pub volume_locks: crate::filesystem::locks::VolumeLockMode,
	/// How read sessions scan large chunk runs; see `BulkRunConfig`.
	pub bulk_run: crate::filesystem::session::BulkRunConfig,
	/// The process-wide byte budget for bulk runs, shared by every tablet's sessions.
	pub bulk_budget: Option<std::sync::Arc<crate::filesystem::bulk_budget::BulkReadBudget>>,
	/// Fewest keys an SST needs before SlateDB builds its filters. A tablet's memtable
	/// flushes hold only tens to hundreds of keys, and an SST without a filter costs a data
	/// block read for every point get and scope scan that its key range covers, which on L0
	/// is every lookup. Filters for a few hundred keys are a few hundred bytes.
	pub min_filter_keys: u32,
	/// Block codec for new SSTs, including compaction output. `None` writes uncompressed.
	/// Each SST records its codec, so existing SSTs stay readable after a change.
	pub compression_codec: Option<CompressionCodec>,
}

impl Default for ColdStoreConfig {
	fn default() -> Self {
		Self {
			checkpoints: Default::default(),
			l0_sst_size_bytes: 64 * 1024 * 1024,
			l0_max_ssts: 16,
			compacted_sst_size_bytes: 16 * 1024 * 1024,
			max_unflushed_bytes: 512 * 1024 * 1024,
			unflushed_budget_bytes: DEFAULT_UNFLUSHED_BUDGET_BYTES,
			compactor_poll_interval_ms: 5000,
			manifest_poll_interval_ms: 5000,
			compactor_commit_interval_ms: 250,
			max_concurrent_compactions: 2,
			max_read_sessions: depot_fs_core::wire::DEFAULT_MAX_SESSIONS,
			read_session_idle_ms: depot_fs_core::wire::DEFAULT_READ_SESSION_IDLE_MS,
			latest_session_idle_ms: depot_fs_core::wire::DEFAULT_LATEST_SESSION_IDLE_MS,
			flush_stage_concurrency: crate::filesystem::store::DEFAULT_FLUSH_STAGE_CONCURRENCY,
			volume_lock_stripes: crate::filesystem::locks::DEFAULT_VOLUME_LOCK_STRIPES,
			volume_locks: Default::default(),
			bulk_run: Default::default(),
			bulk_budget: None,
			min_compaction_sources: 4,
			max_compaction_sources: 16,
			min_filter_keys: 1,
			compression_codec: Some(CompressionCodec::Lz4),
		}
	}
}

/// Default process budget for unflushed SlateDB bytes across every maintained tablet.
pub const DEFAULT_UNFLUSHED_BUDGET_BYTES: usize = 1024 * 1024 * 1024;

/// A database never gets less than this share, so a worker with many tablets still flushes
/// full-sized L0 SSTs of at least half of it.
pub const MIN_UNFLUSHED_SHARE_BYTES: usize = 8 * 1024 * 1024;

/// Above this many maintained tablets each database runs one compaction at a time, so the
/// process does not run two per tablet. SlateDB has no limiter shared across databases.
pub const SINGLE_COMPACTION_ABOVE_TABLETS: usize = 8;

impl ColdStoreConfig {
	/// Limits for one database on a worker that maintains `maintained` tablets.
	///
	/// The unflushed budget is split evenly, bounded by `max_unflushed_bytes` above and
	/// `MIN_UNFLUSHED_SHARE_BYTES` below. SlateDB requires its unflushed limit to exceed the L0
	/// SST size, so the L0 SST size drops to half the share when the share is smaller than twice
	/// the configured size. A database keeps the limits it opened with; after the maintained
	/// count grows, older databases keep their larger share until they reopen.
	pub fn for_maintained_tablets(&self, maintained: usize) -> Self {
		let tablets = maintained.max(1);
		let share = (self.unflushed_budget_bytes / tablets)
			.min(self.max_unflushed_bytes)
			.max(MIN_UNFLUSHED_SHARE_BYTES);
		let mut config = self.clone();
		config.max_unflushed_bytes = share;
		config.l0_sst_size_bytes = self.l0_sst_size_bytes.min(share / 2);
		if tablets > SINGLE_COMPACTION_ABOVE_TABLETS {
			config.max_concurrent_compactions = 1;
		}
		config
	}

	/// The volume log is the write-ahead log. Frozen parents retain their files
	/// without running compaction or garbage collection while clones are prepared.
	fn settings(&self, frozen: bool) -> Result<Settings> {
		self.checkpoints.validate()?;
		ensure!(
			self.l0_sst_size_bytes > 0
				&& self.l0_max_ssts >= self.min_compaction_sources
				&& self.max_unflushed_bytes > self.l0_sst_size_bytes
				&& self.unflushed_budget_bytes > 0
				&& self.compactor_poll_interval_ms > 0
				&& self.manifest_poll_interval_ms > 0
				&& self.compactor_commit_interval_ms > 0
				&& self.max_concurrent_compactions > 0
				&& self.compacted_sst_size_bytes > 0
				&& self.min_compaction_sources >= 2
				&& self.max_compaction_sources >= self.min_compaction_sources,
			"invalid cold-store limits"
		);
		let mut compactor = CompactorOptions {
			poll_interval: Duration::from_millis(self.compactor_poll_interval_ms),
			commit_compacted_interval: Duration::from_millis(self.compactor_commit_interval_ms),
			max_concurrent_compactions: self.max_concurrent_compactions,
			..Default::default()
		};
		if let Some(worker) = compactor.worker.as_mut() {
			worker.compactions_poll_interval =
				Duration::from_millis(self.compactor_poll_interval_ms);
			worker.max_sst_size = self.compacted_sst_size_bytes;
		}
		compactor.scheduler_options.insert(
			"min_compaction_sources".into(),
			self.min_compaction_sources.to_string(),
		);
		compactor.scheduler_options.insert(
			"max_compaction_sources".into(),
			self.max_compaction_sources.to_string(),
		);
		let mut settings = Settings {
			wal_enabled: false,
			flush_interval: None,
			l0_sst_size_bytes: self.l0_sst_size_bytes,
			l0_max_ssts: self.l0_max_ssts,
			l0_max_ssts_per_key: self.l0_max_ssts,
			max_unflushed_bytes: self.max_unflushed_bytes,
			min_filter_keys: self.min_filter_keys,
			manifest_poll_interval: Duration::from_millis(self.manifest_poll_interval_ms),
			compactor_options: if frozen { None } else { Some(compactor) },
			compression_codec: self.compression_codec,
			..Default::default()
		};
		if frozen {
			settings.garbage_collector_options = None;
		}
		Ok(settings)
	}
}

/// A database opened at the tablet grant epoch. Closing it joins its flusher,
/// compactor and garbage collector before the owner may release the grant.
pub struct ColdStore {
	pub(crate) filesystem: std::sync::Arc<crate::filesystem::store::FilesystemStore>,
	db: Db,
	compaction_stalls: std::sync::Arc<crate::metrics::CompactionStalls>,
	/// Whether this database's unflushed bytes are at its `max_unflushed_bytes`, where every
	/// write waits for a memtable upload.
	unflushed_full: tokio::sync::watch::Receiver<bool>,
	pub tablet_grant_epoch: u64,
}

impl ColdStore {
	/// Prepare one immutable split step through the same store owner that serves frozen reads.
	pub(crate) async fn prepare_split(
		&self,
		factory: &dyn SlateDbFactory,
		tablet: &rivet_data::generated::depot_tablet_manifest_v1::Tablet,
		step: crate::tablet_maintainer_machine::state::split::SplitStep,
		guard: std::sync::Arc<dyn rivet_env::MutationGuard>,
	) -> Result<u64> {
		ensure!(
			self.tablet_grant_epoch == tablet.tablet_grant_epoch,
			"split grant mismatch"
		);
		super::split::prepare(&self.db, factory, tablet, step, guard).await
	}

	/// Opens a store with a default filesystem layer on the production environment, for tests
	/// that drive the store without a tablet manifest. Existing split clones must never be
	/// initialized empty.
	///
	/// With the WAL disabled, a stopped owner can lose its unflushed memtable.
	/// Trims follow the flush containing both filesystem versions and their record, so those lost
	/// writes remain in replica logs and the next maintainer safely repeats them.
	#[cfg(any(test, feature = "test-support"))]
	pub async fn open(
		factory: &dyn SlateDbFactory,
		tablet_id: Uuid,
		tablet_grant_epoch: NonZeroU64,
		creation: DatabaseCreation,
		frozen: bool,
		config: &ColdStoreConfig,
	) -> Result<Self> {
		let (compaction_stalls, unflushed_full) =
			crate::metrics::CompactionStalls::for_tablet(tablet_id)
				.with_unflushed_limit(config.max_unflushed_bytes);
		let compaction_stalls = std::sync::Arc::new(compaction_stalls);
		let db = factory
			.open_with_settings(
				TabletVolumePrefix::new(tablet_id).pack(),
				tablet_grant_epoch,
				creation,
				config.settings(frozen)?,
				Some(compaction_stalls.clone()),
			)
			.await?;
		let mut observer = crate::metrics::ColdStoreObserver::new(
			tablet_id,
			config.l0_max_ssts,
			factory.shared_cache(),
		);
		observer.observe(&db);
		let observer = std::sync::Arc::new(parking_lot::Mutex::new(observer));
		let env = rivet_env::backends::production::ProductionEnv::new()?;
		let filesystem = crate::filesystem::store::FilesystemStore::new(
			db.clone(),
			rivet_env::Env::clock(&env),
			std::sync::Arc::new(|| true),
			std::sync::Arc::new(parking_lot::Mutex::new(std::collections::BTreeMap::new())),
			crate::filesystem::session::SessionConfig::default(),
			1024,
			rivet_env::Env::rng(&env, "depot.filesystem.sessions"),
		)?
		.with_observer(observer);
		Ok(Self {
			filesystem: std::sync::Arc::new(filesystem),
			compaction_stalls,
			unflushed_full,
			db,
			tablet_grant_epoch: tablet_grant_epoch.get(),
		})
	}

	/// Opens the filesystem store with retention filters and expiring read sessions.
	/// Apply and lifecycle work share one database, grant, queue and task owner.
	pub async fn open_filesystem(
		factory: &dyn SlateDbFactory,
		tablet: &rivet_data::generated::depot_tablet_manifest_v1::Tablet,
		config: &ColdStoreConfig,
		env: &dyn rivet_env::Env,
		guard: std::sync::Arc<dyn rivet_env::MutationGuard>,
		base_reader: Option<std::sync::Arc<dyn crate::filesystem::base::BaseChunkReader>>,
	) -> Result<Self> {
		let tablet_id = Uuid::from_bytes(tablet.id);
		let epoch = NonZeroU64::new(tablet.tablet_grant_epoch).context("zero tablet grant")?;
		let frozen = tablet.split.is_some();
		// A split child must find the clone its parent's checkpoint published. A parentless
		// tablet may be created empty, even when frozen: a split can be accepted before the
		// tablet's first maintainer opened its store, and that database never existed.
		let creation = if tablet.parent_tablet_id.is_some() {
			DatabaseCreation::RequireExisting
		} else {
			DatabaseCreation::Allow
		};
		let catalog =
			std::sync::Arc::new(parking_lot::Mutex::new(std::collections::BTreeMap::new()));
		let policy = crate::filesystem::filter::storage_policy_in_range(
			catalog.clone(),
			env.clock(),
			Some(tablet.hash_range.clone()),
		);
		let session_config = crate::filesystem::session::SessionConfig {
			scan: policy.scan.clone(),
			max_sessions: config.max_read_sessions,
			read_session_idle_ms: config.read_session_idle_ms,
			latest_session_idle_ms: config.latest_session_idle_ms,
			bulk: config.bulk_run,
			..Default::default()
		};
		let (compaction_stalls, unflushed_full) =
			crate::metrics::CompactionStalls::for_tablet(tablet_id)
				.with_unflushed_limit(config.max_unflushed_bytes);
		let compaction_stalls = std::sync::Arc::new(compaction_stalls);
		let db = factory
			.open_with_storage_policy(
				TabletVolumePrefix::new(tablet_id).pack(),
				epoch,
				creation,
				config.settings(frozen)?,
				Some(compaction_stalls.clone()),
				policy,
			)
			.await?;
		let mut filesystem = crate::filesystem::store::FilesystemStore::new(
			db.clone(),
			env.clock(),
			guard,
			catalog,
			session_config,
			1024,
			env.rng("depot.filesystem.sessions"),
		)?
		.with_tablet_range(tablet.hash_range.clone())?
		.with_checkpoints(config.checkpoints.clone())?
		.with_flush_stage_concurrency(config.flush_stage_concurrency)?
		.with_volume_lock_stripes(config.volume_lock_stripes)?
		.with_volume_locks(config.volume_locks);
		if let Some(reader) = base_reader {
			filesystem = filesystem.with_base_reader(reader);
		}
		let mut observer = crate::metrics::ColdStoreObserver::new(
			tablet_id,
			config.l0_max_ssts,
			factory.shared_cache(),
		);
		observer.observe(&db);
		let observer = std::sync::Arc::new(parking_lot::Mutex::new(observer));
		let filesystem = std::sync::Arc::new(filesystem.with_observer(observer));
		Ok(Self {
			filesystem,
			db,
			compaction_stalls,
			unflushed_full,
			tablet_grant_epoch: epoch.get(),
		})
	}
	pub(crate) fn filesystem(&self) -> &crate::filesystem::store::FilesystemStore {
		&self.filesystem
	}

	/// Follows whether this database's unflushed bytes are at its limit. The receiver's
	/// `changed` fails once the database is dropped.
	pub(crate) fn unflushed_full(&self) -> tokio::sync::watch::Receiver<bool> {
		self.unflushed_full.clone()
	}

	/// Reads the name's pointer row, including the current owner's memtable. Recovery and
	/// opening combine its cold floor with member observations before using the log.
	/// The maintainer keeps rows in its state, so each row is detached from SlateDB's buffer
	/// and does not keep a cached block alive.
	///
	/// Different volume keys can share one placement and name, such as one actor's database
	/// across a destroy and recreate, so the row may belong to another volume key than
	/// `volume_key`. It is then that key's newest stored incarnation of the name.
	#[tracing::instrument(skip_all)]
	pub async fn row(&self, volume_key: depot_types::VolumeKey) -> Result<Option<FsVolumeRecord>> {
		let key = VolumeRecordKey::for_volume(&volume_key);
		let record = self
			.db
			.get(key.pack())
			.await?
			.map(|bytes| key.deserialize(bytes).map(|record| record.detach()))
			.transpose()?;
		ensure!(
			record.as_ref().is_none_or(|record| {
				depot_types::partition_key_hash(&record.volume_key())
					== depot_types::partition_key_hash(&volume_key)
					&& record.name == volume_key.name
			}),
			"volume record identity mismatch"
		);
		Ok(record)
	}

	/// Reads one incarnation's own record row, whatever the name's pointer names.
	#[tracing::instrument(skip_all)]
	pub async fn incarnation_row(&self, key: &VolumeLogKey) -> Result<Option<FsVolumeRecord>> {
		let address = incarnation_record_address(key);
		let record = self
			.db
			.get(address.pack())
			.await?
			.map(|bytes| address.deserialize(bytes).map(|record| record.detach()))
			.transpose()?;
		ensure!(
			record
				.as_ref()
				.is_none_or(|record| record.volume_key() == key.0.volume_key
					&& record.volume_incarnation == key.0.volume_incarnation),
			"volume incarnation record identity mismatch"
		);
		Ok(record)
	}

	/// Flushes lifecycle metadata in one batch after all earlier filesystem mutations. A pass
	/// may carry several incarnations of one name; each writes its own row, and the name's
	/// pointer ends at the one with the highest incarnation epoch.
	pub async fn write_rows(
		&self,
		rows: &[(depot_types::VolumeKey, FsVolumeRecord)],
	) -> Result<()> {
		let _compaction_wait = self.compaction_stalls.begin_pass();
		let snapshot = self.db.snapshot().await?;
		let mut unique = std::collections::BTreeSet::new();
		for (key, row) in rows {
			ensure!(
				unique.insert((key, row.volume_incarnation)),
				"duplicate volume row in one pass"
			);
			self.validate_row_update(&snapshot, key, row).await?;
		}
		self.filesystem
			.flush(&rows.iter().map(|(_, row)| row.clone()).collect::<Vec<_>>())
			.await
	}

	/// Writes lifecycle rows after all earlier filesystem mutations without uploading the
	/// memtable, checked like `write_rows`. Nothing here makes them durable: the tablet's
	/// next ordinary pass does, and until then a crash loses them like any other memtable
	/// write, which the replicas' tombstones recover. Retention stays as it was loaded, so
	/// checkpoints the rows release keep their pins until the rows are swept.
	pub async fn stage_rows(
		&self,
		rows: &[(depot_types::VolumeKey, FsVolumeRecord)],
	) -> Result<()> {
		let snapshot = self.db.snapshot().await?;
		let mut unique = std::collections::BTreeSet::new();
		for (key, row) in rows {
			ensure!(
				unique.insert((key, row.volume_incarnation)),
				"duplicate volume row in one pass"
			);
			self.validate_row_update(&snapshot, key, row).await?;
		}
		self.filesystem
			.stage_flush(&rows.iter().map(|(_, row)| row.clone()).collect::<Vec<_>>())
			.await
	}

	

	/// Joins every upstream task before ownership can be released or reassigned, after
	/// uploading the memtable. No stop or move path uses it: a maintainer closes with
	/// `close_without_upload`, since the upload records no cold position. Tests use it to
	/// contrast the two.
	#[tracing::instrument(skip_all)]
	pub async fn close(self) -> Result<()> {
		self.db.close().await?;
		Ok(())
	}

	/// Joins every upstream task like `close`, but drops the memtable instead of uploading it.
	/// A maintainer that stops or moves closes this way: no cold position advances from that
	/// upload, and every write in the memtable is either in a volume log or the maintainer
	/// log, which the next maintainer replays, or progress the next maintainer repeats, such
	/// as a materialize or sweep cursor or an alarm's retry backoff. Uploads already handed to
	/// the uploader finish; everything else is dropped.
	#[tracing::instrument(skip_all)]
	pub async fn close_without_upload(self) -> Result<()> {
		let options = slatedb::config::CloseOptions::default().with_flush_type(None);
		self.db.close_with_options(options).await?;
		Ok(())
	}
}











/// The address of `key`'s own record row.
fn incarnation_record_address(
	key: &VolumeLogKey,
) -> depot_fs_core::keys::slatedb::VolumeIncarnationRecordKey {
	let scope = depot_fs_core::keys::slatedb::Scope::new(
		&key.0.volume_key,
		uuid::Uuid::from_bytes(key.0.volume_incarnation),
	);
	depot_fs_core::keys::slatedb::VolumeIncarnationRecordKey::new(
		scope.hash,
		scope.name,
		scope.incarnation,
	)
}
