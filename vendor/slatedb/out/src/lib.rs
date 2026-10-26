#![cfg_attr(test, allow(clippy::unwrap_used))]
#![warn(clippy::panic)]
#![cfg_attr(test, allow(clippy::panic))]
#![allow(clippy::result_large_err, clippy::too_many_arguments)]
// Disallow non-approved non-deterministic types and functions in production code
#![deny(clippy::disallowed_types, clippy::disallowed_methods)]
#![cfg_attr(
    test,
    allow(
        clippy::disallowed_macros,
        clippy::disallowed_types,
        clippy::disallowed_methods
    )
)]

/// Re-export the bytes crate.
///
/// This is useful for users of the crate who want to use SlateDB
/// without having to depend on the bytes crate directly.
pub use bytes;

/// Re-export the fail-parallel crate.
///
/// This is useful for users of the crate who want to use SlateDB
/// with failpoints in their tests without having to depend on the
/// fail-parallel crate directly.
pub use fail_parallel;

/// Re-export the object store crate.
///
/// This is useful for users of the crate who want to use SlateDB
/// without having to depend on the object store crate directly.
pub use object_store;

pub use batch::WriteBatch;
pub use block_cache_policy::BlockCachePolicy;
pub use bytes_range::ByteRangeBounds;
pub use cached_object_store::stats as cached_object_store_stats;
pub use checkpoint::{Checkpoint, CheckpointCreateResult};
#[cfg(feature = "compaction_filters")]
pub use compaction_filter::{
    CompactionFilter, CompactionFilterDecision, CompactionFilterError, CompactionFilterSupplier,
    CompactionJobContext,
};
pub use compaction_worker::CompactionWorker;
pub use compactor::CompactorBuilder;
pub use compactor_state::VersionedCompactions;
pub use config::{Settings, SstBlockSize};
pub use db::builder::{CloneSourceSpec, CompactionWorkerBuilder};
pub use db::{Db, DbBuilder, DbReaderBuilder, DbStatus, SegmentPrefix, WriteHandle};
pub use db_cache::stats as db_cache_stats;
pub use db_cache::CacheTarget;
pub use db_iter::{DbIterator, DbRecencyIterator};
pub use db_reader::{DbReader, DbReaderMode};
pub use db_snapshot::DbSnapshot;
pub use db_transaction::DbTransaction;
pub use error::{CloseReason, Error, ErrorKind};
pub use filter::BloomFilter;
pub use filter_policy::{
    BloomFilterPolicy, Filter, FilterBuilder, FilterContext, FilterPolicy, FilterQuery,
    FilterTarget,
};
pub use format::sst::BlockTransformer;
pub use garbage_collector::stats as garbage_collector_stats;
pub use garbage_collector::{GarbageCollectorBuilder, GcFilter};
pub use instrumented_object_store::stats as instrumented_object_store_stats;
pub use iter::IterationOrder;
pub use manifest::VersionedManifest;
pub use merge_operator::{MergeOperator, MergeOperatorError};
pub use ops::{DbCacheManagerOps, DbMetadataOps, DbReadOps, DbTransactionOps, DbWriteOps};
pub use paths::PathResolver;
pub use prefix_extractor::{PrefixExtractor, PrefixTarget};
pub use slatedb_common::{DbRand, IdentifiedObjectMetadata, ObjectMetadata};
#[cfg(test)]
pub use sst_builder::BlockFormat;
pub use sst_reader::{SstFile, SstIndex, SstReader};
pub use sst_stats::{BlockStats, SstStats};
pub use transaction_manager::IsolationLevel;
pub use types::KeyValue;
pub use types::{RowEntry, ValueDeletable};
pub use wal::slatedb::writer::stats as wal_buffer_stats;
pub use wal_reader::{WalFile, WalFileIterator, WalReader};

pub mod admin;
pub mod cached_object_store;
pub mod clock;
#[cfg(feature = "bencher")]
pub mod compaction_execute_bench;
pub mod compaction_worker;
pub mod compactor;
pub mod config;
pub mod db_cache;
pub mod db_stats;
pub mod manifest;
pub mod object_store_tag;
pub mod prefix_extractor;
pub mod seq_tracker;
pub mod size_tiered_compaction;
pub mod wal;

mod batch;
#[cfg(feature = "bench-internal")]
pub use batch::benches as write_batch_benches;
mod batch_write;
mod blob;
mod block_cache_policy;
mod block_iterator;
mod block_iterator_v2;
#[cfg(feature = "bench-internal")]
pub use block_iterator_v2::benches as block_iterator_v2_benches;
#[cfg(any(test, feature = "bencher"))]
mod bytes_generator;
mod bytes_range;
mod checkpoint;
mod clone;
#[cfg(feature = "compaction_filters")]
mod compaction_filter;
#[cfg(feature = "compaction_filters")]
mod compaction_filter_iterator;
mod compactions_store;
mod compactor_executor;
mod compactor_state;
mod compactor_state_protocols;
#[allow(dead_code)]
mod comparable_range;
mod db;
mod db_cache_manager;
mod db_common;
mod db_iter;
mod db_reader;
mod db_snapshot;
mod db_state;
mod db_status;
mod db_transaction;
mod dispatcher;
mod error;
pub mod filter;
mod filter_iterator;
pub mod filter_policy;
mod flatbuffer_types;
mod flush;
mod format;
mod garbage_collector;
mod instrumented_object_store;
mod iter;
mod mem_table;
mod memtable_flusher;
mod merge_iterator;
mod merge_operator;
mod ops;
mod oracle;
mod partitioned_keyspace;
mod paths;
mod peeking_iterator;
#[cfg(test)]
mod proptest_util;
mod reader;
mod retention_iterator;
mod retrying_object_store;
mod segment_iterator;
mod sequenced_store;
mod single_flight;
mod snapshot_manager;
mod sorted_run_iterator;
mod sst_builder;
mod sst_io;
mod sst_iter;
mod sst_reader;
mod sst_stats;
mod subcompaction;
mod tablestore;
#[cfg(test)]
mod test_utils;
mod transaction_manager;
mod types;
mod utils;

mod fence;
mod wal_reader;
mod wal_replay;

// Initialize test infrastructure (deadlock detector, tracing) for all tests.
// This ctor runs at crate load time, ensuring these are set up even for tests
// that don't explicitly use test_utils.
#[cfg(test)]
#[ctor::ctor]
fn init_test_infrastructure() {
    test_utils::init_test_infrastructure();
}
