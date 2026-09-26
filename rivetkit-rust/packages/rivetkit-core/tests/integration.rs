#[path = "integration/common/mod.rs"]
mod common;

#[path = "integration/counter.rs"]
mod counter;

#[path = "integration/metrics.rs"]
mod metrics;

#[path = "integration/sqlite_corruption_fuzz.rs"]
mod sqlite_corruption_fuzz;

#[path = "integration/tokio_runtime_metrics.rs"]
mod tokio_runtime_metrics;

#[path = "migration/v2_2_1/mod.rs"]
mod migration_v2_2_1;
