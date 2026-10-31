//! # DB Cache
//!
//! This module provides a pluggable caching solution for storing and retrieving
//! cached blocks, index and filters associated with SSTable IDs.
//!
//! There are currently two built-in cache implementations:
//! - [Foyer](crate::db_cache::foyer::FoyerCache): Requires the `foyer` feature flag. (Enabled by default)
//! - [Moka](crate::db_cache::moka::MokaCache): Requires the `moka` feature flag.
//!
//! ## Usage
//!
//! To use the cache, you need to configure the [DbOptions](crate::config::DbOptions) with the desired cache implementation.

use std::ops::{Bound, RangeBounds};
#[cfg(feature = "foyer")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, TimeDelta, Utc};
use futures::future::BoxFuture;
use log::{debug, error, trace};
use parking_lot::Mutex;

use crate::db_cache::stats::DbCacheStats;
use crate::db_state::SsTableId;
use crate::filter_policy::NamedFilter;
use crate::flatbuffer_types::SsTableIndexOwned;
use crate::format::block::Block;
use crate::sst_stats::SstStats;
use slatedb_common::clock::SystemClock;
use slatedb_common::metrics::MetricsRecorderHelper;

#[cfg(feature = "foyer")]
pub mod foyer;
#[cfg(feature = "foyer")]
pub mod foyer_hybrid;
#[cfg(feature = "moka")]
pub mod moka;
mod serde;

/// The default max capacity for the user default cache. (64MB)
pub const DEFAULT_MAX_CAPACITY: u64 = 64 * 1024 * 1024;
pub const DEFAULT_BLOCK_CACHE_CAPACITY: u64 = 512 * 1024 * 1024;
pub const DEFAULT_META_CACHE_CAPACITY: u64 = 128 * 1024 * 1024;

/// A `FnOnce` returning a future that produces a [`CachedEntry`] on cache miss.
///
/// Used by [`DbCache::fetch_block`] and friends to load an entry into the cache. The closure
/// is invoked at most once per concurrent fetch group; subsequent callers for the same key
/// receive the same result.
pub type CacheLoader =
    Box<dyn FnOnce() -> BoxFuture<'static, Result<CachedEntry, crate::Error>> + Send + 'static>;

/// The result of a cache lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheLookup {
    /// The fetch loaded the entry from object storage.
    Miss,
    /// The cache supplied the entry, including through a concurrent fetch.
    Hit,
}

/// A fetched entry and the result of its cache lookup.
#[derive(Clone)]
pub struct CacheFetch {
    /// The entry that the fetch returned.
    pub entry: CachedEntry,
    /// Whether the cache supplied the entry or the fetch loaded it.
    pub lookup: CacheLookup,
}

impl CacheFetch {
    /// Create a result for an entry that the cache supplied.
    pub fn hit(entry: CachedEntry) -> Self {
        Self {
            entry,
            lookup: CacheLookup::Hit,
        }
    }

    /// Create a result for an entry that the fetch loaded.
    pub fn miss(entry: CachedEntry) -> Self {
        Self {
            entry,
            lookup: CacheLookup::Miss,
        }
    }
}

/// A trait for slatedb's in-memory cache.
///
/// This trait defines the interface for an in-memory cache,
/// which is used to store and retrieve cached blocks, indices and filters
/// associated with SSTable IDs.
///
/// Example:
///
/// ```
/// use async_trait::async_trait;
/// use slatedb::{Db, Error};
/// use slatedb::db_cache::{DbCache, CachedEntry, CachedKey};
/// use slatedb::object_store::memory::InMemory;
/// use std::collections::HashMap;
/// use std::sync::{Arc, Mutex};
///
/// struct MyCache {
///     inner: Mutex<MyCacheInner>,
/// }
///
/// struct MyCacheInner {
///     data: HashMap<CachedKey, CachedEntry>,
///     usage: u64,
///     capacity: u64
/// }
///
/// impl MyCache {
///     pub fn new(capacity: u64) -> Self {
///         Self {
///             inner: Mutex::new(
///                 MyCacheInner{
///                     data: HashMap::new(),
///                     usage: 0,
///                     capacity,
///                 }
///             )
///         }
///     }
/// }
///
/// #[async_trait]
/// impl DbCache for MyCache {
///     async fn get_block(&self, key: &CachedKey) -> Result<Option<CachedEntry>, Error> {
///         let guard = self.inner.lock().unwrap();
///         Ok(guard.data.get(key).cloned())
///     }
///
///     async fn get_index(&self, key: &CachedKey) -> Result<Option<CachedEntry>, Error> {
///         let guard = self.inner.lock().unwrap();
///         Ok(guard.data.get(key).cloned())
///     }
///
///     async fn get_filter(&self, key: &CachedKey) -> Result<Option<CachedEntry>, Error> {
///         let guard = self.inner.lock().unwrap();
///         Ok(guard.data.get(key).cloned())
///     }
///
///     async fn get_stats(&self, key: &CachedKey) -> Result<Option<CachedEntry>, Error> {
///         let guard = self.inner.lock().unwrap();
///         Ok(guard.data.get(key).cloned())
///     }
///
///     async fn insert(&self, key: CachedKey, value: CachedEntry) {
///         let mut guard = self.inner.lock().unwrap();
///         guard.usage += value.size() as u64;
///         if let Some(v) = guard.data.insert(key, value) {
///             guard.usage -= v.size() as u64;
///         }
///     }
///
///     async fn remove(&self, key: &CachedKey) {
///         let mut guard = self.inner.lock().unwrap();
///         if let Some(v) = guard.data.remove(key) {
///             guard.usage -= v.size() as u64;
///         }
///     }
///
///     fn entry_count(&self) -> u64 {
///         let mut guard = self.inner.lock().unwrap();
///         guard.capacity
///     }
/// }
///
/// #[::tokio::main]
/// async fn main() {
///     let object_store = Arc::new(InMemory::new());
///     let cache = Arc::new(MyCache::new(128u64 * 1024 * 1024));
///     let db = Db::builder("/path/to/db", object_store)
///         .with_db_cache(cache, 0)
///         .build()
///         .await;
/// }
/// ```
#[async_trait]
pub trait DbCache: Send + Sync {
    /// Estimated resident cache weight, excluding allocator overhead. Shared-cache implementations
    /// may report their full weight conservatively; None means this backend cannot measure it.
    fn estimated_memory_usage(&self) -> Option<u64> { None }

    async fn get_block(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error>;
    async fn get_index(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error>;
    async fn get_filter(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error>;
    async fn get_stats(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error>;
    async fn insert(&self, key: CachedKey, value: CachedEntry);
    #[allow(dead_code)]
    async fn remove(&self, key: &CachedKey);
    #[allow(dead_code)]
    fn entry_count(&self) -> u64;

    /// Gracefully close the cache, flushing any in-memory state to disk.
    ///
    /// Implementations backed by hybrid (memory + disk) caches should use
    /// this to ensure cached entries survive process restarts. The default
    /// implementation is a no-op.
    ///
    /// SlateDB only invokes this on caches it created itself (the default
    /// cache built when none is configured). If you pass your own cache to
    /// [`with_db_cache`](crate::db::builder::DbBuilder::with_db_cache), you
    /// own it: close it yourself after closing every `Db` and `DbReader`
    /// that uses it. This allows a single cache to be safely shared across
    /// multiple databases without the first `close()` disabling it for the
    /// others.
    async fn close(&self) -> Result<(), crate::Error> {
        Ok(())
    }

    /// Move every entry for this db_cache_id from memory to disk.
    async fn flush_scope(&self, db_cache_id: u64) -> Result<(), crate::Error> {
        let _ = db_cache_id;
        Ok(())
    }

    async fn flush_to_disk(&self) -> Result<(), crate::Error> {
        Ok(())
    }

    /// Fetch a data-block entry, invoking `loader` on cache miss.
    ///
    /// Custom implementations must report the lookup result in [`CacheFetch::lookup`].
    /// Return [`CacheLookup::Miss`] when this fetch loads the entry from object storage.
    /// Return [`CacheLookup::Hit`] when memory, disk, or a concurrent fetch supplies the entry.
    /// The cache wrapper uses this result to update hit and miss counters.
    /// It does not track whether the supplied loader runs.
    /// This contract also applies to [`Self::fetch_index`], [`Self::fetch_filter`], and [`Self::fetch_stats`].
    ///
    /// Implementations should deduplicate concurrent fetches: if multiple callers request the
    /// same key while it is being loaded, only one should run `loader` and the rest should
    /// share its result. The default implementation does **not** dedup; it simply does a
    /// get-miss-load-insert. Override in implementations whose backing cache exposes a
    /// dedup-aware fetch primitive (e.g. foyer's `Cache::fetch`).
    async fn fetch_block(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        if let Some(entry) = self.get_block(&key).await? {
            return Ok(CacheFetch::hit(entry));
        }
        let entry = loader().await?;
        self.insert(key, entry.clone()).await;
        Ok(CacheFetch::miss(entry))
    }

    /// Fetch an index entry, invoking `loader` on cache miss. See [`Self::fetch_block`].
    async fn fetch_index(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        if let Some(entry) = self.get_index(&key).await? {
            return Ok(CacheFetch::hit(entry));
        }
        let entry = loader().await?;
        self.insert(key, entry.clone()).await;
        Ok(CacheFetch::miss(entry))
    }

    /// Fetch a filter entry, invoking `loader` on cache miss. See [`Self::fetch_block`].
    async fn fetch_filter(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        if let Some(entry) = self.get_filter(&key).await? {
            return Ok(CacheFetch::hit(entry));
        }
        let entry = loader().await?;
        self.insert(key, entry.clone()).await;
        Ok(CacheFetch::miss(entry))
    }

    /// Fetch a stats entry, invoking `loader` on cache miss. See [`Self::fetch_block`].
    async fn fetch_stats(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        if let Some(entry) = self.get_stats(&key).await? {
            return Ok(CacheFetch::hit(entry));
        }
        let entry = loader().await?;
        self.insert(key, entry.clone()).await;
        Ok(CacheFetch::miss(entry))
    }
}

/// An SST component that can be inserted into the block cache, either by
/// warming an existing SST via
/// [`DbCacheManagerOps::warm_sst`](crate::DbCacheManagerOps::warm_sst) or as
/// the SST is written via [`BlockCachePolicy`](crate::BlockCachePolicy).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CacheTarget {
    /// All filter blocks on the SST, if any exist.
    Filters,
    /// The SST index.
    Index,
    /// The SST stats block, if one exists.
    Stats,
    /// Data blocks whose key span overlaps the supplied key range.
    Data((Bound<Bytes>, Bound<Bytes>)),
}

impl CacheTarget {
    /// Every target: all data blocks, the index, the filters, and the stats.
    pub fn all() -> [Self; 4] {
        [
            Self::data::<&[u8], _>(..),
            Self::Index,
            Self::Filters,
            Self::Stats,
        ]
    }

    /// Convenience constructor for [`CacheTarget::Data`] that accepts any
    /// [`RangeBounds`], mirroring the `Db::scan` signature. Pass `..` to
    /// select all data blocks.
    pub fn data<K, T>(range: T) -> Self
    where
        K: AsRef<[u8]>,
        T: RangeBounds<K>,
    {
        let start = range
            .start_bound()
            .map(|b| Bytes::copy_from_slice(b.as_ref()));
        let end = range
            .end_bound()
            .map(|b| Bytes::copy_from_slice(b.as_ref()));
        CacheTarget::Data((start, end))
    }
}

/// A [`DbCache`] paired with the `db_cache_id` to use for it.
///
/// Built by [`with_db_cache`](crate::db::builder::DbBuilder::with_db_cache) to keep the
/// cache and its id together internally, instead of passing them around as two separate,
/// easily-mismatched fields.
pub(crate) struct DbCacheAndScope {
    pub cache: Arc<dyn DbCache>,
    pub db_cache_id: u64,
}

impl DbCacheAndScope {
    pub(crate) fn new(cache: Arc<dyn DbCache>, db_cache_id: u64) -> Self {
        Self { cache, db_cache_id }
    }
}

/// A key used to identify a cached entry.
///
/// The key is composed of a scope ID (set per [`DbCacheWrapper`] instance), an SSTable ID,
/// and a block ID. `db_cache_id` is readable through [`Self::db_cache_id`]; the other fields stay
/// private to this module, so the SST/block layout is not exposed publicly.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CachedKey {
    /// Scope identifier set per `DbCacheWrapper`, so multiple `Db` instances sharing
    /// one cache don't collide on WAL or compacted entries.
    pub(crate) db_cache_id: u64,
    pub(crate) sst_id: SsTableId,
    pub(crate) block_id: u64,
}

impl CachedKey {
    /// Returns the scope id this key belongs to.
    ///
    /// A [`DbCache`] implementation reads this to tell which keys belong
    /// to the scope passed to [`DbCache::flush_scope`].
    pub fn db_cache_id(&self) -> u64 {
        self.db_cache_id
    }

    fn with_scope(&self, db_cache_id: u64) -> Self {
        Self {
            db_cache_id,
            sst_id: self.sst_id,
            block_id: self.block_id,
        }
    }
}

impl From<(SsTableId, u64)> for CachedKey {
    fn from((sst_id, block_id): (SsTableId, u64)) -> Self {
        Self {
            db_cache_id: 0,
            sst_id,
            block_id,
        }
    }
}

/// A filter cached in its on-disk byte form, paired with the name of the
/// policy that produced it. Only produced by disk-cache deserialization
/// (`db_cache::serde`), which has no access to the configured filter
/// policies; converted to a [`NamedFilter`] by `TableStore::read_filters`
/// on the first hit after deserialization.
#[derive(Clone)]
pub(crate) struct EncodedCachedFilter {
    pub(crate) name: String,
    pub(crate) data: Bytes,
}

#[non_exhaustive]
#[derive(Clone)]
enum CachedItem {
    Block(Arc<Block>),
    SsTableIndex(Arc<SsTableIndexOwned>),
    Filters(Arc<[NamedFilter]>),
    EncodedFilters(Arc<[EncodedCachedFilter]>),
    SstStats(Arc<SstStats>),
}

/// A cached entry stored in the cache.
///
/// The entry stores data in an internal enum that represents the type of cached item.
/// The internal types of the entries that are stored in the cache are private,
/// so the implementation details of the cache are not exposed publicly.
#[derive(Clone)]
pub struct CachedEntry {
    item: CachedItem,
}

impl CachedEntry {
    /// Create a new `CachedEntry` with the given block.
    pub(crate) fn with_block(block: Arc<Block>) -> Self {
        Self {
            item: CachedItem::Block(block),
        }
    }

    /// Create a new `CachedEntry` with the given SSTable index.
    pub(crate) fn with_sst_index(sst_index: Arc<SsTableIndexOwned>) -> Self {
        Self {
            item: CachedItem::SsTableIndex(sst_index),
        }
    }

    /// Create a new `CachedEntry` with the given decoded filters.
    pub(crate) fn with_filters(filters: Arc<[NamedFilter]>) -> Self {
        Self {
            item: CachedItem::Filters(filters),
        }
    }

    /// Create a new `CachedEntry` with the given SST stats.
    pub(crate) fn with_sst_stats(stats: Arc<SstStats>) -> Self {
        Self {
            item: CachedItem::SstStats(stats),
        }
    }

    pub(crate) fn block(&self) -> Option<Arc<Block>> {
        match &self.item {
            CachedItem::Block(block) => Some(block.clone()),
            _ => None,
        }
    }

    pub(crate) fn sst_index(&self) -> Option<Arc<SsTableIndexOwned>> {
        match &self.item {
            CachedItem::SsTableIndex(sst_index) => Some(sst_index.clone()),
            _ => None,
        }
    }

    pub(crate) fn filters(&self) -> Option<Arc<[NamedFilter]>> {
        match &self.item {
            CachedItem::Filters(filters) => Some(filters.clone()),
            _ => None,
        }
    }

    pub(crate) fn encoded_filters(&self) -> Option<Arc<[EncodedCachedFilter]>> {
        match &self.item {
            CachedItem::EncodedFilters(filters) => Some(filters.clone()),
            _ => None,
        }
    }

    pub(crate) fn sst_stats(&self) -> Option<Arc<SstStats>> {
        match &self.item {
            CachedItem::SstStats(stats) => Some(stats.clone()),
            _ => None,
        }
    }

    /// Returns the size of the cached entry in bytes.
    ///
    /// This method is public to allow external cache implementations
    /// to use it to implement custom weighers.
    pub fn size(&self) -> usize {
        match &self.item {
            CachedItem::Block(block) => block.size(),
            CachedItem::SsTableIndex(sst_index) => sst_index.size(),
            CachedItem::Filters(filters) => filters.iter().map(|nf| nf.filter.size()).sum(),
            CachedItem::EncodedFilters(filters) => filters.iter().map(|ef| ef.data.len()).sum(),
            CachedItem::SstStats(stats) => stats.size(),
        }
    }

    pub fn clamp_allocated_size(&self) -> Self {
        match &self.item {
            CachedItem::Block(block) => Self::with_block(Arc::new(block.clamp_allocated_size())),
            CachedItem::SsTableIndex(sst_index) => {
                Self::with_sst_index(Arc::new(sst_index.clamp_allocated_size()))
            }
            CachedItem::Filters(filters) => Self::with_filters(
                filters
                    .iter()
                    .map(|nf| NamedFilter {
                        name: nf.name.clone(),
                        filter: nf.filter.clamp_allocated_size(),
                    })
                    .collect::<Vec<_>>()
                    .into(),
            ),
            CachedItem::EncodedFilters(filters) => Self {
                item: CachedItem::EncodedFilters(filters.clone()),
            },
            CachedItem::SstStats(stats) => {
                Self::with_sst_stats(Arc::new(stats.clamp_allocated_size()))
            }
        }
    }
}

pub struct SplitCache {
    // Cache for block data
    block_cache: Option<Arc<dyn DbCache>>,
    // Cache for indices and filters
    meta_cache: Option<Arc<dyn DbCache>>,
}

impl SplitCache {
    pub fn new() -> Self {
        Self {
            block_cache: None,
            meta_cache: None,
        }
    }

    pub fn with_block_cache(mut self, cache: Option<Arc<dyn DbCache>>) -> Self {
        self.block_cache = cache;
        self
    }

    pub fn with_meta_cache(mut self, cache: Option<Arc<dyn DbCache>>) -> Self {
        self.meta_cache = cache;
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

impl Default for SplitCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DbCache for SplitCache {
    fn estimated_memory_usage(&self) -> Option<u64> {
        let block = self.block_cache.as_ref().map_or(Some(0), |cache| cache.estimated_memory_usage())?;
        let meta = self.meta_cache.as_ref().map_or(Some(0), |cache| cache.estimated_memory_usage())?;
        Some(block.saturating_add(meta))
    }

    async fn get_block(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        if let Some(cache) = &self.block_cache {
            cache.get_block(key).await
        } else {
            Ok(None)
        }
    }

    async fn get_index(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        if let Some(cache) = &self.meta_cache {
            cache.get_index(key).await
        } else {
            Ok(None)
        }
    }

    async fn get_filter(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        if let Some(cache) = &self.meta_cache {
            cache.get_filter(key).await
        } else {
            Ok(None)
        }
    }

    async fn get_stats(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        if let Some(cache) = &self.meta_cache {
            cache.get_stats(key).await
        } else {
            Ok(None)
        }
    }

    async fn insert(&self, key: CachedKey, value: CachedEntry) {
        match &value.item {
            CachedItem::Block(_) => {
                if let Some(ref cache) = self.block_cache {
                    cache.insert(key, value.clamp_allocated_size()).await;
                } else {
                    trace!("no block cache available for insertion");
                }
            }
            CachedItem::SsTableIndex(_) | CachedItem::Filters(_) | CachedItem::SstStats(_) => {
                if let Some(ref cache) = self.meta_cache {
                    cache.insert(key, value.clamp_allocated_size()).await;
                } else {
                    trace!("no meta cache available for insertion");
                }
            }
            // EncodedFilters only exist as the transient output of
            // disk-cache deserialization, which happens inside the
            // underlying cache impl (foyer) and never flows back through
            // `SplitCache::insert`. A direct insert of an encoded entry
            // would indicate the invariant was bypassed.
            CachedItem::EncodedFilters(_) => {
                error!(
                    "SplitCache::insert called with EncodedFilters; encoded \
                     entries only exist inside foyer's deserialization path"
                );
                debug_assert!(false, "EncodedFilters in SplitCache::insert");
            }
        }
    }

    #[allow(dead_code)]
    async fn remove(&self, key: &CachedKey) {
        // Because `CachedKey` is uniquely identified by (scope ID, SST ID, offset), given a
        // `CachedKey`, it will only appear in the block cache or meta cache, which is safe and
        // will not cause duplicate deletion.
        if let Some(ref cache) = self.block_cache {
            cache.remove(key).await;
        }
        if let Some(ref cache) = self.meta_cache {
            cache.remove(key).await;
        }
    }

    fn entry_count(&self) -> u64 {
        self.block_cache.as_ref().map_or(0, |c| c.entry_count())
            + self.meta_cache.as_ref().map_or(0, |c| c.entry_count())
    }

    async fn close(&self) -> Result<(), crate::Error> {
        if let Some(ref cache) = self.block_cache {
            cache.close().await?;
        }
        if let Some(ref cache) = self.meta_cache {
            cache.close().await?;
        }
        Ok(())
    }

    async fn flush_scope(&self, db_cache_id: u64) -> Result<(), crate::Error> {
        let block_result = match &self.block_cache {
            Some(cache) => cache.flush_scope(db_cache_id).await,
            None => Ok(()),
        };
        let meta_result = match &self.meta_cache {
            Some(cache) => cache.flush_scope(db_cache_id).await,
            None => Ok(()),
        };
        block_result.and(meta_result)
    }

    async fn fetch_block(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        if let Some(cache) = &self.block_cache {
            cache.fetch_block(key, loader).await
        } else {
            loader().await.map(CacheFetch::miss)
        }
    }

    async fn fetch_index(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        if let Some(cache) = &self.meta_cache {
            cache.fetch_index(key, loader).await
        } else {
            loader().await.map(CacheFetch::miss)
        }
    }

    async fn fetch_filter(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        if let Some(cache) = &self.meta_cache {
            cache.fetch_filter(key, loader).await
        } else {
            loader().await.map(CacheFetch::miss)
        }
    }

    async fn fetch_stats(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        if let Some(cache) = &self.meta_cache {
            cache.fetch_stats(key, loader).await
        } else {
            loader().await.map(CacheFetch::miss)
        }
    }
}

/// Wraps a [`DbCache`] to add statistics, error logging, and cache scoping.
///
/// ## Scoping
/// When multiple `Db`/`DbReader` instances share the same underlying cache object, this
/// wrapper assigns a `db_cache_id` so their entries do not collide. All cache operations
/// transparently rewrite keys to include the wrapper's `db_cache_id`, isolating WAL and
/// compacted SST entries per wrapper.
///
/// `db_cache_id` is supplied by the caller at construction time (see [`Self::new`]),
/// not derived by SlateDB: pass the same id across a legitimate reopen of the same
/// logical database to recover its warm entries, and different ids for logically
/// different databases sharing the cache.
pub(crate) struct DbCacheWrapper {
    stats: DbCacheStats,
    system_clock: Arc<dyn SystemClock>,
    cache: Arc<dyn DbCache>,
    /// Identifier applied to every key passed through this wrapper, supplied by
    /// the caller of [`Self::new`]. This prevents different `DbCacheWrapper`
    /// instances backed by different logical `Db`s from clobbering each other's entries
    /// in a cache they share. `0` is a valid choice when only one instance will ever
    /// use this cache.
    db_cache_id: u64,
    // Records the last time that the wrapper logged an error from the wrapped cache at error
    // level. Used to ensure we only log at error level once every ERROR_LOG_INTERVAL.
    last_err_log_time: Mutex<Option<DateTime<Utc>>>,
}

impl DbCacheWrapper {
    pub(crate) fn new(
        cache: Arc<dyn DbCache>,
        recorder: &MetricsRecorderHelper,
        system_clock: Arc<dyn SystemClock>,
        db_cache_id: u64,
    ) -> Self {
        Self {
            stats: DbCacheStats::new(recorder),
            cache,
            db_cache_id,
            last_err_log_time: Mutex::new(None),
            system_clock,
        }
    }
}

// The minimum interval between which the wrapper logs cache errors at error level. This is used to
// ensure we don't spam the logs on non-transient errors from the cache.
const ERROR_LOG_INTERVAL: TimeDelta = TimeDelta::seconds(1);

impl DbCacheWrapper {
    fn scoped_key(&self, key: &CachedKey) -> CachedKey {
        key.with_scope(self.db_cache_id)
    }

    fn record_fetch_outcome(&self, block_type: &str, result: &Result<CacheFetch, crate::Error>) {
        match result {
            Ok(CacheFetch {
                lookup: CacheLookup::Miss,
                ..
            }) => self.record_miss(block_type),
            Ok(CacheFetch {
                lookup: CacheLookup::Hit,
                ..
            }) => self.record_hit(block_type),
            Err(err) => self.record_get_err(block_type, err),
        }
    }

    fn record_hit(&self, block_type: &str) {
        match block_type {
            "block" => self.stats.data_block_hit.increment(1),
            "index" => self.stats.index_hit.increment(1),
            "filter" => self.stats.filter_hit.increment(1),
            "stats" => self.stats.stats_hit.increment(1),
            _ => {}
        }
    }

    fn record_miss(&self, block_type: &str) {
        match block_type {
            "block" => self.stats.data_block_miss.increment(1),
            "index" => self.stats.index_miss.increment(1),
            "filter" => self.stats.filter_miss.increment(1),
            "stats" => self.stats.stats_miss.increment(1),
            _ => {}
        }
    }

    fn record_get_err(&self, block_type: &str, err: &crate::Error) {
        let log_at_err = {
            let mut guard = self.last_err_log_time.lock();
            match *guard {
                None => {
                    *guard = Some(self.system_clock.now());
                    true
                }
                Some(t) if self.system_clock.now() - t > ERROR_LOG_INTERVAL => {
                    *guard = Some(self.system_clock.now());
                    true
                }
                _ => false,
            }
        };
        if log_at_err {
            error!(
                "error getting block from cache [block_type={} error={:?}]",
                block_type, err
            );
        } else {
            debug!(
                "error getting block from cache [block_type={} error={:?}]",
                block_type, err
            );
        }
        self.stats.get_error.increment(1);
    }
}

#[async_trait]
impl DbCache for DbCacheWrapper {
    fn estimated_memory_usage(&self) -> Option<u64> { self.cache.estimated_memory_usage() }

    async fn get_block(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        let scoped_key = self.scoped_key(key);
        let entry = match self.cache.get_block(&scoped_key).await {
            Ok(e) => e,
            Err(err) => {
                self.record_get_err("block", &err);
                return Err(err);
            }
        };
        if entry.is_some() {
            self.stats.data_block_hit.increment(1);
        } else {
            self.stats.data_block_miss.increment(1);
        }
        Ok(entry)
    }

    async fn get_index(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        let scoped_key = self.scoped_key(key);
        let entry = match self.cache.get_index(&scoped_key).await {
            Ok(e) => e,
            Err(err) => {
                self.record_get_err("index", &err);
                return Err(err);
            }
        };
        if entry.is_some() {
            self.stats.index_hit.increment(1);
        } else {
            self.stats.index_miss.increment(1);
        }
        Ok(entry)
    }

    async fn get_filter(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        let scoped_key = self.scoped_key(key);
        let entry = match self.cache.get_filter(&scoped_key).await {
            Ok(e) => e,
            Err(err) => {
                self.record_get_err("filter", &err);
                return Err(err);
            }
        };
        if entry.is_some() {
            self.stats.filter_hit.increment(1);
        } else {
            self.stats.filter_miss.increment(1);
        }
        Ok(entry)
    }

    async fn get_stats(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        let scoped_key = self.scoped_key(key);
        let entry = match self.cache.get_stats(&scoped_key).await {
            Ok(e) => e,
            Err(err) => {
                self.record_get_err("stats", &err);
                return Err(err);
            }
        };
        if entry.is_some() {
            self.stats.stats_hit.increment(1);
        } else {
            self.stats.stats_miss.increment(1);
        }
        Ok(entry)
    }

    async fn insert(&self, key: CachedKey, value: CachedEntry) {
        let scoped_key = self.scoped_key(&key);
        self.cache.insert(scoped_key, value).await;
    }

    #[allow(dead_code)]
    async fn remove(&self, key: &CachedKey) {
        let scoped_key = self.scoped_key(key);
        self.cache.remove(&scoped_key).await
    }

    fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    async fn close(&self) -> Result<(), crate::Error> {
        self.cache.close().await
    }

    async fn flush_to_disk(&self) -> Result<(), crate::Error> {
        self.cache.flush_scope(self.db_cache_id).await
    }

    async fn fetch_block(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        let scoped_key = self.scoped_key(&key);
        let result = self.cache.fetch_block(scoped_key, loader).await;
        self.record_fetch_outcome("block", &result);
        result
    }

    async fn fetch_index(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        let scoped_key = self.scoped_key(&key);
        let result = self.cache.fetch_index(scoped_key, loader).await;
        self.record_fetch_outcome("index", &result);
        result
    }

    async fn fetch_filter(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        let scoped_key = self.scoped_key(&key);
        let result = self.cache.fetch_filter(scoped_key, loader).await;
        self.record_fetch_outcome("filter", &result);
        result
    }

    async fn fetch_stats(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        let scoped_key = self.scoped_key(&key);
        let result = self.cache.fetch_stats(scoped_key, loader).await;
        self.record_fetch_outcome("stats", &result);
        result
    }
}

/// Wraps a user-provided [`DbCache`] so that [`DbCache::close`] becomes a no-op.
///
/// SlateDB does not own caches passed in via `with_db_cache`: the caller may be sharing one
/// cache across several `Db`/`DbReader` instances, so the first `Db::close()` must not shut
/// it down for the others. The caller closes the inner cache themselves once every instance
/// using it is closed.
///
/// Note: every trait method must be forwarded explicitly, including the ones with default
/// implementations. Falling back to a default here would silently replace the inner cache's
/// behavior (e.g. foyer's dedup-aware `fetch_*`) for user-provided caches only.
pub(crate) struct UnownedDbCache {
    inner: Arc<dyn DbCache>,
}

impl UnownedDbCache {
    pub(crate) fn new(inner: Arc<dyn DbCache>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl DbCache for UnownedDbCache {
    fn estimated_memory_usage(&self) -> Option<u64> { self.inner.estimated_memory_usage() }

    async fn get_block(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        self.inner.get_block(key).await
    }

    async fn get_index(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        self.inner.get_index(key).await
    }

    async fn get_filter(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        self.inner.get_filter(key).await
    }

    async fn get_stats(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        self.inner.get_stats(key).await
    }

    async fn insert(&self, key: CachedKey, value: CachedEntry) {
        self.inner.insert(key, value).await
    }

    async fn remove(&self, key: &CachedKey) {
        self.inner.remove(key).await
    }

    fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    /// The point of this type: never propagate close to a cache we don't own.
    async fn close(&self) -> Result<(), crate::Error> {
        Ok(())
    }

    async fn flush_scope(&self, db_cache_id: u64) -> Result<(), crate::Error> {
        self.inner.flush_scope(db_cache_id).await
    }

    async fn flush_to_disk(&self) -> Result<(), crate::Error> {
        self.inner.flush_to_disk().await
    }

    async fn fetch_block(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        self.inner.fetch_block(key, loader).await
    }

    async fn fetch_index(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        self.inner.fetch_index(key, loader).await
    }

    async fn fetch_filter(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        self.inner.fetch_filter(key, loader).await
    }

    async fn fetch_stats(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        self.inner.fetch_stats(key, loader).await
    }
}

/// Tracks whether this caller ran the loader.
#[cfg(feature = "foyer")]
#[derive(Clone)]
struct LoaderRan(Arc<AtomicBool>);

#[cfg(feature = "foyer")]
impl LoaderRan {
    fn was_called(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(feature = "foyer")]
fn instrumented_loader(loader: CacheLoader) -> (CacheLoader, LoaderRan) {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_for_closure = flag.clone();
    let wrapped: CacheLoader = Box::new(move || {
        flag_for_closure.store(true, Ordering::Relaxed);
        loader()
    });
    (wrapped, LoaderRan(flag))
}

pub mod stats {
    use slatedb_common::metrics::{CounterFn, MetricsRecorderHelper};
    use std::sync::Arc;

    macro_rules! dbcache_stat_name {
        ($suffix:expr) => {
            concat!("slatedb.db_cache.", $suffix)
        };
    }

    pub const ACCESS_COUNT: &str = dbcache_stat_name!("access_count");
    pub const ERROR_COUNT: &str = dbcache_stat_name!("error_count");

    pub(super) struct DbCacheStats {
        pub(super) filter_hit: Arc<dyn CounterFn>,
        pub(super) filter_miss: Arc<dyn CounterFn>,
        pub(super) index_hit: Arc<dyn CounterFn>,
        pub(super) index_miss: Arc<dyn CounterFn>,
        pub(super) data_block_hit: Arc<dyn CounterFn>,
        pub(super) data_block_miss: Arc<dyn CounterFn>,
        pub(super) stats_hit: Arc<dyn CounterFn>,
        pub(super) stats_miss: Arc<dyn CounterFn>,
        pub(super) get_error: Arc<dyn CounterFn>,
    }

    impl DbCacheStats {
        pub(super) fn new(recorder: &MetricsRecorderHelper) -> Self {
            Self {
                filter_hit: recorder
                    .counter(ACCESS_COUNT)
                    .labels(&[("entry_kind", "filter"), ("result", "hit")])
                    .register(),
                filter_miss: recorder
                    .counter(ACCESS_COUNT)
                    .labels(&[("entry_kind", "filter"), ("result", "miss")])
                    .register(),
                index_hit: recorder
                    .counter(ACCESS_COUNT)
                    .labels(&[("entry_kind", "index"), ("result", "hit")])
                    .register(),
                index_miss: recorder
                    .counter(ACCESS_COUNT)
                    .labels(&[("entry_kind", "index"), ("result", "miss")])
                    .register(),
                data_block_hit: recorder
                    .counter(ACCESS_COUNT)
                    .labels(&[("entry_kind", "data_block"), ("result", "hit")])
                    .register(),
                data_block_miss: recorder
                    .counter(ACCESS_COUNT)
                    .labels(&[("entry_kind", "data_block"), ("result", "miss")])
                    .register(),
                stats_hit: recorder
                    .counter(ACCESS_COUNT)
                    .labels(&[("entry_kind", "stats"), ("result", "hit")])
                    .register(),
                stats_miss: recorder
                    .counter(ACCESS_COUNT)
                    .labels(&[("entry_kind", "stats"), ("result", "miss")])
                    .register(),
                get_error: recorder.counter(ERROR_COUNT).register(),
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_utils {
    use crate::db_cache::{CachedEntry, CachedKey, DbCache};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// A cache that always returns an error from get operations.
    pub(crate) struct FailingCache;

    #[async_trait]
    impl DbCache for FailingCache {
        async fn get_block(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
            Err(
                crate::error::SlateDBError::from(Arc::new(std::io::Error::other("injected error")))
                    .into(),
            )
        }
        async fn get_index(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
            Err(
                crate::error::SlateDBError::from(Arc::new(std::io::Error::other("injected error")))
                    .into(),
            )
        }
        async fn get_filter(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
            Err(
                crate::error::SlateDBError::from(Arc::new(std::io::Error::other("injected error")))
                    .into(),
            )
        }
        async fn get_stats(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
            Err(
                crate::error::SlateDBError::from(Arc::new(std::io::Error::other("injected error")))
                    .into(),
            )
        }
        async fn insert(&self, _: CachedKey, _: CachedEntry) {}
        async fn remove(&self, _: &CachedKey) {}
        fn entry_count(&self) -> u64 {
            0
        }
        async fn flush_scope(&self, _: u64) -> Result<(), crate::Error> {
            Err(
                crate::error::SlateDBError::from(Arc::new(std::io::Error::other("injected error")))
                    .into(),
            )
        }
    }

    pub(crate) struct TestCache {
        items: Mutex<HashMap<CachedKey, CachedEntry>>,
        spilled: Mutex<HashMap<CachedKey, CachedEntry>>,
    }

    impl TestCache {
        pub(crate) fn new() -> Self {
            Self {
                items: Mutex::new(HashMap::new()),
                spilled: Mutex::new(HashMap::new()),
            }
        }

        pub(crate) fn keys(&self) -> Vec<CachedKey> {
            self.items.lock().unwrap().keys().cloned().collect()
        }

        pub(crate) fn spilled_keys(&self) -> Vec<CachedKey> {
            self.spilled.lock().unwrap().keys().cloned().collect()
        }
    }

    #[async_trait]
    impl DbCache for TestCache {
        async fn get_block(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
            let guard = self.items.lock().unwrap();
            Ok(guard.get(key).cloned())
        }

        async fn get_index(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
            let guard = self.items.lock().unwrap();
            Ok(guard.get(key).cloned())
        }

        async fn get_filter(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
            let guard = self.items.lock().unwrap();
            Ok(guard.get(key).cloned())
        }

        async fn get_stats(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
            let guard = self.items.lock().unwrap();
            Ok(guard.get(key).cloned())
        }

        async fn insert(&self, key: CachedKey, value: CachedEntry) {
            let mut guard = self.items.lock().unwrap();
            guard.insert(key, value);
        }

        async fn remove(&self, key: &CachedKey) {
            let mut guard = self.items.lock().unwrap();
            guard.remove(key);
        }

        fn entry_count(&self) -> u64 {
            let guard = self.items.lock().unwrap();
            guard.iter().count() as u64
        }

        async fn flush_scope(&self, db_cache_id: u64) -> Result<(), crate::Error> {
            let mut items = self.items.lock().unwrap();
            let matching: Vec<CachedKey> = items
                .keys()
                .filter(|k| k.db_cache_id == db_cache_id)
                .cloned()
                .collect();
            let mut spilled = self.spilled.lock().unwrap();
            for key in matching {
                if let Some(value) = items.remove(&key) {
                    spilled.insert(key, value);
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::db_cache::{
        CacheFetch, CacheLoader, CacheLookup, CachedEntry, CachedKey, DbCache, DbCacheWrapper,
        SplitCache,
    };
    use crate::db_state::SsTableId;
    use crate::filter_policy::{BloomFilterPolicy, FilterPolicy, NamedFilter};
    use crate::format::sst::BlockBuilder;
    use slatedb_common::clock::DefaultSystemClock;

    use crate::flatbuffer_types::test_utils::assert_index_clamped;

    #[cfg(feature = "foyer")]
    use super::foyer::FoyerCache;
    #[cfg(feature = "foyer")]
    use super::foyer_hybrid::FoyerHybridCache;
    #[cfg(feature = "moka")]
    use super::moka::MokaCache;
    use crate::db_cache::test_utils::TestCache;
    use crate::format::sst::{EncodedSsTable, SsTableFormat};
    use crate::test_utils::build_test_sst;
    use crate::types::{RowEntry, ValueDeletable};
    #[cfg(feature = "foyer")]
    use foyer::HybridCacheBuilder;
    use rstest::{fixture, rstest};
    use slatedb_common::metrics::{
        lookup_metric_with_labels, DefaultMetricsRecorder, MetricLevel, MetricsRecorderHelper,
    };
    use std::sync::Arc;
    use ulid::Ulid;

    const SST_ID: SsTableId = SsTableId::new(Ulid::from_parts(0u64, 0u128));

    #[tokio::test]
    async fn test_wrapper_counts_reported_fetch_lookup() {
        struct ReportedLookupCache {
            lookup: CacheLookup,
            entry: CachedEntry,
        }

        #[async_trait::async_trait]
        impl DbCache for ReportedLookupCache {
            async fn get_block(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
                panic!("The wrapper must call fetch_block.");
            }
            async fn get_index(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
                panic!("The wrapper must call fetch_index.");
            }
            async fn get_filter(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
                panic!("The wrapper must call fetch_filter.");
            }
            async fn get_stats(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
                panic!("The wrapper must call fetch_stats.");
            }
            async fn insert(&self, _: CachedKey, _: CachedEntry) {}
            async fn remove(&self, _: &CachedKey) {}
            fn entry_count(&self) -> u64 {
                0
            }
            async fn fetch_block(
                &self,
                _: CachedKey,
                _: CacheLoader,
            ) -> Result<CacheFetch, crate::Error> {
                Ok(CacheFetch {
                    entry: self.entry.clone(),
                    lookup: self.lookup,
                })
            }
            async fn fetch_index(
                &self,
                key: CachedKey,
                loader: CacheLoader,
            ) -> Result<CacheFetch, crate::Error> {
                self.fetch_block(key, loader).await
            }
            async fn fetch_filter(
                &self,
                key: CachedKey,
                loader: CacheLoader,
            ) -> Result<CacheFetch, crate::Error> {
                self.fetch_block(key, loader).await
            }
            async fn fetch_stats(
                &self,
                key: CachedKey,
                loader: CacheLoader,
            ) -> Result<CacheFetch, crate::Error> {
                self.fetch_block(key, loader).await
            }
        }

        for lookup in [CacheLookup::Miss, CacheLookup::Hit] {
            for method in ["data_block", "index", "filter", "stats"] {
                let recorder = Arc::new(DefaultMetricsRecorder::new());
                let helper = MetricsRecorderHelper::new(recorder.clone(), MetricLevel::default());
                let mut builder = BlockBuilder::new_latest(4096);
                assert!(builder
                    .add(RowEntry::new_value(b"key", b"value", 0))
                    .unwrap());
                let block = Arc::new(builder.build().unwrap());
                let cache = DbCacheWrapper::new(
                    Arc::new(ReportedLookupCache {
                        lookup,
                        entry: CachedEntry::with_block(block.clone()),
                    }),
                    &helper,
                    Arc::new(DefaultSystemClock::default()),
                    1,
                );
                let key = CachedKey::from((SST_ID, 0));
                let loader: CacheLoader =
                    Box::new(|| panic!("This cache supplies its own result."));
                let fetch = match method {
                    "data_block" => cache.fetch_block(key, loader).await,
                    "index" => cache.fetch_index(key, loader).await,
                    "filter" => cache.fetch_filter(key, loader).await,
                    "stats" => cache.fetch_stats(key, loader).await,
                    _ => unreachable!(),
                }
                .unwrap();
                assert_eq!(fetch.lookup, lookup);
                assert!(Arc::ptr_eq(&fetch.entry.block().unwrap(), &block));
                for entry_kind in ["data_block", "index", "filter", "stats"] {
                    for (result, expected_lookup) in
                        [("hit", CacheLookup::Hit), ("miss", CacheLookup::Miss)]
                    {
                        let expected = i64::from(entry_kind == method && lookup == expected_lookup);
                        assert_eq!(
                            lookup_metric_with_labels(
                                &recorder,
                                super::stats::ACCESS_COUNT,
                                &[("entry_kind", entry_kind), ("result", result)],
                            ),
                            Some(expected),
                            "method={method}, lookup={lookup:?}, entry_kind={entry_kind}, result={result}",
                        );
                    }
                }
            }
        }
    }

    #[rstest]
    #[case::test_cache(Arc::new(TestCache::new()), true)]
    #[case::split_cache(Arc::new(SplitCache::new()), false)]
    #[case::split_cache_with_delegates(
        Arc::new(
            SplitCache::new()
                .with_block_cache(Some(Arc::new(TestCache::new())))
                .with_meta_cache(Some(Arc::new(TestCache::new())))
        ),
        true
    )]
    #[cfg_attr(feature = "foyer", case::foyer(Arc::new(FoyerCache::new()), true))]
    #[cfg_attr(
        feature = "foyer",
        case::foyer_hybrid(
            Arc::new(FoyerHybridCache::new_with_cache(
                HybridCacheBuilder::new()
                    .memory(1024 * 1024)
                    .with_weighter(|_, v: &CachedEntry| v.size())
                    .storage()
                    .build()
                    .await
                    .unwrap()
            )),
            true
        )
    )]
    #[cfg_attr(feature = "moka", case::moka(Arc::new(MokaCache::new()), true))]
    #[tokio::test]
    async fn test_fetch_lookup_outcomes(
        #[case] cache: Arc<dyn DbCache>,
        #[case] retains_entries: bool,
    ) {
        for (offset, method) in ["data_block", "index", "filter", "stats"]
            .into_iter()
            .enumerate()
        {
            let key = CachedKey::from((SST_ID, offset as u64));
            let mut builder = BlockBuilder::new_latest(4096);
            assert!(builder
                .add(RowEntry::new_value(b"key", b"value", 0))
                .unwrap());
            let block = Arc::new(builder.build().unwrap());
            for attempt in 0..2 {
                let entry = CachedEntry::with_block(block.clone());
                let loader: CacheLoader = Box::new(move || Box::pin(async move { Ok(entry) }));
                let fetch = match method {
                    "data_block" => cache.fetch_block(key.clone(), loader).await,
                    "index" => cache.fetch_index(key.clone(), loader).await,
                    "filter" => cache.fetch_filter(key.clone(), loader).await,
                    "stats" => cache.fetch_stats(key.clone(), loader).await,
                    _ => unreachable!(),
                }
                .unwrap();
                assert_eq!(
                    fetch.lookup,
                    if attempt == 0 || !retains_entries {
                        CacheLookup::Miss
                    } else {
                        CacheLookup::Hit
                    }
                );
                assert_eq!(fetch.entry.block().unwrap().size(), block.size());
            }
        }
    }

    #[cfg(feature = "foyer")]
    #[tokio::test]
    async fn test_concurrent_fetch_lookup() {
        let cache = FoyerCache::new();
        let key = CachedKey::from((SST_ID, 0));
        let loader_started = Arc::new(tokio::sync::Notify::new());
        let release_loader = Arc::new(tokio::sync::Notify::new());
        let loader: CacheLoader = {
            let loader_started = loader_started.clone();
            let release_loader = release_loader.clone();
            Box::new(move || {
                Box::pin(async move {
                    loader_started.notify_one();
                    release_loader.notified().await;
                    let mut builder = BlockBuilder::new_latest(4096);
                    assert!(builder
                        .add(RowEntry::new_value(b"key", b"value", 0))
                        .unwrap());
                    Ok(CachedEntry::with_block(Arc::new(builder.build().unwrap())))
                })
            })
        };
        let first = cache.fetch_block(key.clone(), loader);
        tokio::pin!(first);
        assert!(futures::poll!(&mut first).is_pending());
        loader_started.notified().await;

        let second = cache.fetch_block(
            key,
            Box::new(|| panic!("The concurrent fetch must share the first loader.")),
        );
        tokio::pin!(second);
        assert!(futures::poll!(&mut second).is_pending());
        release_loader.notify_one();

        let first = first.await.unwrap();
        let second = second.await.unwrap();
        assert_eq!(first.lookup, CacheLookup::Miss);
        assert_eq!(second.lookup, CacheLookup::Hit);
        assert!(Arc::ptr_eq(
            &first.entry.block().unwrap(),
            &second.entry.block().unwrap()
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn test_should_count_filter_hits(
        cache: (DbCacheWrapper, Arc<DefaultMetricsRecorder>),
        #[future(awt)] sst: EncodedSsTable,
    ) {
        let (cache, registry) = cache;
        // given:
        let key = CachedKey::from((SST_ID, 12345u64));
        cache
            .insert(key.clone(), CachedEntry::with_filters(sst.filters))
            .await;

        for i in 1..4 {
            // when:
            let _ = cache.get_filter(&key).await;

            // then:
            assert_eq!(
                Some(0),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "filter"), ("result", "miss")]
                )
            );
            assert_eq!(
                Some(i as i64),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "filter"), ("result", "hit")]
                )
            );
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_should_count_filter_misses(cache: (DbCacheWrapper, Arc<DefaultMetricsRecorder>)) {
        let (cache, registry) = cache;
        // given:
        let key = CachedKey::from((SST_ID, 12345u64));

        for i in 1..4 {
            // when:
            let _ = cache.get_filter(&key).await;

            // then:
            assert_eq!(
                Some(i as i64),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "filter"), ("result", "miss")]
                )
            );
            assert_eq!(
                Some(0),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "filter"), ("result", "hit")]
                )
            );
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_should_count_index_hits(
        cache: (DbCacheWrapper, Arc<DefaultMetricsRecorder>),
        #[future(awt)] sst: EncodedSsTable,
    ) {
        let (cache, registry) = cache;
        // given:
        let key = CachedKey::from((SST_ID, 12345u64));
        cache
            .insert(
                key.clone(),
                CachedEntry::with_sst_index(Arc::new(sst.index)),
            )
            .await;

        for i in 1..4 {
            // when:
            let _ = cache.get_index(&key).await;

            // then:
            assert_eq!(
                Some(0),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "index"), ("result", "miss")]
                )
            );
            assert_eq!(
                Some(i as i64),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "index"), ("result", "hit")]
                )
            );
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_should_clamp_entries_to_cache(
        cache: (DbCacheWrapper, Arc<DefaultMetricsRecorder>),
        sst_format: SsTableFormat,
        #[future(awt)] sst: EncodedSsTable,
    ) {
        let (cache, _registry) = cache;
        // given:
        let bytes = sst.remaining_as_bytes();
        let index = Arc::new(sst_format.read_index_raw(&sst.info, &bytes).await.unwrap());
        let key = CachedKey::from((SST_ID, 12345u64));
        cache
            .insert(key.clone(), CachedEntry::with_sst_index(index.clone()))
            .await;

        // when:
        let cached = cache.get_index(&key).await.unwrap().unwrap();

        // then:
        assert_index_clamped(index.as_ref(), cached.sst_index().unwrap().as_ref());
    }

    #[rstest]
    #[tokio::test]
    async fn test_should_count_index_misses(cache: (DbCacheWrapper, Arc<DefaultMetricsRecorder>)) {
        let (cache, registry) = cache;
        // given:
        let key = CachedKey::from((SST_ID, 12345u64));

        for i in 1..4 {
            // when:
            let _ = cache.get_index(&key).await;

            // then:
            assert_eq!(
                Some(i as i64),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "index"), ("result", "miss")]
                )
            );
            assert_eq!(
                Some(0),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "index"), ("result", "hit")]
                )
            );
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_should_count_data_block_hits(
        cache: (DbCacheWrapper, Arc<DefaultMetricsRecorder>),
        sst_format: SsTableFormat,
        #[future(awt)] sst: EncodedSsTable,
    ) {
        let (cache, registry) = cache;
        // given:
        let data = sst.remaining_as_bytes();
        let block = sst_format
            .read_block_raw(&sst.info, &sst.index, 0, &data)
            .await
            .unwrap();
        let key = CachedKey::from((SST_ID, 12345u64));
        cache
            .insert(key.clone(), CachedEntry::with_block(Arc::new(block)))
            .await;

        for i in 1..4 {
            // when:
            let _ = cache.get_block(&key).await;

            // then:
            assert_eq!(
                Some(0),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "data_block"), ("result", "miss")]
                )
            );
            assert_eq!(
                Some(i as i64),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "data_block"), ("result", "hit")]
                )
            );
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_should_count_data_block_misses(
        cache: (DbCacheWrapper, Arc<DefaultMetricsRecorder>),
    ) {
        let (cache, registry) = cache;
        // given:
        let key = CachedKey::from((SST_ID, 12345u64));

        for i in 1..4 {
            // when:
            let _ = cache.get_block(&key).await;

            // then:
            assert_eq!(
                Some(i as i64),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "data_block"), ("result", "miss")]
                )
            );
            assert_eq!(
                Some(0),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "data_block"), ("result", "hit")]
                )
            );
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_should_count_stats_hits(cache: (DbCacheWrapper, Arc<DefaultMetricsRecorder>)) {
        let (cache, registry) = cache;
        // given:
        let key = CachedKey::from((SST_ID, 12345u64));
        let stats = crate::sst_stats::SstStats::default();
        cache
            .insert(key.clone(), CachedEntry::with_sst_stats(Arc::new(stats)))
            .await;

        for i in 1..4 {
            // when:
            let _ = cache.get_stats(&key).await;

            // then:
            assert_eq!(
                Some(0),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "stats"), ("result", "miss")]
                )
            );
            assert_eq!(
                Some(i as i64),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "stats"), ("result", "hit")]
                )
            );
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_should_count_stats_misses(cache: (DbCacheWrapper, Arc<DefaultMetricsRecorder>)) {
        let (cache, registry) = cache;
        // given:
        let key = CachedKey::from((SST_ID, 12345u64));

        for i in 1..4 {
            // when:
            let _ = cache.get_stats(&key).await;

            // then:
            assert_eq!(
                Some(i as i64),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "stats"), ("result", "miss")]
                )
            );
            assert_eq!(
                Some(0),
                lookup_metric_with_labels(
                    &registry,
                    super::stats::ACCESS_COUNT,
                    &[("entry_kind", "stats"), ("result", "hit")]
                )
            );
        }
    }

    #[tokio::test]
    async fn test_should_count_get_errors() {
        // given: a cache that always returns errors
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), MetricLevel::default());
        let failing_cache: Arc<dyn DbCache> = Arc::new(super::test_utils::FailingCache);
        let cache = DbCacheWrapper::new(
            failing_cache,
            &helper,
            Arc::new(DefaultSystemClock::default()),
            1,
        );
        let key = CachedKey::from((SST_ID, 12345u64));

        // when: each get method returns an error
        let _ = cache.get_block(&key).await;
        let _ = cache.get_index(&key).await;
        let _ = cache.get_filter(&key).await;
        let _ = cache.get_stats(&key).await;

        // then:
        assert_eq!(
            slatedb_common::metrics::lookup_metric(&recorder, super::stats::ERROR_COUNT),
            Some(4)
        );
    }

    #[tokio::test]
    async fn test_cache_wrapper_scopes_keys() {
        let recorder_a = MetricsRecorderHelper::noop();
        let recorder_b = MetricsRecorderHelper::noop();
        let system_clock = Arc::new(DefaultSystemClock::default());
        let shared_cache: Arc<dyn DbCache> = Arc::new(TestCache::new());
        let cache_a =
            DbCacheWrapper::new(shared_cache.clone(), &recorder_a, system_clock.clone(), 1);
        let cache_b = DbCacheWrapper::new(shared_cache.clone(), &recorder_b, system_clock, 2);
        assert_ne!(cache_a.db_cache_id, cache_b.db_cache_id);

        let policy = BloomFilterPolicy::new(1);
        let mut builder = policy.builder();
        builder.add_entry(&RowEntry::new(
            bytes::Bytes::from_static(b"a"),
            ValueDeletable::Value(bytes::Bytes::new()),
            0,
            None,
            None,
        ));
        let filter = builder.build();
        let named = NamedFilter {
            name: BloomFilterPolicy::NAME.to_string(),
            filter,
        };
        let key = CachedKey::from((SST_ID, 1u64));

        cache_a
            .insert(
                key.clone(),
                CachedEntry::with_filters(Arc::from([named.clone()])),
            )
            .await;

        assert!(cache_a.get_filter(&key).await.unwrap().is_some());
        assert!(cache_b.get_filter(&key).await.unwrap().is_none());

        cache_b
            .insert(key.clone(), CachedEntry::with_filters(Arc::from([named])))
            .await;

        assert_eq!(2, shared_cache.entry_count());
    }

    #[tokio::test]
    async fn test_cache_wrapper_scopes_index_entries() {
        let recorder_a = MetricsRecorderHelper::noop();
        let recorder_b = MetricsRecorderHelper::noop();
        let system_clock = Arc::new(DefaultSystemClock::default());
        let shared_cache: Arc<dyn DbCache> = Arc::new(TestCache::new());
        let cache_a =
            DbCacheWrapper::new(shared_cache.clone(), &recorder_a, system_clock.clone(), 1);
        let cache_b = DbCacheWrapper::new(shared_cache.clone(), &recorder_b, system_clock, 2);

        let sst = build_test_sst(&SsTableFormat::default(), 1).await;
        let index = Arc::new(sst.index);
        let key = CachedKey::from((SST_ID, 2u64));

        cache_a
            .insert(key.clone(), CachedEntry::with_sst_index(index.clone()))
            .await;

        assert!(cache_a.get_index(&key).await.unwrap().is_some());
        assert!(cache_b.get_index(&key).await.unwrap().is_none());

        cache_b
            .insert(key.clone(), CachedEntry::with_sst_index(index))
            .await;

        assert_eq!(2, shared_cache.entry_count());
    }

    #[tokio::test]
    async fn test_cache_wrapper_scopes_block_entries() {
        let recorder_a = MetricsRecorderHelper::noop();
        let recorder_b = MetricsRecorderHelper::noop();
        let system_clock = Arc::new(DefaultSystemClock::default());
        let shared_cache: Arc<dyn DbCache> = Arc::new(TestCache::new());
        let cache_a =
            DbCacheWrapper::new(shared_cache.clone(), &recorder_a, system_clock.clone(), 1);
        let cache_b = DbCacheWrapper::new(shared_cache.clone(), &recorder_b, system_clock, 2);

        let mut builder = BlockBuilder::new_latest(4096);
        assert!(builder.add(RowEntry::new_value(b"k1", b"v1", 0)).unwrap());
        let block = Arc::new(builder.build().unwrap());
        let key = CachedKey::from((SST_ID, 3u64));

        cache_a
            .insert(key.clone(), CachedEntry::with_block(block.clone()))
            .await;

        assert!(cache_a.get_block(&key).await.unwrap().is_some());
        assert!(cache_b.get_block(&key).await.unwrap().is_none());

        cache_b
            .insert(key.clone(), CachedEntry::with_block(block))
            .await;

        assert_eq!(2, shared_cache.entry_count());
    }

    #[tokio::test]
    async fn test_cache_wrapper_flush_scope_only_affects_its_own_scope() {
        let recorder_a = MetricsRecorderHelper::noop();
        let recorder_b = MetricsRecorderHelper::noop();
        let system_clock = Arc::new(DefaultSystemClock::default());
        let shared_cache = Arc::new(TestCache::new());
        let cache_a =
            DbCacheWrapper::new(shared_cache.clone(), &recorder_a, system_clock.clone(), 1);
        let cache_b = DbCacheWrapper::new(shared_cache.clone(), &recorder_b, system_clock, 2);

        let mut builder = BlockBuilder::new_latest(4096);
        assert!(builder.add(RowEntry::new_value(b"k1", b"v1", 0)).unwrap());
        let block = Arc::new(builder.build().unwrap());
        let key = CachedKey::from((SST_ID, 4u64));

        cache_a
            .insert(key.clone(), CachedEntry::with_block(block.clone()))
            .await;
        cache_b
            .insert(key.clone(), CachedEntry::with_block(block))
            .await;
        assert_eq!(2, shared_cache.entry_count());

        cache_a.flush_to_disk().await.unwrap();

        assert!(cache_a.get_block(&key).await.unwrap().is_none());
        assert!(cache_b.get_block(&key).await.unwrap().is_some());
        assert_eq!(1, shared_cache.entry_count());
        assert_eq!(1, shared_cache.spilled_keys().len());
    }

    #[tokio::test]
    async fn test_split_cache_flush_scope_attempts_meta_cache_despite_block_cache_error() {
        let meta = Arc::new(TestCache::new());
        let split = SplitCache::new()
            .with_block_cache(Some(Arc::new(super::test_utils::FailingCache)))
            .with_meta_cache(Some(meta.clone()))
            .build();
        let key = CachedKey::from((SST_ID, 6u64));
        meta.insert(
            key.clone(),
            CachedEntry::with_sst_stats(Arc::new(crate::sst_stats::SstStats::default())),
        )
        .await;

        let result = split.flush_scope(0).await;

        assert!(result.is_err(), "block cache's error should surface");
        assert!(
            meta.spilled_keys().contains(&key),
            "meta cache's flush should still run despite the block cache's error"
        );
    }

    #[tokio::test]
    async fn test_cache_wrapper_recovers_same_scope_for_same_db_cache_id() {
        let recorder_1 = MetricsRecorderHelper::noop();
        let recorder_2 = MetricsRecorderHelper::noop();
        let system_clock = Arc::new(DefaultSystemClock::default());
        let shared_cache: Arc<dyn DbCache> = Arc::new(TestCache::new());

        // given: a wrapper (simulating a `Db` reopen passed the same caller-supplied
        // db_cache_id) populates an entry...
        let first_open =
            DbCacheWrapper::new(shared_cache.clone(), &recorder_1, system_clock.clone(), 1);
        let key = CachedKey::from((SST_ID, 5u64));
        let mut builder = BlockBuilder::new_latest(4096);
        assert!(builder.add(RowEntry::new_value(b"k1", b"v1", 0)).unwrap());
        let block = Arc::new(builder.build().unwrap());
        first_open
            .insert(key.clone(), CachedEntry::with_block(block))
            .await;
        drop(first_open); // simulates `Db::close()`

        // when: a brand-new wrapper is constructed with the *same* db_cache_id...
        let reopened = DbCacheWrapper::new(shared_cache, &recorder_2, system_clock, 1);

        // then: it recovers the same scope, so it sees the earlier instance's entry —
        // this is what lets a caller that reuses a db_cache_id across a `Db` reopen see
        // disk entries an earlier instance evacuated there before closing.
        assert!(
            reopened.get_block(&key).await.unwrap().is_some(),
            "reopening with the same db_cache_id should recover the previous instance's entries"
        );
    }

    #[fixture]
    fn cache() -> (DbCacheWrapper, Arc<DefaultMetricsRecorder>) {
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), MetricLevel::default());
        let cache = SplitCache::new()
            .with_block_cache(Some(Arc::new(TestCache::new())))
            .with_meta_cache(Some(Arc::new(TestCache::new())))
            .build();

        let wrapper = DbCacheWrapper::new(
            Arc::new(cache),
            &helper,
            Arc::new(DefaultSystemClock::default()),
            1,
        );
        (wrapper, recorder)
    }

    #[fixture]
    fn sst_format() -> SsTableFormat {
        SsTableFormat {
            block_size: 128,
            ..SsTableFormat::default()
        }
    }

    #[fixture]
    async fn sst(sst_format: SsTableFormat) -> EncodedSsTable {
        build_test_sst(&sst_format, 1).await
    }

    /// Canary for `UnownedDbCache`'s delegation: every defaulted trait method must be
    /// forwarded to the inner cache — except `close()`, which must be suppressed. If a
    /// forwarding method is dropped (e.g. when a new defaulted method is added to
    /// `DbCache`), the trait default runs against the wrapper instead of the inner
    /// cache's override and this test fails.
    #[tokio::test]
    async fn test_unowned_cache_suppresses_close_and_forwards_overrides() {
        use crate::db_cache::{CacheLoader, UnownedDbCache};
        use crate::format::block::Block;
        use std::sync::atomic::{AtomicBool, Ordering};

        fn build_block(key: &[u8]) -> Arc<Block> {
            let mut builder = BlockBuilder::new_latest(4096);
            assert!(builder.add(RowEntry::new_value(key, b"v", 0)).unwrap());
            Arc::new(builder.build().unwrap())
        }

        /// A cache whose `fetch_*` overrides always return `marker` without running the
        /// loader (like foyer's dedup-aware fetches). If the trait's default fetch runs
        /// instead, the loader's entry comes back and the marker assertion fails.
        struct ProbeCache {
            close_called: AtomicBool,
            flush_scope_called: AtomicBool,
            flush_to_disk_called: AtomicBool,
            marker: CachedEntry,
        }

        #[async_trait::async_trait]
        impl DbCache for ProbeCache {
            async fn get_block(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
                Ok(None)
            }
            async fn get_index(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
                Ok(None)
            }
            async fn get_filter(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
                Ok(None)
            }
            async fn get_stats(&self, _: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
                Ok(None)
            }
            async fn insert(&self, _: CachedKey, _: CachedEntry) {}
            async fn remove(&self, _: &CachedKey) {}
            fn entry_count(&self) -> u64 {
                0
            }
            async fn close(&self) -> Result<(), crate::Error> {
                self.close_called.store(true, Ordering::SeqCst);
                Ok(())
            }
            async fn flush_scope(&self, _: u64) -> Result<(), crate::Error> {
                self.flush_scope_called.store(true, Ordering::SeqCst);
                Ok(())
            }
            async fn flush_to_disk(&self) -> Result<(), crate::Error> {
                self.flush_to_disk_called.store(true, Ordering::SeqCst);
                Ok(())
            }
            async fn fetch_block(
                &self,
                _: CachedKey,
                _: CacheLoader,
            ) -> Result<CacheFetch, crate::Error> {
                Ok(CacheFetch::hit(self.marker.clone()))
            }
            async fn fetch_index(
                &self,
                _: CachedKey,
                _: CacheLoader,
            ) -> Result<CacheFetch, crate::Error> {
                Ok(CacheFetch::hit(self.marker.clone()))
            }
            async fn fetch_filter(
                &self,
                _: CachedKey,
                _: CacheLoader,
            ) -> Result<CacheFetch, crate::Error> {
                Ok(CacheFetch::hit(self.marker.clone()))
            }
            async fn fetch_stats(
                &self,
                _: CachedKey,
                _: CacheLoader,
            ) -> Result<CacheFetch, crate::Error> {
                Ok(CacheFetch::hit(self.marker.clone()))
            }
        }

        let marker_block = build_block(b"marker");
        let probe = Arc::new(ProbeCache {
            close_called: AtomicBool::new(false),
            flush_scope_called: AtomicBool::new(false),
            flush_to_disk_called: AtomicBool::new(false),
            marker: CachedEntry::with_block(marker_block.clone()),
        });
        let unowned = UnownedDbCache::new(probe.clone());

        let loader = || -> CacheLoader {
            Box::new(|| Box::pin(async { Ok(CachedEntry::with_block(build_block(b"loader"))) }))
        };
        let key = || CachedKey::from((SST_ID, 0u64));

        let fetched = [
            unowned.fetch_block(key(), loader()).await.unwrap(),
            unowned.fetch_index(key(), loader()).await.unwrap(),
            unowned.fetch_filter(key(), loader()).await.unwrap(),
            unowned.fetch_stats(key(), loader()).await.unwrap(),
        ];
        for entry in fetched {
            assert!(
                Arc::ptr_eq(&entry.entry.block().unwrap(), &marker_block),
                "fetch was not forwarded to the inner cache's override"
            );
        }

        unowned.close().await.unwrap();
        assert!(
            !probe.close_called.load(Ordering::SeqCst),
            "close() must not propagate to a cache slatedb does not own"
        );

        unowned.flush_scope(0).await.unwrap();
        assert!(
            probe.flush_scope_called.load(Ordering::SeqCst),
            "flush_scope was not forwarded to the inner cache"
        );

        unowned.flush_to_disk().await.unwrap();
        assert!(
            probe.flush_to_disk_called.load(Ordering::SeqCst),
            "flush_to_disk was not forwarded to the inner cache"
        );
    }
}
