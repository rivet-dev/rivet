#[path = "integration/common/mod.rs"]
mod common;

#[path = "integration/counter.rs"]
mod counter;

#[path = "integration/metrics_endpoint.rs"]
mod metrics_endpoint;

#[path = "integration/sqlite_corruption_fuzz.rs"]
mod sqlite_corruption_fuzz;

#[path = "migration/v2_2_1/mod.rs"]
mod migration_v2_2_1;
