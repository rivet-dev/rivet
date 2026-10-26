use slatedb_common::metrics::{CounterFn, GaugeFn, MetricsRecorderHelper};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

macro_rules! oscache_stat_name {
    ($suffix:expr) => {
        concat!("slatedb.object_store_cache.", $suffix)
    };
}

pub const PART_HIT_COUNT: &str = oscache_stat_name!("part_hit_count");
pub const PART_ACCESS_COUNT: &str = oscache_stat_name!("part_access_count");
pub const CACHE_KEYS: &str = oscache_stat_name!("cache_keys");
pub const CACHE_BYTES: &str = oscache_stat_name!("cache_bytes");
pub const EVICTED_KEYS: &str = oscache_stat_name!("evicted_keys");
pub const EVICTED_BYTES: &str = oscache_stat_name!("evicted_bytes");

#[derive(Clone)]
pub struct CachedObjectStoreStats {
    pub(super) object_store_cache_part_hits: Arc<dyn CounterFn>,
    pub(super) object_store_cache_part_access: Arc<dyn CounterFn>,
    pub(super) object_store_cache_keys: Arc<dyn GaugeFn>,
    pub(super) object_store_cache_bytes: Arc<dyn GaugeFn>,
    pub(super) object_store_cache_evicted_keys: Arc<dyn CounterFn>,
    pub(super) object_store_cache_evicted_bytes: Arc<dyn CounterFn>,
}

impl Debug for CachedObjectStoreStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedObjectStoreStats")
            .field("object_store_cache_part_hits", &"<counter>")
            .field("object_store_cache_part_access", &"<counter>")
            .field("object_store_cache_keys", &"<gauge>")
            .field("object_store_cache_bytes", &"<gauge>")
            .field("object_store_cache_evicted_keys", &"<counter>")
            .field("object_store_cache_evicted_bytes", &"<counter>")
            .finish()
    }
}

impl CachedObjectStoreStats {
    pub(crate) fn new(recorder: &MetricsRecorderHelper) -> Self {
        Self {
            object_store_cache_part_hits: recorder.counter(PART_HIT_COUNT).register(),
            object_store_cache_part_access: recorder.counter(PART_ACCESS_COUNT).register(),
            object_store_cache_keys: recorder.gauge(CACHE_KEYS).register(),
            object_store_cache_bytes: recorder.gauge(CACHE_BYTES).register(),
            object_store_cache_evicted_keys: recorder.counter(EVICTED_KEYS).register(),
            object_store_cache_evicted_bytes: recorder.counter(EVICTED_BYTES).register(),
        }
    }
}
