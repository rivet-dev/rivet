//! Production SlateDB builder with injected stores, system clock, and RNG seed.
//! SlateDB's internal tasks and scheduler remain upstream production boundaries.

use crate::MutationGuard;
use crate::{
	Clock, Env,
	capabilities::slatedb::{
		DatabaseCreation, DeleteDatabaseResult, DroppedCaches, ProjectedClone, SharedCacheUsage,
		SlateDbCacheUsage, SlateDbFactory, StoragePolicy,
	},
	rand::RngCore,
};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use object_store::{ObjectStore, path::Path};
use slatedb::db_cache::{
	CacheFetch, CacheLoader, CachedEntry, CachedKey, DbCache, SplitCache,
	foyer::{FoyerCache, FoyerCacheOptions},
};
use slatedb_common::clock::{SystemClock, SystemClockTicker};
use std::{
	future::Future,
	num::NonZeroU64,
	pin::Pin,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	time::Duration,
};
use uuid::Uuid;

mod cleanup;
mod clone;

pub struct ProductionSlateDbFactory {
	env: Arc<dyn Env>,
	/// The object store itself, for administration, clones and cleanup.
	store: Arc<dyn ObjectStore>,
	/// The store databases and their compactors read and write through: `store`, or the
	/// disk cache in front of it.
	db_store: Arc<dyn ObjectStore>,
	cache: Option<Arc<SharedSlateDbCache>>,
	/// The disk cache's directory, when `db_store` reads through one.
	disk_cache_directory: Option<std::path::PathBuf>,
}

/// A local disk cache of object parts shared by every database of a factory.
///
/// SlateDB admits every foreground SST read, the SSTs a memtable flush uploads and the
/// SSTs compaction writes, so a block evicted from the memory cache is a local read
/// instead of an object-store GET. Manifests, WAL files and compaction input bypass it.
/// Cache files are keyed by object path, and SST ids are never reused, so entries from a
/// deleted or recreated database are never read; they age out at the capacity.
pub struct DiskCacheOptions {
	pub directory: std::path::PathBuf,
	pub capacity_bytes: u64,
	/// Bytes fetched per miss and stored per file; a positive multiple of 1 KiB.
	pub part_bytes: usize,
	pub recorder: Option<Arc<dyn slatedb_common::metrics::MetricsRecorder>>,
	/// Also write SSTs that memtable flushes upload into the cache.
	pub cache_on_flush: bool,
	/// Also write SSTs that compactions upload into the cache.
	pub cache_on_compaction: bool,
}

impl ProductionSlateDbFactory {
	/// Each database this factory opens keeps SlateDB's default per-database caches.
	pub fn new(env: Arc<dyn Env>, store: Arc<dyn ObjectStore>) -> Self {
		Self {
			env,
			db_store: store.clone(),
			store,
			cache: None,
			disk_cache_directory: None,
		}
	}

	/// Every database this factory opens reads and writes through one disk cache. Starts
	/// the cache's evictor, which keeps it under `capacity_bytes`.
	#[allow(
		clippy::disallowed_methods,
		reason = "production SlateDB cache construction boundary"
	)]
	pub async fn with_disk_cache(mut self, options: DiskCacheOptions) -> Result<Self> {
		ensure!(
			options.capacity_bytes > 0,
			"disk cache capacity must be positive"
		);
		let capacity = usize::try_from(options.capacity_bytes)
			.context("disk cache capacity does not fit in memory")?;
		let mut builder = slatedb::cached_object_store::CachedObjectStore::builder(
			options.directory.clone(),
			self.store.clone(),
		)
		.with_max_cache_size_bytes(Some(capacity))
		.with_part_size_bytes(options.part_bytes)
		.with_cache_on_flush(options.cache_on_flush)
		.with_cache_on_compaction(options.cache_on_compaction);
		if let Some(recorder) = options.recorder {
			builder = builder.with_metrics_recorder(recorder);
		}
		let cached = builder.build().await.with_context(|| {
			format!(
				"failed to open the cold-store disk cache at {}",
				options.directory.display()
			)
		})?;
		self.db_store = cached;
		self.disk_cache_directory = Some(options.directory);
		Ok(self)
	}

	/// Every database this factory opens shares `cache` instead of allocating its own.
	pub fn with_shared_cache(mut self, cache: Arc<SharedSlateDbCache>) -> Self {
		self.cache = Some(cache);
		self
	}
}

/// One bounded block cache and one bounded metadata cache shared by every database of a
/// factory, so a process's cache memory does not grow with its open databases.
///
/// Entries are scoped by a cache id drawn fresh for every open. WAL SST ids are
/// per-database sequence numbers, so a reopened or recreated database at the same path must
/// never see another lifetime's entries; entries of a closed database age out of the LRU.
/// Databases hold the caches as unowned, so closing one never closes them. The caches are
/// in memory only and hold no resources beyond it, so dropping the last handle frees them.
pub struct SharedSlateDbCache {
	block: Arc<ClearableCache>,
	meta: Arc<ClearableCache>,
	cache: Arc<dyn DbCache>,
	block_capacity_bytes: u64,
	meta_capacity_bytes: u64,
	next_db_cache_id: AtomicU64,
}

impl SharedSlateDbCache {
	pub fn new(block_capacity_bytes: u64, meta_capacity_bytes: u64) -> Result<Arc<Self>> {
		ensure!(
			block_capacity_bytes > 0 && meta_capacity_bytes > 0,
			"SlateDB cache capacities must be positive"
		);
		let block = Arc::new(ClearableCache::new(block_capacity_bytes));
		let meta = Arc::new(ClearableCache::new(meta_capacity_bytes));
		let cache = Arc::new(
			SplitCache::new()
				.with_block_cache(Some(block.clone() as Arc<dyn DbCache>))
				.with_meta_cache(Some(meta.clone() as Arc<dyn DbCache>))
				.build(),
		);
		Ok(Arc::new(Self {
			block,
			meta,
			cache,
			block_capacity_bytes,
			meta_capacity_bytes,
			next_db_cache_id: AtomicU64::new(1),
		}))
	}

	fn next_db_cache_id(&self) -> u64 {
		self.next_db_cache_id.fetch_add(1, Ordering::Relaxed)
	}

	/// Empties both caches and returns the bytes they held.
	pub fn clear(&self) -> u64 {
		self.block.clear().saturating_add(self.meta.clear())
	}
}

/// A foyer cache that can be emptied while databases use it. Clearing swaps in a fresh cache
/// of the same capacity; reads and fetches already running finish against the old one, which
/// is freed when they do, so a clear never blocks or fails a read.
struct ClearableCache {
	capacity_bytes: u64,
	current: parking_lot::RwLock<Arc<FoyerCache>>,
}

impl ClearableCache {
	fn new(capacity_bytes: u64) -> Self {
		Self {
			capacity_bytes,
			current: parking_lot::RwLock::new(Self::fresh(capacity_bytes)),
		}
	}

	fn fresh(capacity_bytes: u64) -> Arc<FoyerCache> {
		Arc::new(FoyerCache::new_with_opts(FoyerCacheOptions {
			max_capacity: capacity_bytes,
			..Default::default()
		}))
	}

	fn inner(&self) -> Arc<FoyerCache> {
		self.current.read().clone()
	}

	/// Replaces the cache with an empty one and returns the bytes the old one held.
	fn clear(&self) -> u64 {
		let old = std::mem::replace(&mut *self.current.write(), Self::fresh(self.capacity_bytes));
		old.estimated_memory_usage().unwrap_or(0)
	}
}

#[async_trait::async_trait]
impl DbCache for ClearableCache {
	fn estimated_memory_usage(&self) -> Option<u64> {
		self.inner().estimated_memory_usage()
	}

	async fn get_block(&self, key: &CachedKey) -> Result<Option<CachedEntry>, slatedb::Error> {
		self.inner().get_block(key).await
	}

	async fn get_index(&self, key: &CachedKey) -> Result<Option<CachedEntry>, slatedb::Error> {
		self.inner().get_index(key).await
	}

	async fn get_filter(&self, key: &CachedKey) -> Result<Option<CachedEntry>, slatedb::Error> {
		self.inner().get_filter(key).await
	}

	async fn get_stats(&self, key: &CachedKey) -> Result<Option<CachedEntry>, slatedb::Error> {
		self.inner().get_stats(key).await
	}

	async fn insert(&self, key: CachedKey, value: CachedEntry) {
		self.inner().insert(key, value).await
	}

	async fn remove(&self, key: &CachedKey) {
		self.inner().remove(key).await
	}

	fn entry_count(&self) -> u64 {
		self.inner().entry_count()
	}

	async fn fetch_block(
		&self,
		key: CachedKey,
		loader: CacheLoader,
	) -> Result<CacheFetch, slatedb::Error> {
		self.inner().fetch_block(key, loader).await
	}

	async fn fetch_index(
		&self,
		key: CachedKey,
		loader: CacheLoader,
	) -> Result<CacheFetch, slatedb::Error> {
		self.inner().fetch_index(key, loader).await
	}

	async fn fetch_filter(
		&self,
		key: CachedKey,
		loader: CacheLoader,
	) -> Result<CacheFetch, slatedb::Error> {
		self.inner().fetch_filter(key, loader).await
	}

	async fn fetch_stats(
		&self,
		key: CachedKey,
		loader: CacheLoader,
	) -> Result<CacheFetch, slatedb::Error> {
		self.inner().fetch_stats(key, loader).await
	}
}

/// Removes every disk-cache file under `root` and returns how many and their bytes. Only the
/// names the cache writes count: object parts (`_part...`) and heads (`_head...`), including
/// their temporary files, so a misconfigured cache directory never loses anything else. A
/// missing root removes nothing. Directories stay, so the cache keeps its layout; a part
/// removed while being read reads as a miss and is fetched from object storage again.
#[allow(
	clippy::disallowed_methods,
	reason = "the production SlateDB adapter owns its disk cache directory"
)]
fn remove_files_under(root: &std::path::Path) -> std::io::Result<(u64, u64)> {
	let mut files = 0u64;
	let mut bytes = 0u64;
	let mut pending = vec![root.to_path_buf()];
	while let Some(dir) = pending.pop() {
		let entries = match std::fs::read_dir(&dir) {
			Ok(entries) => entries,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
			Err(error) => return Err(error),
		};
		for entry in entries {
			let entry = entry?;
			let kind = entry.file_type()?;
			if kind.is_dir() {
				pending.push(entry.path());
			} else if kind.is_file() {
				let name = entry.file_name();
				let name = name.to_string_lossy();
				if !name.starts_with("_part") && !name.starts_with("_head") {
					continue;
				}
				let len = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
				match std::fs::remove_file(entry.path()) {
					Ok(()) => {
						files += 1;
						bytes = bytes.saturating_add(len);
					}
					Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
					Err(error) => return Err(error),
				}
			}
		}
	}
	Ok((files, bytes))
}

impl SharedCacheUsage for SharedSlateDbCache {
	fn usage(&self) -> SlateDbCacheUsage {
		SlateDbCacheUsage {
			block_bytes: self.block.estimated_memory_usage().unwrap_or(0),
			block_capacity_bytes: self.block_capacity_bytes,
			meta_bytes: self.meta.estimated_memory_usage().unwrap_or(0),
			meta_capacity_bytes: self.meta_capacity_bytes,
		}
	}
}

impl SlateDbFactory for ProductionSlateDbFactory {
	#[allow(
		clippy::disallowed_methods,
		reason = "injected SlateDB checkpoint reader boundary"
	)]
	fn checkpoint_page(
		&self,
		path: Path,
		checkpoint: Uuid,
		manifest: u64,
		range: (std::ops::Bound<bytes::Bytes>, std::ops::Bound<bytes::Bytes>),
		limit: usize,
	) -> Pin<Box<dyn Future<Output = Result<Vec<(bytes::Bytes, bytes::Bytes)>>> + Send + '_>> {
		Box::pin(async move {
			ensure!(limit > 0 && limit <= 128, "invalid checkpoint page size");
			let admin = slatedb::admin::AdminBuilder::new(path.clone(), self.store.clone())
				.with_system_clock(Arc::new(SlateClock(self.env.clock())))
				.build();
			let latest = admin
				.read_manifest(None)
				.await?
				.context("checkpoint source missing")?;
			ensure!(
				latest.checkpoints().iter().any(|c| c.id == checkpoint
					&& c.manifest_id == manifest
					&& c.expire_time.is_none()),
				"checkpoint source changed"
			);
			let reader = slatedb::DbReader::builder(path, self.db_store.clone())
				.with_reader_mode(slatedb::DbReaderMode::Checkpoint(checkpoint))
				.with_db_cache_disabled()
				.with_system_clock(Arc::new(SlateClock(self.env.clock())))
				.with_seed(self.env.rng("slatedb.checkpoint_page").next_u64())
				.build()
				.await?;
			let result = async {
				let mut scan = reader.scan(range).await?;
				let mut rows = Vec::new();
				let mut bytes = 0usize;
				while rows.len() < limit {
					let Some(row) = scan.next().await? else {
						break;
					};
					let next_bytes = bytes
						.saturating_add(row.key.len())
						.saturating_add(row.value.len());
					if next_bytes > 256 * 1024 {
						ensure!(!rows.is_empty(), "checkpoint row exceeds byte bound");
						break;
					}
					bytes = next_bytes;
					rows.push((row.key, row.value));
				}
				Ok::<_, anyhow::Error>(rows)
			}
			.await;
			reader.close().await?;
			result
		})
	}

	fn shared_cache(&self) -> Option<Arc<dyn SharedCacheUsage>> {
		self.cache
			.clone()
			.map(|cache| cache as Arc<dyn SharedCacheUsage>)
	}

	#[allow(
		clippy::disallowed_methods,
		reason = "the production SlateDB adapter removes disk cache files off the executor"
	)]
	fn drop_caches(
		&self,
		prefix: Option<Path>,
	) -> Pin<Box<dyn Future<Output = Result<DroppedCaches>> + Send + '_>> {
		Box::pin(async move {
			ensure!(
				self.cache.is_some() || self.disk_cache_directory.is_some(),
				"this SlateDB factory keeps per-database caches, which cannot be dropped"
			);
			let memory_bytes = self.cache.as_ref().map_or(0, |cache| cache.clear());
			let (disk_files, disk_bytes) = match &self.disk_cache_directory {
				Some(directory) => {
					// Cache parts live at the object's path under the cache directory.
					let root = match &prefix {
						Some(prefix) => directory.join(prefix.as_ref()),
						None => directory.clone(),
					};
					tokio::task::spawn_blocking(move || remove_files_under(&root))
						.await
						.context("disk cache removal task failed")?
						.context("failed to remove disk-cached object parts")?
				}
				None => (0, 0),
			};
			Ok(DroppedCaches {
				memory_bytes,
				disk_files,
				disk_bytes,
			})
		})
	}

	fn delete_unpinned_database(
		&self,
		path: Path,
		retained_checkpoint: Uuid,
		guard: Arc<dyn MutationGuard>,
	) -> Pin<Box<dyn Future<Output = Result<DeleteDatabaseResult>> + Send + '_>> {
		Box::pin(async move {
			ensure!(
				!path.as_ref().is_empty(),
				"cannot delete an empty database prefix"
			);
			ensure!(!retained_checkpoint.is_nil(), "retained checkpoint is nil");
			ensure!(guard.may_send(), "database cleanup authority revoked");
			let guarded = cleanup::CleanupStore::new(self.store.clone(), guard.clone());
			let builder = slatedb::admin::AdminBuilder::new(path, guarded)
				.with_system_clock(Arc::new(SlateClock(self.env.clock())));
			let admin = builder.build();

			// Child and reader checkpoints retain data even if the current parent
			// manifest no longer references it. Never bypass any such pin.
			if let Some(manifest) = admin.read_manifest(None).await? {
				if manifest
					.checkpoints()
					.iter()
					.any(|cp| cp.id != retained_checkpoint)
				{
					return Ok(DeleteDatabaseResult::Pinned);
				}
				ensure!(guard.may_send(), "database cleanup authority revoked");
				if manifest
					.checkpoints()
					.iter()
					.any(|cp| cp.id == retained_checkpoint)
				{
					admin.delete_checkpoint(retained_checkpoint).await?;
				}
				// A retry may have already removed the retained checkpoint. Re-read
				// before deletion rather than treating that absence as permission.
				if admin
					.read_manifest(None)
					.await?
					.is_some_and(|manifest| !manifest.checkpoints().is_empty())
				{
					return Ok(DeleteDatabaseResult::Pinned);
				}
			}
			ensure!(guard.may_send(), "database cleanup authority revoked");
			admin.delete_db(true).await?;
			Ok(DeleteDatabaseResult::Deleted)
		})
	}

	fn open_with_settings(
		&self,
		path: Path,
		epoch: NonZeroU64,
		creation: DatabaseCreation,
		settings: slatedb::config::Settings,
		recorder: Option<Arc<dyn slatedb_common::metrics::MetricsRecorder>>,
	) -> Pin<Box<dyn Future<Output = Result<slatedb::Db>> + Send + '_>> {
		self.open_with_storage_policy(
			path,
			epoch,
			creation,
			settings,
			recorder,
			StoragePolicy::default(),
		)
	}

	#[allow(
		clippy::disallowed_methods,
		reason = "production SlateDB construction boundary"
	)]
	fn open_with_storage_policy(
		&self,
		path: Path,
		epoch: NonZeroU64,
		creation: DatabaseCreation,
		settings: slatedb::config::Settings,
		recorder: Option<Arc<dyn slatedb_common::metrics::MetricsRecorder>>,
		policy: StoragePolicy,
	) -> Pin<Box<dyn Future<Output = Result<slatedb::Db>> + Send + '_>> {
		Box::pin(async move {
			self.env.clock().observe()?;
			if creation == DatabaseCreation::RequireExisting {
				let manifest = slatedb::admin::AdminBuilder::new(path.clone(), self.store.clone())
					.with_system_clock(Arc::new(SlateClock(self.env.clock())))
					.build()
					.read_manifest(None)
					.await?
					.context("committed database is missing; refusing empty initialization")?;
				ensure!(
					manifest.initialized(),
					"committed database is not initialized"
				);
			}
			let seed = self.env.rng(&format!("slatedb.{path}")).next_u64();
			let compactor = policy
				.compaction_filter
				.clone()
				.filter(|_| settings.compactor_options.is_some())
				.map(|supplier| {
					let mut builder =
						slatedb::CompactorBuilder::new(path.clone(), self.db_store.clone())
							.with_system_clock(Arc::new(SlateClock(self.env.clock())))
							.with_seed(seed)
							.with_compaction_filter_supplier(supplier);
					if let Some(options) = &settings.compactor_options {
						builder = builder.with_options(options.clone());
					}
					if let Some(filters) = &policy.filters {
						builder = builder.with_filter_policies(filters.clone());
					}
					builder
				});
			let mut builder = slatedb::Db::builder(path, self.db_store.clone())
				.with_system_clock(Arc::new(SlateClock(self.env.clock())))
				.with_seed(seed)
				.with_settings(settings)
				.with_leadership_epoch(epoch)
				.with_block_cache_policy(policy.block_cache);
			if let Some(cache) = &self.cache {
				builder = builder.with_db_cache(cache.cache.clone(), cache.next_db_cache_id());
			}
			if let Some(size) = policy.sst_block_size {
				builder = builder.with_sst_block_size(size);
			}
			if let Some(filters) = policy.filters {
				builder = builder.with_filter_policies(filters);
			}
			if let Some(compactor) = compactor {
				builder = builder.with_compactor_builder(compactor);
			}
			if let Some(recorder) = recorder {
				builder = builder.with_metrics_recorder(recorder);
			}
			Ok(builder.build().await?)
		})
	}

	fn final_checkpoint<'a>(
		&'a self,
		db: &'a slatedb::Db,
		id: Uuid,
		guard: Arc<dyn MutationGuard>,
	) -> Pin<Box<dyn Future<Output = Result<slatedb::CheckpointCreateResult>> + Send + 'a>> {
		Box::pin(async move {
			ensure!(!id.is_nil(), "nil final checkpoint ID");
			ensure!(guard.may_send(), "checkpoint authority revoked");
			db.flush_with_options(slatedb::config::FlushOptions {
				flush_type: slatedb::config::FlushType::MemTable,
			})
			.await?;
			ensure!(guard.may_send(), "checkpoint authority revoked after flush");
			let checkpoint = db
				.create_checkpoint(
					slatedb::config::CheckpointScope::All,
					&slatedb::config::CheckpointOptions {
						checkpoint_id: Some(id),
						..Default::default()
					},
				)
				.await?;
			ensure!(
				guard.may_send(),
				"checkpoint outcome requires reconciliation after revocation"
			);
			Ok(checkpoint)
		})
	}

	#[allow(
		clippy::disallowed_methods,
		reason = "production SlateDB clone construction boundary"
	)]
	fn clone_projected(
		&self,
		request: ProjectedClone,
		guard: Arc<dyn MutationGuard>,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
		Box::pin(async move {
			ensure!(guard.may_send(), "clone authority revoked");
			ensure!(
				request.source != request.destination && !request.checkpoint_id.is_nil(),
				"invalid clone identity"
			);
			let store = clone::CloneStore::new(self.store.clone(), guard.clone());
			let source = slatedb::admin::AdminBuilder::new(request.source.clone(), store.clone())
				.with_system_clock(Arc::new(SlateClock(self.env.clock())))
				.build();
			let latest = source
				.read_manifest(None)
				.await?
				.context("source database missing")?;
			let checkpoint = latest
				.checkpoints()
				.iter()
				.find(|c| c.id == request.checkpoint_id)
				.context("retained source checkpoint missing")?;
			ensure!(
				checkpoint.manifest_id == request.manifest_id
					&& checkpoint.expire_time.is_none()
					&& checkpoint.name.is_none(),
				"source checkpoint does not match committed intent"
			);
			let original = source
				.read_manifest(Some(request.manifest_id))
				.await?
				.context("source checkpoint manifest missing")?;
			let builder = slatedb::admin::AdminBuilder::new(request.destination, store)
				.with_system_clock(Arc::new(SlateClock(self.env.clock())))
				.with_seed(self.env.rng("slatedb.clone").next_u64());
			let destination = builder.build();
			// Refuse a different projection before attempting any repair. The upstream
			// retry check validates lineage but does not compare projection ranges.
			if let Some(existing) = destination.read_manifest(None).await? {
				ensure!(
					existing.matches_projected_data_from(&original, request.range.clone())?,
					"clone projection differs or child writer has advanced"
				);
			}
			ensure!(
				guard.may_send(),
				"clone authority revoked after verification"
			);
			destination
				.create_clone_builder_from_source(
					slatedb::CloneSourceSpec::with_checkpoint(
						request.source.clone(),
						request.checkpoint_id,
					)
					.with_projection_range(request.range.clone()),
				)
				.build()
				.await?;
			let child = destination
				.read_manifest(None)
				.await?
				.context("clone manifest missing after creation")?;
			ensure!(
				child.initialized()
					&& child.matches_projected_data_from(&original, request.range)?,
				"clone projection verification failed"
			);
			ensure!(
				child
					.external_dbs()
					.iter()
					.any(|db| db.path == request.source.as_ref()
						&& db.source_checkpoint_id == request.checkpoint_id),
				"clone checkpoint lineage differs"
			);
			ensure!(
				guard.may_send(),
				"clone outcome requires reconciliation after revocation"
			);
			Ok(())
		})
	}
}

struct SlateClock(Arc<dyn Clock>);

impl std::fmt::Debug for SlateClock {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("SlateClock")
	}
}

/// SlateDB's clock cannot return errors. Production fails stop instead of letting
/// upstream storage continue with fabricated time after a clock failure.
fn clock_failure(error: impl std::fmt::Display) -> ! {
	tracing::error!(%error, "SlateDB clock failed");
	std::process::exit(1)
}

impl SystemClock for SlateClock {
	fn now(&self) -> DateTime<Utc> {
		let time = self
			.0
			.observe()
			.unwrap_or_else(|error| clock_failure(error));
		DateTime::from_timestamp_millis(time.unix_ms)
			.unwrap_or_else(|| clock_failure("wall timestamp is out of range"))
	}

	fn sleep<'a>(&'a self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
		Box::pin(async move {
			if let Err(error) = crate::sleep(self.0.as_ref(), duration).await {
				clock_failure(error);
			}
		})
	}

	fn ticker<'a>(&'a self, duration: Duration) -> SystemClockTicker<'a> {
		SystemClockTicker::new(self, duration)
	}
}
