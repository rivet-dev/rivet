//! Configuration options for SlateDB.
//!
//! This module provides structures and functions to manage database configuration options,
//! including reading from files and environment variables.
//!
//! # Examples
//!
//! Loading the default configuration for `Settings`:
//!
//! ```rust
//! use slatedb::config::Settings;
//! let config = Settings::default();
//! ```
//!
//! Loading `Settings` from a specific file:
//!
//! ```rust
//! use slatedb::config::Settings;
//! let config = Settings::from_file("config.toml").expect("Failed to load options from file");
//! ```
//!
//! Loading `Settings` from environment variables:
//!
//! ```rust
//! use slatedb::config::Settings;
//! let config = Settings::from_env("SLATEDB_").expect("Failed to load options from env");
//! ```
//!
//! Loading `Settings` from predefined files, SlateDb.toml, SlateDb.json, SlateDb.yaml, or SlateDb.yml.
//! This method also merges any environment variable that starts with `SLATEDB_` to the final `Settings` struct:
//!
//! ```rust
//! use slatedb::config::Settings;
//! let config = Settings::load().expect("Failed to load options");
//! ```
//!
//! # Configuration formats
//!
//! SlateDB supports three configuration formats: TOML, JSON, and YAML.
//! Duration options in the configuration are represented as human-friendly strings,
//! allowing you to specify time intervals in a more intuitive way, such as "100ms" or "1s".
//!
//! Representing `Settings` with TOML:
//!
//! ```toml
//! flush_interval = "100ms"
//! wal_enabled = false
//! manifest_poll_interval = "1s"
//! manifest_update_timeout = "300s"
//! min_filter_keys = 1000
//! l0_sst_size_bytes = 67108864
//! max_wal_flushes_before_l0_flush = 4096
//! l0_max_ssts = 8
//! l0_max_ssts_per_key = 8
//! l0_flush_parallelism = 4
//! max_unflushed_bytes = 536870912
//! metric_level = "Info"
//!
//! [compactor_options]
//! poll_interval = "5s"
//! max_concurrent_compactions = 4
//! enable_trivial_move = false
//!
//! [compactor_options.worker]
//! max_sst_size = 1073741824
//!
//! [compactor_options.scheduler_options]
//! min_compaction_sources = "4"
//! max_compaction_sources = "8"
//! include_size_threshold = "4.0"
//!
//! [object_store_cache_options]
//! root_folder = "/tmp/slatedb-cache"
//! max_cache_size_bytes = 17179869184
//! part_size_bytes = 4194304
//! scan_interval = "3600s"
//!
//! [garbage_collector_options.manifest_options]
//! interval = "300s"
//! min_age = "86400s"
//!
//! [garbage_collector_options.wal_options]
//! interval = "60s"
//! min_age = "60s"
//!
//! [garbage_collector_options.compacted_options]
//! interval = "300s"
//! min_age = "86400s"
//!
//! [garbage_collector_options.compactions_options]
//! interval = "300s"
//! min_age = "86400s"
//! ```
//!
//! Representing `Settings` with JSON:
//!
//! ```json
//!{
//!  "flush_interval": "100ms",
//!  "wal_enabled": false,
//!  "manifest_poll_interval": "1s",
//!  "manifest_update_timeout": "300s",
//!  "min_filter_keys": 1000,
//!  "l0_sst_size_bytes": 67108864,
//!  "max_wal_flushes_before_l0_flush": 4096,
//!  "l0_max_ssts": 8,
//!  "l0_max_ssts_per_key": 8,
//!  "l0_flush_parallelism": 4,
//!  "max_unflushed_bytes": 536870912,
//!  "metric_level": "Info",
//!  "compactor_options": {
//!    "poll_interval": "5s",
//!    "max_concurrent_compactions": 4,
//!    "enable_trivial_move": false,
//!    "worker": {
//!      "max_sst_size": 1073741824
//!    },
//!    "scheduler_options": {
//!      "min_compaction_sources": "4",
//!      "max_compaction_sources": "8",
//!      "include_size_threshold": "4.0"
//!    }
//!  },
//!  "compression_codec": null,
//!  "object_store_cache_options": {
//!    "root_folder": "/tmp/slatedb-cache",
//!    "max_cache_size_bytes": 17179869184,
//!    "part_size_bytes": 4194304,
//!    "scan_interval": "3600s"
//!  },
//!  "garbage_collector_options": {
//!    "manifest_options": {
//!      "interval": "300s",
//!      "min_age": "86400s"
//!    },
//!    "wal_options": {
//!      "interval": "60s",
//!      "min_age": "60s"
//!    },
//!    "compacted_options": {
//!      "interval": "300s",
//!      "min_age": "86400s"
//!    },
//!    "compactions_options": {
//!      "interval": "300s",
//!      "min_age": "86400s"
//!    }
//!  }
//!}
//!```
//!
//! Representing `Settings` with YAML:
//!
//! ```yaml
//! flush_interval: '100ms'
//! wal_enabled: false
//! manifest_poll_interval: '1s'
//! manifest_update_timeout: '300s'
//! min_filter_keys: 1000
//! l0_sst_size_bytes: 67108864
//! max_wal_flushes_before_l0_flush: 4096
//! l0_max_ssts: 8
//! l0_max_ssts_per_key: 8
//! l0_flush_parallelism: 1
//! max_unflushed_bytes: 536870912
//! metric_level: Info
//! compactor_options:
//!   poll_interval: '5s'
//!   max_concurrent_compactions: 4
//!   enable_trivial_move: false
//!   worker:
//!     max_sst_size: 1073741824
//!   scheduler_options:
//!     min_compaction_sources: "4"
//!     max_compaction_sources: "8"
//!     include_size_threshold: "4.0"
//! compression_codec: null
//! object_store_cache_options:
//!   root_folder: /tmp/slatedb-cache
//!   max_cache_size_bytes: 17179869184
//!   part_size_bytes: 4194304
//!   scan_interval: '3600s'
//! garbage_collector_options:
//!   manifest_options:
//!     interval: '300s'
//!     min_age: '86400s'
//!   wal_options:
//!     interval: '60s'
//!     min_age: '60s'
//!   compacted_options:
//!     interval: '300s'
//!     min_age: '86400s'
//!   compactions_options:
//!     interval: '300s'
//!     min_age: '86400s'
//! ```
//!
use crate::filter_policy::FilterContext;
use crate::iter::IterationOrder;
use duration_str::{deserialize_duration, deserialize_option_duration};
use figment::providers::{Env, Format, Json, Toml, Yaml};
use figment::{Figment, Metadata, Provider};
use log::warn;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::path::Path;
use std::{str::FromStr, time::Duration};
use uuid::Uuid;

pub use slatedb_common::metrics::MetricLevel;

use crate::error::SlateDBError;

use crate::garbage_collector::{DEFAULT_INTERVAL, DEFAULT_MIN_AGE};

fn default_true() -> bool {
    true
}

/// Enum representing different levels of cache preloading on startup
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum PreloadLevel {
    /// Preload only L0 SSTs (most recently written files)
    L0Sst,
    /// Preload all SSTs (both L0 and compacted levels)
    AllSst,
}

/// Enum representing valid SST block sizes
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Default)]
pub enum SstBlockSize {
    /// 1KiB blocks
    Block1Kib,
    /// 2KiB blocks
    Block2Kib,
    /// 4KiB blocks (default)
    #[default]
    Block4Kib,
    /// 8KiB blocks
    Block8Kib,
    /// 16KiB blocks
    Block16Kib,
    /// 32KiB blocks
    Block32Kib,
    /// 64KiB blocks
    Block64Kib,
    /// Other block sizes
    #[cfg(test)]
    Other(usize),
}

impl SstBlockSize {
    /// Get the block size in bytes
    pub fn as_bytes(&self) -> usize {
        match self {
            SstBlockSize::Block1Kib => 1024,
            SstBlockSize::Block2Kib => 2048,
            SstBlockSize::Block4Kib => 4096,
            SstBlockSize::Block8Kib => 8192,
            SstBlockSize::Block16Kib => 16384,
            SstBlockSize::Block32Kib => 32768,
            SstBlockSize::Block64Kib => 65536,
            #[cfg(test)]
            SstBlockSize::Other(size) => *size,
        }
    }
}

/// Describes the durability of data based on the medium (e.g. in-memory, object storags)
/// that the data is currently stored in. Currently this is used to define a
/// durability filter for data served by a read.
#[non_exhaustive]
#[derive(Clone, Default, Debug, Copy, PartialEq)]
pub enum DurabilityLevel {
    /// Includes only data currently stored durably in object storage.
    Remote,

    /// Includes data with level Remote and data currently only stored in-memory awaiting flush
    /// to object storage.
    #[default]
    Memory,
}

/// Options for tracing a read operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TracingOptions {
    pub trace_id: String,
}

impl TracingOptions {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
        }
    }
}

/// Configuration for client read operations. `ReadOptions` is supplied for each
/// read call and controls the behavior of the read.
#[derive(Clone, Debug)]
pub struct ReadOptions {
    /// Specifies the minimum durability level for data returned by this read. For example,
    /// if set to Remote then slatedb returns the latest version of a row that has been durably
    /// stored in object storage.
    pub durability_filter: DurabilityLevel,
    /// Whether to include dirty data in the scan. "dirty" means that the data is not considered
    /// as "committed" yet, whose seq number is greater than the last committed seq number.
    pub dirty: bool,
    /// Whether fetched data blocks should be cached. SST indexes, filters,
    /// and stats are cached independently of this setting.
    pub cache_blocks: bool,
    /// Optional context forwarded to custom filter policies; ignored by
    /// built-in filters. See [`FilterContext`].
    pub filter_context: Option<FilterContext>,
    /// Optional caller-provided tracing settings.
    pub tracing_options: Option<TracingOptions>,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            durability_filter: DurabilityLevel::default(),
            dirty: false,
            cache_blocks: true,
            filter_context: None,
            tracing_options: None,
        }
    }
}

impl ReadOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_dirty(self, dirty: bool) -> Self {
        Self { dirty, ..self }
    }

    pub fn with_durability_filter(self, durability_filter: DurabilityLevel) -> Self {
        Self {
            durability_filter,
            ..self
        }
    }

    pub fn with_cache_blocks(self, cache_blocks: bool) -> Self {
        Self {
            cache_blocks,
            ..self
        }
    }

    pub fn with_filter_context(self, filter_context: Option<FilterContext>) -> Self {
        Self {
            filter_context,
            ..self
        }
    }

    pub fn with_tracing_options(self, tracing_options: Option<TracingOptions>) -> Self {
        Self {
            tracing_options,
            ..self
        }
    }
}
#[derive(Clone, Debug)]
pub struct ScanOptions {
    /// Specifies the minimum durability level for data returned by this scan. For example,
    /// if set to Remote then slatedb returns the latest version of a row that has been durably
    /// stored in object storage.
    pub durability_filter: DurabilityLevel,
    /// Whether to include dirty data in the scan. "dirty" means that the data is not considered
    /// as "committed" yet, whose seq number is greater than the last committed seq number.
    pub dirty: bool,
    /// The target number of bytes to fetch in a single request while iterating over SSTs.
    /// Each fetch will read the minimum number of blocks such that the resulting read is at least
    /// this size or reaches the end of the file. The default is 1, which results in each fetch
    /// reading one block.
    pub read_ahead_bytes: usize,
    /// Whether or not fetched data blocks should be cached. SST indexes,
    /// filters, and stats are cached independently of this setting.
    pub cache_blocks: bool,
    /// The maximum number of concurrent tasks for fetching blocks during scans.
    /// Higher values can improve throughput but use more resources. The default is 1.
    pub max_fetch_tasks: usize,
    /// The iteration order for the scan. Defaults to [`IterationOrder::Ascending`].
    pub order: IterationOrder,
    /// Optional context forwarded to custom filter policies; ignored by
    /// built-in filters. See [`FilterContext`].
    ///
    /// Consulted by `scan_prefix`, and by `scan` only when a registered
    /// policy reports [`crate::filter_policy::FilterPolicy::supports_range_queries`].
    pub filter_context: Option<FilterContext>,
    /// Optional caller-provided tracing settings.
    pub tracing_options: Option<TracingOptions>,
}

impl Default for ScanOptions {
    /// Create a new ScanOptions with `read_level` set to [`DurabilityLevel::Memory`].
    fn default() -> Self {
        Self {
            durability_filter: DurabilityLevel::default(),
            dirty: false,
            read_ahead_bytes: 1,
            cache_blocks: false,
            max_fetch_tasks: 1,
            order: IterationOrder::Ascending,
            filter_context: None,
            tracing_options: None,
        }
    }
}

impl ScanOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_dirty(self, dirty: bool) -> Self {
        Self { dirty, ..self }
    }

    pub fn with_durability_filter(self, durability_filter: DurabilityLevel) -> Self {
        Self {
            durability_filter,
            ..self
        }
    }

    pub fn with_read_ahead_bytes(self, read_ahead_bytes: usize) -> Self {
        Self {
            read_ahead_bytes,
            ..self
        }
    }

    pub fn with_cache_blocks(self, cache_blocks: bool) -> Self {
        Self {
            cache_blocks,
            ..self
        }
    }

    pub fn with_max_fetch_tasks(self, max_fetch_tasks: usize) -> Self {
        Self {
            max_fetch_tasks,
            ..self
        }
    }

    pub fn with_order(self, order: IterationOrder) -> Self {
        Self { order, ..self }
    }

    pub fn with_filter_context(self, filter_context: Option<FilterContext>) -> Self {
        Self {
            filter_context,
            ..self
        }
    }

    pub fn with_tracing_options(self, tracing_options: Option<TracingOptions>) -> Self {
        Self {
            tracing_options,
            ..self
        }
    }
}

/// Enum representing the type of flush to perform.
#[derive(Clone, Debug)]
pub enum FlushType {
    /// Freeze the active memtable [crate::mem_table::KVTable] and write
    /// all immutable memtable entries (including the formerly active
    /// memtable) to the object store.
    MemTable,
    /// Freeze the active WAL [crate::mem_table::KVTable] and write all
    /// immutable WAL entries (including the formerly active WAL) to the
    /// object store.
    Wal,
}

#[derive(Clone)]
pub struct FlushOptions {
    /// The type of flush to perform.
    pub flush_type: FlushType,
}

impl Default for FlushOptions {
    fn default() -> Self {
        Self {
            flush_type: FlushType::Wal,
        }
    }
}

/// Options controlling how a database is closed.
#[derive(Clone, Debug)]
pub struct CloseOptions {
    /// The type of flush to perform before closing.
    ///
    /// When `None`, no final flush is triggered. Memtables already being
    /// flushed continue through the existing shutdown pipeline, and writes
    /// that are not durable may be lost. When set to `Some` flushes the
    /// database in accordance with the specified [`FlushType`]
    /// Defaults to `Some(FlushType::MemTable)`.
    pub flush_type: Option<FlushType>,
}

impl Default for CloseOptions {
    fn default() -> Self {
        Self {
            flush_type: Some(FlushType::MemTable),
        }
    }
}

impl CloseOptions {
    /// Configure the type of flush to perform before closing.
    pub fn with_flush_type(mut self, flush_type: Option<FlushType>) -> Self {
        self.flush_type = flush_type;
        self
    }
}

/// Configuration for client write operations. `WriteOptions` is supplied for each
/// write call and controls the behavior of the write.
#[derive(Clone, Debug, Default)]
pub struct WriteOptions {
    #[cfg(dst)]
    /// Force the current timestamp for DST operations. See #719 for details.
    pub now: i64,
    /// An optional user-defined sequence number for this write. When non-zero, the
    /// provided value is used instead of the internally generated sequence number.
    /// The value must be strictly greater than the current maximum sequence number
    /// or the write will fail with an `InvalidSequenceNumber` error.
    pub seqnum: u64,
}

/// Configuration for client put operations. `PutOptions` is supplied for each
/// row inserted. This differs from [`WriteOptions`] in that a write may encompass
/// multiple puts (such as the case with batched writes)
#[derive(Clone, Default, PartialEq, Debug)]
pub struct PutOptions {
    /// The time-to-live (ttl) for this insertion. If this insert overwrites an existing
    /// database entry, the TTL for the most recent entry will be canonical.
    ///
    /// Default: the TTL configured in DbOptions when opening a SlateDB session
    pub ttl: Ttl,
}

impl PutOptions {
    pub(crate) fn expire_ts_from(
        &self,
        default_ttl_millis: Option<u64>,
        now_millis: i64,
    ) -> Option<i64> {
        match self.ttl {
            Ttl::Default => match default_ttl_millis {
                None => None,
                Some(default_ttl_millis) => Self::checked_expire_ts(now_millis, default_ttl_millis),
            },
            Ttl::NoExpiry => None,
            Ttl::ExpireAfterMillis(ttl_millis) => Self::checked_expire_ts(now_millis, ttl_millis),
            Ttl::ExpireAtMillis(timestamp_millis) => Some(timestamp_millis),
        }
    }

    fn checked_expire_ts(now_millis: i64, ttl_millis: u64) -> Option<i64> {
        // for overflow, we will just assume no TTL
        if ttl_millis > i64::MAX as u64 {
            return None;
        };
        let expire_ts = now_millis + (ttl_millis as i64);
        if expire_ts < now_millis {
            return None;
        };

        Some(expire_ts)
    }
}

/// Configuration for client merge operations. `MergeOptions` is supplied for each
/// merge operand inserted into the database.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct MergeOptions {
    /// The time-to-live (ttl) for this merge operand. This behaves the same as
    /// [`PutOptions::ttl`], where the latest non-expired value or merge operand
    /// dictates the canonical TTL for a key.
    pub ttl: Ttl,
}

impl MergeOptions {
    // TODO(agavra): deduplicate this with PutOptions::expire_ts_from
    pub(crate) fn expire_ts_from(
        &self,
        default_ttl_millis: Option<u64>,
        now_millis: i64,
    ) -> Option<i64> {
        match self.ttl {
            Ttl::Default => match default_ttl_millis {
                None => None,
                Some(default_ttl_millis) => Self::checked_expire_ts(now_millis, default_ttl_millis),
            },
            Ttl::NoExpiry => None,
            Ttl::ExpireAfterMillis(ttl_millis) => Self::checked_expire_ts(now_millis, ttl_millis),
            Ttl::ExpireAtMillis(timestamp_millis) => Some(timestamp_millis),
        }
    }

    fn checked_expire_ts(now_millis: i64, ttl_millis: u64) -> Option<i64> {
        // for overflow, we will just assume no TTL
        if ttl_millis > i64::MAX as u64 {
            return None;
        };
        let expire_ts = now_millis + (ttl_millis as i64);
        if expire_ts < now_millis {
            return None;
        };

        Some(expire_ts)
    }
}

/// Time-to-live policy applied to an inserted value or merge operand.
///
/// TTL durations are expressed in milliseconds. Absolute expiration timestamps are
/// expressed as milliseconds since the Unix epoch.
///
/// Expiration is applied during compaction and is therefore best effort; an expired
/// value may remain visible until compaction processes it.
#[non_exhaustive]
#[derive(Clone, Default, PartialEq, Debug)]
pub enum Ttl {
    /// Use [`Settings::default_ttl_millis`].
    #[default]
    Default,
    /// Store the value without an expiration.
    NoExpiry,
    /// Expire the value after the specified number of milliseconds.
    ExpireAfterMillis(u64),
    /// Expire the value at the specified Unix timestamp in milliseconds.
    ExpireAtMillis(i64),
}

/// Defines the scope targeted by a given checkpoint. If set to All, then the checkpoint will
/// include all writes that were issued at the time that create_checkpoint is called. SlateDB will
/// flush WALs (if enabled) and do a best-effort flush of memtables to L0 SSTs in order to
/// optimize for readers. The memtable flush will not await if blocked by backpressure. If set
/// to Durable, then the checkpoint includes only writes that were durable at the time of the
/// call. This will be faster, but may not include data from recent writes.
#[non_exhaustive]
#[derive(Debug, Copy, Clone)]
pub enum CheckpointScope {
    All,
    Durable,
}

/// Specify options to provide when creating a checkpoint.
#[derive(Debug, Clone, Default)]
pub struct CheckpointOptions {
    /// Optional caller-chosen identity for an idempotent, retained checkpoint. Reusing this ID
    /// returns the original checkpoint even if newer writes have since become durable. The caller
    /// must never reuse it for another logical checkpoint or delete it while retries are possible.
    /// This mode requires lifetime, source and name to be None and rejects the nil UUID.
    /// None preserves the existing behavior of allocating a fresh ID for each invocation.
    pub checkpoint_id: Option<Uuid>,

    /// Optionally specifies the lifetime of the checkpoint to create. The expire time will be
    /// set to the current wallclock time plus the specified lifetime. If lifetime is None, then
    /// the checkpoint is created without an expiry time.
    pub lifetime: Option<Duration>,

    /// Optionally specifies an existing checkpoint to use as the source for this checkpoint. This
    /// is useful for users to establish checkpoints from existing checkpoints, but with a different
    /// lifecycle and/or metadata.
    pub source: Option<Uuid>,

    /// Optionally specifies a name for the checkpoint. Can be used to list the checkpoints.
    pub name: Option<String>,
}

impl CheckpointOptions {
    pub(crate) fn validate(&self) -> Result<(), SlateDBError> {
        if self.checkpoint_id.is_some_and(|id| id.is_nil())
            || (self.checkpoint_id.is_some()
                && (self.lifetime.is_some() || self.source.is_some() || self.name.is_some()))
        {
            return Err(SlateDBError::InvalidConfiguration(
                "caller-chosen checkpoint IDs require a non-nil ID and retained, unnamed, source-free options".into(),
            ));
        }
        Ok(())
    }
}

/// Settings represents the configuration options that a user can tweak to customize
/// the database engine to their use case.
///
/// This is separate from components (like block_cache, clock, etc.) which are responsible
/// for performing the work in the database.
///
/// Note: `compactor_options` is mutually exclusive with `DbBuilder::with_compactor_builder`.
/// Setting both will result in an error.
///
/// For backward compatibility, DBOptions is a type alias for Settings.
#[derive(Clone, Deserialize, Serialize)]
pub struct Settings {
    /// How frequently to flush the write-ahead log to object storage.
    ///
    /// When setting this configuration, users must consider:
    ///
    /// * **Latency**: The higher the flush interval, the longer it will take for
    ///   writes to be committed to object storage. Writers blocking on `put` calls
    ///   will wait longer for the write. Readers reading committed writes will also
    ///   see data later.
    /// * **API cost**: The lower the flush interval, the more frequently PUT calls
    ///   will be made to object storage. This can increase your object storage costs.
    ///
    /// We recommend setting this value based on your cost and latency tolerance. A
    /// 100ms flush interval should result in $130/month in PUT costs on S3 standard.
    ///
    /// Keep in mind that the flush interval does not include the network latency. A
    /// 100ms flush interval will result in a 100ms + the time it takes to send the
    /// bytes to object storage.
    ///
    /// If this value is None, automatic flushing will be disabled. The application
    /// can flush by calling `Db::flush()` manually, and by closing the database.
    #[serde(deserialize_with = "deserialize_option_duration")]
    #[serde(serialize_with = "serialize_option_duration")]
    pub flush_interval: Option<Duration>,

    /// If set to false, SlateDB will disable the WAL and write directly into the memtable
    #[cfg(feature = "wal_disable")]
    pub wal_enabled: bool,

    /// How frequently to poll for new manifest files. Refreshing the manifest file
    /// allows the db to detect fencing operations and newly compacted data.
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(serialize_with = "serialize_duration")]
    pub manifest_poll_interval: Duration,

    /// The maximum amount of time to wait for a manifest update before giving up.
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(serialize_with = "serialize_duration")]
    pub manifest_update_timeout: Duration,

    /// Write SSTables with a bloom filter if the number of keys in the SSTable
    /// is greater than or equal to this value. Reads on small SSTables might be
    /// faster without a bloom filter.
    pub min_filter_keys: u32,

    /// The minimum size a memtable needs to be before it is frozen and flushed to
    /// L0 object storage. Writes will still be flushed to the object storage WAL
    /// (based on flush_interval) regardless of this value. Memtable sizes are checked
    /// every `flush_interval`.
    ///
    /// When setting this configuration, users must consider:
    ///
    /// * **Recovery time**: The larger the L0 SSTable size threshold, the less
    ///   frequently it will be written. As a result, the more recovery data there
    ///   will be in the WAL if a process restarts.
    /// * **Number of L0 SSTs/SRs**: The smaller the L0 SSTable size threshold, the
    ///   more SSTs and Sorted Runs there will be. L0 SSTables are not range
    ///   partitioned; each is its own sorted table. Similarly, each Sorted Run also
    ///   stores the entire keyspace. As such, reads that don't hit the WAL or memtable
    ///   may need to scan all L0 SSTables and Sorted Runs. The more there are, the
    ///   slower the scan will be.
    /// * **Memory usage**: The larger the L0 SSTable size threshold, the larger the
    ///   unflushed in-memory memtable will grow. This shouldn't be a concern for most
    ///   workloads, but it's worth considering for workloads with very high L0
    ///   SSTable sizes.
    /// * **API cost**: Smaller L0 SSTable sizes will result in more frequent writes
    ///   to object storage. This can increase your object storage costs.
    /// * **Secondary reader latency**: Secondary (non-writer) clients only see L0+
    ///   writes; they don't see WAL writes. Thus, the higher the L0 SSTable size, the
    ///   less frequently they will be written, and the longer it will take for
    ///   secondary readers to see new data.
    pub l0_sst_size_bytes: usize,

    /// The maximum number of WAL flushes that can occur before the active memtable is
    /// frozen and flushed to L0, regardless of memtable size.
    ///
    /// For databases with low write throughput, this can cause data to be available in
    /// L0 SSTs sooner, making it accessible to readers.
    ///
    /// This also bounds the amount of WAL data that needs to be replayed on recovery: once
    /// this many WAL flushes have occurred since the last memtable freeze, the active
    /// memtable will be frozen even if it has not reached `l0_sst_size_bytes`.
    pub max_wal_flushes_before_l0_flush: u64,

    /// Defines the max total number of SSTs in L0 across the entire key space. Memtables
    /// will not be flushed if the total L0 count (including in-flight uploads) would exceed
    /// this value, until compaction can compact the ssts into compacted.
    ///
    /// This cap primarily bounds manifest size and global bookkeeping. Read amplification
    /// and write backpressure are governed by [`Self::l0_max_ssts_per_key`], which enforces
    /// a cap on L0 SSTs overlapping any single key. After a manifest union (rescaling),
    /// the total L0 count can exceed a single source's `l0_max_ssts` while no individual
    /// key is covered by more than `l0_max_ssts_per_key` SSTs; `l0_max_ssts` should be
    /// set generously in that case (e.g. `l0_max_ssts_per_key * expected_max_shards`).
    pub l0_max_ssts: usize,

    /// Defines the max number of L0 SSTs whose effective ranges cover any single key.
    /// Memtables will not be flushed if dispatching a new L0 upload would cause any point
    /// in the key space to be covered by more L0 SSTs than this value.
    ///
    /// This is the per-key analogue of [`Self::l0_max_ssts`]: it bounds the number of L0
    /// SSTs a point read may need to consult (read amplification) and drives write
    /// backpressure. Because in-flight uploads have no known key range yet, each reserved
    /// slot is treated conservatively as contributing to the peak at every point.
    pub l0_max_ssts_per_key: usize,

    /// Number of parallel workers for flushing immutable memtables to L0 SSTs.
    /// Higher values increase L0 flush throughput at the cost of more concurrent
    /// object store uploads. Increasing parallelism may require a higher `l0_max_ssts`
    /// to avoid backpressure from compaction not keeping up with the higher steady-state
    /// flush rate.
    pub l0_flush_parallelism: usize,

    /// Defines the max number of unflushed key/value pair bytes that should reside in memory
    /// before applying backpressure to writers. This includes key/value pairs in both the
    /// immutable WAL flush queue and the immutable memtable flush queue. Writes will be
    /// paused if the total number of unflushed bytes exceeds this value until data is flushed
    /// to object storage.
    pub max_unflushed_bytes: usize,

    /// Configuration options for the compactor. The embedded compaction worker
    /// is configured via [`CompactorOptions::worker`].
    pub compactor_options: Option<CompactorOptions>,

    /// The compression algorithm to use for SSTables.
    pub compression_codec: Option<CompressionCodec>,

    /// The object store cache options. When `root_folder` is set, the database
    /// wraps its main object store in a
    /// [`CachedObjectStore`](crate::cached_object_store::CachedObjectStore)
    /// built from these options. To construct and share the cache yourself,
    /// build one with
    /// [`CachedObjectStore::builder`](crate::cached_object_store::CachedObjectStore::builder)
    /// and pass it to [`Db::builder`](crate::Db::builder) instead, leaving
    /// these options unset.
    pub object_store_cache_options: ObjectStoreCacheOptions,

    /// Configuration options for the garbage collector.
    pub garbage_collector_options: Option<GarbageCollectorOptions>,

    /// Controls which metrics are active.
    ///
    /// Metrics below this threshold are replaced with no-op handles when they
    /// are registered. Defaults to [`MetricLevel::Info`].
    #[serde(default)]
    pub metric_level: MetricLevel,

    /// The default time-to-live (TTL), in milliseconds, for insertions (note that
    /// re-inserting a key with any value will update the TTL to use
    /// `default_ttl_millis`).
    ///
    /// Default: no TTL (insertions will remain until deleted)
    pub default_ttl_millis: Option<u64>,

    /// Maximum number of wrapper-level retries for a single object-store
    /// operation, on top of the `object_store` client's own HTTP retries.
    /// Applies to both foreground (user API) and background-task operations,
    /// since both share the same retrying object store.
    ///
    /// * `None` (default): retry transient errors indefinitely (historical behavior).
    /// * `Some(n)`: give up after `n` retries and return the underlying error.
    #[serde(default)]
    pub object_store_max_retries: Option<u32>,

    /// The block format for SST files. This is only available in tests
    /// to verify backward compatibility between V1 and V2 formats.
    #[cfg(test)]
    #[serde(skip)]
    pub block_format: Option<crate::sst_builder::BlockFormat>,
}

// Implement Debug manually for DbOptions.
// This is needed because DbOptions contains several boxed trait objects
// which doesn't implement Debug.
impl std::fmt::Debug for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data = f.debug_struct("DbOptions");
        data.field("flush_interval", &self.flush_interval);
        #[cfg(feature = "wal_disable")]
        {
            data.field("wal_enabled", &self.wal_enabled);
        }
        data.field("manifest_poll_interval", &self.manifest_poll_interval)
            .field("manifest_update_timeout", &self.manifest_update_timeout)
            .field("min_filter_keys", &self.min_filter_keys)
            .field("max_unflushed_bytes", &self.max_unflushed_bytes)
            .field("l0_sst_size_bytes", &self.l0_sst_size_bytes)
            .field(
                "max_wal_flushes_before_l0_flush",
                &self.max_wal_flushes_before_l0_flush,
            )
            .field("l0_max_ssts", &self.l0_max_ssts)
            .field("l0_max_ssts_per_key", &self.l0_max_ssts_per_key)
            .field("l0_flush_parallelism", &self.l0_flush_parallelism)
            .field("compactor_options", &self.compactor_options)
            .field("compression_codec", &self.compression_codec)
            .field(
                "object_store_cache_options",
                &self.object_store_cache_options,
            )
            .field("garbage_collector_options", &self.garbage_collector_options)
            .field("metric_level", &self.metric_level)
            .field("default_ttl_millis", &self.default_ttl_millis);
        data.finish()
    }
}

impl Settings {
    /// Converts the Settings to a JSON string representation
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Validates that the settings are internally consistent, rejecting field
    /// combinations that would deadlock or fail at runtime.
    ///
    /// # Errors
    ///
    /// Returns an [`crate::Error`] with [`crate::ErrorKind::Invalid`] describing
    /// the first invalid setting or combination encountered.
    pub fn validate(&self) -> Result<(), crate::Error> {
        if self.l0_flush_parallelism == 0 {
            return Err(SlateDBError::InvalidConfiguration(
                "l0_flush_parallelism must be at least 1".into(),
            )
            .into());
        }
        if self.max_wal_flushes_before_l0_flush < 4096 {
            return Err(SlateDBError::InvalidConfiguration(
                "max_wal_flushes_before_l0_flush must be at least 4096".into(),
            )
            .into());
        }
        // `max_unflushed_bytes` (the backpressure threshold) must exceed
        // `l0_sst_size_bytes` (the memtable freeze threshold) so that memory can
        // hold a memtable up to the freeze point before backpressure kicks in.
        if self.max_unflushed_bytes <= self.l0_sst_size_bytes {
            return Err(SlateDBError::InvalidConfiguration(format!(
                "max_unflushed_bytes ({}) must be greater than l0_sst_size_bytes ({})",
                self.max_unflushed_bytes, self.l0_sst_size_bytes,
            ))
            .into());
        }
        Ok(())
    }

    /// Loads Settings from a file.
    ///
    /// This function attempts to read and parse a configuration file to create a Settings instance.
    /// The file format is determined by its extension:
    /// - ".json" for JSON format
    /// - ".toml" for TOML format
    /// - ".yaml" or ".yml" for YAML format
    ///
    /// # Arguments
    ///
    /// * `path` - A path-like object pointing to the configuration file.
    ///
    /// # Returns
    ///
    /// * `Ok(Settings)` if the file was successfully read and parsed.
    /// * `Err(Error)` if there was an error reading or parsing the file.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file extension is not recognized (not json, toml, yaml, or yml).
    /// - The file cannot be read or parsed according to its presumed format.
    ///
    /// # Examples
    ///
    /// ```
    /// use slatedb::config::Settings;
    /// use std::path::Path;
    ///
    /// let config = Settings::from_file("config.toml").expect("Failed to load options from file");
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Settings, crate::Error> {
        let path = path.as_ref();
        let Some(ext) = path.extension() else {
            return Err(SlateDBError::UnknownConfigurationFormat(path.into()).into());
        };

        let mut builder = Figment::from(Settings::default());
        match ext.to_str().unwrap_or_default() {
            "json" => builder = builder.merge(Json::file(path)),
            "toml" => builder = builder.merge(Toml::file(path)),
            "yaml" | "yml" => builder = builder.merge(Yaml::file(path)),
            _ => return Err(SlateDBError::UnknownConfigurationFormat(path.into()).into()),
        }
        builder
            .extract()
            .map_err(|e| SlateDBError::InvalidConfigurationFormat(Box::new(e)).into())
    }

    /// Loads Settings from environment variables with a specified prefix.
    ///
    /// This function attempts to create a Settings instance by reading environment variables
    /// that start with the given prefix. Nested options are separated by a dot (.) in the environment variable names.
    ///
    /// For example, if the prefix is "SLATEDB_" and there's an environment variable named "SLATEDB_DB_FLUSH_INTERVAL",
    /// it would correspond to the `flush_interval` field within the `Settings` struct.
    /// If there is an environment variable named "SLATEDB_OBJECT_STORE_CACHE_OPTIONS.ROOT_FOLDER",
    /// it would correspond to the `root_folder` field within the `ObjectStoreCacheOptions` within `Settings`".
    ///
    /// # Arguments
    ///
    /// * `prefix` - A string that specifies the prefix for the environment variables to be considered.
    /// * `default` - The base `Settings` value to merge environment overrides into.
    ///
    /// # Returns
    ///
    /// * `Ok(Settings)` if the environment variables were successfully read and parsed.
    /// * `Err(Error)` if there was an error reading or parsing the environment variables.
    ///
    /// # Examples
    ///
    /// ```
    /// use slatedb::config::Settings;
    ///
    /// // Assuming environment variables like SLATEDB_FLUSH_INTERVAL, SLATEDB_WAL_ENABLED, etc. are set
    /// let config = Settings::from_env_with_default("SLATEDB_", Settings::default()).expect("Failed to load options from env");
    /// ```
    pub fn from_env_with_default(
        prefix: &str,
        default: Settings,
    ) -> Result<Settings, crate::Error> {
        Figment::from(default)
            .merge(Env::prefixed(prefix))
            .extract()
            .map_err(|e| SlateDBError::InvalidConfigurationFormat(Box::new(e)).into())
    }

    /// Loads Settings from environment variables with a specified prefix.
    ///
    /// This function attempts to create a Settings instance by reading environment variables
    /// that start with the given prefix. Nested options are separated by a dot (.) in the environment variable names.
    ///
    /// For example, if the prefix is "SLATEDB_" and there's an environment variable named "SLATEDB_DB_FLUSH_INTERVAL",
    /// it would correspond to the `flush_interval` field within the `Settings` struct.
    /// If there is an environment variable named "SLATEDB_OBJECT_STORE_CACHE_OPTIONS.ROOT_FOLDER",
    /// it would correspond to the `root_folder` field within the `ObjectStoreCacheOptions` within `Settings`".
    ///
    /// # Arguments
    ///
    /// * `prefix` - A string that specifies the prefix for the environment variables to be considered.
    ///
    /// # Returns
    ///
    /// * `Ok(Settings)` if the environment variables were successfully read and parsed.
    /// * `Err(Error)` if there was an error reading or parsing the environment variables.
    ///
    /// # Examples
    ///
    /// ```
    /// use slatedb::config::Settings;
    ///
    /// // Assuming environment variables like SLATEDB_FLUSH_INTERVAL, SLATEDB_WAL_ENABLED, etc. are set
    /// let config = Settings::from_env("SLATEDB_").expect("Failed to load options from env");
    /// ```
    pub fn from_env(prefix: &str) -> Result<Settings, crate::Error> {
        Settings::from_env_with_default(prefix, Settings::default())
    }

    /// Loads Settings from multiple configuration sources in a specific order.
    ///
    /// This function attempts to create a Settings instance by merging configurations
    /// from various sources in the following order:
    /// 1. Default options
    /// 2. JSON file ("SlateDb.json")
    /// 3. TOML file ("SlateDb.toml")
    /// 4. YAML files ("SlateDb.yaml" and "SlateDb.yml")
    /// 5. Environment variables prefixed with "SLATEDB_"
    ///
    /// Each subsequent source overrides the values from the previous sources if they exist.
    ///
    /// # Returns
    ///
    /// * `Ok(Settings)` if the configuration was successfully loaded and parsed.
    /// * `Err(Error)` if there was an error reading or parsing the configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use slatedb::config::Settings;
    ///
    /// let config = Settings::load().expect("Failed to load options");
    /// ```
    pub fn load() -> Result<Settings, crate::Error> {
        Figment::from(Settings::default())
            .merge(Json::file("SlateDb.json"))
            .merge(Toml::file("SlateDb.toml"))
            .merge(Yaml::file("SlateDb.yaml"))
            .merge(Yaml::file("SlateDb.yml"))
            .admerge(Env::prefixed("SLATEDB_"))
            .extract()
            .map_err(|e| SlateDBError::InvalidConfigurationFormat(Box::new(e)).into())
    }
}

impl Provider for Settings {
    fn metadata(&self) -> Metadata {
        Metadata::named("SlateDb configuration options")
    }

    fn data(
        &self,
    ) -> Result<figment::value::Map<figment::Profile, figment::value::Dict>, figment::Error> {
        figment::providers::Serialized::defaults(self.clone()).data()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            flush_interval: Some(Duration::from_millis(100)),
            #[cfg(feature = "wal_disable")]
            wal_enabled: true,
            manifest_poll_interval: Duration::from_secs(1),
            manifest_update_timeout: Duration::from_secs(300),
            min_filter_keys: 1000,
            max_unflushed_bytes: 1_073_741_824,
            l0_sst_size_bytes: 64 * 1024 * 1024,
            max_wal_flushes_before_l0_flush: 4096,
            l0_max_ssts: 8,
            l0_max_ssts_per_key: 8,
            l0_flush_parallelism: 4,
            compactor_options: Some(CompactorOptions::default()),
            compression_codec: None,
            object_store_cache_options: ObjectStoreCacheOptions::default(),
            garbage_collector_options: Some(GarbageCollectorOptions::default()),
            metric_level: MetricLevel::default(),
            default_ttl_millis: None,
            object_store_max_retries: None,
            #[cfg(test)]
            block_format: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DbReaderOptions {
    /// How frequently to poll for new manifest files and WAL data. Refreshing the manifest
    /// file allows readers to detect newly compacted data. The reader will also look for
    /// new writes to the WAL at this poll interval. Readers using
    /// [`crate::DbReaderMode::Checkpoint`] do not poll the manifest or WAL.
    pub manifest_poll_interval: Duration,

    /// For readers using [`crate::DbReaderMode::ManagedCheckpoint`], the client maintains a
    /// checkpoint against the latest database state. The checkpoint's expire time is set to the
    /// current time plus this value. This lifetime must always be greater than
    /// `manifest_poll_interval * 2`. This option is ignored by other reader modes.
    pub checkpoint_lifetime: Duration,

    /// The max size of a single in-memory table used to buffer WAL entries
    /// Defaults to 64MB
    pub max_memtable_bytes: u64,

    /// Options for the local disk cache. If `root_folder` is set, the reader
    /// will wrap its object store in a `CachedObjectStore` backed by the
    /// local filesystem, mirroring the behaviour of `Db`.
    pub object_store_cache_options: ObjectStoreCacheOptions,

    /// When true, skip WAL replay entirely, in every reader mode. The reader reads no WAL
    /// when it opens or when it refreshes its state, so it only sees data that has been
    /// flushed to L0 or lower levels. This is useful for read-heavy workloads that
    /// don't need to see the most recent writes and want to minimize the
    /// cost of opening many readers.
    ///
    /// When combined with a reader mode that polls manifests, the reader will still see newly
    /// compacted data as manifests are updated.
    ///
    /// Defaults to false.
    pub skip_wal_replay: bool,

    /// Optional metrics reporting level for standalone readers. Defaults to
    /// [`MetricLevel::default`] when unset.
    pub metric_level: Option<MetricLevel>,

    /// Controls wrapper-level retries for this reader's object-store operations.
    /// Defaults to unbounded retries.
    #[serde(default)]
    pub object_store_max_retries: Option<u32>,
}

impl Default for DbReaderOptions {
    fn default() -> Self {
        Self {
            manifest_poll_interval: Duration::from_secs(10),
            checkpoint_lifetime: Duration::from_secs(10 * 60),
            max_memtable_bytes: 64 * 1024 * 1024,
            object_store_cache_options: ObjectStoreCacheOptions::default(),
            skip_wal_replay: false,
            metric_level: None,
            object_store_max_retries: None,
        }
    }
}

/// The compression algorithm to use for SSTables.
#[non_exhaustive]
#[derive(Clone, Copy, Deserialize, PartialEq, Debug, Serialize)]
pub enum CompressionCodec {
    #[cfg(feature = "snappy")]
    /// Snappy compression algorithm.
    Snappy,
    #[cfg(feature = "zlib")]
    /// Zlib compression algorithm.
    Zlib,
    #[cfg(feature = "lz4")]
    /// Lz4 compression algorithm.
    Lz4,
    #[cfg(feature = "zstd")]
    /// Zstd compression algorithm.
    Zstd,
}

impl FromStr for CompressionCodec {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "snappy")]
            "snappy" => Ok(Self::Snappy),
            #[cfg(feature = "zlib")]
            "zlib" => Ok(Self::Zlib),
            #[cfg(feature = "lz4")]
            "lz4" => Ok(Self::Lz4),
            #[cfg(feature = "zstd")]
            "zstd" => Ok(Self::Zstd),
            _ => Err(SlateDBError::InvalidCompressionCodec.into()),
        }
    }
}

/// Options for the compactor.
#[derive(Clone, Deserialize, Serialize)]
pub struct CompactorOptions {
    /// The interval at which the compactor checks for a new manifest and decides
    /// if a compaction must be scheduled
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(serialize_with = "serialize_duration")]
    pub poll_interval: Duration,

    /// Timeout to limit how long manifest updates are retried before giving up.
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(serialize_with = "serialize_duration")]
    pub manifest_update_timeout: Duration,

    /// The maximum number of concurrent compactions to execute at once
    pub max_concurrent_compactions: usize,

    /// Whether the coordinator may complete compactions with non-overlapping
    /// input SSTs by moving them directly into the destination sorted run,
    /// without dispatching a worker job. Because a trivial move does not rewrite
    /// rows, it does not remove tombstones, apply compaction filters, or process
    /// merges during that compaction. It also preserves the input SST sizes,
    /// which can increase manifest size and read amplification compared with
    /// rewriting inputs into larger output SSTs. Defaults to false.
    #[serde(default)]
    pub enable_trivial_move: bool,

    /// Scheduler-specific options expressed as string key/value pairs.
    #[serde(default)]
    pub scheduler_options: HashMap<String, String>,

    /// Options for the in-process compaction worker spawned alongside the
    /// coordinator. When `Some` (the default), an embedded
    /// [`CompactionWorker`](crate::compaction_worker::CompactionWorker)
    /// executes compactions in the same process. Set to `None` when all workers
    /// run as separate processes.
    ///
    /// On deserialization an omitted `worker` key defaults to
    /// `Some(CompactionWorkerOptions::default())` so existing configs keep
    /// spawning the embedded worker. Set the key explicitly to `null` to
    /// disable it.
    #[serde(default = "default_compaction_worker_options")]
    pub worker: Option<CompactionWorkerOptions>,

    /// Optional metrics reporting level for standalone compactors. When a
    /// compactor is owned by a [`Settings`] configured DB, unset means inherit
    /// [`Settings::metric_level`].
    pub metric_level: Option<MetricLevel>,

    /// The interval at which the compaction coordinator commits compactions
    /// marked `Compacted` to the manifest
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(serialize_with = "serialize_duration")]
    pub commit_compacted_interval: Duration,

    /// How long the compactor checkpoint protects input SSTs while publishing a
    /// manifest that replaces them with compacted output.
    ///
    /// A scan, get, snapshot, or transaction that began before the manifest update
    /// and still needs an input SST must finish within this duration. If it runs
    /// longer, garbage collection may delete the SST, causing the operation to fail
    /// with a [`crate::ErrorKind::Data`] error backed by
    /// [`object_store::Error::NotFound`]. Shorter lifetimes reduce retained storage;
    /// longer lifetimes give in-flight reads more time to finish. Defaults to 15
    /// minutes.
    #[serde(
        default = "default_compactor_checkpoint_lifetime",
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    pub checkpoint_lifetime: Duration,

    /// How long the coordinator will wait without a heartbeat before reclaiming
    /// a `Running` compaction from its worker and resetting it to `Submitted`.
    /// A worker that crashes or stalls will have its jobs reclaimed after this
    /// timeout. Default is 30 seconds.
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(serialize_with = "serialize_duration")]
    pub worker_heartbeat_timeout: Duration,

    /// Controls wrapper-level retries for this compactor's object-store
    /// operations. Defaults to unbounded retries.
    #[serde(default)]
    pub object_store_max_retries: Option<u32>,
}

/// Default options for the compactor. Currently, only a
/// `SizeTieredCompactionScheduler` compaction strategy is implemented.
impl Default for CompactorOptions {
    /// Returns a `CompactorOptions` with a 5 second poll interval, a 15 minute
    /// checkpoint lifetime, and an embedded worker enabled with default
    /// [`CompactionWorkerOptions`].
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(5),
            manifest_update_timeout: Duration::from_secs(300),
            max_concurrent_compactions: 4,
            enable_trivial_move: false,
            scheduler_options: HashMap::new(),
            worker: Some(CompactionWorkerOptions::default()),
            metric_level: None,
            commit_compacted_interval: Duration::from_secs(1),
            checkpoint_lifetime: default_compactor_checkpoint_lifetime(),
            worker_heartbeat_timeout: Duration::from_secs(30),
            object_store_max_retries: None,
        }
    }
}

// Implement Debug manually for CompactorOptions.
impl std::fmt::Debug for CompactorOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompactorOptions")
            .field("poll_interval", &self.poll_interval)
            .field("manifest_update_timeout", &self.manifest_update_timeout)
            .field(
                "max_concurrent_compactions",
                &self.max_concurrent_compactions,
            )
            .field("enable_trivial_move", &self.enable_trivial_move)
            .field("scheduler_options", &self.scheduler_options)
            .field("worker", &self.worker)
            .field("metric_level", &self.metric_level)
            .field("commit_compacted_interval", &self.commit_compacted_interval)
            .field("checkpoint_lifetime", &self.checkpoint_lifetime)
            .field("worker_heartbeat_timeout", &self.worker_heartbeat_timeout)
            .field("object_store_max_retries", &self.object_store_max_retries)
            .finish()
    }
}

/// Options for the compaction worker.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompactionWorkerOptions {
    /// How many jobs a single worker may hold simultaneously.
    pub max_concurrent_compactions: usize,

    /// How often a worker checks `.compactions` for new jobs.
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(serialize_with = "serialize_duration")]
    pub compactions_poll_interval: Duration,

    /// How often a worker heartbeats the jobs it owns, refreshing their
    /// liveness and publishing their latest progress to `.compactions`.
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(serialize_with = "serialize_duration")]
    pub heartbeat_interval: Duration,

    /// Maximum size of an output SST before a new one is rolled.
    pub max_sst_size: usize,

    /// Maximum number of concurrent tasks for fetching SST blocks during
    /// compaction. Higher values can improve throughput but use more resources.
    pub max_fetch_tasks: usize,

    /// The target number of bytes to fetch in a single request while iterating over
    /// input SSTs during compaction. Each fetch will read the minimum number of blocks
    /// such that the resulting read is at least this size or reaches the end of the file.
    /// The default is 2MiB.
    ///
    /// This pairs with [`CompactionWorkerOptions::max_fetch_tasks`]:
    /// `bytes_to_fetch` is the size of each read-ahead request while
    /// `max_fetch_tasks` is how many run concurrently per input SST, so peak
    /// outstanding read-ahead per SST iterator is roughly
    /// `bytes_to_fetch * max_fetch_tasks`. With the defaults
    /// (`bytes_to_fetch = 2MiB`, `max_fetch_tasks = 4`) that is ~8MiB prefetched
    /// ahead of the cursor.
    pub bytes_to_fetch: usize,

    /// The maximum number of subcompactions to split a single compaction into
    /// (RFC-0028). Each subcompaction covers a disjoint sub-range of the key
    /// space and executes concurrently with its siblings, so a single large
    /// compaction can use multiple cores. Any value `<= 1` disables
    /// subcompactions. The default is 4.
    ///
    /// The planner targets sub-ranges of
    /// `max(total_input_bytes / max_subcompactions, max_sst_size)`, so a
    /// compaction smaller than `max_subcompactions * max_sst_size` is split
    /// into fewer (or zero) ranges rather than fragmented into undersized
    /// SSTs. There is deliberately no separate minimum-size knob; the
    /// [`max_sst_size`](CompactionWorkerOptions::max_sst_size) floor subsumes
    /// it.
    pub max_subcompactions: usize,

    /// Write SSTables with a bloom filter if the number of keys in the SSTable
    /// is greater than or equal to this value. Reads on small SSTables might be
    /// faster without a bloom filter.
    ///
    /// Must match the writer's [`Settings::min_filter_keys`] configuration so
    /// that SSTs rewritten by the worker carry filters consistent with those
    /// produced by the DB.
    pub min_filter_keys: u32,

    /// The compression algorithm to use for SSTables the worker writes.
    ///
    /// Must match the writer's [`Settings::compression_codec`] configuration so
    /// that SSTs rewritten by the worker are encoded consistently with those
    /// produced by the DB.
    pub compression_codec: Option<CompressionCodec>,

    /// Optional metrics reporting level for standalone compaction workers.
    /// Defaults to [`MetricLevel::default`] when unset.
    pub metric_level: Option<MetricLevel>,
}

/// Default options for the compaction worker.
impl Default for CompactionWorkerOptions {
    /// Returns a `CompactionWorkerOptions` with a 5 second poll interval
    fn default() -> Self {
        Self {
            max_concurrent_compactions: 4,
            compactions_poll_interval: Duration::from_secs(5),
            heartbeat_interval: Duration::from_secs(10),
            max_sst_size: 256 * 1024 * 1024,
            max_fetch_tasks: 4,
            bytes_to_fetch: 2 * 1024 * 1024,
            max_subcompactions: 4,
            min_filter_keys: 1000,
            compression_codec: None,
            metric_level: None,
        }
    }
}

/// Default for [`CompactorOptions::worker`] when the key is omitted on
/// deserialization. Mirrors [`CompactorOptions::default`] so existing configs
/// without a `worker` key keep spawning the embedded worker. (A bare
/// `#[serde(default)]` would use `Option::default()` == `None`, silently
/// disabling it.)
fn default_compaction_worker_options() -> Option<CompactionWorkerOptions> {
    Some(CompactionWorkerOptions::default())
}

fn default_compactor_checkpoint_lifetime() -> Duration {
    Duration::from_mins(15)
}

/// Options for the Size-Tiered Compaction Scheduler
#[derive(Clone, Copy, Debug)]
pub struct SizeTieredCompactionSchedulerOptions {
    /// The minimum number of sources to include together in a single compaction step.
    pub min_compaction_sources: usize,

    /// The maximum number of sources to include together in a single compaction step.
    pub max_compaction_sources: usize,

    /// The size threshold that the scheduler will use to determine if a sorted run should
    /// be included in a given compaction. A sorted run S will be added to a compaction C if S's
    /// size is less than this value times the min size of the runs currently included in C.
    pub include_size_threshold: f32,

    /// The projected sorted-run count above which fallback consolidation starts for each tree.
    /// Zero disables the trigger. This value does not impose a hard limit.
    /// The scheduler rejects one with a warning and disables the fallback.
    /// Enabled values must be at least two.
    ///
    /// The projected count includes active compactions and newly selected work.
    /// The fallback limits its sources to reach this count without going below it.
    /// Normal size-based scheduling can reduce the count further.
    ///
    /// A threshold of two can cause frequent fallback merges as new runs arrive.
    /// Repeatedly merging new data with a growing run increases storage reads and writes
    /// and consumes compaction capacity. Other low thresholds can have the same cost.
    ///
    /// When normal scheduling finds no work, the fallback ignores `min_compaction_sources`
    /// and `include_size_threshold`. It requires at least two sources and respects
    /// `max_compaction_sources`, source conflicts, and existing limits on compaction work.
    ///
    /// The fallback tries consecutive groups from newest to oldest. It stops each group
    /// at `max_compaction_sources` and keeps the newest eligible sources in that group.
    pub sorted_run_consolidation_threshold: usize,
}

impl Default for SizeTieredCompactionSchedulerOptions {
    fn default() -> Self {
        Self {
            min_compaction_sources: 4,
            max_compaction_sources: 8,
            include_size_threshold: 4.0,
            sorted_run_consolidation_threshold: 0,
        }
    }
}

impl From<&HashMap<String, String>> for SizeTieredCompactionSchedulerOptions {
    fn from(map: &HashMap<String, String>) -> Self {
        let mut options = SizeTieredCompactionSchedulerOptions::default();
        for (key, value) in map {
            match key.as_str() {
                "min_compaction_sources" => match value.parse::<usize>() {
                    Ok(parsed) => options.min_compaction_sources = parsed,
                    Err(err) => {
                        warn!(
                            "invalid scheduler option value for min_compaction_sources: '{}': {}",
                            value, err
                        );
                    }
                },
                "max_compaction_sources" => match value.parse::<usize>() {
                    Ok(parsed) => options.max_compaction_sources = parsed,
                    Err(err) => {
                        warn!(
                            "invalid scheduler option value for max_compaction_sources: '{}': {}",
                            value, err
                        );
                    }
                },
                "include_size_threshold" => match value.parse::<f32>() {
                    Ok(parsed) => options.include_size_threshold = parsed,
                    Err(err) => {
                        warn!(
                            "invalid scheduler option value for include_size_threshold: '{}': {}",
                            value, err
                        );
                    }
                },
                "sorted_run_consolidation_threshold" => match value.parse::<usize>() {
                    Ok(parsed) => options.sorted_run_consolidation_threshold = parsed,
                    Err(err) => {
                        warn!(
                            "invalid scheduler option value for sorted_run_consolidation_threshold: '{}': {}",
                            value, err
                        );
                    }
                },
                _ => {
                    warn!("unknown scheduler option '{}'; ignoring", key);
                }
            }
        }

        options
    }
}

impl From<HashMap<String, String>> for SizeTieredCompactionSchedulerOptions {
    fn from(map: HashMap<String, String>) -> Self {
        Self::from(&map)
    }
}

impl From<SizeTieredCompactionSchedulerOptions> for HashMap<String, String> {
    fn from(options: SizeTieredCompactionSchedulerOptions) -> Self {
        let mut map = HashMap::new();
        map.insert(
            "min_compaction_sources".to_string(),
            options.min_compaction_sources.to_string(),
        );
        map.insert(
            "max_compaction_sources".to_string(),
            options.max_compaction_sources.to_string(),
        );
        map.insert(
            "include_size_threshold".to_string(),
            options.include_size_threshold.to_string(),
        );
        map.insert(
            "sorted_run_consolidation_threshold".to_string(),
            options.sorted_run_consolidation_threshold.to_string(),
        );
        map
    }
}

/// Garbage collector options.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GarbageCollectorOptions {
    /// Garbage collection options for the manifest directory.
    ///
    /// None means garbage collection is disabled for the manifest directory.
    pub manifest_options: Option<GarbageCollectorDirectoryOptions>,

    /// Garbage collection options for the WAL directory.
    ///
    /// None means garbage collection is disabled for the WAL directory.
    pub wal_options: Option<GarbageCollectorDirectoryOptions>,

    /// Garbage collection options for zero-byte WAL fence objects.
    ///
    /// WARNING: Setting this to a non-None value can cause data loss if set
    /// too aggressively. It's possible for the following scenario to occur:
    ///
    /// - t0: A writer W1 calculates position 7 for its next WAL entry
    /// - t1: A writer W2 writes a fence at position 7
    /// - t2: `min_age` + 1 passes
    /// - t3: The garbage collector runs and deletes the fence at position 7
    /// - t4: W1 writes its WAL entry at position 7 and returns success
    ///
    /// Because the fence at position 7 was deleted, W1's write will succeed,
    /// but the data at position 7 is invalid. The only way to protect against
    /// this scenario is to ensure fence writes are never deleted. In practice,
    /// setting `min_age` to a very high number (longer than any writer is
    /// expected to run) should be sufficient to prevent this scenario.
    ///
    /// None means garbage collection is disabled for WAL fence objects.
    pub wal_fence_options: Option<GarbageCollectorDirectoryOptions>,

    /// Garbage collection options for the compacted directory.
    ///
    /// None means garbage collection is disabled for the compacted directory.
    pub compacted_options: Option<GarbageCollectorDirectoryOptions>,

    /// Garbage collection options for the compactions directory, which
    /// contains compactor job state `.compactions` files.
    ///
    /// None means garbage collection is disabled for the compactions directory.
    pub compactions_options: Option<GarbageCollectorDirectoryOptions>,

    /// Garbage collection options for detaching a clone from its parent database(s).
    ///
    /// When a clone no longer references any of a parent's SSTs (in its current
    /// manifest or any live checkpoint), the detach pass removes the pinning
    /// checkpoint from the parent and drops the external DB entry from the clone.
    ///
    /// None means detach is disabled.
    pub detach_options: Option<GarbageCollectorScheduleOptions>,

    /// Optional metrics reporting level for standalone garbage collectors. When
    /// a garbage collector is owned by a [`Settings`] configured DB, unset means
    /// inherit [`Settings::metric_level`].
    pub metric_level: Option<MetricLevel>,

    /// Whether manifest and compactions boundary files are advanced before deletion.
    ///
    /// Disable this only for object stores that do not support conditional overwrites (`If-Match`).
    /// Without boundary advancement, a SlateDB client or compactor can begin updating a manifest or
    /// compactions file, stop making progress (for example, because its process or host is
    /// suspended), then resume after the garbage collector's `min_age`. It can then recreate a
    /// deleted metadata ID and incorrectly report its stale update as successful. Set `min_age`
    /// longer than the maximum lifetime of a stale process, and use the same setting for every
    /// garbage collector operating on the database.
    #[serde(default = "default_true")]
    pub boundary_files_enabled: bool,

    /// Controls wrapper-level retries for this garbage collector's object-store
    /// operations. Defaults to unbounded retries.
    #[serde(default)]
    pub object_store_max_retries: Option<u32>,
}

impl GarbageCollectorOptions {
    pub fn is_empty(&self) -> bool {
        self.manifest_options.is_none()
            && self.wal_options.is_none()
            && self.wal_fence_options.is_none()
            && self.compacted_options.is_none()
            && self.compactions_options.is_none()
            && self.detach_options.is_none()
    }
}

/// Default options for the garbage collector for a directory.
///
/// By default, the garbage collector will run every 10 minutes and deletes files
/// that are at least 5 minutes old.
impl Default for GarbageCollectorDirectoryOptions {
    fn default() -> Self {
        Self {
            interval: Some(DEFAULT_INTERVAL),
            min_age: DEFAULT_MIN_AGE,
            dry_run: false,
        }
    }
}

/// Garbage collector options for a directory.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct GarbageCollectorDirectoryOptions {
    /// The interval at which the garbage collector will run in the background
    /// thread.
    ///
    /// If set to None while the parent directory options are enabled, recurring
    /// garbage collection uses the default interval. To disable garbage
    /// collection for a directory, set the parent `*_options` field to None.
    #[serde(deserialize_with = "deserialize_option_duration")]
    #[serde(serialize_with = "serialize_option_duration")]
    pub interval: Option<Duration>,

    /// The minimum age of a file before it can be garbage collected.
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(serialize_with = "serialize_duration")]
    pub min_age: Duration,

    /// If true, log files that would be deleted without deleting them.
    #[serde(default)]
    pub dry_run: bool,
}

/// Schedule options for a GC task that has no file-age threshold.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct GarbageCollectorScheduleOptions {
    /// The interval at which the task will run in the background thread.
    ///
    /// If set to None while the parent task options are enabled, recurring
    /// execution uses the default interval. To disable the task, set the parent
    /// options field to None.
    #[serde(deserialize_with = "deserialize_option_duration")]
    #[serde(serialize_with = "serialize_option_duration")]
    pub interval: Option<Duration>,
}

impl Default for GarbageCollectorScheduleOptions {
    fn default() -> Self {
        Self {
            interval: Some(DEFAULT_INTERVAL),
        }
    }
}

/// Default options for the garbage collector.
///
/// By default, garbage collection is enabled for all managed directories
/// (manifest, WAL, WAL fence, compacted SSTs, and compactions) using
/// [`GarbageCollectorDirectoryOptions::default()`].
/// WAL fence garbage collection runs in dry-run mode by default.
///
/// WAL fence GC is visible by default but does not delete until users
/// explicitly disable `dry_run`. This is a very conservative setting.
/// Users can enable fence GC with a high `min_age` if they want to
/// clean up old fences. Alternatively, this log can be silenced entirely
/// by setting `wal_fence_options` to `None`. See
/// [`GarbageCollectorOptions::wal_fence_options`] for more details.
///
/// To disable garbage collection for a specific file type, set that
/// directory option to `None`.
impl Default for GarbageCollectorOptions {
    fn default() -> Self {
        Self {
            manifest_options: Some(GarbageCollectorDirectoryOptions::default()),
            wal_options: Some(GarbageCollectorDirectoryOptions::default()),
            wal_fence_options: Some(GarbageCollectorDirectoryOptions {
                dry_run: true,
                ..GarbageCollectorDirectoryOptions::default()
            }),
            compacted_options: Some(GarbageCollectorDirectoryOptions::default()),
            compactions_options: Some(GarbageCollectorDirectoryOptions::default()),
            detach_options: Some(GarbageCollectorScheduleOptions::default()),
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        }
    }
}

/// Options for the object store cache. This cache is not enabled unless an explicit cache
/// root folder is set. The object store cache will split an object into align-sized parts
/// in the local, and save them into the local cache storage.
///
/// The local cache default uses file system as storage, it can also be extended to use other
/// like RocksDB, Redis, etc. in the future.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObjectStoreCacheOptions {
    /// The root folder where the cache files are stored. If not set, the cache will be
    /// disabled.
    pub root_folder: Option<std::path::PathBuf>,

    /// The limit of the cache size in bytes, the default value is 16gb on 64 bit systems and
    /// 4gb on 32 bit systems.
    pub max_cache_size_bytes: Option<usize>,

    /// The size of each part file, the part size is expected to be aligned with 1kb,
    /// its default value is 4mb.
    pub part_size_bytes: usize,

    /// Whether to cache compacted SSTs produced by memtable flushes to the
    /// local disk cache, for faster subsequent reads.
    ///
    /// Default is false.
    pub cache_on_flush: bool,

    /// Whether to cache compacted SSTs produced by compaction to the local
    /// disk cache, for faster subsequent reads.
    ///
    /// Default is false.
    pub cache_on_compaction: bool,

    /// Whether to preload SST files into cache during database startup. When enabled,
    /// the database will load SST files into the cache up to the cache size limit
    /// to warm up the cache for faster access. Default is None (no preloading).
    pub preload_disk_cache_on_startup: Option<PreloadLevel>,

    /// Interval to scan the cache directory to rebuild the in-memory map for evictor.
    /// The default value is 1 hour. If set to None, the cache directory will be only
    /// scanned once on start up.
    #[serde(deserialize_with = "deserialize_option_duration")]
    #[serde(
        serialize_with = "serialize_option_duration",
        skip_serializing_if = "Option::is_none"
    )]
    pub scan_interval: Option<Duration>,

    /// The maximum number of file handles to keep open in the file handle cache.
    /// When the limit is reached, the least recently used handle is closed.
    /// Default is 1000.
    pub max_open_file_handles: usize,
}

impl Default for ObjectStoreCacheOptions {
    fn default() -> Self {
        Self {
            root_folder: None,
            #[cfg(target_pointer_width = "32")]
            max_cache_size_bytes: Some(usize::MAX),
            #[cfg(not(target_pointer_width = "32"))]
            max_cache_size_bytes: Some(16 * 1024 * 1024 * 1024),
            part_size_bytes: 4 * 1024 * 1024,
            cache_on_flush: false,
            cache_on_compaction: false,
            preload_disk_cache_on_startup: None,
            scan_interval: Some(Duration::from_secs(3600)),
            max_open_file_handles: 1000,
        }
    }
}

// Custom serializer for Duration
fn serialize_duration<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    let duration_str = if secs > 0 && millis > 0 {
        format!("{secs}s+{millis:03}ms")
    } else if millis > 0 {
        format!("{millis:03}ms")
    } else {
        format!("{secs}s")
    };
    serializer.serialize_str(&duration_str)
}

// Custom serializer for Option<Duration>
fn serialize_option_duration<S>(
    duration: &Option<Duration>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match duration {
        Some(d) => serialize_duration(d, serializer),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_db_options_load_from_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SLATEDB_FLUSH_INTERVAL", "1s");
            jail.set_env(
                "SLATEDB_OBJECT_STORE_CACHE_OPTIONS.ROOT_FOLDER",
                "/tmp/slatedb-root",
            );

            let options =
                Settings::from_env("SLATEDB_").expect("failed to load db options from environment");
            assert_eq!(Some(Duration::from_secs(1)), options.flush_interval);
            assert_eq!(
                Some(PathBuf::from("/tmp/slatedb-root")),
                options.object_store_cache_options.root_folder
            );
            Ok(())
        });
    }

    #[test]
    fn test_db_options_load_metric_level_from_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SLATEDB_METRIC_LEVEL", "Debug");

            let options =
                Settings::from_env("SLATEDB_").expect("failed to load db options from environment");
            assert_eq!(MetricLevel::Debug, options.metric_level);

            Ok(())
        });
    }

    #[test]
    fn test_db_options_load_metric_level_case_insensitive() {
        for (value, expected) in [
            ("debug", MetricLevel::Debug),
            ("DEBUG", MetricLevel::Debug),
            ("Debug", MetricLevel::Debug),
        ] {
            figment::Jail::expect_with(|jail| {
                jail.set_env("SLATEDB_METRIC_LEVEL", value);

                let options = Settings::from_env("SLATEDB_")
                    .expect("failed to load db options from environment");
                assert_eq!(expected, options.metric_level);

                Ok(())
            });
        }
    }

    #[test]
    fn test_db_options_default_metric_level() {
        let options = Settings::default();
        assert_eq!(MetricLevel::default(), options.metric_level);
    }

    #[test]
    fn test_compactor_checkpoint_lifetime_config() {
        let default_lifetime = Duration::from_mins(15);
        assert_eq!(
            default_lifetime,
            CompactorOptions::default().checkpoint_lifetime
        );

        let mut value = serde_json::to_value(CompactorOptions::default()).unwrap();
        value.as_object_mut().unwrap().insert(
            "checkpoint_lifetime".to_string(),
            serde_json::Value::String("42s".to_string()),
        );
        let configured: CompactorOptions = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(Duration::from_secs(42), configured.checkpoint_lifetime);

        value.as_object_mut().unwrap().remove("checkpoint_lifetime");
        let omitted: CompactorOptions = serde_json::from_value(value).unwrap();
        assert_eq!(default_lifetime, omitted.checkpoint_lifetime);
    }

    #[test]
    fn test_gc_boundary_files_are_enabled_by_default_when_omitted() {
        fn without_boundary_setting<T: Serialize>(value: T) -> serde_json::Value {
            let mut value = serde_json::to_value(value).unwrap();
            value
                .as_object_mut()
                .unwrap()
                .remove("boundary_files_enabled");
            value
        }

        let gc: GarbageCollectorOptions =
            serde_json::from_value(without_boundary_setting(GarbageCollectorOptions::default()))
                .unwrap();

        assert!(gc.boundary_files_enabled);
    }

    #[test]
    fn test_db_options_load_from_json_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "config.json",
                r#"
{
    "flush_interval": "1s",
    "metric_level": "Debug",
     "object_store_cache_options": {
        "root_folder": "/tmp/slatedb-root"
    }
}
"#,
            )
            .expect("failed to create db options config file");

            let options = Settings::from_file("config.json")
                .expect("failed to load db options from environment");
            assert_eq!(Some(Duration::from_secs(1)), options.flush_interval);
            assert_eq!(MetricLevel::Debug, options.metric_level);
            assert_eq!(
                Some(PathBuf::from("/tmp/slatedb-root")),
                options.object_store_cache_options.root_folder
            );
            Ok(())
        });
    }

    #[test]
    fn test_db_options_env_with_default_respects_overrides() {
        figment::Jail::expect_with(|_jail| {
            let options = Settings::from_env_with_default(
                "SLATEDB_",
                Settings {
                    flush_interval: Some(Duration::from_millis(40)),
                    ..Default::default()
                },
            )
            .expect("failed to load db options from environment");

            assert_eq!(Some(Duration::from_millis(40)), options.flush_interval);

            Ok(())
        });
    }

    #[test]
    fn test_db_options_load_from_toml_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "config.toml",
                r#"
flush_interval = "1s"
metric_level = "Debug"
[object_store_cache_options]
root_folder = "/tmp/slatedb-root"
"#,
            )
            .expect("failed to create db options config file");

            let options = Settings::from_file("config.toml")
                .expect("failed to load db options from environment");
            assert_eq!(Some(Duration::from_secs(1)), options.flush_interval);
            assert_eq!(MetricLevel::Debug, options.metric_level);
            assert_eq!(
                Some(PathBuf::from("/tmp/slatedb-root")),
                options.object_store_cache_options.root_folder
            );
            Ok(())
        });
    }

    #[test]
    fn test_db_options_load_from_yaml_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "config.yaml",
                r#"
flush_interval: "1s"
metric_level: Debug
object_store_cache_options:
    root_folder: "/tmp/slatedb-root"
"#,
            )
            .expect("failed to create db options config file");

            let options = Settings::from_file("config.yaml")
                .expect("failed to load db options from environment");
            assert_eq!(Some(Duration::from_secs(1)), options.flush_interval);
            assert_eq!(MetricLevel::Debug, options.metric_level);
            assert_eq!(
                Some(PathBuf::from("/tmp/slatedb-root")),
                options.object_store_cache_options.root_folder
            );
            Ok(())
        });
    }

    #[test]
    fn test_db_options_load_with_default_locations() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SLATEDB_FLUSH_INTERVAL", "1s");

            jail.create_file(
                "SlateDb.yaml",
                r#"
object_store_cache_options:
    root_folder: "/tmp/slatedb-root"
"#,
            )
            .expect("failed to create db options config file");

            let options = Settings::load().expect("failed to load db options from environment");
            assert_eq!(Some(Duration::from_secs(1)), options.flush_interval);
            assert_eq!(
                Some(PathBuf::from("/tmp/slatedb-root")),
                options.object_store_cache_options.root_folder
            );
            Ok(())
        });
    }

    #[test]
    fn test_default_read_options() {
        let options = ReadOptions::default();
        assert_eq!(options.durability_filter, DurabilityLevel::Memory);
        assert!(!options.dirty);
        assert!(options.cache_blocks);

        let options = ScanOptions::default();
        assert_eq!(options.durability_filter, DurabilityLevel::Memory);
        assert!(!options.dirty);
        assert_eq!(options.read_ahead_bytes, 1);
        assert!(!options.cache_blocks);
        assert_eq!(options.max_fetch_tasks, 1);
    }

    #[test]
    fn test_default_tracing_options_are_none() {
        assert!(ReadOptions::default().tracing_options.is_none());
        assert!(ScanOptions::default().tracing_options.is_none());
    }

    #[test]
    fn test_scan_options_with_max_fetch_tasks() {
        let options = ScanOptions::default().with_max_fetch_tasks(4);
        assert_eq!(options.max_fetch_tasks, 4);

        // Verify other fields remain unchanged
        assert_eq!(options.durability_filter, DurabilityLevel::Memory);
        assert!(!options.dirty);
        assert_eq!(options.read_ahead_bytes, 1);
        assert!(!options.cache_blocks);
    }

    #[test]
    fn test_read_options_with_tracing_options() {
        let options =
            ReadOptions::default().with_tracing_options(Some(TracingOptions::new("read-trace")));
        assert_eq!(
            options
                .tracing_options
                .as_ref()
                .map(|tracing_options| tracing_options.trace_id.as_str()),
            Some("read-trace")
        );
    }

    #[test]
    fn test_scan_options_with_tracing_options() {
        let options =
            ScanOptions::default().with_tracing_options(Some(TracingOptions::new("scan-trace")));
        assert_eq!(
            options
                .tracing_options
                .as_ref()
                .map(|tracing_options| tracing_options.trace_id.as_str()),
            Some("scan-trace")
        );
    }

    #[test]
    fn test_size_tiered_compaction_scheduler_options_roundtrip() {
        let options = SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 3,
            max_compaction_sources: 9,
            include_size_threshold: 7.0,
            sorted_run_consolidation_threshold: 5,
        };

        let map: HashMap<String, String> = options.into();
        let roundtripped = SizeTieredCompactionSchedulerOptions::from(map);

        assert_eq!(roundtripped.min_compaction_sources, 3);
        assert_eq!(roundtripped.max_compaction_sources, 9);
        assert_eq!(roundtripped.include_size_threshold, 7.0);
        assert_eq!(roundtripped.sorted_run_consolidation_threshold, 5);
    }

    #[test]
    fn should_return_exact_timestamp_for_put_expire_at() {
        // given
        let opts = PutOptions {
            ttl: Ttl::ExpireAtMillis(12345),
        };

        // when
        let expire_ts = opts.expire_ts_from(None, 100);

        // then
        assert_eq!(expire_ts, Some(12345));
    }

    #[test]
    fn should_ignore_default_ttl_for_put_expire_at() {
        // given
        let opts = PutOptions {
            ttl: Ttl::ExpireAtMillis(12345),
        };

        // when
        let expire_ts = opts.expire_ts_from(Some(9999), 100);

        // then
        assert_eq!(expire_ts, Some(12345));
    }

    #[test]
    fn should_allow_past_timestamp_for_put_expire_at() {
        // given
        let opts = PutOptions {
            ttl: Ttl::ExpireAtMillis(50),
        };

        // when
        let expire_ts = opts.expire_ts_from(None, 100);

        // then: past timestamp is allowed (entry will expire on next read/compaction)
        assert_eq!(expire_ts, Some(50));
    }

    #[test]
    fn should_return_exact_timestamp_for_merge_expire_at() {
        // given
        let opts = MergeOptions {
            ttl: Ttl::ExpireAtMillis(12345),
        };

        // when
        let expire_ts = opts.expire_ts_from(None, 100);

        // then
        assert_eq!(expire_ts, Some(12345));
    }

    #[test]
    fn should_return_deterministic_expire_ts_for_expire_at() {
        // given: same ExpireAtMillis value used at different times
        let opts = PutOptions {
            ttl: Ttl::ExpireAtMillis(99999),
        };

        // when
        let ts1 = opts.expire_ts_from(None, 100);
        let ts2 = opts.expire_ts_from(None, 200);
        let ts3 = opts.expire_ts_from(None, 300);

        // then: all return the same absolute timestamp regardless of `now`
        assert_eq!(ts1, Some(99999));
        assert_eq!(ts2, Some(99999));
        assert_eq!(ts3, Some(99999));
    }

    #[test]
    fn test_validate_accepts_default_settings() {
        assert!(Settings::default().validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_zero_l0_flush_parallelism() {
        let settings = Settings {
            l0_flush_parallelism: 0,
            ..Settings::default()
        };
        let err = settings.validate().expect_err("expected invalid settings");
        assert!(err.to_string().contains("l0_flush_parallelism"));
    }

    #[test]
    fn test_validate_rejects_low_max_wal_flushes_before_l0_flush() {
        let settings = Settings {
            max_wal_flushes_before_l0_flush: 4095,
            ..Settings::default()
        };
        let err = settings.validate().expect_err("expected invalid settings");
        assert!(err.to_string().contains("max_wal_flushes_before_l0_flush"));
    }

    #[test]
    fn test_validate_rejects_max_unflushed_bytes_not_greater_than_l0_sst_size() {
        // Equal is invalid: must be strictly greater.
        let equal = Settings {
            l0_sst_size_bytes: 64 * 1024 * 1024,
            max_unflushed_bytes: 64 * 1024 * 1024,
            ..Settings::default()
        };
        let err = equal.validate().expect_err("expected invalid settings");
        assert!(err.to_string().contains("max_unflushed_bytes"));

        let smaller = Settings {
            l0_sst_size_bytes: 64 * 1024 * 1024,
            max_unflushed_bytes: 32 * 1024 * 1024,
            ..Settings::default()
        };
        assert!(smaller.validate().is_err());
    }

    #[test]
    fn test_validate_accepts_max_unflushed_bytes_greater_than_l0_sst_size() {
        let settings = Settings {
            l0_sst_size_bytes: 64 * 1024 * 1024,
            max_unflushed_bytes: 64 * 1024 * 1024 + 1,
            ..Settings::default()
        };
        assert!(settings.validate().is_ok());
    }
}
