use async_trait::async_trait;

use crate::error::SlateDBError;
use crate::types::RowEntry;

#[derive(Clone, Copy, Debug)]
pub enum IterationOrder {
    Ascending,
    Descending,
}

/// Note: this is intentionally its own trait instead of an Iterator<Item=KeyValue>,
/// because next will need to be made async to support SSTs, which are loaded over
/// the network.
/// See: https://github.com/slatedb/slatedb/issues/12

#[async_trait]
pub(crate) trait RowEntryIterator: Send + Sync {
    /// Performs any expensive initialization required before regular iteration.
    ///
    /// This method should be idempotent and can be called multiple times, only
    /// the first initialization should perform expensive operations.
    async fn init(&mut self) -> Result<(), SlateDBError>;

    /// Returns the next entry in the iterator, which may be a key-value pair or
    /// a tombstone of a deleted key-value pair.
    ///
    /// Will fail with `SlateDBError::IteratorNotInitialized` if the iterator is
    /// not yet initialized.
    ///
    /// NOTE: we don't initialize the iterator when calling next and instead
    /// require the caller to explicitly initialize the iterator. This is in order
    /// to ensure that optimizations which eagerly initialize the iterator are not
    /// lost in a refactor and instead would throw errors.
    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError>;

    /// Seek to the next (inclusive) key
    ///
    /// Will fail with `SlateDBError::IteratorNotInitialized` if the iterator is
    /// not yet initialized.
    ///
    /// NOTE: we don't initialize the iterator when calling seek and instead
    /// require the caller to explicitly initialize the iterator. This is in order
    /// to ensure that optimizations which eagerly initialize the iterator are not
    /// lost in a refactor and instead would throw errors.
    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError>;
}

/// Iterator trait that tracks bytes processed for progress reporting.
///
/// This extends `RowEntryIterator` with progress tracking capability.
/// Only iterators used in the compaction pipeline implement this trait.
/// The bottom-most iterator (MergeIterator) tracks actual bytes, while
/// wrapper iterators delegate to their inner iterator.
pub(crate) trait TrackedRowEntryIterator: RowEntryIterator {
    /// Returns the total bytes processed (key + value length) by this iterator.
    fn bytes_processed(&self) -> u64;
}

#[async_trait]
impl<'a> RowEntryIterator for Box<dyn RowEntryIterator + 'a> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        self.as_mut().init().await
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        self.as_mut().next().await
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        self.as_mut().seek(next_key).await
    }
}

#[async_trait]
impl<'a> RowEntryIterator for Box<dyn TrackedRowEntryIterator + 'a> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        self.as_mut().init().await
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        self.as_mut().next().await
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        self.as_mut().seek(next_key).await
    }
}

impl<'a> TrackedRowEntryIterator for Box<dyn TrackedRowEntryIterator + 'a> {
    fn bytes_processed(&self) -> u64 {
        self.as_ref().bytes_processed()
    }
}

pub(crate) struct EmptyIterator;

impl EmptyIterator {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RowEntryIterator for EmptyIterator {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        Ok(None)
    }

    async fn seek(&mut self, _next_key: &[u8]) -> Result<(), SlateDBError> {
        Ok(())
    }
}
