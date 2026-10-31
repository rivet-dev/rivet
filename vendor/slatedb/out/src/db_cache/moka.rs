//! # Moka Cache
//!
//! This module provides an implementation of an in-memory cache using the Moka library.
//! The cache is designed to store and retrieve cached blocks, indexes, and filters
//! associated with SSTable IDs.
//!
//! ## Features
//!
//! - **Asynchronous Operations**: Utilizes Moka's `future::Cache` to perform cache operations asynchronously.
//! - **Custom Weigher**: Implements a custom weigher to account for the size of cached blocks.
//! - **Flexible Configuration**: Allows customization of cache parameters such as maximum capacity, TTL, and TTI.
//!
//! ## Examples
//!
//! ```
//! use slatedb::{Db, Error};
//! use slatedb::db_cache::moka::MokaCache;
//! use slatedb::object_store::memory::InMemory;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let object_store = Arc::new(InMemory::new());
//!     let db = Db::builder("test_db", object_store)
//!         .with_db_cache(Arc::new(MokaCache::new()), 0)
//!         .build()
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
use crate::db_cache::{CachedEntry, CachedKey, DbCache, DEFAULT_MAX_CAPACITY};
use async_trait::async_trait;
use std::time::Duration;

/// The options for the Moka cache.
#[derive(Clone, Copy, Debug)]
pub struct MokaCacheOptions {
    pub max_capacity: u64,
    pub time_to_live: Option<Duration>,
    pub time_to_idle: Option<Duration>,
}

impl Default for MokaCacheOptions {
    fn default() -> Self {
        Self {
            max_capacity: DEFAULT_MAX_CAPACITY,
            time_to_live: None,
            time_to_idle: None,
        }
    }
}

/// A cache implementation using the Moka library.
///
/// This struct wraps a Moka cache, providing an in-memory caching solution
/// for storing and retrieving cached blocks associated with SSTable IDs.
///
/// # Fields
///
/// * `inner` - The underlying Moka cache instance, which maps `CachedKey`
///   keys to `CachedEntry` values.
///
/// # Notes
///
/// The cache is configured based on the provided `MokaCacheOptions`,
/// including settings for capacity, time-to-live (TTL), and time-to-idle (TTI).
/// It uses a custom weigher to account for the size of cached blocks.
pub struct MokaCache {
    inner: moka::future::Cache<CachedKey, CachedEntry>,
}

impl MokaCache {
    pub fn new() -> Self {
        Self::new_with_opts(MokaCacheOptions::default())
    }

    pub fn new_with_opts(options: MokaCacheOptions) -> Self {
        let mut builder = moka::future::Cache::builder()
            .weigher(|_, v: &CachedEntry| v.size() as u32)
            .max_capacity(options.max_capacity);

        // TODO: We need to use a SystemClock for DST support
        // Moka does not currently allow us to inject a clock.
        if let Some(ttl) = options.time_to_live {
            builder = builder.time_to_live(ttl);
        }

        // TODO: We need to use a SystemClock for DST support
        // Moka does not currently allow us to inject a clock.
        if let Some(tti) = options.time_to_idle {
            builder = builder.time_to_idle(tti);
        }

        let cache = builder.build();

        Self { inner: cache }
    }
}

impl Default for MokaCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DbCache for MokaCache {
    fn estimated_memory_usage(&self) -> Option<u64> { Some(self.inner.weighted_size()) }

    async fn get_block(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        Ok(self.inner.get(key).await)
    }

    async fn get_index(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        Ok(self.inner.get(key).await)
    }

    async fn get_filter(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        Ok(self.inner.get(key).await)
    }

    async fn get_stats(&self, key: &CachedKey) -> Result<Option<CachedEntry>, crate::Error> {
        Ok(self.inner.get(key).await)
    }

    async fn insert(&self, key: CachedKey, value: CachedEntry) {
        self.inner.insert(key, value).await;
    }

    async fn remove(&self, key: &CachedKey) {
        self.inner.remove(key).await;
    }

    fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }
}
