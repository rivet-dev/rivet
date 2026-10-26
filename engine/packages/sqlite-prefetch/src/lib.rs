//! SQLite page prefetch planning for the actor page protocol server.
//!
//! One library owns every SQLite read optimization: the contiguous read-ahead window, scan
//! detection from interior B-tree pages, the early and frequent recent pages, and the overflow
//! map of each database's chains. It runs in the page protocol server inside pegboard-envoy and
//! is the only code, client or server, that prefetches SQLite pages. It has no I/O: the server
//! feeds it events and acts on the plans it returns, through [`driver`], which the testing VFS
//! drives the same way.
//!
//! Page access traces are `tracing` events at the target `sqlite_prefetch::trace`, enabled with
//! `RUST_LOG=sqlite_prefetch::trace=trace`. They carry page numbers, kinds, sizes and timings
//! only, never page bytes, SQL or keys.
mod bitset;
pub mod driver;
mod metrics;
pub mod overflow;
pub mod page;
mod payloads;
pub mod prefetcher;
pub mod recency;
pub mod scan;
pub mod window;

#[cfg(feature = "harness")]
pub mod harness;

pub use overflow::commit::{CommitBuilder, KvOp, Sidecar};
pub use overflow::{Lookup, LookupResult};
pub use prefetcher::{CommitPlan, Config, OpenRows, Plan, Prefetcher, Source};

/// The `tracing` target of page access traces.
pub const TRACE_TARGET: &str = "sqlite_prefetch::trace";
