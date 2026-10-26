use async_trait::async_trait;

use crate::error::SlateDBError;
use crate::iter::{RowEntryIterator, TrackedRowEntryIterator};
use crate::types::RowEntry;

/// An iterator adapter that can peek at the next [`RowEntry`] without advancing.
pub(crate) struct PeekingIterator<T: RowEntryIterator> {
    iterator: T,
    peeked: Option<RowEntry>,
    has_peeked: bool,
}

impl<T: RowEntryIterator> PeekingIterator<T> {
    pub(crate) fn new(iterator: T) -> Self {
        Self {
            iterator,
            peeked: None,
            has_peeked: false,
        }
    }

    /// Peek at the next entry without advancing the iterator.
    ///
    /// Multiple calls to `peek` will return the same entry until `next` is called.
    ///
    /// ## Returns
    /// - `Ok(Some(&RowEntry))` if an entry is available.
    /// - `Ok(None)` if the iterator is exhausted.
    ///
    /// ## Errors
    /// - `SlateDBError::IteratorNotInitialized` if the iterator has not been initialized.
    /// - `SlateDBError`: any error returned by the underlying iterator.
    pub(crate) async fn peek(&mut self) -> Result<Option<&RowEntry>, SlateDBError> {
        if !self.has_peeked {
            self.peeked = self.iterator.next().await?;
            self.has_peeked = true;
        }
        Ok(self.peeked.as_ref())
    }
}

#[async_trait]
impl<T: RowEntryIterator> RowEntryIterator for PeekingIterator<T> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        self.iterator.init().await
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        if self.has_peeked {
            self.has_peeked = false;
            return Ok(self.peeked.take());
        }
        self.iterator.next().await
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        self.peeked = None;
        self.has_peeked = false;
        self.iterator.seek(next_key).await
    }
}

impl<T: TrackedRowEntryIterator> TrackedRowEntryIterator for PeekingIterator<T> {
    fn bytes_processed(&self) -> u64 {
        self.iterator.bytes_processed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestIterator;
    use crate::types::RowEntry;

    #[tokio::test]
    async fn peek_does_not_advance() {
        let iter = TestIterator::new()
            .with_entry(b"key1", b"value1", 1)
            .with_entry(b"key2", b"value2", 2);
        let mut iter = PeekingIterator::new(iter);

        iter.init().await.unwrap();

        let first_peek = iter.peek().await.unwrap().unwrap().clone();
        let second_peek = iter.peek().await.unwrap().unwrap().clone();
        assert_eq!(first_peek, second_peek);

        let first_next = iter.next().await.unwrap().unwrap();
        assert_eq!(first_next, RowEntry::new_value(b"key1", b"value1", 1));

        let second_next = iter.next().await.unwrap().unwrap();
        assert_eq!(second_next, RowEntry::new_value(b"key2", b"value2", 2));
    }

    #[tokio::test]
    async fn seek_clears_peeked_entry() {
        let iter = TestIterator::new()
            .with_entry(b"key1", b"value1", 1)
            .with_entry(b"key2", b"value2", 2);
        let mut iter = PeekingIterator::new(iter);

        iter.init().await.unwrap();
        let _ = iter.peek().await.unwrap();
        iter.seek(b"key2").await.unwrap();

        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next, RowEntry::new_value(b"key2", b"value2", 2));
        let peek = iter.peek().await.unwrap();
        assert!(peek.is_none());
    }

    #[tokio::test]
    async fn peek_on_empty_iterator_returns_none() {
        let iter = TestIterator::new();
        let mut iter = PeekingIterator::new(iter);

        iter.init().await.unwrap();
        assert!(iter.peek().await.unwrap().is_none());
        assert!(iter.next().await.unwrap().is_none());
    }
}
