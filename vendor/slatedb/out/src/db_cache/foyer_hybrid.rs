//! # Foyer Hybrid Cache
//!
//! This module provides an implementation of a hybrid (in-memory + on-disk) cache using the Foyer
//! library. The cache is designed to store and retrieve cached blocks, indexes, and filters
//! associated with SSTable IDs.
//!
//! ## Features
//!
//! - **Hybrid Cache**: Caches blocks using a tiered cache that stores data across memory and
//!   local disk.
//! - **Custom Weigher**: Implements a custom weigher to account for the size of cached blocks.
//! - **Flexible Configuration**: The HybridCache instance is passed to the cache, so you are free
//!   to configure it as needed for your use case.
//!
//! ## Notes
//! Foyer HybridCache manages its own memory and disk cache, so when it coexists with `CachedObjectStore`
//! there may be some duplication of cached data. In other words, disk cache may suffer from write
//! amplification. If you don't want to suffer from write amplification, then we recommend that you do not
//! coexist Foyer HybridCache with `CachedObjectStore`.
//!
//! Benefits of coexistence:
//! `CachedObjectStore` provides prefetching, which may speed up your read performance and
//! reduce unnecessary object store accesses.
//!
//! Drawbacks of coexistence:
//! Disk may suffer from write amplification. Data will first be written to `CachedObjectStore`, then
//! flow into the memory part of Foyer HybridCache, and finally written back to the disk area managed
//! by Foyer HybridCache itself. slateDB cannot control Foyer HybridCache to write data to its disk area.
//!
//! ## Examples
//!
//! ```
//! use foyer::{
//!     BlockEngineConfig, DeviceBuilder, FsDeviceBuilder, HybridCacheBuilder, PsyncIoEngineConfig,
//! };
//! use slatedb::Db;
//! use slatedb::db_cache::CachedEntry;
//! use slatedb::db_cache::foyer_hybrid::FoyerHybridCache;
//! use slatedb::object_store::memory::InMemory;
//! use std::sync::Arc;
//!
//! #[::tokio::main]
//! async fn main() {
//!     let object_store = Arc::new(InMemory::new());
//!     let cache = HybridCacheBuilder::new()
//!         .with_name("hybrid_cache")
//!         .memory(1024 * 1024)
//!         .with_weighter(|_, v: &CachedEntry| v.size())
//!         .storage()
//!         .with_io_engine_config(PsyncIoEngineConfig::new())
//!         .with_engine_config(
//!             BlockEngineConfig::new(
//!                 FsDeviceBuilder::new("/tmp/slatedb-cache")
//!                     .with_capacity(1024 * 1024)
//!                     .build()
//!                     .unwrap(),
//!             )
//!             .with_block_size(64 * 1024),
//!         )
//!         .build()
//!         .await
//!         .unwrap();
//!     let cache = Arc::new(FoyerHybridCache::new_with_cache(cache));
//!     let db = Db::builder("path/to/db", object_store)
//!         .with_db_cache(cache, 0)
//!         .build()
//!         .await
//!         .unwrap();
//! }
//! ```
//!

use crate::{
    db_cache::{
        instrumented_loader, CacheFetch, CacheLoader, CacheLookup, CachedEntry, CachedKey, DbCache,
    },
    error::SlateDBError,
    utils::format_bytes_si,
};
use async_trait::async_trait;
use log::info;
use std::sync::Arc;

pub struct FoyerHybridCache {
    inner: foyer::HybridCache<CachedKey, CachedEntry>,
}

impl FoyerHybridCache {
    pub fn new_with_cache(cache: foyer::HybridCache<CachedKey, CachedEntry>) -> Self {
        Self { inner: cache }
    }
}

impl FoyerHybridCache {
    async fn get(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        self.inner
            .get(key)
            .await
            .map_err(|e| SlateDBError::from(e).into())
            .map(|maybe_v| maybe_v.map(|v| v.value().clone()))
    }
}

#[async_trait]
impl DbCache for FoyerHybridCache {
    async fn get_block(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        self.get(key).await
    }

    async fn get_index(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        self.get(key).await
    }

    async fn get_filter(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        self.get(key).await
    }

    async fn get_stats(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        self.get(key).await
    }

    async fn insert(&self, key: CachedKey, value: CachedEntry) {
        self.inner.insert(key, value);
    }

    async fn remove(&self, key: &CachedKey) {
        self.inner.remove(key);
    }

    fn entry_count(&self) -> u64 {
        // foyer cache doesn't support an entry count estimate
        0
    }

    async fn close(&self) -> Result<(), crate::Error> {
        let memory_bytes = self.inner.memory().usage();
        info!(
            "foyer hybrid cache: closing \
             (flushing {} from memory to disk)",
            format_bytes_si(memory_bytes as u64)
        );
        let result = self
            .inner
            .close()
            .await
            .map_err(|e| crate::Error::from(SlateDBError::from(e)));
        match &result {
            Ok(()) => info!("foyer hybrid cache: closed successfully"),
            Err(e) => info!("foyer hybrid cache: close failed [error={e:?}]"),
        }
        result
    }

    /// Send every entry for this scope to disk and wait for the write to finish.
    ///
    /// A no-op if this cache has no disk tier: `flush_if` removes matching
    /// entries from memory even then, which would lose data instead of moving
    /// it. `HybridCache::is_hybrid` cannot tell us this (it misreports `true`
    /// for a no-op disk tier as of foyer 0.22.4), so check the device
    /// capacity instead.
    async fn flush_scope(&self, db_cache_id: u64) -> Result<(), crate::Error> {
        if self.inner.storage().device().capacity() == 0 {
            return Ok(());
        }
        self.inner
            .flush_if(|k, _v| k.db_cache_id == db_cache_id)
            .await;
        self.inner.storage().wait().await;
        Ok(())
    }

    async fn fetch_block(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        self.dedup_fetch(key, loader).await
    }

    async fn fetch_index(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        self.dedup_fetch(key, loader).await
    }

    async fn fetch_filter(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        self.dedup_fetch(key, loader).await
    }

    async fn fetch_stats(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        self.dedup_fetch(key, loader).await
    }
}

impl FoyerHybridCache {
    /// Use foyer's `HybridCache::get_or_fetch`, which deduplicates concurrent loads across
    /// memory + disk tiers. See [`crate::db_cache::foyer::FoyerCache::dedup_fetch`] for the
    /// error-handling rationale.
    async fn dedup_fetch(
        &self,
        key: CachedKey,
        loader: CacheLoader,
    ) -> Result<CacheFetch, crate::Error> {
        let (loader, loader_ran) = instrumented_loader(loader);
        let fetch = self
            .inner
            .get_or_fetch(&key, move || async move { loader().await });
        match fetch.await {
            Ok(entry) => Ok(CacheFetch {
                entry: entry.value().clone(),
                lookup: if loader_ran.was_called() {
                    CacheLookup::Miss
                } else {
                    CacheLookup::Hit
                },
            }),
            Err(err) => Err(SlateDBError::FoyerError(Arc::new(err)).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db_cache::foyer_hybrid::FoyerHybridCache;
    use crate::db_cache::{CachedEntry, CachedKey, DbCache};
    use crate::db_state::SsTableId;
    use crate::format::sst::BlockBuilder;
    use foyer::{
        BlockEngineConfig, DeviceBuilder, FsDeviceBuilder, HybridCacheBuilder, PsyncIoEngineConfig,
    };
    use rand::RngCore;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tempfile::{tempdir, TempDir};
    use ulid::Ulid;

    const SST_ID: SsTableId = SsTableId::new(Ulid::from_parts(123, 0));

    #[tokio::test]
    async fn test_hybrid_cache() {
        let (cache, _dir) = setup().await;
        let mut items = HashMap::new();
        for b in 0u64..256 {
            let k = CachedKey::from((SST_ID, b));
            let v = build_block();
            cache.insert(k.clone(), v.clone()).await;
            items.insert(k, v);
        }
        let mut found = 0;
        let mut notfound = 0;
        for (k, v) in items {
            let cached_v = cache.get_block(&k).await.unwrap();
            if let Some(cached_v) = cached_v {
                assert!(v.block().unwrap().as_ref() == cached_v.block().unwrap().as_ref());
                found += 1;
            } else {
                notfound += 1;
            }
        }
        println!("found: {}, notfound: {}", found, notfound);
        assert!(found > 0);
    }

    fn build_block() -> CachedEntry {
        let mut rng = rand::rng();
        let mut builder = BlockBuilder::new_latest(1024);
        loop {
            let mut k = vec![0u8; 32];
            rng.fill_bytes(&mut k);
            let mut v = vec![0u8; 128];
            rng.fill_bytes(&mut v);
            if builder.add_value(&k, &v, None, None) {
                break;
            }
        }
        let block = Arc::new(builder.build().unwrap());
        CachedEntry::with_block(block)
    }

    async fn setup() -> (FoyerHybridCache, TempDir) {
        let tempdir = tempdir().unwrap();
        let cache = open_cache(tempdir.path()).await;
        (cache, tempdir)
    }

    #[tokio::test]
    async fn test_fetch_lookup_from_memory_and_disk() {
        use crate::db_cache::{CacheLoader, CacheLookup};

        let (cache, _tempdir) = setup().await;
        let key = CachedKey::from((SsTableId::new(Ulid::new()), 0));
        let loader = || -> CacheLoader { Box::new(|| Box::pin(async { Ok(build_block()) })) };
        let first = cache.fetch_block(key.clone(), loader()).await.unwrap();
        assert_eq!(first.lookup, CacheLookup::Miss);
        let second = cache.fetch_block(key.clone(), loader()).await.unwrap();
        assert_eq!(second.lookup, CacheLookup::Hit);
        cache.flush_scope(0).await.unwrap();
        assert!(cache.inner.memory().get(&key).is_none());
        let third = cache.fetch_block(key, loader()).await.unwrap();
        assert_eq!(third.lookup, CacheLookup::Hit);
        assert_eq!(third.entry.size(), first.entry.size());
        cache.close().await.unwrap();
    }

    async fn open_cache(path: &std::path::Path) -> FoyerHybridCache {
        let cache = HybridCacheBuilder::new()
            .with_name("hybrid_cache_test")
            .memory(1024 * 1024)
            .with_weighter(|_, v: &CachedEntry| v.size())
            .storage()
            .with_io_engine_config(PsyncIoEngineConfig::new())
            .with_engine_config(
                BlockEngineConfig::new(
                    FsDeviceBuilder::new(path)
                        .with_capacity(4 * 1024 * 1024)
                        .build()
                        .unwrap(),
                )
                .with_block_size(64 * 1024),
            )
            .build()
            .await
            .unwrap();
        FoyerHybridCache::new_with_cache(cache)
    }

    #[tokio::test]
    async fn should_persist_blocks_to_disk_on_close() {
        // given: a cache with entries
        let dir = tempdir().unwrap();
        let cache = open_cache(dir.path()).await;
        let mut keys = Vec::new();
        for b in 0u64..64 {
            let k = CachedKey::from((SST_ID, b));
            cache.insert(k.clone(), build_block()).await;
            keys.push(k);
        }

        // when: close (flush to disk) and reopen
        cache.close().await.unwrap();
        drop(cache);
        let cache = open_cache(dir.path()).await;

        // then: all entries should be recoverable from disk
        let mut found = 0;
        for k in &keys {
            if cache.get_block(k).await.unwrap().is_some() {
                found += 1;
            }
        }
        assert_eq!(found, keys.len(), "all entries should survive close+reopen");
    }

    #[tokio::test]
    async fn should_move_resident_key_from_memory_to_disk_on_flush_scope() {
        let dir = tempdir().unwrap();
        let cache = open_cache(dir.path()).await;
        let key = CachedKey::from((SST_ID, 1u64));
        cache.insert(key.clone(), build_block()).await;
        assert!(
            cache.inner.memory().get(&key).is_some(),
            "entry should be resident in memory right after insert"
        );

        cache.flush_scope(key.db_cache_id).await.unwrap();

        assert!(
            cache.inner.memory().get(&key).is_none(),
            "entry should have been evicted from the memory tier"
        );
        let loaded = cache.inner.storage().load(&key).await.unwrap();
        assert!(
            loaded.entry().is_some(),
            "entry should have been flushed to the disk tier"
        );
    }

    #[tokio::test]
    async fn should_noop_flush_scope_when_nothing_matches() {
        let dir = tempdir().unwrap();
        let cache = open_cache(dir.path()).await;
        let key = CachedKey::from((SST_ID, 1u64));

        cache.flush_scope(key.db_cache_id).await.unwrap();

        assert!(cache.inner.memory().get(&key).is_none());
        let loaded = cache.inner.storage().load(&key).await.unwrap();
        assert!(
            loaded.is_miss(),
            "flushing an empty scope must not create a disk entry"
        );
    }

    #[tokio::test]
    async fn should_keep_entry_in_memory_when_flush_scope_has_no_disk_tier() {
        let cache = HybridCacheBuilder::new()
            .memory(1024 * 1024)
            .with_weighter(|_, v: &CachedEntry| v.size())
            .storage()
            .build()
            .await
            .unwrap();
        assert_eq!(
            cache.storage().device().capacity(),
            0,
            "a cache built with no engine config has no disk tier"
        );
        let cache = FoyerHybridCache::new_with_cache(cache);
        let key = CachedKey::from((SST_ID, 1u64));
        cache.insert(key.clone(), build_block()).await;

        cache.flush_scope(key.db_cache_id).await.unwrap();

        assert!(
            cache.inner.memory().get(&key).is_some(),
            "entry must stay in memory: there is no disk tier to receive it"
        );
    }

    #[tokio::test]
    async fn should_leave_other_scopes_untouched_when_flushing_one_scope() {
        let dir = tempdir().unwrap();
        let cache = open_cache(dir.path()).await;
        let flushed_key = CachedKey {
            db_cache_id: 1,
            ..CachedKey::from((SST_ID, 2u64))
        };
        let kept_key = CachedKey {
            db_cache_id: 2,
            ..CachedKey::from((SST_ID, 2u64))
        };
        cache.insert(flushed_key.clone(), build_block()).await;
        cache.insert(kept_key.clone(), build_block()).await;

        cache.flush_scope(flushed_key.db_cache_id).await.unwrap();

        assert!(
            cache.inner.memory().get(&kept_key).is_some(),
            "other scope's key should remain resident in memory"
        );
        let loaded = cache.inner.storage().load(&kept_key).await.unwrap();
        assert!(
            loaded.is_miss(),
            "other scope's key should not have been flushed to disk"
        );
    }
}
