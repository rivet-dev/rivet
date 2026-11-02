use crate::error::SlateDBError;
use crate::manifest::store::FenceableManifest;
use crate::{CloseReason, ErrorKind, RowEntry, VersionedManifest};
use async_trait::async_trait;
use futures::future::BoxFuture;
use object_store::path::Path;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::ops::{Bound, Range, RangeFrom};
use std::sync::Arc;
use std::time::Duration;

pub(crate) mod slatedb;
#[cfg(test)]
pub(crate) mod test_utils;
pub(crate) mod wal_disabled;

pub use crate::wal::slatedb::reader::{
    SlateDbWalReader, SlateDbWalReaderBuilder, SlateDbWalReaderOptions,
};

/// A range of WAL File IDs
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalFileRange(pub Bound<u64>, pub Bound<u64>);

impl From<Range<u64>> for WalFileRange {
    fn from(range: Range<u64>) -> Self {
        WalFileRange(Bound::Included(range.start), Bound::Excluded(range.end))
    }
}

impl From<RangeFrom<u64>> for WalFileRange {
    fn from(range: RangeFrom<u64>) -> Self {
        WalFileRange(Bound::Included(range.start), Bound::Unbounded)
    }
}

impl TryFrom<WalFileRange> for Range<u64> {
    type Error = ();

    fn try_from(range: WalFileRange) -> Result<Self, Self::Error> {
        match (range.0, range.1) {
            (Bound::Included(start), Bound::Excluded(end)) => Ok(start..end),
            _ => Err(()),
        }
    }
}

/// Defines the types of errors that can be returned by WAL implementations.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum WalError {
    /// The WAL writer was fenced
    Fenced,
    /// A WalIterator observed that the tail of the WAL was truncated while iterating.
    WalTruncated(u64),
    /// Operation against wal after it was closed
    Closed,
    /// WAL is unavailable, e.g. due to an I/O error or error in the backing storage system
    Unavailable(Arc<dyn Error + Sync + Send + 'static>),
    /// WAL implementation detected invalid data/corruption
    DataError(Arc<dyn Error + Sync + Send + 'static>),
    /// Indicates that the WAL is in some unexpected/unrecoverable state.
    InternalError(Arc<dyn Error + Sync + Send + 'static>),
}

impl Display for WalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WalError::Fenced => write!(f, "WAL writer was fenced"),
            WalError::WalTruncated(wal_id) => write!(f, "WAL was truncated at file {}", *wal_id),
            WalError::Closed => write!(f, "WAL is closed"),
            WalError::Unavailable(source) => write!(f, "WAL is unavailable: {source}"),
            WalError::DataError(source) => write!(f, "WAL data error: {source}"),
            WalError::InternalError(source) => write!(f, "WAL internal error: {source}"),
        }
    }
}

impl Error for WalError {}

/// The writer's manifest after fencing. Created by calling [`ManifestFencer::fence`]
pub struct WriterManifest {
    manifest: FenceableManifest,
}

impl From<WriterManifest> for FenceableManifest {
    fn from(manifest: WriterManifest) -> Self {
        manifest.manifest
    }
}

impl From<FenceableManifest> for WriterManifest {
    fn from(manifest: FenceableManifest) -> Self {
        WriterManifest { manifest }
    }
}

impl WriterManifest {
    /// Returns the current manifest.
    pub fn manifest(&self) -> VersionedManifest {
        let (id, manifest) = self.manifest.manifest();
        VersionedManifest::from_manifest(id, manifest.clone())
    }

    /// Returns the WAL ID up to which SlateDB has guaranteed to have stored all data in the
    /// LSM tree.
    pub fn replay_after_wal_id(&self) -> u64 {
        self.manifest().core().replay_after_wal_id
    }

    /// Returns the writer's epoch
    pub fn epoch(&self) -> u64 {
        self.manifest().writer_epoch()
    }

    /// Refreshes the current manifest. Implementations of `WriterInit::fence_and_init` can
    /// use this to detect whether the manifest has been fenced while executing the fencing
    /// protocol. SlateDB will call this after calling [`WriterInit::fence_and_init`]
    pub async fn refresh(&mut self) -> Result<(), WalError> {
        self.manifest.refresh().await?;
        Ok(())
    }
}

/// The result returned by [`WriterInit::fence_and_init`]
pub struct WriterInitResult {
    /// An iterator that returns writes that must be replayed before starting SlateDB to recover
    /// data from the WAL.
    pub replay_iterator: Box<dyn WalIterator>,
    /// The WAL writer that will be used to append new writes to the WAL
    pub wal_writer: Box<dyn WalWriter>,
}

/// API for fencing and initializing a new WAL writer for use by [`crate::db::Db`]. SlateDB requires
/// WAL implementations to execute a fencing protocol that guarantees (1) that earlier writers no
/// longer write to the db and (2) all rows present in the WAL but not in the LSM tree (L0 and
/// sorted runs) are recovered.
///
/// Every [`crate::db::Db`] instance is assigned a unique `u64` epoch. The epoch is assigned when
/// fencing the Manifest. A given Db instance writes both the WAL and its Manifest (e.g. with new
/// SSTs) independently. The fencing protocol that yields epoch E must ensure that:
/// (1) After the first write to the Manifest with epoch E, there are no further writes to either
///     the Manifest or WAL with epoch E' < E
/// (2) After the first write to the WAL with epoch E, there are no further writes to either the
///     Manifest or WAL with epoch E' < E
/// (3) All rows from the WAL from writers with epoch E' < E that are not present in L0/SRs are
///     replayed before serving reads/writes.
///
/// `[WriterInit::fence_and_init]` is responsible for
/// (1) Fencing the WAL such that no writers with an epoch earlier than [`WriterManifest::epoch`]
/// (2) Constructing a [`WalWriter`] instance that the writer uses to append new WAL entries.
/// (3) Resolving the end of the WAL and constructing a [`WalReplayIterator`] that returns all
///     rows in WAL files between [`WriterManifest::replay_after_wal_id`] (exclusive) and the
///     current end of the WAL.
#[async_trait]
pub trait WriterInit: Send + Sync + 'static {
    /// Fences the WAL and returns a [`WriterInitResult`] with a [`WalWriter`] and
    /// [`WalReplayIterator`] used to recover writes that have not yet been flushed to the tree.
    async fn fence_and_init(
        &self,
        manifest: &mut WriterManifest,
    ) -> Result<WriterInitResult, WalError>;
}

/// Describes the current status of the WAL
#[derive(Debug, Clone)]
pub struct WalStatus {
    /// Set to Some if the WAL has permanently shut down, along with the reason. The reason should
    /// be [`WalError::Closed`] on a normal shutdown, and some other [`WalError`] variant on
    /// failure.
    pub closed_reason: Option<WalError>,
    /// The estimated in-memory bytes used by the WAL to buffer unflushed writes.
    pub estimated_bytes: usize,
    /// The id of the last WAL file that was durably flushed
    pub last_flushed_wal_id: u64,
    /// The last sequence number that was durably flushed
    pub last_flushed_seq: Option<u64>,
    /// The number of writes currently buffered
    #[allow(dead_code)]
    pub buffered_wal_entries_count: usize,
}

/// An event emitted by a [`WalWriter`] to subscribers.
#[derive(Debug, Clone)]
pub enum WalEvent {
    /// Emitted when a WAL file is durably flushed to storage. On receipt of this event, SlateDB
    /// advances the durable sequence number and notifies durability waiters.
    WalFlushed(WalStatus),
    /// Emitted when the WAL has closed with the final wal status containing the closed reason
    WalClosed(WalStatus),
}

/// A listener that's called back on WAL events.
pub type WalStatusListener = Arc<dyn Fn(WalEvent) + Send + Sync + 'static>;

/// An observer that can read the current [`WalStatus`] and subscribe to event callbacks.
#[async_trait]
pub trait WalObserver: Send + Sync + 'static {
    /// Returns the current [`WalStatus`].
    fn status(&self) -> Result<WalStatus, WalStatus>;

    /// Adds a listener that subscribes to event callbacks.
    fn subscribe(&self, listener: WalStatusListener) -> Result<(), WalError>;
}

pub type FlushResultFuture = BoxFuture<'static, Result<(), WalError>>;

/// The WAL's write API. Used by SlateDB to append new WAL writes. Is returned by
/// [`WalWriterInit::fence_and_init_writer`].
///
/// Each call to [`WalWriter::append`] takes a single SlateDB write batch, where all rows share
/// the same sequence number ([`RowEntry::seq`]). [`WalWriter`] (optionally accumulates/buffers
/// rows and) writes consecutive write batches into consecutive WAL Files, where each WAL File
/// contains some rows from the total sequence of rows. Specifically:
/// - WAL Files must have a total order and each WAL File must have a u64 id that is greater than
///   all earlier WAL Files.
/// - Reading WAL Files in order should yield rows in sequence order.
/// - The writes in a given write batch must be written to WAL files atomically. That is, a
///   [`WalIterator`] should either observe all the writes with a given sequence number or none
///   of them.
#[async_trait]
pub trait WalWriter: Send {
    /// Append a write batch to the WAL.
    async fn append(&mut self, write_batch: &[RowEntry]) -> Result<(), WalError>;

    /// Triggers a flush of all appended write batches to durable storage. Returns a
    /// future that receives the result of the flush once it completes.
    async fn flush(&mut self) -> Result<FlushResultFuture, WalError>;

    /// Returns true if the WAL implementation wants to request that the current in-memory
    /// writes be flushed to a new l0. WAL implementations can use this to (1) bound the range
    /// of writes that need to be replayed when SlateDB restarts, and (2) push data to L0s earlier
    /// so that it's available to readers, which poll the latest manifest.
    ///
    /// ## Arguments
    /// - `replay_after_wal_id`: The WAL ID used as the replay point for the last memtable
    ///   that was flushed to L0
    fn should_flush_memtable(&self, _replay_after_wal_id: u64) -> bool {
        false
    }

    /// Returns a `WalObserver` for reading [`WalStatus`] and subscribing to events.
    fn observer(&self) -> Box<dyn WalObserver>;

    /// Returns the current `WalStatus`. If the [`WalWriter`] has failed, then returns Err with the
    /// final [`WalStatus`] and the reason for the failure in [`WalStatus::closed_reason`]. This
    /// allows callers to observe the final status after the [`WalWriter`] has shut down, e.g. to
    /// read the last flushed wal file or sequence number.
    fn status(&self) -> Result<WalStatus, WalStatus>;

    /// Close the `WalWriter` and release resources
    async fn close(&mut self) -> Result<(), WalError>;
}

/// Rows returned by [`WalIterator`]
#[derive(Clone)]
pub struct WalRows {
    /// The rows read from the WAL File. All the rows with a given sequence number must be present
    /// in th same [`WalRows`].
    pub rows: Vec<RowEntry>,
    /// The id of the last WAL File for which all rows have been consumed by the iterator and
    /// returned wither in this [`WalRows`] or a [`WalRows`] returned by an earlier call to
    /// [`WalIterator::next`]
    pub last_consumed_wal_file_id: u64,
}

/// An iterator over rows in some range of the WAL
#[async_trait]
pub trait WalIterator: Send + 'static {
    /// Returns the next set of rows. Rows must be returned in sequence and WAL File order.
    /// Returns None when iterator's range is exhausted. Iterators created using an unbounded
    /// end range that have exhausted the current WAL block until new rows are appended and never
    /// return `None`.
    /// Returns [`WalError::WalTruncated`] if the iterator observes that the WAL was truncated
    /// while iterating.
    async fn next(&mut self) -> Result<Option<WalRows>, WalError>;
}

/// API for reading from the WAL. Used by the Reader/
#[async_trait]
pub trait WalReader: Send + Sync + 'static {
    /// Returns an iterator over the specified range of WAL File IDs. The start of the range must
    /// not be `Unbounded`. If the end of the range is `Unbounded` then the returned iterator
    /// continues returning writes as new writes are appended to the WAL. Otherwise, it returns
    /// `None` upon reaching the end of the range.
    async fn iterator(
        &self,
        wal_file_id_range: WalFileRange,
    ) -> Result<Box<dyn WalIterator>, WalError>;

    /// Returns the ID of the last WAL file currently present after `replay_after_wal_id`, or
    /// `replay_after_wal_id` if no later WAL file is present. Implementations may use
    /// `replay_after_wal_id` as a known lower bound when locating the end of the WAL.
    async fn last_wal_file_id(&self, replay_after_wal_id: u64) -> Result<u64, WalError>;
}

/// Trait that defines the contract between SlateDB's garbage collector and a custom WAL
/// implementation. SlateDB tracks the set of currently referenced WAL ranges in its manifest.
/// When the Garbage Collector runs, it computes this set and calls [`WalGc::collect`] so that
/// the implementation can clean up any un-referenced WAL storage.
#[async_trait]
pub trait WalGc: Send + Sync + 'static {
    /// Hook for garbage collecting the WAL. Takes a list of ranges of WAL Files that are currently
    /// referenced by some active Manifest. The implementation may delete any WAL File that is not
    /// included in the ranges in this list.
    async fn collect(
        &self,
        referenced_ranges: Vec<WalFileRange>,
        min_age: Duration,
        dry_run: bool,
    ) -> Result<(), WalError>;
}

/// Administrative operations for a WAL implementation.
#[async_trait]
pub trait WalAdmin: Send + Sync + 'static {
    /// Creates a garbage collector scoped to the WAL at `path`.
    ///
    /// ## Arguments
    /// - `path`: The database path whose WAL should be garbage collected.
    ///
    /// ## Returns
    /// A garbage collector that can remove unreferenced WAL files at `path`.
    fn garbage_collector(&self, path: &Path) -> Arc<dyn WalGc>;

    /// Deletes the WAL at `path`.
    ///
    /// ## Arguments
    /// - `path`: The database path whose WAL should be deleted.
    /// - `dry_run`: If set to true, the implementation should just return the list of resources
    ///              that would be deleted without actually deleting anything.
    ///
    /// ## Returns
    /// `Ok(resources)` after the WAL has been deleted, or a [`WalError`] if deletion fails, where
    /// `resources` is a list of descriptions of resources that were deleted by this fn. This
    /// list is used to display the output of deleting the WAL (e.g. in logs or tool output)
    async fn delete_wal(&self, path: &Path, dry_run: bool) -> Result<Vec<String>, WalError>;

    /// Given a path and WAL ID range, returns true if the WAL at that path is empty within the
    /// specified range. A WAL is empty if it holds no records.
    ///
    /// ## Arguments
    /// - `path`: The database path containing the WAL.
    /// - `replay_after_wal_id`: The exclusive lower bound of the WAL range to inspect.
    /// - `wal_id_last_seen`: The inclusive upper bound of the WAL range to inspect.
    ///
    /// ## Returns
    /// `Ok(true)` if the referenced WAL contains no records, `Ok(false)` if it contains records,
    /// or a [`WalError`] if the WAL could not be inspected.
    async fn is_empty(
        &self,
        path: &Path,
        replay_after_wal_id: u64,
        wal_id_last_seen: u64,
    ) -> Result<bool, WalError>;

    /// Given a source path and manifest, copy the referenced WAL to a destination path and return
    /// a replay range. This call must be idempotent (TODO: clarify)
    ///
    /// ## Arguments
    /// - `from_path`: The db path that holds the source WAL range to be copied
    /// - `from_manifest`: The source manifest that identifies the WAL to copy
    /// - `to_path`: The db path of the clone that the WAL is being copied to.
    ///
    /// ## Returns
    /// A (u64, u64) pair. The first item will be used as the replay start point (exclusive). The
    /// second item should be the id of the last WAL file id in the copied WAL.
    async fn clone_wal(
        &self,
        from_path: &Path,
        from_manifest: VersionedManifest,
        to_path: &Path,
    ) -> Result<(u64, u64), WalError>;
}

impl From<WalStatus> for WalError {
    fn from(status: WalStatus) -> Self {
        status
            .closed_reason
            .expect("unexpected conversion of wal status with no error")
    }
}

impl From<WalStatus> for SlateDBError {
    fn from(status: WalStatus) -> Self {
        WalError::from(status).into()
    }
}

impl From<SlateDBError> for WalError {
    fn from(value: SlateDBError) -> Self {
        let public: crate::Error = value.clone().into();
        match public.kind() {
            ErrorKind::Closed(CloseReason::Fenced) => WalError::Fenced,
            ErrorKind::Closed(CloseReason::Clean) => WalError::Closed,
            ErrorKind::Closed(_) => WalError::InternalError(Arc::new(value)),
            ErrorKind::Unavailable => WalError::Unavailable(Arc::new(value)),
            ErrorKind::Invalid => WalError::InternalError(Arc::new(value)),
            ErrorKind::Data => WalError::DataError(Arc::new(value)),
            ErrorKind::Internal => WalError::InternalError(Arc::new(value)),
            ErrorKind::Transaction => WalError::InternalError(Arc::new(value)),
        }
    }
}

impl From<WalError> for SlateDBError {
    fn from(value: WalError) -> Self {
        match value {
            WalError::Fenced => SlateDBError::Fenced,
            WalError::WalTruncated(wal_id) => SlateDBError::WalTruncated(wal_id),
            WalError::Closed => SlateDBError::Closed,
            WalError::Unavailable(err) => SlateDBError::WalUnavailable(err),
            WalError::DataError(err) => SlateDBError::WalDataError(err),
            WalError::InternalError(err) => SlateDBError::WalInternalError(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WalError;
    use std::sync::Arc;

    #[test]
    fn wal_error_display() {
        let source = || {
            Arc::new(std::io::Error::other("source error"))
                as Arc<dyn std::error::Error + Send + Sync + 'static>
        };

        assert_eq!(WalError::Fenced.to_string(), "WAL writer was fenced");
        assert_eq!(
            WalError::WalTruncated(123).to_string(),
            "WAL was truncated at file 123"
        );
        assert_eq!(WalError::Closed.to_string(), "WAL is closed");
        assert_eq!(
            WalError::Unavailable(source()).to_string(),
            "WAL is unavailable: source error"
        );
        assert_eq!(
            WalError::DataError(source()).to_string(),
            "WAL data error: source error"
        );
        assert_eq!(
            WalError::InternalError(source()).to_string(),
            "WAL internal error: source error"
        );
    }
}
