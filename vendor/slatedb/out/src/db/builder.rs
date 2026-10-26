// Local leadership/lifecycle modifications; see LEADERSHIP.md in this package.
//! This module provides a builder for creating new Db instances.
//!
//! The `DbBuilder` struct is used to configure and open a SlateDB database.
//! It provides a fluent API for setting various options and components.
//!
//! # Examples
//!
//! Basic usage of the `DbBuilder` struct:
//!
//! ```
//! use slatedb::{Db, Error};
//! use slatedb::object_store::memory::InMemory;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let object_store = Arc::new(InMemory::new());
//!     let db = Db::builder("test_db", object_store)
//!         .build()
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
//! Example with custom settings:
//!
//! ```
//! use slatedb::{Db, config::Settings, Error};
//! use slatedb::object_store::memory::InMemory;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let object_store = Arc::new(InMemory::new());
//!     let db = Db::builder("test_db", object_store)
//!         .with_settings(Settings {
//!             min_filter_keys: 2000,
//!             ..Default::default()
//!         })
//!         .build()
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
//! Example with a custom block cache:
//!
//! ```
//! use slatedb::{Db, Error};
//! use slatedb::db_cache::foyer::FoyerCache;
//! use slatedb::object_store::memory::InMemory;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let object_store = Arc::new(InMemory::new());
//!     let db = Db::builder("test_db", object_store)
//!         .with_db_cache(Arc::new(FoyerCache::new()), 0)
//!         .build()
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
//! Example with a custom clock:
//!
//! ```
//! use slatedb::{Db, Error};
//! use slatedb::object_store::memory::InMemory;
//! use slatedb_common::clock::DefaultSystemClock;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let object_store = Arc::new(InMemory::new());
//!     let clock = Arc::new(DefaultSystemClock::new());
//!     let db = Db::builder("test_db", object_store)
//!         .with_system_clock(clock)
//!         .build()
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
//! Example with a custom SST block size:
//!
//! ```
//! use slatedb::{Db, Error};
//! use slatedb::config::SstBlockSize;
//! use slatedb::object_store::memory::InMemory;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let object_store = Arc::new(InMemory::new());
//!     let db = Db::builder("test_db", object_store)
//!         .with_sst_block_size(SstBlockSize::Block8Kib) // 8KiB blocks
//!         .build()
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
use std::collections::{BTreeSet, HashMap};
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

use bytes::Bytes;
use fail_parallel::FailPointRegistry;
use log::info;
use log::warn;
use object_store::path::Path;
use object_store::ObjectStore;
use rand::RngCore;
use std::num::NonZeroU64;
use tokio::runtime::Handle;

use crate::admin::Admin;
use crate::batch_write::WriteBatchEventHandler;
use crate::batch_write::WRITE_BATCH_TASK_NAME;
use crate::block_cache_policy::BlockCachePolicy;
use crate::cached_object_store::CachedObjectStore;
use crate::clone::{SegmentFilterFn, SegmentProjectionFn};
#[cfg(feature = "compaction_filters")]
use crate::compaction_filter::CompactionFilterSupplier;
use crate::compaction_worker::{
    CompactionWorker, CompactionWorkerHandler, WorkerMessage, COMPACTION_WORKER_TASK_NAME,
};
use crate::sequenced_store::BoundaryReuse;
use crate::compactions_store::CompactionsStore;
use crate::compactor::stats::CompactionStats;
use crate::compactor::CompactorEventHandler;
use crate::compactor::CompactorMessage;
use crate::compactor::SizeTieredCompactionSchedulerSupplier;
use crate::compactor::COMPACTOR_TASK_NAME;
use crate::compactor::{CompactionSchedulerSupplier, Compactor};
use crate::config::DbReaderOptions;
use crate::config::GarbageCollectorOptions;
use crate::config::{CompactionWorkerOptions, CompactorOptions};
use crate::config::{Settings, SstBlockSize};
use crate::db::Db;
use crate::db::DbInner;
use crate::db_cache::SplitCache;
use crate::db_cache::{DbCache, DbCacheAndScope, DbCacheWrapper, UnownedDbCache};
use crate::db_reader::{DbReader, DbReaderMode};
use crate::db_status::{ClosedResultWriter, DbStatusManager};
use crate::dispatcher::MessageHandlerExecutor;
use crate::error::SlateDBError;
use crate::fence::{WriterFenceResult, WriterFencer};
use crate::filter_policy::{BloomFilterPolicy, FilterPolicy};
use crate::format::sst::{BlockTransformer, SsTableFormat};
use crate::garbage_collector::GC_TASK_NAME;
use crate::garbage_collector::{GarbageCollector, GcFilter};
use crate::instrumented_object_store::{InstrumentedObjectStore, ObjectStoreComponent};
use crate::manifest::store::{ManifestStore, StoredManifest};
use crate::manifest::ManifestCore;
use crate::memtable_flusher::MemtableFlusher;
use crate::merge_operator::MergeOperatorType;
use crate::paths::PathResolver;
use crate::retrying_object_store::RetryingObjectStore;
use crate::tablestore::{TableStore, TableStoreKind};
use crate::utils::ObjectStoreType;
use crate::utils::SafeSender;
use crate::utils::WatchableOnceCell;
use crate::wal;
use crate::wal::slatedb::admin::SlateDbWalAdmin;
use crate::wal::slatedb::store::WalTableStore;
use crate::wal::wal_disabled::DisabledWalObserver;
use crate::wal::{WalAdmin, WalGc, WalObserver};
use slatedb_common::clock::DefaultSystemClock;
use slatedb_common::clock::SystemClock;
use slatedb_common::metrics::MetricsRecorder;
use slatedb_common::metrics::MetricsRecorderHelper;
use slatedb_common::metrics::NoopMetricsRecorder;
use slatedb_common::DbRand;
use uuid::Uuid;

/// A builder for creating a new Db instance.
///
/// This builder provides a fluent API for configuring and opening a SlateDB database.
/// It separates the concerns of configuration options (settings) and components.
pub struct DbBuilder<P: Into<Path>> {
    path: P,
    settings: Settings,
    main_object_store: Arc<dyn ObjectStore>,
    wal_object_store: Option<Arc<dyn ObjectStore>>,
    db_cache: Option<DbCacheAndScope>,
    block_cache_policy: BlockCachePolicy,
    system_clock: Option<Arc<dyn SystemClock>>,
    gc_runtime: Option<Handle>,
    write_runtime: Option<Handle>,
    compactor_builder: Option<CompactorBuilder<Path>>,
    gc_builder: Option<GarbageCollectorBuilder<Path>>,
    fp_registry: Arc<FailPointRegistry>,
    seed: Option<u64>,
    sst_block_size: Option<SstBlockSize>,
    merge_operator: Option<MergeOperatorType>,
    block_transformer: Option<Arc<dyn BlockTransformer>>,
    filter_policies: Vec<Arc<dyn FilterPolicy>>,
    metrics_recorder: Arc<dyn MetricsRecorder>,
    segment_extractor: Option<Arc<dyn crate::prefix_extractor::PrefixExtractor>>,
    wal_writer_init: Option<Box<dyn wal::WriterInit>>,
    leadership_epoch: Option<NonZeroU64>,
}

impl<P: Into<Path>> DbBuilder<P> {
    /// Creates a new builder for a database at the given path.
    ///
    /// `main_object_store` may be any [`ObjectStore`], including a wrapper
    /// like
    /// [`CachedObjectStore`](crate::cached_object_store::CachedObjectStore)
    /// built over the raw backend to serve SST reads from a local disk cache.
    pub fn new(path: P, main_object_store: Arc<dyn ObjectStore>) -> Self {
        Self {
            path,
            main_object_store,
            settings: Settings::default(),
            wal_object_store: None,
            db_cache: default_db_cache(),
            block_cache_policy: BlockCachePolicy::default(),
            system_clock: None,
            gc_runtime: None,
            write_runtime: None,
            compactor_builder: None,
            gc_builder: None,
            fp_registry: Arc::new(FailPointRegistry::new()),
            seed: None,
            sst_block_size: None,
            merge_operator: None,
            block_transformer: None,
            filter_policies: default_filter_policies(),
            metrics_recorder: Arc::new(NoopMetricsRecorder::new()),
            segment_extractor: None,
            wal_writer_init: None,
            leadership_epoch: None,
        }
    }

    /// Supply the exact epoch for this database's writer and embedded compactor.
    ///
    /// The epoch must exceed the persisted writer, compactor-manifest, and
    /// compactions epochs. Equal or older epochs fail with fencing; retries never
    /// allocate a different epoch. An unsuccessful build may consume its epoch.
    /// Every managed
    /// writer and compactor must participate in the same external token protocol.
    /// Independent automatic-epoch compactors must not access a managed database.
    /// Omitting this option preserves ordinary automatic epoch allocation.
    pub fn with_leadership_epoch(mut self, epoch: NonZeroU64) -> Self {
        self.leadership_epoch = Some(epoch);
        self
    }

    /// Set the segment extractor (RFC-0024). When configured, every
    /// write is routed through the extractor and the database tracks
    /// per-segment LSM state. The extractor must be configured at
    /// database creation time and remain configured thereafter. Its name
    /// must remain stable; its implementation may evolve only if it preserves
    /// routing for all existing key schemas and keeps segment prefixes across
    /// schema versions an antichain (no prefix may be a proper prefix of
    /// another).
    pub fn with_segment_extractor(
        mut self,
        extractor: Arc<dyn crate::prefix_extractor::PrefixExtractor>,
    ) -> Self {
        self.segment_extractor = Some(extractor);
        self
    }

    /// Sets the database settings.
    pub fn with_settings(mut self, settings: Settings) -> Self {
        if self.compactor_builder.is_some() && settings.compactor_options.is_some() {
            warn!("compactor_builder and settings.compactor_options both set; compactor_builder will take precedence");
        }
        if self.gc_builder.is_some() && settings.garbage_collector_options.is_some() {
            warn!("gc_builder and settings.garbage_collector_options both set; gc_builder will take precedence");
        }
        self.settings = settings;
        self
    }

    /// Sets the separate object store dedicated specifically for WAL.
    ///
    /// NOTE: WAL durability and availability properties depend on the properties
    /// of the underlying object store. Make sure the configured object store is
    /// durable and available enough for your use case.
    pub fn with_wal_object_store(mut self, wal_object_store: Arc<dyn ObjectStore>) -> Self {
        self.wal_object_store = Some(wal_object_store);
        self
    }

    /// Sets the WAL writer initializer used to fence the WAL and create the writer.
    /// Use this to plug in a custom WAL implementation. SlateDB's object-store-backed
    /// WAL is used by default.
    pub fn with_wal_writer(mut self, wal_writer_init: Box<dyn wal::WriterInit>) -> Self {
        self.wal_writer_init = Some(wal_writer_init);
        self
    }

    /// Sets the cache to use for the database for caching sst blocks
    ///
    /// SlateDB uses a cache to efficiently store and retrieve blocks and SST metadata locally.
    /// [`slatedb::db_cache::SplitCache`] is used by default.
    ///
    /// A cache passed in here remains owned by the caller: it is safe to share it across
    /// multiple `Db`/`DbReader` instances, and [`Db::close`](crate::Db::close) will *not*
    /// close it. Call [`DbCache::close`] yourself after closing every database that uses it.
    ///
    /// `db_cache_id` isolates this database's entries from any other `Db`/`DbReader` sharing
    /// the same cache. Each database should have its own unique `db_cache_id`. The ID can be
    /// reused by the database when it's re-opened to recover any persisted cache data.
    pub fn with_db_cache(mut self, db_cache: Arc<dyn DbCache>, db_cache_id: u64) -> Self {
        // Wrap so Db::close()/DbReader::close() can't close a cache the
        // caller owns and may be sharing with other instances.
        self.db_cache = Some(DbCacheAndScope::new(
            Arc::new(UnownedDbCache::new(db_cache)),
            db_cache_id,
        ));
        self
    }

    /// Disables the sst block/metadata cache
    pub fn with_db_cache_disabled(mut self) -> Self {
        self.db_cache = None;
        self
    }

    /// Sets the policy for inserting flush and compaction output into the
    /// decoded block cache.
    pub fn with_block_cache_policy(mut self, policy: BlockCachePolicy) -> Self {
        self.block_cache_policy = policy;
        self
    }

    /// Sets the system clock to use for the database. System timestamps are used for
    /// scheduling operations such as compaction and garbage collection.
    pub fn with_system_clock(mut self, clock: Arc<dyn SystemClock>) -> Self {
        self.system_clock = Some(clock);
        self
    }

    /// Sets the garbage collection runtime to use for the database.
    pub fn with_gc_runtime(mut self, gc_runtime: Handle) -> Self {
        self.gc_runtime = Some(gc_runtime);
        self
    }

    /// Runs the batch-writer task on `runtime` instead of the one the database is built on; all
    /// other components stay put. On a current-thread runtime the writing threads drive
    /// themselves, a write becomes a task switch rather than a cross-thread wake-up.
    ///
    /// The caller must drive `runtime` for every write, [`Db::flush`], [`Db::close`] and
    /// [`Db::create_checkpoint`] — they go through the batch-writer and otherwise hang; reads do
    /// not. Build it with [`enable_time`] and no IO driver, which would cost a syscall per write.
    ///
    /// [`enable_time`]: tokio::runtime::Builder::enable_time
    pub fn with_write_runtime(mut self, runtime: Handle) -> Self {
        self.write_runtime = Some(runtime);
        self
    }

    /// Sets a custom CompactorBuilder for compaction orchestration.
    ///
    /// Setting a [`CompactorBuilder`] will ignore any previous
    /// [`Settings::compactor_options`] configuration passed in through
    /// [`DbBuilder::with_settings`] since the [`CompactorBuilder`] provides its own
    /// configuration.
    pub fn with_compactor_builder(mut self, compactor_builder: CompactorBuilder<P>) -> Self {
        self.compactor_builder = Some(compactor_builder.into_path_builder());
        self
    }

    /// Sets a custom GarbageCollectorBuilder for garbage collection.
    ///
    /// Setting a [`GarbageCollectorBuilder`] will ignore any previous
    /// [`Settings::garbage_collector_options`] configuration passed in through
    /// [`DbBuilder::with_settings`] since the [`GarbageCollectorBuilder`] provides
    /// its own configuration.
    pub fn with_gc_builder(mut self, gc_builder: GarbageCollectorBuilder<P>) -> Self {
        self.gc_builder = Some(gc_builder.into_path_builder());
        self
    }

    /// Sets the metrics recorder to use for the database.
    pub fn with_metrics_recorder(mut self, recorder: Arc<dyn MetricsRecorder>) -> Self {
        self.metrics_recorder = recorder;
        self
    }

    /// Sets the fail point registry to use for the database.
    pub fn with_fp_registry(mut self, fp_registry: Arc<FailPointRegistry>) -> Self {
        self.fp_registry = fp_registry;
        self
    }

    /// Sets the seed to use for the database's random number generator. All random behavior
    /// in SlateDB will use randomm number generators based off of this seed. This includes
    /// random bytes for UUIDs and ULIDS, as well as random pickers in cache eviction policies.
    ///
    /// If not set, SlateDB uses the OS's random number generator to generate a seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Sets the block size for SSTable blocks. Blocks are the unit of reading
    /// and caching in SlateDB. Smaller blocks can reduce read amplification but
    /// may increase metadata overhead. Larger blocks are more efficient for
    /// sequential scans but may waste bandwidth for point lookups.
    ///
    /// Note: When compression is enabled, blocks are compressed individually.
    /// Larger blocks typically achieve better compression ratios.
    ///
    /// # Arguments
    ///
    /// * `block_size` - The block size variant to use (1KB, 2KB, 4KB, 8KB, 16KB, 32KB, or 64KB).
    ///
    /// # Returns
    ///
    /// The builder instance for chaining.
    pub fn with_sst_block_size(mut self, block_size: SstBlockSize) -> Self {
        self.sst_block_size = Some(block_size);
        self
    }

    /// Sets the merge operator to use for the database. The merge operator allows
    /// applications to bypass the traditional read/modify/write cycle by expressing
    /// partial updates using an associative operator.
    ///
    /// # Arguments
    ///
    /// * `merge_operator` - An Arc-wrapped merge operator implementation.
    ///
    /// # Returns
    ///
    /// The builder instance for chaining.
    pub fn with_merge_operator(mut self, merge_operator: MergeOperatorType) -> Self {
        self.merge_operator = Some(merge_operator);
        self
    }

    /// Sets the block transformer to use for the database. The block transformer
    /// allows custom encoding/decoding of block data before storage and after
    /// retrieval. This can be used for encryption or other transformations.
    ///
    /// The transformer is applied after compression on write and before
    /// decompression on read. The checksum is calculated on the transformed data.
    ///
    /// # Arguments
    ///
    /// * `block_transformer` - An Arc-wrapped block transformer implementation.
    ///
    /// # Returns
    ///
    /// The builder instance for chaining.
    pub fn with_block_transformer(mut self, block_transformer: Arc<dyn BlockTransformer>) -> Self {
        self.block_transformer = Some(block_transformer);
        self
    }

    /// Sets the filter policies used for SST filter construction and evaluation.
    ///
    /// Each policy produces a separate filter per SST, stored in a composite
    /// filter block. On read, all filters are evaluated with AND logic: an
    /// SST is skipped only if every filter returns `false`.
    ///
    /// Defaults to `vec![Arc::new(BloomFilterPolicy::new(10))]`. Pass an
    /// empty vec to disable filters entirely.
    ///
    /// A standalone compactor or a `DbReader` (distributed setup) must be
    /// configured with the same policies, otherwise compaction will
    /// silently drop filter sub-blocks whose policy name is not
    /// recognized, and readers will fall back to scanning SSTs whose
    /// filters they cannot decode.
    pub fn with_filter_policies(mut self, policies: Vec<Arc<dyn FilterPolicy>>) -> Self {
        self.filter_policies = policies;
        self
    }

    /// Builds and opens the database.
    pub async fn build(self) -> Result<Db, crate::Error> {
        self.settings.validate()?;

        let path = self.path.into();
        // TODO: proper URI generation, for now it works just as a flag
        let wal_object_store_uri = self.wal_object_store.as_ref().map(|_| String::new());

        let rand = Arc::new(self.seed.map(DbRand::new).unwrap_or_default());
        let system_clock = self
            .system_clock
            .unwrap_or_else(|| Arc::new(DefaultSystemClock::new()));

        let metrics_recorder = self.metrics_recorder.clone();
        let recorder =
            MetricsRecorderHelper::new(self.metrics_recorder, self.settings.metric_level);
        let max_retries = self.settings.object_store_max_retries;

        // Wraps a store in a retry and instrumentation layer, recording I/O
        // under the given component and store-type metric labels. Each
        // component (db, compactor, gc) gets its own layer over the store its
        // builder holds.
        let wrap_object_store = |store: Arc<dyn ObjectStore>,
                                 component: ObjectStoreComponent,
                                 store_type: ObjectStoreType| {
            instrumented_retrying_object_store(
                store,
                &recorder,
                component,
                store_type,
                rand.clone(),
                system_clock.clone(),
                max_retries,
            )
        };
        // Set up the object store with optional caching from the settings,
        // producing the same layering as a caller-built
        // [`CachedObjectStore`] passed to [`DbBuilder::new`]: the cache sits
        // under the retry and instrumentation layers, so the same cache
        // instance can be shared with the WAL store, compactor, and GC.
        let cached_object_store = CachedObjectStore::from_config(
            self.main_object_store.clone(),
            &self.settings.object_store_cache_options,
            &recorder,
            system_clock.clone(),
            rand.clone(),
        )
        .await?;
        let maybe_cached_main_object_store: Arc<dyn ObjectStore> = match &cached_object_store {
            Some(cached_store) => cached_store.clone(),
            None => self.main_object_store.clone(),
        };

        let retrying_main_object_store = wrap_object_store(
            maybe_cached_main_object_store,
            ObjectStoreComponent::Db,
            ObjectStoreType::Main,
        );
        let retrying_wal_object_store: Option<Arc<dyn ObjectStore>> = self
            .wal_object_store
            .clone()
            .map(|s| wrap_object_store(s, ObjectStoreComponent::Db, ObjectStoreType::Wal));
        let wal_object_store = retrying_wal_object_store
            .clone()
            .unwrap_or_else(|| retrying_main_object_store.clone());

        // Log the database opening
        if let Ok(settings_json) = self.settings.to_json_string() {
            info!(
                "opening SlateDB database [path={}, settings={}]",
                path, settings_json
            );
        } else {
            info!(
                "opening SlateDB database [path={}, settings={:?}]",
                path, self.settings
            );
        }

        // Setup the components
        let block_format = {
            #[cfg(test)]
            {
                self.settings.block_format
            }
            #[cfg(not(test))]
            {
                None
            }
        };
        let sst_format = SsTableFormat {
            min_filter_keys: self.settings.min_filter_keys,
            filter_policies: self.filter_policies.clone(),
            compression_codec: self.settings.compression_codec,
            block_size: self.sst_block_size.unwrap_or_default().as_bytes(),
            block_transformer: self.block_transformer.clone(),
            block_format,
            ..SsTableFormat::default()
        };

        // Setup the manifest store and load latest manifest. The garbage collector this
        // database runs bounds how long a boundary check can be reused.
        let gc_options = match &self.gc_builder {
            Some(gc_builder) => Some(&gc_builder.options),
            None => self.settings.garbage_collector_options.as_ref(),
        };
        let manifest_store = Arc::new(ManifestStore::with_boundary_reuse(
            &path,
            retrying_main_object_store.clone(),
            BoundaryReuse::for_collector(system_clock.clone(), gc_options, |options| {
                options.manifest_options.as_ref()
            }),
        ));
        let compactions_store = Arc::new(CompactionsStore::with_boundary_reuse(
            &path,
            retrying_main_object_store.clone(),
            BoundaryReuse::for_collector(system_clock.clone(), gc_options, |options| {
                options.compactions_options.as_ref()
            }),
        ));
        let latest_manifest =
            StoredManifest::try_load(manifest_store.clone(), system_clock.clone()).await?;

        if let Some(latest_manifest) = &latest_manifest {
            latest_manifest
                .db_state()
                .validate_wal_object_store_uri(wal_object_store_uri.as_deref())?;
        }

        // RFC-0024: when the database already exists, the configured
        // extractor must match what is persisted in the manifest, and
        // the persisted segment prefixes must still be claimed by the
        // current extractor. For a fresh database the extractor name
        // (if any) is stamped onto the new manifest below.
        if let Some(latest_manifest) = &latest_manifest {
            latest_manifest
                .db_state()
                .validate_extractor_configuration(self.segment_extractor.as_deref())?;
        }

        // Extract external SSTs from manifest if available
        let external_ssts = match &latest_manifest {
            Some(latest_stored_manifest) => latest_stored_manifest.manifest().external_ssts(),
            None => HashMap::new(),
        };

        // Create path resolver and table store
        let path_resolver = PathResolver::new_with_external_ssts(path.clone(), external_ssts);
        let db_cache = self.db_cache.as_ref().map(|db_cache| {
            Arc::new(DbCacheWrapper::new(
                db_cache.cache.clone(),
                &recorder,
                system_clock.clone(),
                db_cache.db_cache_id,
            )) as Arc<dyn DbCache>
        });
        let table_store = Arc::new(TableStore::new_with_fp_registry(
            retrying_main_object_store.clone(),
            sst_format.clone(),
            path_resolver.clone(),
            self.fp_registry.clone(),
            db_cache.clone(),
            TableStoreKind::Main,
            self.block_cache_policy.clone(),
        ));
        let wal_store = Arc::new(WalTableStore::new_with_fp_registry(
            wal_object_store,
            sst_format.clone(),
            path_resolver.clone(),
            self.fp_registry.clone(),
            TableStoreKind::Main,
        ));

        // Initialize the database
        let stored_manifest = match latest_manifest {
            Some(manifest) => manifest,
            None => {
                let mut state = ManifestCore::new_with_wal_object_store(wal_object_store_uri);
                // RFC-0024 lazy V2 bump: when the user configures an
                // extractor at creation time, persist its name on the
                // initial manifest so reopens can detect any later
                // reconfiguration. Databases without an extractor keep
                // writing V1 manifests.
                if let Some(extractor) = &self.segment_extractor {
                    state.segment_extractor_name = Some(extractor.name().to_string());
                }
                StoredManifest::create_new_db(manifest_store.clone(), state, system_clock.clone())
                    .await?
            }
        };

        let manifest_dirty = stored_manifest.prepare_dirty()?;
        let status_manager = Arc::new(DbStatusManager::new_with_initial_values(
            manifest_dirty.value.core.last_l0_seq,
            manifest_dirty.into(),
            BTreeSet::new(),
        ));

        let task_executor = Arc::new(MessageHandlerExecutor::new(
            status_manager.clone(),
            system_clock.clone(),
        ));

        let fencer = WriterFencer::new(
            status_manager.result_reader(),
            recorder.clone(),
            wal_store.clone(),
            &self.settings,
            system_clock.clone(),
            task_executor.clone(),
            self.wal_writer_init,
        )
        .with_leadership_epoch(self.leadership_epoch)
        .with_fp_registry(self.fp_registry.clone());
        fail_parallel::fail_point!(
            self.fp_registry.clone(),
            "leadership-before-writer-fence",
            |_| { Err(crate::Error::internal("injected pre-fence failure".into())) }
        );
        let WriterFenceResult {
            manifest,
            replay_iterator,
            mut wal_writer,
        } = fencer.fence(stored_manifest).await?;
        let (wal_writer, wal_observer) = if DbInner::wal_enabled_in_options(&self.settings) {
            let wal_observer = wal_writer.observer();
            (Some(wal_writer), wal_observer)
        } else {
            wal_writer.close().await.map_err(SlateDBError::from)?;
            let Err(final_status) = wal_writer.status() else {
                return Err(crate::Error::internal(
                    "closed wal writer did not return terminal status".to_string(),
                ));
            };
            let wal_observer =
                Box::new(DisabledWalObserver::new(final_status)) as Box<dyn WalObserver>;
            (None, wal_observer)
        };

        let manifest_dirty = manifest.prepare_dirty()?;
        status_manager.report_fence_manifest(
            manifest_dirty.value.core.last_l0_seq,
            manifest_dirty.clone().into(),
        );

        // Setup communication channels wired to the shared closed state.
        let reader = status_manager.result_reader();
        let (write_tx, write_rx) = SafeSender::unbounded_channel(reader);

        // Create the database inner state
        let memtable_flusher = Arc::new(MemtableFlusher::new(status_manager.as_ref()));
        let inner = Arc::new(
            DbInner::new(
                self.settings.clone(),
                system_clock.clone(),
                rand.clone(),
                table_store.clone(),
                manifest_dirty,
                Arc::clone(&memtable_flusher),
                write_tx,
                wal_observer,
                recorder.clone(),
                self.fp_registry.clone(),
                self.merge_operator.clone(),
                status_manager,
                self.segment_extractor.clone(),
            )
            .await?,
        );

        // Retain all initialized state until open succeeds or error cleanup completes.
        // The caller must own this future through cancellation/timeout; dropping a
        // partially initialized builder is not an asynchronous cleanup operation.
        let tokio_handle = Handle::current();
        let mut monitor = None;
        let initialized: Result<(), SlateDBError> = async {
            // Setup background tasks
            task_executor.add_handler(
                WRITE_BATCH_TASK_NAME.to_string(),
                Box::new(WriteBatchEventHandler::new(inner.clone(), wal_writer)),
                write_rx,
                self.write_runtime.as_ref().unwrap_or(&tokio_handle),
            )?;

            // Selects the store a background component (compactor, GC) reads and
            // writes through, before the component wraps it in its own retry and
            // instrumentation layer.
            //
            // When the component runs against the DB's own store (the auto from
            // settings path, or a caller-supplied builder holding a clone of the
            // DB's store) and object store caching is configured, the DB's cache
            // instance is shared. A different caller-supplied store is used as
            // given (e.g. a custom compaction reader takes effect instead of being
            // silently ignored) and stays cacheless.
            let background_component_store =
                |raw_store: Arc<dyn ObjectStore>| -> Arc<dyn ObjectStore> {
                    match &cached_object_store {
                        Some(cached) if Arc::ptr_eq(&raw_store, &self.main_object_store) => {
                            cached.clone()
                        }
                        _ => raw_store,
                    }
                };

            // The compactor reads/writes through the object store held by its
            // builder: the DB's own store on the auto from settings path, or the
            // store the caller passed to their own `CompactorBuilder`.
            let compactor_builder = self.compactor_builder.or_else(|| {
                self.settings.compactor_options.as_ref().map(|opts| {
                    CompactorBuilder::new(path.clone(), self.main_object_store.clone())
                        .with_options(opts.clone())
                })
            });

            if let Some(mut compactor_builder) = compactor_builder {
                compactor_builder.options.metric_level = compactor_builder
                    .options
                    .metric_level
                    .or(Some(self.settings.metric_level));
                let mut builder = compactor_builder
                    .with_system_clock(system_clock.clone())
                    .with_metrics_recorder(metrics_recorder.clone())
                    .with_seed(rand.rng().next_u64());

                if let Some(operator) = self.merge_operator {
                    builder = builder.with_merge_operator(operator);
                }
                builder = builder.with_fp_registry(self.fp_registry.clone());

                let compactor_main_object_store = wrap_object_store(
                    background_component_store(builder.main_object_store.clone()),
                    ObjectStoreComponent::Compactor,
                    ObjectStoreType::Main,
                );
                let compactor_table_store = Arc::new(TableStore::new_with_fp_registry(
                    compactor_main_object_store,
                    sst_format.clone(),
                    path_resolver.clone(),
                    self.fp_registry.clone(),
                    db_cache.clone(),
                    TableStoreKind::Compactor,
                    self.block_cache_policy.clone(),
                ));
                fail_parallel::fail_point!(
                    self.fp_registry.clone(),
                    "leadership-before-compactor",
                    |_| {
                        Err(SlateDBError::from(std::io::Error::other(
                            "injected compactor init failure",
                        )))
                    }
                );
                let compactor_handlers = builder
                    .build_handler(
                        compactor_table_store,
                        manifest_store.clone(),
                        compactions_store.clone(),
                        self.leadership_epoch,
                    )
                    .await?;
                task_executor.add_handler(
                    COMPACTOR_TASK_NAME.to_string(),
                    Box::new(compactor_handlers.handler),
                    compactor_handlers.rx,
                    &tokio_handle,
                )?;
                if let Some((worker_handler, worker_rx)) = compactor_handlers.worker {
                    task_executor.add_handler(
                        COMPACTION_WORKER_TASK_NAME.to_string(),
                        Box::new(worker_handler),
                        worker_rx,
                        &tokio_handle,
                    )?;
                }
            }

            // Same store selection as the compactor above. Sharing the DB's cache
            // also means an SST deleted by the GC has its cache entries evicted.
            let gc_builder = self.gc_builder.or_else(|| {
                self.settings
                    .garbage_collector_options
                    .filter(|opts| !opts.is_empty())
                    .map(|opts| {
                        GarbageCollectorBuilder::new(path.clone(), self.main_object_store.clone())
                            .with_options(opts)
                    })
            });

            if let Some(mut gc_builder) = gc_builder {
                gc_builder.options.metric_level = gc_builder
                    .options
                    .metric_level
                    .or(Some(self.settings.metric_level));
                let gc_object_store = wrap_object_store(
                    background_component_store(gc_builder.main_object_store.clone()),
                    ObjectStoreComponent::Gc,
                    ObjectStoreType::Main,
                );
                let gc_table_store = Arc::new(TableStore::new_with_fp_registry(
                    gc_object_store.clone(),
                    sst_format.clone(),
                    path_resolver.clone(),
                    self.fp_registry.clone(),
                    None,
                    TableStoreKind::GC,
                    BlockCachePolicy::default(),
                ));
                let gc_wal_store = Arc::new(WalTableStore::new_with_fp_registry(
                    retrying_wal_object_store
                        .clone()
                        .unwrap_or_else(|| gc_object_store.clone()),
                    sst_format.clone(),
                    path_resolver.clone(),
                    self.fp_registry.clone(),
                    TableStoreKind::GC,
                ));
                let gc = gc_builder
                    .with_system_clock(system_clock.clone())
                    .with_metrics_recorder(metrics_recorder.clone())
                    .with_seed(rand.rng().next_u64())
                    .build_collector(
                        gc_table_store,
                        gc_wal_store,
                        manifest_store.clone(),
                        compactions_store.clone(),
                        gc_object_store,
                    );
                // Garbage collector only uses tickers, so pass in a dummy rx channel
                let (_, rx) = async_channel::unbounded();
                let gc_handle = self.gc_runtime.as_ref().unwrap_or(&tokio_handle);
                task_executor.add_handler(GC_TASK_NAME.to_string(), Box::new(gc), rx, gc_handle)?;
            }

            // Start the memtable flusher before WAL replay so that
            // replayed immutable memtables can be flushed concurrently.
            memtable_flusher.start(
                inner.clone(),
                manifest,
                &tokio_handle,
                &task_executor,
                inner.status_manager.as_ref(),
            )?;

            // Monitor background tasks
            monitor = Some(task_executor.monitor_on(&tokio_handle)?);
            fail_parallel::fail_point!(
                self.fp_registry.clone(),
                "leadership-before-replay",
                |_| {
                    Err(SlateDBError::from(std::io::Error::other(
                        "injected replay failure",
                    )))
                }
            );

            // Replay WAL
            inner.replay_wal(replay_iterator).await?;

            // Preload cache if enabled
            if let Some(cached_obj_store) = cached_object_store {
                inner
                    .preload_cache(&cached_obj_store, &path_resolver)
                    .await?;
            }

            Ok(())
        }
        .await;
        if let Err(error) = initialized {
            inner.status_manager.write_result(Err(error.clone()));
            if let Some(monitor) = monitor {
                task_executor.cancel_all_tasks();
                let db = Db {
                    inner,
                    task_executor,
                };
                if let Err(cleanup_error) = db
                    .close_with_options(
                        crate::config::CloseOptions::default().with_flush_type(None),
                    )
                    .await
                {
                    warn!("failed open cleanup [error={:?}]", cleanup_error);
                }
                if let Err(cleanup_error) = monitor.await {
                    warn!("failed open monitor cleanup [error={:?}]", cleanup_error);
                }
            } else {
                task_executor.discard_pending_tasks();
                if let Err(cleanup_error) = inner.table_store.close_cache().await {
                    warn!("failed open cache cleanup [error={:?}]", cleanup_error);
                }
            }
            return Err(error.into());
        }

        // Create and return the Db instance
        Ok(Db {
            inner,
            task_executor,
        })
    }
}

/// Builder for creating new Admin instances.
///
/// This provides a fluent API for configuring an Admin object.
pub struct AdminBuilder<P: Into<Path>> {
    path: P,
    main_object_store: Arc<dyn ObjectStore>,
    wal_object_store: Option<Arc<dyn ObjectStore>>,
    wal_admin: Option<Arc<dyn WalAdmin>>,
    system_clock: Arc<dyn SystemClock>,
    rand: Arc<DbRand>,
    object_store_max_retries: Option<u32>,
    #[cfg(feature = "compaction_filters")]
    compaction_filter_supplier: Option<Arc<dyn CompactionFilterSupplier>>,
    merge_operator: Option<MergeOperatorType>,
}

impl<P: Into<Path>> AdminBuilder<P> {
    /// Creates a new AdminBuilder with the given path and object store.
    pub fn new(path: P, main_object_store: Arc<dyn ObjectStore>) -> Self {
        Self {
            path,
            main_object_store,
            wal_object_store: None,
            wal_admin: None,
            system_clock: Arc::new(DefaultSystemClock::new()),
            rand: Arc::new(DbRand::default()),
            object_store_max_retries: None,
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier: None,
            merge_operator: None,
        }
    }

    /// Sets the system clock to use for administrative functions.
    pub fn with_system_clock(mut self, system_clock: Arc<dyn SystemClock>) -> Self {
        self.system_clock = system_clock;
        self
    }

    /// Sets the WAL object store to use for administrative functions.
    ///
    /// When configured, administrative operations that need to access WAL data
    /// (such as garbage collection) will use this object store instead of the
    /// main object store.
    pub fn with_wal_object_store(mut self, wal_object_store: Arc<dyn ObjectStore>) -> Self {
        self.wal_object_store = Some(wal_object_store);
        self
    }

    /// Sets the WAL admin API implementation to use for admin operations like db deletion
    /// and cloning. Users should set this if using a custom WAL implementation.
    pub fn with_wal_admin(mut self, wal_admin: Arc<dyn WalAdmin>) -> Self {
        self.wal_admin = Some(wal_admin);
        self
    }

    /// Sets the random number generator to use for randomness.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rand = Arc::new(DbRand::new(seed));
        self
    }

    /// Sets the compaction filter supplier for the compactor run by this admin.
    ///
    /// When running a standalone compactor via [`Admin::run_compactor`], ensure it is
    /// configured with the same filter supplier as the `DbBuilder`.
    #[cfg(feature = "compaction_filters")]
    pub fn with_compaction_filter_supplier(
        mut self,
        supplier: Arc<dyn CompactionFilterSupplier>,
    ) -> Self {
        self.compaction_filter_supplier = Some(supplier);
        self
    }

    /// Sets the merge operator to use when running a compactor.
    pub fn with_merge_operator(mut self, merge_operator: MergeOperatorType) -> Self {
        self.merge_operator = Some(merge_operator);
        self
    }

    /// Builds and returns an Admin instance.
    pub fn build(self) -> Admin {
        // Store the raw object stores here. Admin wraps them in a
        // `RetryingObjectStore` per-operation (see `Admin::retrying_store`)
        // rather than at build time, because several admin operations delegate
        // to sub-builders (compactor/GC) that add their own retry layer, and
        // wrapping here would double-wrap them.
        let main_object_store = self.main_object_store;
        let wal_object_store = self
            .wal_object_store
            .clone()
            .unwrap_or_else(|| main_object_store.clone());
        let wal_admin = self.wal_admin.unwrap_or_else(|| {
            let retrying_object_store = Arc::new(RetryingObjectStore::new(
                wal_object_store.clone(),
                self.rand.clone(),
                self.system_clock.clone(),
                self.object_store_max_retries,
            ));
            Arc::new(SlateDbWalAdmin::new(
                retrying_object_store,
                Arc::new(FailPointRegistry::new()),
            ))
        });
        Admin {
            path: self.path.into(),
            main_object_store,
            wal_object_store: self.wal_object_store,
            wal_admin,
            system_clock: self.system_clock,
            rand: self.rand,
            object_store_max_retries: self.object_store_max_retries,
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier: self.compaction_filter_supplier,
            merge_operator: self.merge_operator,
        }
    }
}

/// Builder for creating new GarbageCollector instances.
///
/// This provides a fluent API for configuring a GarbageCollector object.
pub struct GarbageCollectorBuilder<P: Into<Path>> {
    path: P,
    main_object_store: Arc<dyn ObjectStore>,
    wal_object_store: Option<Arc<dyn ObjectStore>>,
    wal_gc: Option<Arc<dyn WalGc>>,
    options: GarbageCollectorOptions,
    gc_filter: Option<Arc<dyn GcFilter>>,
    metrics_recorder: Arc<dyn MetricsRecorder>,
    system_clock: Arc<dyn SystemClock>,
    rand: Arc<DbRand>,
}

impl<P: Into<Path>> GarbageCollectorBuilder<P> {
    pub fn new(path: P, main_object_store: Arc<dyn ObjectStore>) -> Self {
        Self {
            path,
            main_object_store,
            wal_object_store: None,
            wal_gc: None,
            options: GarbageCollectorOptions::default(),
            gc_filter: None,
            metrics_recorder: Arc::new(NoopMetricsRecorder::new()),
            system_clock: Arc::new(DefaultSystemClock::default()),
            rand: Arc::new(DbRand::default()),
        }
    }

    /// Converts this builder into one with a concrete `Path` type.
    pub fn into_path_builder(self) -> GarbageCollectorBuilder<Path> {
        GarbageCollectorBuilder {
            path: self.path.into(),
            main_object_store: self.main_object_store,
            wal_object_store: self.wal_object_store,
            wal_gc: self.wal_gc,
            options: self.options,
            gc_filter: self.gc_filter,
            metrics_recorder: self.metrics_recorder,
            system_clock: self.system_clock,
            rand: self.rand,
        }
    }

    /// Sets the options to use for the garbage collector.
    pub fn with_options(mut self, options: GarbageCollectorOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets a garbage-collection filter for deletion candidates.
    ///
    /// The filter receives objects that SlateDB has already determined are
    /// eligible for GC and returns the subset that may be physically deleted.
    pub fn with_gc_filter(mut self, gc_filter: Arc<dyn GcFilter>) -> Self {
        self.gc_filter = Some(gc_filter);
        self
    }

    /// Sets a user-provided metrics recorder for the garbage collector.
    pub fn with_metrics_recorder(mut self, recorder: Arc<dyn MetricsRecorder>) -> Self {
        self.metrics_recorder = recorder;
        self
    }

    /// Sets the system clock to use for the garbage collector.
    pub fn with_system_clock(mut self, system_clock: Arc<dyn SystemClock>) -> Self {
        self.system_clock = system_clock;
        self
    }

    /// Sets the random number generator seed to use for the garbage collector.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rand = Arc::new(DbRand::new(seed));
        self
    }

    /// Sets the WAL object store to use for the garbage collector.
    pub fn with_wal_object_store(mut self, wal_object_store: Arc<dyn ObjectStore>) -> Self {
        self.wal_object_store = Some(wal_object_store);
        self
    }

    pub fn with_wal_gc(mut self, wal_gc: Arc<dyn WalGc>) -> Self {
        self.wal_gc = Some(wal_gc);
        self
    }

    /// Builds a GarbageCollector using pre-existing stores (used by DbBuilder).
    pub(crate) fn build_collector(
        self,
        table_store: Arc<TableStore>,
        wal_store: Arc<WalTableStore>,
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        object_store: Arc<dyn ObjectStore>,
    ) -> GarbageCollector {
        let recorder = MetricsRecorderHelper::new(
            self.metrics_recorder,
            self.options.metric_level.unwrap_or_default(),
        );
        GarbageCollector::new(
            manifest_store,
            compactions_store,
            table_store,
            wal_store,
            object_store,
            self.options,
            &recorder,
            self.system_clock,
            self.gc_filter,
            self.wal_gc,
        )
    }

    /// Builds and returns a GarbageCollector instance.
    pub fn build(self) -> GarbageCollector {
        let path: Path = self.path.into();
        let recorder = MetricsRecorderHelper::new(
            self.metrics_recorder,
            self.options.metric_level.unwrap_or_default(),
        );
        let retrying_main_object_store = instrumented_retrying_object_store(
            self.main_object_store,
            &recorder,
            ObjectStoreComponent::Gc,
            ObjectStoreType::Main,
            self.rand.clone(),
            self.system_clock.clone(),
            self.options.object_store_max_retries,
        );
        let retrying_wal_object_store = self.wal_object_store.map(|s| {
            instrumented_retrying_object_store(
                s,
                &recorder,
                ObjectStoreComponent::Gc,
                ObjectStoreType::Wal,
                self.rand.clone(),
                self.system_clock.clone(),
                self.options.object_store_max_retries,
            )
        });
        let manifest_store = Arc::new(ManifestStore::new(
            &path,
            retrying_main_object_store.clone(),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &path,
            retrying_main_object_store.clone(),
        ));
        let table_store = Arc::new(TableStore::new(
            retrying_main_object_store.clone(),
            SsTableFormat::default(), // read only SSTs can use default
            path.clone(),
            None, // no need for cache in GC
            TableStoreKind::GC,
            BlockCachePolicy::default(),
        ));
        let wal_store = Arc::new(WalTableStore::new(
            retrying_wal_object_store.unwrap_or_else(|| retrying_main_object_store.clone()),
            SsTableFormat::default(),
            path.clone(),
            TableStoreKind::GC,
        ));
        GarbageCollector::new(
            manifest_store,
            compactions_store,
            table_store,
            wal_store,
            retrying_main_object_store,
            self.options,
            &recorder,
            self.system_clock,
            self.gc_filter,
            self.wal_gc,
        )
    }
}

/// The compactor coordinator handler and optional embedded worker handler produced by
/// [`CompactorBuilder::build_handler`]. Each handler and its receiver must be registered
/// with the task executor in `DbBuilder::build`.
pub(crate) struct CompactorHandlers {
    /// The coordinator event handler.
    pub(crate) handler: CompactorEventHandler,
    /// Receiver for the coordinator's messages.
    pub(crate) rx: async_channel::Receiver<CompactorMessage>,
    /// The embedded worker handler and its receiver, present when
    /// [`CompactorOptions::worker`] is `Some`.
    pub(crate) worker: Option<(
        CompactionWorkerHandler,
        async_channel::Receiver<WorkerMessage>,
    )>,
}

/// Builder for creating new Compactor instances.
///
/// This provides a fluent API for configuring a Compactor object.
pub struct CompactorBuilder<P: Into<Path>> {
    path: P,
    main_object_store: Arc<dyn ObjectStore>,
    compaction_runtime: Handle,
    options: CompactorOptions,
    scheduler_supplier: Option<Arc<dyn CompactionSchedulerSupplier>>,
    rand: Arc<DbRand>,
    metrics_recorder: Arc<dyn MetricsRecorder>,
    system_clock: Arc<dyn SystemClock>,
    closed_result: Arc<dyn ClosedResultWriter>,
    merge_operator: Option<MergeOperatorType>,
    fp_registry: Arc<FailPointRegistry>,
    block_transformer: Option<Arc<dyn BlockTransformer>>,
    filter_policies: Vec<Arc<dyn FilterPolicy>>,
    #[cfg(feature = "compaction_filters")]
    compaction_filter_supplier: Option<Arc<dyn CompactionFilterSupplier>>,
}

#[allow(unused)]
impl<P: Into<Path>> CompactorBuilder<P> {
    pub fn new(path: P, main_object_store: Arc<dyn ObjectStore>) -> Self {
        Self {
            path,
            main_object_store,
            compaction_runtime: Handle::current(),
            options: CompactorOptions::default(),
            scheduler_supplier: None,
            rand: Arc::new(DbRand::default()),
            metrics_recorder: Arc::new(NoopMetricsRecorder::new()),
            system_clock: Arc::new(DefaultSystemClock::default()),
            closed_result: Arc::new(WatchableOnceCell::new()),
            merge_operator: None,
            fp_registry: Arc::new(FailPointRegistry::new()),
            block_transformer: None,
            filter_policies: default_filter_policies(),
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier: None,
        }
    }

    pub fn into_path_builder(self) -> CompactorBuilder<Path> {
        CompactorBuilder {
            path: self.path.into(),
            main_object_store: self.main_object_store,
            compaction_runtime: self.compaction_runtime,
            options: self.options,
            scheduler_supplier: self.scheduler_supplier,
            rand: self.rand,
            metrics_recorder: self.metrics_recorder,
            system_clock: self.system_clock,
            closed_result: self.closed_result,
            merge_operator: self.merge_operator,
            fp_registry: self.fp_registry,
            block_transformer: self.block_transformer,
            filter_policies: self.filter_policies,
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier: self.compaction_filter_supplier,
        }
    }

    /// Sets the tokio handle to use for background tasks.
    #[allow(unused)]
    pub fn with_runtime(mut self, compaction_runtime: Handle) -> Self {
        self.compaction_runtime = compaction_runtime;
        self
    }

    /// Sets the options to use for the compactor.
    pub fn with_options(mut self, options: CompactorOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets a user-provided metrics recorder for the compactor.
    pub fn with_metrics_recorder(mut self, recorder: Arc<dyn MetricsRecorder>) -> Self {
        self.metrics_recorder = recorder;
        self
    }

    /// Sets the system clock to use for the compactor.
    pub fn with_system_clock(mut self, system_clock: Arc<dyn SystemClock>) -> Self {
        self.system_clock = system_clock;
        self
    }

    /// Creates the random number generator to use for the compactor with given seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rand = Arc::new(DbRand::new(seed));
        self
    }

    /// Sets the compaction scheduler supplier to use for the compactor.
    pub fn with_scheduler_supplier(
        mut self,
        scheduler_supplier: Arc<dyn CompactionSchedulerSupplier>,
    ) -> Self {
        self.scheduler_supplier = Some(scheduler_supplier);
        self
    }

    /// Sets the merge operator to use for the compactor.
    pub fn with_merge_operator(mut self, merge_operator: MergeOperatorType) -> Self {
        self.merge_operator = Some(merge_operator);
        self
    }

    pub(crate) fn with_fp_registry(mut self, fp_registry: Arc<FailPointRegistry>) -> Self {
        self.fp_registry = fp_registry;
        self
    }

    /// Sets the block transformer to use for the compactor.
    pub fn with_block_transformer(mut self, block_transformer: Arc<dyn BlockTransformer>) -> Self {
        self.block_transformer = Some(block_transformer);
        self
    }

    /// Sets the filter policies used when the compactor rewrites SSTs.
    ///
    /// Must match the writer's `DbBuilder::with_filter_policies` configuration,
    /// otherwise compacted SSTs will be written with different (or no) filter
    /// policies and existing filters may be silently dropped during compaction.
    ///
    /// Defaults to `vec![Arc::new(BloomFilterPolicy::new(10))]`. Pass an
    /// empty vec to disable filters entirely.
    pub fn with_filter_policies(mut self, policies: Vec<Arc<dyn FilterPolicy>>) -> Self {
        self.filter_policies = policies;
        self
    }

    /// Sets the compaction filter supplier for the compactor. The filter supplier
    /// creates filter instances that can inspect, drop, or modify entries during
    /// compaction.
    ///
    /// **Warning:** Enabling compaction filters may affect snapshot consistency.
    /// Use compaction filters only if you know the consequences of using them.
    ///
    /// See [`crate::CompactionFilter`] for detailed documentation, examples, and the
    /// filter API.
    ///
    /// # Arguments
    ///
    /// * `supplier` - An Arc-wrapped compaction filter supplier implementation.
    ///
    /// # Returns
    ///
    /// The builder instance for chaining.
    #[cfg(feature = "compaction_filters")]
    pub fn with_compaction_filter_supplier(
        mut self,
        supplier: Arc<dyn CompactionFilterSupplier>,
    ) -> Self {
        self.compaction_filter_supplier = Some(supplier);
        self
    }

    /// Builds and returns a Compactor instance.
    pub fn build(self) -> Compactor {
        let path: Path = self.path.into();
        let recorder = MetricsRecorderHelper::new(
            self.metrics_recorder,
            self.options.metric_level.unwrap_or_default(),
        );
        let retrying_main_object_store = instrumented_retrying_object_store(
            self.main_object_store,
            &recorder,
            ObjectStoreComponent::Compactor,
            ObjectStoreType::Main,
            self.rand.clone(),
            self.system_clock.clone(),
            self.options.object_store_max_retries,
        );
        let manifest_store = Arc::new(ManifestStore::new(
            &path,
            retrying_main_object_store.clone(),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &path,
            retrying_main_object_store.clone(),
        ));
        let sst_format = SsTableFormat {
            filter_policies: self.filter_policies.clone(),
            block_transformer: self.block_transformer.clone(),
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            retrying_main_object_store,
            sst_format,
            path,
            None,
            TableStoreKind::Compactor,
            BlockCachePolicy::default(),
        ));

        let scheduler_supplier = self
            .scheduler_supplier
            .unwrap_or(Arc::new(SizeTieredCompactionSchedulerSupplier));

        Compactor::new(
            manifest_store,
            compactions_store,
            table_store,
            self.options,
            scheduler_supplier,
            self.compaction_runtime,
            self.rand,
            &recorder,
            self.system_clock,
            self.fp_registry,
            self.closed_result,
            self.merge_operator,
            #[cfg(feature = "compaction_filters")]
            self.compaction_filter_supplier,
        )
    }

    /// Build a CompactorEventHandler and optionally an embedded worker from this builder's
    /// configuration.
    ///
    /// The embedded worker handler is present when [`CompactorOptions::worker`] is `Some`.
    /// Each handler and its receiver must be registered with the task executor in
    /// DbBuilder::build.
    pub(crate) async fn build_handler(
        self,
        table_store: Arc<TableStore>,
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        leadership_epoch: Option<NonZeroU64>,
    ) -> Result<CompactorHandlers, SlateDBError> {
        let recorder = MetricsRecorderHelper::new(
            self.metrics_recorder,
            self.options.metric_level.unwrap_or_default(),
        );
        let options = Arc::new(self.options);
        let scheduler_supplier = self
            .scheduler_supplier
            .unwrap_or(Arc::new(SizeTieredCompactionSchedulerSupplier));
        let (_tx, rx) = async_channel::unbounded();
        let scheduler = Arc::from(scheduler_supplier.compaction_scheduler(&options));
        let stats = Arc::new(CompactionStats::new(&recorder));
        let handler = CompactorEventHandler::new_with_epoch(
            manifest_store.clone(),
            compactions_store.clone(),
            options.clone(),
            scheduler,
            self.rand.clone(),
            stats.clone(),
            self.system_clock.clone(),
            recorder.clone(),
            leadership_epoch,
        )
        .await?;
        let worker = options.worker.clone().map(|worker_options| {
            CompactionWorkerHandler::build_worker_handler(
                manifest_store,
                compactions_store,
                table_store,
                Arc::new(worker_options),
                self.compaction_runtime,
                self.rand,
                stats,
                recorder.clone(),
                self.system_clock,
                self.fp_registry,
                self.merge_operator,
                #[cfg(feature = "compaction_filters")]
                self.compaction_filter_supplier,
            )
        });
        Ok(CompactorHandlers {
            handler,
            rx,
            worker,
        })
    }
}

/// Builder for [`CompactionWorker`].
///
/// Mirrors `CompactorBuilder`: the user supplies a DB path and object store,
/// optionally overrides options/clock/seed/merge operator, then calls
/// [`CompactionWorkerBuilder::build`].
pub struct CompactionWorkerBuilder<P: Into<Path>> {
    path: P,
    main_object_store: Arc<dyn ObjectStore>,
    worker_runtime: Option<Handle>,
    options: CompactionWorkerOptions,
    rand: Arc<DbRand>,
    metrics_recorder: Arc<dyn MetricsRecorder>,
    system_clock: Arc<dyn SystemClock>,
    merge_operator: Option<MergeOperatorType>,
    fp_registry: Arc<FailPointRegistry>,
    block_transformer: Option<Arc<dyn BlockTransformer>>,
    filter_policies: Vec<Arc<dyn FilterPolicy>>,
    sst_block_size: Option<SstBlockSize>,
    #[cfg(feature = "compaction_filters")]
    compaction_filter_supplier: Option<Arc<dyn CompactionFilterSupplier>>,
}

impl<P: Into<Path>> CompactionWorkerBuilder<P> {
    pub fn new(path: P, main_object_store: Arc<dyn ObjectStore>) -> Self {
        Self {
            path,
            main_object_store,
            worker_runtime: None,
            options: CompactionWorkerOptions::default(),
            rand: Arc::new(DbRand::default()),
            metrics_recorder: Arc::new(NoopMetricsRecorder::new()),
            system_clock: Arc::new(DefaultSystemClock::default()),
            merge_operator: None,
            fp_registry: Arc::new(FailPointRegistry::new()),
            block_transformer: None,
            filter_policies: default_filter_policies(),
            sst_block_size: None,
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier: None,
        }
    }

    pub fn with_options(mut self, options: CompactionWorkerOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_runtime(mut self, runtime: Handle) -> Self {
        self.worker_runtime = Some(runtime);
        self
    }

    pub fn with_system_clock(mut self, system_clock: Arc<dyn SystemClock>) -> Self {
        self.system_clock = system_clock;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rand = Arc::new(DbRand::new(seed));
        self
    }

    pub fn with_metrics_recorder(mut self, recorder: Arc<dyn MetricsRecorder>) -> Self {
        self.metrics_recorder = recorder;
        self
    }

    pub fn with_merge_operator(mut self, merge_operator: MergeOperatorType) -> Self {
        self.merge_operator = Some(merge_operator);
        self
    }

    /// Sets the block transformer the worker uses when reading and rewriting
    /// SSTs during compaction.
    ///
    /// Must match the writer's `DbBuilder::with_block_transformer` configuration.
    /// Without it the worker reads and writes SST blocks untransformed, so a DB
    /// configured with a transformer (e.g. encryption) cannot offload compaction
    /// to standalone workers.
    pub fn with_block_transformer(mut self, block_transformer: Arc<dyn BlockTransformer>) -> Self {
        self.block_transformer = Some(block_transformer);
        self
    }

    /// Sets the filter policies the worker uses when it rewrites SSTs.
    ///
    /// Must match the writer's `DbBuilder::with_filter_policies` configuration,
    /// otherwise compacted SSTs will be written with different (or no) filter
    /// policies and existing filters may be silently dropped during compaction.
    ///
    /// Defaults to `vec![Arc::new(BloomFilterPolicy::new(10))]`. Pass an
    /// empty vec to disable filters entirely.
    pub fn with_filter_policies(mut self, policies: Vec<Arc<dyn FilterPolicy>>) -> Self {
        self.filter_policies = policies;
        self
    }

    /// Sets the SST block size the worker uses when it rewrites SSTs.
    ///
    /// Must match the writer's `DbBuilder::with_sst_block_size` configuration so
    /// that SSTs rewritten by the worker are encoded consistently with those
    /// produced by the DB. Defaults to [`SstBlockSize::default`].
    pub fn with_sst_block_size(mut self, block_size: SstBlockSize) -> Self {
        self.sst_block_size = Some(block_size);
        self
    }

    #[cfg(feature = "compaction_filters")]
    pub fn with_compaction_filter_supplier(
        mut self,
        supplier: Arc<dyn CompactionFilterSupplier>,
    ) -> Self {
        self.compaction_filter_supplier = Some(supplier);
        self
    }

    pub async fn build(self) -> Result<CompactionWorker, crate::Error> {
        let path: Path = self.path.into();
        let manifest_store = Arc::new(ManifestStore::new(&path, self.main_object_store.clone()));
        let compactions_store =
            Arc::new(CompactionsStore::new(&path, self.main_object_store.clone()));
        let table_store = Arc::new(TableStore::new(
            self.main_object_store,
            SsTableFormat {
                filter_policies: self.filter_policies.clone(),
                block_transformer: self.block_transformer.clone(),
                block_size: self.sst_block_size.unwrap_or_default().as_bytes(),
                min_filter_keys: self.options.min_filter_keys,
                compression_codec: self.options.compression_codec,
                ..SsTableFormat::default()
            },
            path,
            None,
            TableStoreKind::Compactor,
            BlockCachePolicy::default(),
        ));
        let recorder = MetricsRecorderHelper::new(
            self.metrics_recorder,
            self.options.metric_level.unwrap_or_default(),
        );
        let stats = Arc::new(CompactionStats::new(&recorder));
        let worker_runtime = self.worker_runtime.unwrap_or_else(Handle::current);
        let options = Arc::new(self.options);
        let closed_result: Arc<dyn ClosedResultWriter> = Arc::new(WatchableOnceCell::new());
        let task_executor = Arc::new(MessageHandlerExecutor::new(
            closed_result,
            self.system_clock.clone(),
        ));
        let (handler, rx) = CompactionWorkerHandler::build_worker_handler(
            manifest_store,
            compactions_store,
            table_store,
            options,
            worker_runtime,
            self.rand,
            stats,
            recorder,
            self.system_clock,
            self.fp_registry,
            self.merge_operator,
            #[cfg(feature = "compaction_filters")]
            self.compaction_filter_supplier,
        );
        task_executor
            .add_handler(
                COMPACTION_WORKER_TASK_NAME.to_string(),
                Box::new(handler),
                rx,
                &Handle::current(),
            )
            .expect("failed to spawn compaction worker task");
        Ok(CompactionWorker::new(task_executor))
    }
}

/// Builder for creating new DbReader instances.
///
/// This provides a fluent API for configuring a DbReader object.
/// It separates the concerns of configuration options (DbReaderOptions) and components.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use slatedb::{Db, DbReader, Error};
/// use slatedb::object_store::{ObjectStore, memory::InMemory};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
///     // First create a database
///     let db = Db::open("test_db", Arc::clone(&object_store)).await?;
///     db.close().await?;
///     // Then open a reader
///     let reader = DbReader::builder("test_db", object_store)
///         .build()
///         .await?;
///     Ok(())
/// }
/// ```
///
/// With custom options:
///
/// ```
/// use slatedb::{Db, DbReader, config::DbReaderOptions, Error};
/// use slatedb::object_store::{ObjectStore, memory::InMemory};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
///     // First create a database
///     let db = Db::open("test_db", Arc::clone(&object_store)).await?;
///     db.close().await?;
///     // Then open a reader with custom options
///     let reader = DbReader::builder("test_db", object_store)
///         .with_options(DbReaderOptions {
///             manifest_poll_interval: std::time::Duration::from_secs(5),
///             ..Default::default()
///         })
///         .build()
///         .await?;
///     Ok(())
/// }
/// ```
pub struct DbReaderBuilder<P: Into<Path>> {
    path: P,
    object_store: Arc<dyn ObjectStore>,
    wal_object_store: Option<Arc<dyn ObjectStore>>,
    wal_reader: Option<Arc<dyn wal::WalReader>>,
    db_cache: Option<DbCacheAndScope>,
    mode: DbReaderMode,
    merge_operator: Option<MergeOperatorType>,
    block_transformer: Option<Arc<dyn BlockTransformer>>,
    filter_policies: Vec<Arc<dyn FilterPolicy>>,
    segment_extractor: Option<Arc<dyn crate::prefix_extractor::PrefixExtractor>>,
    options: DbReaderOptions,
    system_clock: Arc<dyn SystemClock>,
    rand: Arc<DbRand>,
    metrics_recorder: Arc<dyn MetricsRecorder>,
}

impl<P: Into<Path>> DbReaderBuilder<P> {
    /// Creates a new DbReaderBuilder with the given path and object store.
    pub fn new(path: P, object_store: Arc<dyn ObjectStore>) -> Self {
        Self {
            path,
            object_store,
            wal_object_store: None,
            wal_reader: None,
            db_cache: default_db_cache(),
            mode: DbReaderMode::default(),
            merge_operator: None,
            block_transformer: None,
            filter_policies: default_filter_policies(),
            segment_extractor: None,
            options: DbReaderOptions::default(),
            system_clock: Arc::new(DefaultSystemClock::default()),
            rand: Arc::new(DbRand::default()),
            metrics_recorder: Arc::new(NoopMetricsRecorder::new()),
        }
    }

    /// Sets how the reader chooses and refreshes database state.
    pub fn with_reader_mode(mut self, mode: DbReaderMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets a separate object store for the WAL.
    /// Use this when the database was configured with a separate WAL object store.
    pub fn with_wal_object_store(mut self, wal_object_store: Arc<dyn ObjectStore>) -> Self {
        self.wal_object_store = Some(wal_object_store);
        self
    }

    /// Sets the WAL reader used to replay WAL rows. Use this to plug in a custom
    /// WAL implementation. SlateDB's object-store-backed WAL is used by default.
    pub fn with_wal_reader(mut self, wal_reader: Arc<dyn wal::WalReader>) -> Self {
        self.wal_reader = Some(wal_reader);
        self
    }

    /// Sets the merge operator to use when reading merge operands.
    pub fn with_merge_operator(mut self, merge_operator: MergeOperatorType) -> Self {
        self.merge_operator = Some(merge_operator);
        self
    }

    /// Sets the segment extractor (RFC-0024). When configured, the reader
    /// re-derives each WAL-replayed entry's segment prefix so that in-memory
    /// segments are reported via [`DbReader::subscribe`]. Must match the
    /// extractor the database was created with.
    pub fn with_segment_extractor(
        mut self,
        extractor: Arc<dyn crate::prefix_extractor::PrefixExtractor>,
    ) -> Self {
        self.segment_extractor = Some(extractor);
        self
    }

    /// Sets the options to use for the reader.
    pub fn with_options(mut self, options: DbReaderOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the cache to use for the database for caching sst blocks
    ///
    /// SlateDB uses a cache to efficiently store and retrieve blocks and SST metadata locally.
    /// [`slatedb::db_cache::SplitCache`] is used by default.
    ///
    /// A cache passed in here remains owned by the caller: it is safe to share it across
    /// multiple `Db`/`DbReader` instances, and [`DbReader::close`](crate::DbReader::close)
    /// will *not* close it. Call [`DbCache::close`] yourself after closing every database
    /// that uses it.
    ///
    /// `db_cache_id` isolates this reader's entries from any other `Db`/`DbReader` sharing
    /// the same cache. Pass the same id as the `Db` it's reading, or the same ID as a prior open
    /// of this same reader, to share/recover persisted entries.
    pub fn with_db_cache(mut self, db_cache: Arc<dyn DbCache>, db_cache_id: u64) -> Self {
        // Wrap so Db::close()/DbReader::close() can't close a cache the
        // caller owns and may be sharing with other instances.
        self.db_cache = Some(DbCacheAndScope::new(
            Arc::new(UnownedDbCache::new(db_cache)),
            db_cache_id,
        ));
        self
    }

    /// Disables the sst block/metadata cache
    pub fn with_db_cache_disabled(mut self) -> Self {
        self.db_cache = None;
        self
    }

    /// Sets the system clock to use for the reader.
    pub fn with_system_clock(mut self, system_clock: Arc<dyn SystemClock>) -> Self {
        self.system_clock = system_clock;
        self
    }

    /// Sets the seed to use for the reader's random number generator.
    /// All random behavior in SlateDB will use random number generators
    /// based off of this seed.
    ///
    /// If not set, SlateDB uses the OS's random number generator to generate a seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rand = Arc::new(DbRand::new(seed));
        self
    }

    /// Sets the metrics recorder to use for the reader.
    pub fn with_metrics_recorder(mut self, recorder: Arc<dyn MetricsRecorder>) -> Self {
        self.metrics_recorder = recorder;
        self
    }

    /// Sets the block transformer to use for the database. The block transformer
    /// allows custom encoding/decoding of block data before storage and after
    /// retrieval. This can be used for encryption or other transformations.
    ///
    /// The transformer is applied after compression on write and before
    /// decompression on read. The checksum is calculated on the transformed data.
    ///
    /// # Arguments
    ///
    /// * `block_transformer` - An Arc-wrapped block transformer implementation.
    ///
    /// # Returns
    ///
    /// The builder instance for chaining.
    pub fn with_block_transformer(mut self, block_transformer: Arc<dyn BlockTransformer>) -> Self {
        self.block_transformer = Some(block_transformer);
        self
    }

    /// Sets the filter policies used when decoding SST filter blocks.
    ///
    /// Must match (or be a superset of) the writer's policies so that any
    /// filter sub-block in an SST can be decoded. Unrecognized policy names
    /// are silently dropped: the SST is still readable, just without the
    /// skipped filter.
    ///
    /// Defaults to `vec![Arc::new(BloomFilterPolicy::new(10))]`.
    pub fn with_filter_policies(mut self, policies: Vec<Arc<dyn FilterPolicy>>) -> Self {
        self.filter_policies = policies;
        self
    }

    /// Builds and returns a DbReader instance.
    pub async fn build(self) -> Result<DbReader, crate::Error> {
        let path = self.path.into();
        let recorder = MetricsRecorderHelper::new(
            self.metrics_recorder,
            self.options.metric_level.unwrap_or_default(),
        );
        // TODO: proper URI generation, for now it works just as a flag
        let wal_object_store_uri = self.wal_object_store.as_ref().map(|_| String::new());
        // Set up the object store with optional caching from the reader
        // options, with the cache under the retry and instrumentation layers,
        // matching a caller-built [`CachedObjectStore`] passed to the reader.
        let maybe_cached = CachedObjectStore::from_config(
            self.object_store.clone(),
            &self.options.object_store_cache_options,
            &recorder,
            self.system_clock.clone(),
            self.rand.clone(),
        )
        .await?;
        let maybe_cached_object_store: Arc<dyn ObjectStore> = match &maybe_cached {
            Some(cached) => Arc::clone(cached) as Arc<dyn ObjectStore>,
            None => self.object_store.clone(),
        };

        let retrying_object_store = instrumented_retrying_object_store(
            maybe_cached_object_store,
            &recorder,
            ObjectStoreComponent::Reader,
            ObjectStoreType::Main,
            self.rand.clone(),
            self.system_clock.clone(),
            self.options.object_store_max_retries,
        );

        let retrying_wal_object_store = self.wal_object_store.map(|wal_object_store| {
            instrumented_retrying_object_store(
                wal_object_store,
                &recorder,
                ObjectStoreComponent::Reader,
                ObjectStoreType::Wal,
                self.rand.clone(),
                self.system_clock.clone(),
                self.options.object_store_max_retries,
            )
        });
        let wal_object_store = retrying_wal_object_store
            .clone()
            .unwrap_or_else(|| retrying_object_store.clone());

        // Validate WAL object store configuration.
        let manifest_store = Arc::new(ManifestStore::new(&path, retrying_object_store.clone()));
        let latest_manifest =
            StoredManifest::try_load(Arc::clone(&manifest_store), self.system_clock.clone())
                .await?;
        if let Some(latest_manifest) = &latest_manifest {
            latest_manifest
                .db_state()
                .validate_wal_object_store_uri(wal_object_store_uri.as_deref())?;
        }

        // Resolve external SSTs against the manifest the reader will actually
        // read from: the pinned checkpoint's manifest when a checkpoint id is
        // given (compaction may have pruned re-localized external SSTs from
        // the latest manifest), and the latest manifest otherwise.
        let external_ssts = match (&latest_manifest, self.mode) {
            (Some(latest_stored_manifest), DbReaderMode::Checkpoint(checkpoint_id)) => {
                let checkpoint = latest_stored_manifest
                    .db_state()
                    .find_checkpoint(checkpoint_id)
                    .ok_or(SlateDBError::CheckpointMissing(checkpoint_id))?;
                manifest_store
                    .read_manifest(checkpoint.manifest_id)
                    .await?
                    .external_ssts()
            }
            (Some(latest_stored_manifest), _) => latest_stored_manifest.manifest().external_ssts(),
            (None, _) => HashMap::new(),
        };

        let wrapped_cache = self.db_cache.as_ref().map(|db_cache| {
            Arc::new(DbCacheWrapper::new(
                db_cache.cache.clone(),
                &recorder,
                self.system_clock.clone(),
                db_cache.db_cache_id,
            )) as Arc<dyn DbCache>
        });

        let sst_format = SsTableFormat {
            filter_policies: self.filter_policies,
            block_transformer: self.block_transformer,
            ..SsTableFormat::default()
        };
        let path_resolver = PathResolver::new_with_external_ssts(path.clone(), external_ssts);
        let fp_registry = Arc::new(FailPointRegistry::new());
        let table_store = Arc::new(TableStore::new_with_fp_registry(
            retrying_object_store,
            sst_format.clone(),
            path_resolver.clone(),
            Arc::clone(&fp_registry),
            wrapped_cache,
            TableStoreKind::Reader,
            BlockCachePolicy::default(),
        ));
        let wal_store = Arc::new(WalTableStore::new_with_fp_registry(
            wal_object_store,
            sst_format,
            path_resolver,
            fp_registry,
            TableStoreKind::Reader,
        ));

        let reader = DbReader::open_internal(
            manifest_store,
            table_store,
            wal_store,
            self.mode,
            self.wal_reader,
            self.merge_operator,
            self.segment_extractor,
            self.options,
            self.system_clock,
            self.rand,
            recorder,
        )
        .await
        .map_err(crate::Error::from)?;

        if let Some(cached) = &maybe_cached {
            reader.preload_cache(cached, path).await?;
        }

        Ok(reader)
    }
}

fn default_filter_policies() -> Vec<Arc<dyn FilterPolicy>> {
    vec![Arc::new(BloomFilterPolicy::new(10))]
}

fn default_db_cache() -> Option<DbCacheAndScope> {
    let block_cache = default_block_cache();
    let meta_cache = default_meta_cache();
    let cache = Arc::new(
        SplitCache::new()
            .with_block_cache(block_cache)
            .with_meta_cache(meta_cache)
            .build(),
    ) as Arc<dyn DbCache>;
    Some(DbCacheAndScope::new(cache, 0))
}

/// Specifies the source database and checkpoint for a clone operation.
pub struct CloneSourceSpec<R: RangeBounds<Bytes> + Clone = (Bound<Bytes>, Bound<Bytes>)> {
    /// Path to the parent/source database.
    pub path: Path,
    /// Optional checkpoint ID to clone from. If None, the latest state is used.
    pub checkpoint: Option<Uuid>,
    /// Optional range to limit the visible keys in the source database.
    pub projection_range: Option<R>,
}

impl<R: RangeBounds<Bytes> + Clone> Clone for CloneSourceSpec<R> {
    fn clone(&self) -> Self {
        CloneSourceSpec {
            path: self.path.clone(),
            checkpoint: self.checkpoint,
            projection_range: self.projection_range.clone(),
        }
    }
}

impl<R: RangeBounds<Bytes> + Clone> CloneSourceSpec<R> {
    /// Set the visible range to limit the keys visible in the source database.
    pub fn with_projection_range(mut self, range: R) -> Self {
        self.projection_range = Some(range);
        self
    }
}

impl CloneSourceSpec<(Bound<Bytes>, Bound<Bytes>)> {
    /// Create a new clone source pointing to the latest state of a database.
    pub fn new<P: Into<Path>>(path: P) -> Self {
        CloneSourceSpec {
            path: path.into(),
            checkpoint: None,
            projection_range: None,
        }
    }

    /// Create a new clone source pointing to a specific checkpoint.
    pub fn with_checkpoint<P: Into<Path>>(path: P, checkpoint: Uuid) -> Self {
        CloneSourceSpec {
            path: path.into(),
            checkpoint: Some(checkpoint),
            projection_range: None,
        }
    }
}

/// A builder for configuring database clone operations.
pub struct CloneBuilder<R: RangeBounds<Bytes> + Clone = (Bound<Bytes>, Bound<Bytes>)> {
    clone_path: Path,
    sources: Vec<CloneSourceSpec<R>>,
    object_store: Arc<dyn ObjectStore>,
    wal_object_store: Option<Arc<dyn ObjectStore>>,
    wal_admin: Option<Arc<dyn WalAdmin>>,
    system_clock: Option<Arc<dyn SystemClock>>,
    rand: Option<Arc<DbRand>>,
    projection_range: Option<R>,
    segment_filter: Option<SegmentFilterFn>,
    segment_projection: Option<SegmentProjectionFn>,
}

impl<R: RangeBounds<Bytes> + Clone> CloneBuilder<R> {
    pub(crate) fn new(
        clone_path: Path,
        source: CloneSourceSpec<R>,
        object_store: Arc<dyn ObjectStore>,
    ) -> Self {
        CloneBuilder {
            clone_path,
            sources: vec![source],
            object_store,
            wal_object_store: None,
            wal_admin: None,
            system_clock: None,
            rand: None,
            projection_range: None,
            segment_filter: None,
            segment_projection: None,
        }
    }

    pub fn with_clone_path(mut self, clone_path: Path) -> Self {
        self.clone_path = clone_path;
        self
    }

    pub fn with_source(mut self, source: CloneSourceSpec<R>) -> Self {
        self.sources.push(source);
        self
    }

    pub fn with_object_store(mut self, object_store: Arc<dyn ObjectStore>) -> Self {
        self.object_store = object_store;
        self
    }

    pub fn with_wal_object_store(mut self, wal_object_store: Arc<dyn ObjectStore>) -> Self {
        self.wal_object_store = Some(wal_object_store);
        self
    }

    pub fn with_wal_admin(mut self, wal_admin: Arc<dyn WalAdmin>) -> Self {
        self.wal_admin = Some(wal_admin);
        self
    }

    pub fn with_projection_range(mut self, projection_range: Option<R>) -> Self {
        self.projection_range = projection_range;
        self
    }

    /// Restrict the clone to segments for which `f` returns `true`. The
    /// unsegmented LSM tree participates as a logical segment with the empty
    /// prefix `b""`, so e.g. `|p| !p.is_empty()` keeps only named segments
    /// and `|p| p.is_empty()` keeps only the unsegmented tree.
    pub fn with_segment_filter<F>(mut self, f: F) -> Self
    where
        F: Fn(&[u8]) -> bool + Send + Sync + 'static,
    {
        self.segment_filter = Some(Arc::new(f));
        self
    }

    /// Set a per-segment projection range. `f` receives each segment's prefix
    /// and returns the range to retain. The returned range's bounded ends must
    /// fall within `[prefix, prefix++)`; `Unbounded` ends resolve to the
    /// segment edges. The result is intersected with any global
    /// `projection_range`. Empty returned ranges surface as
    /// `SlateDBError::InvalidProjection`.
    pub fn with_segment_projection<F, Range>(mut self, f: F) -> Self
    where
        F: Fn(&[u8]) -> Range + Send + Sync + 'static,
        Range: RangeBounds<Bytes>,
    {
        self.segment_projection = Some(Arc::new(move |prefix| {
            let r = f(prefix);
            crate::bytes_range::BytesRange::try_new(
                r.start_bound().cloned(),
                r.end_bound().cloned(),
            )
            .ok_or_else(|| SlateDBError::InvalidProjection {
                prefix: Bytes::copy_from_slice(prefix),
                reason: "empty range".into(),
            })
        }));
        self
    }

    pub fn with_system_clock(mut self, system_clock: Arc<dyn SystemClock>) -> Self {
        self.system_clock = Some(system_clock);
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rand = Some(Arc::new(DbRand::new(seed)));
        self
    }

    /// Build and execute the clone operation.
    pub async fn build(self) -> Result<(), crate::Error> {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let wal_admin = if let Some(wal_admin) = self.wal_admin {
            wal_admin
        } else {
            Arc::new(SlateDbWalAdmin::new(
                self.wal_object_store.unwrap_or(self.object_store.clone()),
                fp_registry.clone(),
            ))
        };
        crate::clone::create_clone(
            self.sources,
            self.clone_path,
            self.object_store,
            wal_admin,
            fp_registry,
            self.system_clock
                .unwrap_or_else(|| Arc::new(DefaultSystemClock::new())),
            self.rand.unwrap_or_else(|| Arc::new(Default::default())),
            self.projection_range,
            self.segment_filter,
            self.segment_projection,
        )
        .await
        .map_err(crate::Error::from)
    }
}

fn instrumented_retrying_object_store(
    object_store: Arc<dyn ObjectStore>,
    recorder: &MetricsRecorderHelper,
    component: ObjectStoreComponent,
    store_type: ObjectStoreType,
    rand: Arc<DbRand>,
    system_clock: Arc<dyn SystemClock>,
    max_retries: Option<u32>,
) -> Arc<dyn ObjectStore> {
    let instrumented: Arc<dyn ObjectStore> = Arc::new(InstrumentedObjectStore::new(
        object_store,
        recorder,
        component,
        store_type,
    ));
    Arc::new(RetryingObjectStore::new(
        instrumented,
        rand,
        system_clock,
        max_retries,
    ))
}

#[allow(unreachable_code)]
pub(crate) fn default_block_cache() -> Option<Arc<dyn DbCache>> {
    #[cfg(feature = "foyer")]
    {
        return Some(Arc::new(crate::db_cache::foyer::FoyerCache::new_with_opts(
            crate::db_cache::foyer::FoyerCacheOptions {
                max_capacity: crate::db_cache::DEFAULT_BLOCK_CACHE_CAPACITY,
                ..Default::default()
            },
        )));
    }
    #[cfg(feature = "moka")]
    {
        return Some(Arc::new(crate::db_cache::moka::MokaCache::new_with_opts(
            crate::db_cache::moka::MokaCacheOptions {
                max_capacity: crate::db_cache::DEFAULT_BLOCK_CACHE_CAPACITY,
                time_to_live: None,
                time_to_idle: None,
            },
        )));
    }
    None
}

#[allow(unreachable_code)]
pub(crate) fn default_meta_cache() -> Option<Arc<dyn DbCache>> {
    #[cfg(feature = "foyer")]
    {
        return Some(Arc::new(crate::db_cache::foyer::FoyerCache::new_with_opts(
            crate::db_cache::foyer::FoyerCacheOptions {
                max_capacity: crate::db_cache::DEFAULT_META_CACHE_CAPACITY,
                ..Default::default()
            },
        )));
    }
    #[cfg(feature = "moka")]
    {
        return Some(Arc::new(crate::db_cache::moka::MokaCache::new_with_opts(
            crate::db_cache::moka::MokaCacheOptions {
                max_capacity: crate::db_cache::DEFAULT_META_CACHE_CAPACITY,
                time_to_live: None,
                time_to_idle: None,
            },
        )));
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::cached_object_store::CachedObjectStore;
    use crate::compactions_store::{CompactionsStore, StoredCompactions};
    use crate::config::{CompactorOptions, GarbageCollectorOptions, MetricLevel, Settings};
    use crate::error::ErrorKind;
    use crate::garbage_collector::stats::GC_COUNT;
    use crate::instrumented_object_store::stats::REQUEST_COUNT as OBJECT_STORE_REQUEST_COUNT;
    use crate::manifest::store::{ManifestStore, StoredManifest};
    use crate::manifest::ManifestCore;
    use crate::wal::test_utils::FakeWalWriter;
    use crate::wal::{
        WalError, WalIterator, WalRows, WriterInit, WriterInitResult, WriterManifest,
    };
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::metrics::{
        lookup_metric, lookup_metric_with_labels, DefaultMetricsRecorder, MetricsRecorderHelper,
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    // Aliased because `tempfile::Builder` is also used below.
    use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

    struct EmptyWalIterator;

    #[async_trait::async_trait]
    impl WalIterator for EmptyWalIterator {
        async fn next(&mut self) -> Result<Option<WalRows>, WalError> {
            Ok(None)
        }
    }

    struct RecordingWriterInit {
        called: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl WriterInit for RecordingWriterInit {
        async fn fence_and_init(
            &self,
            manifest: &mut WriterManifest,
        ) -> Result<WriterInitResult, WalError> {
            self.called.store(true, Ordering::Relaxed);
            Ok(WriterInitResult {
                replay_iterator: Box::new(EmptyWalIterator),
                wal_writer: Box::new(FakeWalWriter::new(manifest.replay_after_wal_id())),
            })
        }
    }

    fn object_store_labels(
        component: &'static str,
        store_type: &'static str,
        op: &'static str,
        api: &'static str,
    ) -> [(&'static str, &'static str); 4] {
        [
            ("component", component),
            ("store_type", store_type),
            ("op", op),
            ("api", api),
        ]
    }

    fn assert_debug_metric_registered(
        recorder: &DefaultMetricsRecorder,
        helper: MetricsRecorderHelper,
        name: &str,
    ) {
        let counter = helper.counter(name).level(MetricLevel::Debug).register();
        counter.increment(1);
        assert_eq!(lookup_metric(recorder, name), Some(1));
    }

    #[tokio::test]
    async fn test_db_builder_uses_custom_wal_writer_init() {
        let called = Arc::new(AtomicBool::new(false));
        let db = crate::Db::builder(
            "test_db_builder_uses_custom_wal_writer_init",
            Arc::new(InMemory::new()),
        )
        .with_settings(Settings {
            compactor_options: None,
            garbage_collector_options: None,
            ..Settings::default()
        })
        .with_wal_writer(Box::new(RecordingWriterInit {
            called: Arc::clone(&called),
        }))
        .build()
        .await
        .expect("failed to build db");

        assert!(called.load(Ordering::Relaxed));
        db.close().await.expect("failed to close db");
    }

    #[tokio::test]
    async fn test_db_builder_starts_gc_by_default() {
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = crate::Db::builder(
            "test_db_builder_starts_gc_by_default",
            Arc::new(InMemory::new()),
        )
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .expect("failed to build db");

        assert!(
            lookup_metric(&metrics_recorder, GC_COUNT).is_some(),
            "GC should be initialized by default"
        );

        db.close().await.expect("failed to close db");
    }

    #[tokio::test]
    async fn test_db_builder_disables_gc_when_gc_options_are_none() {
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = crate::Db::builder(
            "test_db_builder_disables_gc_when_gc_options_are_none",
            Arc::new(InMemory::new()),
        )
        .with_settings(Settings {
            garbage_collector_options: None,
            ..Settings::default()
        })
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .expect("failed to build db");

        assert!(
            lookup_metric(&metrics_recorder, GC_COUNT).is_none(),
            "GC should not be initialized when options are None"
        );

        db.close().await.expect("failed to close db");
    }

    #[tokio::test]
    async fn test_db_builder_rejects_zero_l0_flush_parallelism() {
        let result = crate::Db::builder(
            "test_db_builder_rejects_zero_l0_flush_parallelism",
            Arc::new(InMemory::new()),
        )
        .with_settings(Settings {
            l0_flush_parallelism: 0,
            ..Settings::default()
        })
        .build()
        .await;

        let err = match result {
            Ok(_) => panic!("expected invalid l0_flush_parallelism to fail"),
            Err(err) => err,
        };

        assert!(matches!(err.kind(), ErrorKind::Invalid));
        assert!(
            err.to_string()
                .contains("l0_flush_parallelism must be at least 1"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn test_db_builder_rejects_low_max_wal_flushes_before_l0_flush() {
        let result = crate::Db::builder(
            "test_db_builder_rejects_low_max_wal_flushes_before_l0_flush",
            Arc::new(InMemory::new()),
        )
        .with_settings(Settings {
            max_wal_flushes_before_l0_flush: 4095,
            ..Settings::default()
        })
        .build()
        .await;

        let err = match result {
            Ok(_) => panic!("expected invalid max_wal_flushes_before_l0_flush to fail"),
            Err(err) => err,
        };

        assert!(matches!(err.kind(), ErrorKind::Invalid));
        assert!(
            err.to_string()
                .contains("max_wal_flushes_before_l0_flush must be at least 4096"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn test_standalone_compactor_and_gc_builder_metric_level_is_order_independent() {
        let object_store = Arc::new(InMemory::new());

        let compactor_options = CompactorOptions {
            metric_level: Some(MetricLevel::Debug),
            ..Default::default()
        };
        let compactor_recorder = Arc::new(DefaultMetricsRecorder::new());
        let compactor =
            crate::CompactorBuilder::new("test_compactor_metric_level_order", object_store.clone())
                .with_metrics_recorder(compactor_recorder.clone())
                .with_options(compactor_options);
        assert_debug_metric_registered(
            &compactor_recorder,
            MetricsRecorderHelper::new(
                compactor.metrics_recorder,
                compactor.options.metric_level.unwrap_or_default(),
            ),
            "test.compactor_debug_metric",
        );

        let gc_options = GarbageCollectorOptions {
            metric_level: Some(MetricLevel::Debug),
            ..Default::default()
        };
        let gc_recorder = Arc::new(DefaultMetricsRecorder::new());
        let gc = crate::GarbageCollectorBuilder::new("test_gc_metric_level_order", object_store)
            .with_options(gc_options)
            .with_metrics_recorder(gc_recorder.clone());
        assert_debug_metric_registered(
            &gc_recorder,
            MetricsRecorderHelper::new(
                gc.metrics_recorder,
                gc.options.metric_level.unwrap_or_default(),
            ),
            "test.gc_debug_metric",
        );
    }

    #[tokio::test]
    async fn test_shared_recorder_registers_object_store_metrics_for_db_gc_and_compactor() {
        // given:
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let path = "test_shared_recorder_registers_object_store_metrics_for_db_gc_and_compactor";
        let object_store = Arc::new(InMemory::new());
        let db = crate::Db::builder(path, object_store.clone())
            .with_settings(Settings {
                compactor_options: None,
                garbage_collector_options: None,
                ..Settings::default()
            })
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .expect("failed to build db");
        let gc = crate::GarbageCollectorBuilder::new(path, object_store.clone())
            .with_metrics_recorder(metrics_recorder.clone())
            .build();
        let _compactor = crate::CompactorBuilder::new(path, object_store)
            .with_metrics_recorder(metrics_recorder.clone())
            .build();

        // when:
        db.put(b"k1", b"v1")
            .await
            .expect("failed to write db value");
        db.flush().await.expect("failed to flush db");
        gc.run_gc_once().await;

        // then:
        assert!(lookup_metric_with_labels(
            &metrics_recorder,
            OBJECT_STORE_REQUEST_COUNT,
            &object_store_labels("db", "main", "put", "put"),
        )
        .is_some_and(|count| count > 0));
        assert_eq!(
            lookup_metric_with_labels(
                &metrics_recorder,
                OBJECT_STORE_REQUEST_COUNT,
                &object_store_labels("gc", "main", "put", "put"),
            ),
            Some(0)
        );
        assert_eq!(
            lookup_metric_with_labels(
                &metrics_recorder,
                OBJECT_STORE_REQUEST_COUNT,
                &object_store_labels("compactor", "main", "put", "put"),
            ),
            Some(0)
        );

        db.close().await.expect("failed to close db");
    }

    #[tokio::test]
    async fn test_object_store_cache_does_not_cache_metadata_store_reads() {
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("test_object_store_cache_does_not_cache_metadata_store_reads");
        let manifest_store = Arc::new(ManifestStore::new(&path, object_store.clone()));
        StoredManifest::create_new_db(
            manifest_store,
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .expect("failed to seed manifest");
        let compactions_store = Arc::new(CompactionsStore::new(&path, object_store.clone()));
        StoredCompactions::create(compactions_store, 0)
            .await
            .expect("failed to seed compactions");

        let cache_dir = tempfile::Builder::new()
            .prefix("metadata_store_cache_test_")
            .tempdir()
            .expect("failed to create cache dir");
        let cache_path = cache_dir.path().to_path_buf();
        let settings = Settings {
            garbage_collector_options: None,
            ..Settings::default()
        };
        let cached_store = CachedObjectStore::builder(cache_path.clone(), object_store)
            .with_part_size_bytes(1024)
            .build()
            .await
            .expect("failed to build cached store");

        let db = crate::Db::builder(path.clone(), cached_store)
            .with_settings(settings)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .expect("failed to build db");

        let cached_db_path = cache_path.join(path.as_ref());
        assert!(!cached_db_path.join("manifest").exists());
        assert!(!cached_db_path.join("compactions").exists());
        assert!(!cached_db_path.join("gc").exists());

        db.close().await.expect("failed to close db");
    }

    #[tokio::test]
    async fn test_settings_configured_object_store_cache() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("test_settings_configured_object_store_cache");

        // Seed an L0 SST without any cache configured.
        let db = crate::Db::builder(path.clone(), object_store.clone())
            .with_settings(Settings {
                garbage_collector_options: None,
                ..Settings::default()
            })
            .build()
            .await
            .expect("failed to build db");
        db.put(b"k1", b"v1").await.expect("failed to put");
        db.flush().await.expect("failed to flush");
        db.close().await.expect("failed to close db");

        // Reopen with the cache configured through Settings and preload on.
        let cache_dir = tempfile::Builder::new()
            .prefix("settings_cache_test_")
            .tempdir()
            .expect("failed to create cache dir");
        let cache_path = cache_dir.path().to_path_buf();
        let mut settings = Settings {
            garbage_collector_options: None,
            ..Settings::default()
        };
        settings.object_store_cache_options.root_folder = Some(cache_path.clone());
        settings.object_store_cache_options.part_size_bytes = 1024;
        settings
            .object_store_cache_options
            .preload_disk_cache_on_startup = Some(crate::config::PreloadLevel::AllSst);

        let db = crate::Db::builder(path.clone(), object_store)
            .with_settings(settings)
            .build()
            .await
            .expect("failed to build db");

        // The preload populated the cache with the compacted SST's parts.
        let cached_db_path = cache_path.join(path.as_ref());
        let cached_compacted = std::fs::read_dir(cached_db_path.join("compacted"))
            .expect("expected cached compacted dir")
            .count();
        assert!(cached_compacted > 0);
        assert!(!cached_db_path.join("manifest").exists());

        assert_eq!(
            db.get(b"k1").await.expect("failed to get").as_deref(),
            Some(b"v1".as_ref())
        );
        db.close().await.expect("failed to close db");
    }

    // Not a `#[tokio::test]`: the caller has to own both runtimes.
    #[test]
    fn test_write_runtime_serves_writes_and_close() {
        let background = Runtime::new().expect("failed to build background runtime");
        let writer = RuntimeBuilder::new_current_thread()
            .enable_time()
            .build()
            .expect("failed to build writer runtime");

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = background
            .block_on(
                crate::Db::builder(
                    Path::from("test_write_runtime_serves_writes_and_close"),
                    object_store,
                )
                .with_write_runtime(writer.handle().clone())
                .build(),
            )
            .expect("failed to build db");

        writer
            .block_on(db.put(b"k1", b"v1"))
            .expect("failed to put");

        // Reads do not go through the batch-writer.
        let value = background
            .block_on(db.get(b"k1"))
            .expect("failed to get")
            .expect("expected the written value");
        assert_eq!(value.as_ref(), b"v1");

        // Close flushes through the batch-writer and joins it.
        writer.block_on(db.close()).expect("failed to close db");
    }

    #[tokio::test]
    async fn test_write_runtime_defaults_to_the_build_runtime() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = crate::Db::builder(
            Path::from("test_write_runtime_defaults_to_the_build_runtime"),
            object_store,
        )
        .build()
        .await
        .expect("failed to build db");

        db.put(b"k1", b"v1").await.expect("failed to put");
        assert_eq!(
            db.get(b"k1").await.expect("failed to get").as_deref(),
            Some(b"v1".as_ref())
        );
        db.close().await.expect("failed to close db");
    }
}
