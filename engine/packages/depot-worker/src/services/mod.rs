//! Depot service entry points.

pub mod worker;

pub mod log_shard;

pub mod tablet;

pub mod coordinator;

pub mod heartbeat_gc;

pub mod storage;

mod stop_readiness;

pub mod stopping;
