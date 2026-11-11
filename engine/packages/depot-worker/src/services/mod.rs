//! Depot service entry points.

pub mod worker;

pub mod log_shard;

pub mod tablet;

pub mod coordinator;

pub mod group_events;

pub mod heartbeat_gc;

pub mod storage;

pub mod readiness;

mod stop_readiness;

pub mod stopping;
