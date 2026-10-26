use slatedb_common::metrics::{CounterFn, GaugeFn, MetricsRecorderHelper};
use std::sync::Arc;

macro_rules! gc_stat_name {
    ($suffix:expr) => {
        concat!("slatedb.gc.", $suffix)
    };
}

pub const DELETED_COUNT: &str = gc_stat_name!("deleted_count");
pub const GC_COUNT: &str = gc_stat_name!("count");
pub const VERSION_COUNT: &str = gc_stat_name!("version_count");

/// Stats for the garbage collector.
pub struct GcStats {
    pub gc_manifest_count: Arc<dyn CounterFn>,
    pub gc_wal_count: Arc<dyn CounterFn>,
    pub gc_wal_fence_count: Arc<dyn CounterFn>,
    pub gc_compacted_count: Arc<dyn CounterFn>,
    pub gc_compactions_count: Arc<dyn CounterFn>,
    pub gc_detach_count: Arc<dyn CounterFn>,
    pub gc_count: Arc<dyn CounterFn>,
    pub gc_manifest_versions: Arc<dyn GaugeFn>,
    pub gc_compactions_versions: Arc<dyn GaugeFn>,
}

impl GcStats {
    pub(crate) fn new(recorder: &MetricsRecorderHelper) -> Self {
        Self {
            gc_manifest_count: recorder
                .counter(DELETED_COUNT)
                .labels(&[("resource", "manifest")])
                .register(),
            gc_wal_count: recorder
                .counter(DELETED_COUNT)
                .labels(&[("resource", "wal")])
                .register(),
            gc_wal_fence_count: recorder
                .counter(DELETED_COUNT)
                .labels(&[("resource", "wal_fence")])
                .register(),
            gc_compacted_count: recorder
                .counter(DELETED_COUNT)
                .labels(&[("resource", "compacted")])
                .register(),
            gc_compactions_count: recorder
                .counter(DELETED_COUNT)
                .labels(&[("resource", "compactions")])
                .register(),
            gc_detach_count: recorder
                .counter(DELETED_COUNT)
                .labels(&[("resource", "detach")])
                .register(),
            gc_count: recorder.counter(GC_COUNT).register(),
            gc_manifest_versions: recorder
                .gauge(VERSION_COUNT)
                .labels(&[("resource", "manifest")])
                .register(),
            gc_compactions_versions: recorder
                .gauge(VERSION_COUNT)
                .labels(&[("resource", "compactions")])
                .register(),
        }
    }
}
