//! Compaction filter API for custom entry filtering during compaction.
//!
//! This module provides traits for implementing custom compaction filters that can
//! inspect, drop, convert to tombstone, or modify entries during the compaction process.
//!
//! **Warning:** When compaction filters are configured, snapshot consistency may be
//! affected. Filters may modify or drop entries that active snapshots expect to see,
//! causing snapshot reads to return unexpected results. Users who need consistent
//! snapshots should carefully consider their filter logic.
//!
//! # Example
//!
//! ```no_run
//! use async_trait::async_trait;
//! use bytes::Bytes;
//! use slatedb::{
//!     CompactionFilter, CompactionFilterSupplier, CompactionJobContext,
//!     CompactionFilterDecision, CompactionFilterError, RowEntry, ValueDeletable,
//! };
//!
//! /// A filter that converts all entries with a specific key prefix to tombstones.
//! struct PrefixTombstoneFilter {
//!     prefix: Bytes,
//!     tombstone_count: u64,
//! }
//!
//! #[async_trait]
//! impl CompactionFilter for PrefixTombstoneFilter {
//!     async fn filter(&mut self, entry: &RowEntry) -> Result<CompactionFilterDecision, CompactionFilterError> {
//!         if entry.key.starts_with(&self.prefix) {
//!             self.tombstone_count += 1;
//!             Ok(CompactionFilterDecision::Modify(ValueDeletable::Tombstone))
//!         } else {
//!             Ok(CompactionFilterDecision::Keep)
//!         }
//!     }
//!
//!     async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
//!         println!(
//!             "Compaction converted {} entries with prefix {:?} to tombstones",
//!             self.tombstone_count,
//!             self.prefix
//!         );
//!         Ok(())
//!     }
//! }
//!
//! struct PrefixTombstoneFilterSupplier {
//!     prefix: Bytes,
//! }
//!
//! #[async_trait]
//! impl CompactionFilterSupplier for PrefixTombstoneFilterSupplier {
//!     async fn create_compaction_filter(
//!         &self,
//!         _context: &CompactionJobContext,
//!     ) -> Result<Box<dyn CompactionFilter>, CompactionFilterError> {
//!         Ok(Box::new(PrefixTombstoneFilter {
//!             prefix: self.prefix.clone(),
//!             tombstone_count: 0,
//!         }))
//!     }
//! }
//!
//! // Then pass the supplier to Db::builder:
//! // db.builder("mydb", object_store)
//! //     .with_compaction_filter_supplier(Arc::new(supplier))
//! //     .build()
//! //     .await
//! ```

use crate::types::{RowEntry, ValueDeletable};
use async_trait::async_trait;
use thiserror::Error;

/// Context information about a compaction job.
///
/// This struct provides read-only information about the current compaction job
/// to help filters make informed decisions.
#[derive(Debug, Clone)]
pub struct CompactionJobContext {
    /// The destination sorted run ID for this compaction.
    pub destination: u32,
    /// Whether the destination sorted run is the last (oldest) run after compaction.
    pub is_dest_last_run: bool,
    /// Present only when this job covers every SST in its segment at the
    /// manifest's durable sequence barrier. Later writes are outside this proof.
    /// Logical version filters must not infer full history from bottom level alone.
    pub complete_input_through: Option<u64>,
    /// The clock tick representing the time the compaction occurs.
    /// This is used to make decisions about retention of expiring records.
    pub compaction_clock_tick: i64,
    /// Optional minimum sequence number to retain.
    ///
    /// Entries with sequence numbers at or above this threshold are protected by
    /// active snapshots. Dropping or modifying such entries may cause snapshot
    /// reads to return inconsistent results.
    pub retention_min_seq: Option<u64>,
}

/// Decision returned by a compaction filter for each entry.
#[derive(Debug, Clone, PartialEq)]
pub enum CompactionFilterDecision {
    /// Keep the entry unchanged.
    Keep,
    /// Drop the entry entirely. The entry will not appear in the compaction output.
    ///
    /// WARNING: Dropping an entry removes it completely without leaving a tombstone.
    /// This means older versions of the same key in lower levels of the LSM tree
    /// may become visible again ("resurrection"). Only use Drop when this behavior is
    /// acceptable for your use case.
    ///
    /// NOTE: For merge operand entries (`ValueDeletable::Merge`), prefer `Drop` over
    /// `Modify(ValueDeletable::Tombstone)`. A tombstone would delete not just the
    /// current merge operand but also any previous merge operands for the same key.
    /// Dropping the merge operand simply removes it from the merge chain without
    /// affecting others.
    Drop,
    /// Replace the entry's value with a new [`ValueDeletable`].
    ///
    /// The key, sequence number, and `create_ts` remain unchanged.
    ///
    /// ## Behavior by value type:
    ///
    /// - `Modify(ValueDeletable::Value(bytes))`: Replaces the value with new bytes.
    ///   The `expire_ts` is preserved from the original entry.
    ///
    /// - `Modify(ValueDeletable::Tombstone)`: Converts the entry to a tombstone.
    ///   The `expire_ts` is cleared (set to `None`) since tombstones should not expire.
    ///   Use this instead of `Drop` when you need to shadow older versions of the
    ///   same key in older sorted runs.
    ///
    /// - `Modify(ValueDeletable::Merge(bytes))`: Replaces the value with a merge operand.
    ///   The key, sequence number, `create_ts`, and `expire_ts` are preserved from the
    ///   original entry.
    Modify(ValueDeletable),
}

/// Errors that can occur during compaction filter operations.
#[derive(Debug, Error)]
pub enum CompactionFilterError {
    /// Filter creation failed in `create_compaction_filter`. This aborts the
    /// compaction.
    #[error("filter creation failed: {0}")]
    CreationError(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Filter failed while processing an entry. This aborts the compaction.
    #[error("filter error: {0}")]
    FilterError(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Filter failed during `on_compaction_end`. This aborts the compaction.
    #[error("compaction end error: {0}")]
    CompactionEndError(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Filter that processes entries during compaction.
///
/// Each filter instance is created for a single compaction job and executes
/// single-threaded on the compactor thread.
///
/// # Performance
///
/// The `filter()` method is called for every entry during compaction. While it
/// is async to allow I/O operations, frequent I/O will impact compaction throughput.
///
/// If your filter requires expensive computation, configure a dedicated
/// compaction runtime using [`crate::DbBuilder::with_compaction_runtime`] to
/// prevent blocking your application's main runtime.
///
/// # Snapshot Consistency Warning
///
/// When compaction filters are configured, snapshot consistency may be affected.
/// Filters may modify or drop entries that active snapshots expect to see, causing
/// snapshot reads to return unexpected results. Users who need consistent snapshots
/// should carefully consider their filter logic.
#[async_trait]
pub trait CompactionFilter: Send + Sync {
    /// Filter a single entry.
    ///
    /// This method is async to allow I/O operations during filtering.
    /// Return `Err` to abort compaction with the error.
    async fn filter(
        &mut self,
        entry: &RowEntry,
    ) -> Result<CompactionFilterDecision, CompactionFilterError>;

    /// Called after successfully processing all entries.
    ///
    /// Use this hook to flush state, log statistics, or clean up resources.
    /// Return `Err` to abort compaction with the error.
    ///
    /// Note: This is only called after all entries have been processed. If compaction
    /// fails due to an earlier error, this method is not invoked.
    async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError>;
}

/// Factory that creates a [`CompactionFilter`] instance per compaction job.
///
/// The supplier is shared across all compactions and must be thread-safe (`Send + Sync`).
/// It creates a new filter instance for each compaction job, providing isolated state per job.
#[async_trait]
pub trait CompactionFilterSupplier: Send + Sync {
    /// Whether filter decisions depend on earlier, different keys in the input.
    /// Such filters run in one subcompaction and replay the skipped input prefix
    /// into a fresh filter when resuming, before emitting any new output. This
    /// prevents a restart or subcompaction boundary from losing group history.
    fn requires_full_input(&self) -> bool {
        false
    }

    /// Creates a filter for a compaction job. Return Err to abort compaction.
    ///
    /// This method is called each time a compaction job starts or resumes. If a compaction
    /// is interrupted (e.g., compactor restarts) and later resumed, this method will be
    /// called again with a new filter instance. By default, entries already written
    /// to output SSTs are skipped. A filter requiring full input instead observes
    /// the skipped prefix again to reconstruct its history before output resumes.
    ///
    /// This is async to allow I/O during initialization (loading config,
    /// connecting to external services, etc.) before the filter processes entries.
    async fn create_compaction_filter(
        &self,
        context: &CompactionJobContext,
    ) -> Result<Box<dyn CompactionFilter>, CompactionFilterError>;
}
