use crate::compaction_filter::{CompactionFilter, CompactionFilterDecision};
use crate::error::SlateDBError;
use crate::iter::{RowEntryIterator, TrackedRowEntryIterator};
use crate::types::RowEntry;
use async_trait::async_trait;

/// Iterator that applies a compaction filter to entries during compaction.
pub(crate) struct CompactionFilterIterator<T: RowEntryIterator> {
    inner: T,
    filter: Box<dyn CompactionFilter>,
    full_input: bool,
    pending: Option<RowEntry>,
}

impl<T: RowEntryIterator> CompactionFilterIterator<T> {
    /// Creates a new CompactionFilterIterator.
    pub(crate) fn new(inner: T, filter: Box<dyn CompactionFilter>) -> Self {
        Self {
            inner,
            filter,
            full_input: false,
            pending: None,
        }
    }

    pub(crate) fn with_full_input(mut self, full_input: bool) -> Self {
        self.full_input = full_input;
        self
    }

    async fn apply_filter(&mut self, entry: RowEntry) -> Result<Option<RowEntry>, SlateDBError> {
        let decision = self.filter.filter(&entry).await?;

        match decision {
            CompactionFilterDecision::Keep => Ok(Some(entry)),
            CompactionFilterDecision::Drop => Ok(None),
            CompactionFilterDecision::Modify(new_value) => {
                // Clear expire_ts for tombstones, preserve for other values
                let expire_ts = if new_value.is_tombstone() {
                    None
                } else {
                    entry.expire_ts
                };
                Ok(Some(RowEntry {
                    key: entry.key,
                    value: new_value,
                    seq: entry.seq,
                    create_ts: entry.create_ts,
                    expire_ts,
                }))
            }
        }
    }
}

#[async_trait]
impl<T: RowEntryIterator> RowEntryIterator for CompactionFilterIterator<T> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        self.inner.init().await
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        loop {
            match match self.pending.take() {
                Some(entry) => Some(entry),
                None => self.inner.next().await?,
            } {
                Some(entry) => {
                    if let Some(filtered) = self.apply_filter(entry).await? {
                        return Ok(Some(filtered));
                    }
                    // Entry was dropped, get next
                }
                None => {
                    // End of iteration, call cleanup
                    self.filter.on_compaction_end().await?;
                    return Ok(None);
                }
            }
        }
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        if !self.full_input {
            return self.inner.seek(next_key).await;
        }
        // The filter must observe skipped rows even though they already have
        // durable output. Reconstruct its group state before resuming writes.
        loop {
            let entry = match self.pending.take() {
                Some(entry) => Some(entry),
                None => self.inner.next().await?,
            };
            let Some(entry) = entry else { return Ok(()) };
            if entry.key.as_ref() >= next_key {
                self.pending = Some(entry);
                return Ok(());
            }
            self.filter.filter(&entry).await?;
        }
    }
}

impl<T: TrackedRowEntryIterator> TrackedRowEntryIterator for CompactionFilterIterator<T> {
    fn bytes_processed(&self) -> u64 {
        self.inner.bytes_processed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compaction_filter::CompactionFilterError;
    use crate::types::ValueDeletable;
    use bytes::Bytes;
    use std::collections::VecDeque;

    struct MockIterator {
        entries: VecDeque<RowEntry>,
    }

    impl MockIterator {
        fn new(entries: Vec<RowEntry>) -> Self {
            Self {
                entries: entries.into(),
            }
        }
    }

    #[async_trait]
    impl RowEntryIterator for MockIterator {
        async fn init(&mut self) -> Result<(), SlateDBError> {
            Ok(())
        }

        async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
            Ok(self.entries.pop_front())
        }

        async fn seek(&mut self, _next_key: &[u8]) -> Result<(), SlateDBError> {
            Ok(())
        }
    }

    struct KeepAllFilter;

    #[async_trait]
    impl CompactionFilter for KeepAllFilter {
        async fn filter(
            &mut self,
            _entry: &RowEntry,
        ) -> Result<CompactionFilterDecision, CompactionFilterError> {
            Ok(CompactionFilterDecision::Keep)
        }

        async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
            Ok(())
        }
    }

    struct DropPrefixFilter {
        prefix: Vec<u8>,
    }

    #[async_trait]
    impl CompactionFilter for DropPrefixFilter {
        async fn filter(
            &mut self,
            entry: &RowEntry,
        ) -> Result<CompactionFilterDecision, CompactionFilterError> {
            if entry.key.starts_with(&self.prefix) {
                Ok(CompactionFilterDecision::Drop)
            } else {
                Ok(CompactionFilterDecision::Keep)
            }
        }

        async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
            Ok(())
        }
    }

    struct ModifyValueFilter {
        suffix: Vec<u8>,
    }

    #[async_trait]
    impl CompactionFilter for ModifyValueFilter {
        async fn filter(
            &mut self,
            entry: &RowEntry,
        ) -> Result<CompactionFilterDecision, CompactionFilterError> {
            if let Some(v) = entry.value.as_bytes() {
                let mut new_value = v.to_vec();
                new_value.extend_from_slice(&self.suffix);
                Ok(CompactionFilterDecision::Modify(ValueDeletable::Value(
                    Bytes::from(new_value),
                )))
            } else {
                Ok(CompactionFilterDecision::Keep)
            }
        }

        async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
            Ok(())
        }
    }

    /// A filter that converts entries to tombstones based on a condition.
    struct TombstoneFilter {
        /// Convert entries with keys starting with this prefix to tombstones.
        prefix: Vec<u8>,
    }

    #[async_trait]
    impl CompactionFilter for TombstoneFilter {
        async fn filter(
            &mut self,
            entry: &RowEntry,
        ) -> Result<CompactionFilterDecision, CompactionFilterError> {
            if entry.key.starts_with(&self.prefix) {
                Ok(CompactionFilterDecision::Modify(ValueDeletable::Tombstone))
            } else {
                Ok(CompactionFilterDecision::Keep)
            }
        }

        async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
            Ok(())
        }
    }

    fn make_entry(key: &[u8], value: &[u8], seq: u64) -> RowEntry {
        RowEntry {
            key: Bytes::copy_from_slice(key),
            value: ValueDeletable::Value(Bytes::copy_from_slice(value)),
            seq,
            create_ts: None,
            expire_ts: None,
        }
    }

    fn make_tombstone(key: &[u8], seq: u64) -> RowEntry {
        RowEntry {
            key: Bytes::copy_from_slice(key),
            value: ValueDeletable::Tombstone,
            seq,
            create_ts: None,
            expire_ts: None,
        }
    }

    fn make_merge(key: &[u8], value: &[u8], seq: u64) -> RowEntry {
        RowEntry {
            key: Bytes::copy_from_slice(key),
            value: ValueDeletable::Merge(Bytes::copy_from_slice(value)),
            seq,
            create_ts: None,
            expire_ts: None,
        }
    }

    fn make_entry_with_expire_ts(key: &[u8], value: &[u8], seq: u64, expire_ts: i64) -> RowEntry {
        RowEntry {
            key: Bytes::copy_from_slice(key),
            value: ValueDeletable::Value(Bytes::copy_from_slice(value)),
            seq,
            create_ts: None,
            expire_ts: Some(expire_ts),
        }
    }

    #[tokio::test]
    async fn test_keep_all_filter() {
        let entries = vec![
            make_entry(b"key1", b"value1", 1),
            make_entry(b"key2", b"value2", 2),
            make_entry(b"key3", b"value3", 3),
        ];
        let mock_iter = MockIterator::new(entries.clone());
        let mut iter = CompactionFilterIterator::new(mock_iter, Box::new(KeepAllFilter));
        iter.init().await.unwrap();

        let result1 = iter.next().await.unwrap();
        assert_eq!(result1, Some(entries[0].clone()));

        let result2 = iter.next().await.unwrap();
        assert_eq!(result2, Some(entries[1].clone()));

        let result3 = iter.next().await.unwrap();
        assert_eq!(result3, Some(entries[2].clone()));

        let result4 = iter.next().await.unwrap();
        assert_eq!(result4, None);
    }

    #[tokio::test]
    async fn test_drop_prefix_filter() {
        let entries = vec![
            make_entry(b"drop:key1", b"value1", 1),
            make_entry(b"keep:key2", b"value2", 2),
            make_entry(b"drop:key3", b"value3", 3),
            make_entry(b"keep:key4", b"value4", 4),
        ];
        let mock_iter = MockIterator::new(entries.clone());
        let filter = DropPrefixFilter {
            prefix: b"drop:".to_vec(),
        };
        let mut iter = CompactionFilterIterator::new(mock_iter, Box::new(filter));
        iter.init().await.unwrap();

        // Should skip drop:key1 and return keep:key2
        let result1 = iter.next().await.unwrap();
        assert_eq!(result1, Some(entries[1].clone()));

        // Should skip drop:key3 and return keep:key4
        let result2 = iter.next().await.unwrap();
        assert_eq!(result2, Some(entries[3].clone()));

        let result3 = iter.next().await.unwrap();
        assert_eq!(result3, None);
    }

    #[tokio::test]
    async fn test_modify_value_filter() {
        let entries = vec![
            make_entry(b"key1", b"value1", 1),
            make_tombstone(b"key2", 2), // Tombstone should be kept unchanged
        ];
        let mock_iter = MockIterator::new(entries.clone());
        let filter = ModifyValueFilter {
            suffix: b"_modified".to_vec(),
        };
        let mut iter = CompactionFilterIterator::new(mock_iter, Box::new(filter));
        iter.init().await.unwrap();

        // Value should be modified
        let result1 = iter.next().await.unwrap().unwrap();
        assert_eq!(result1.key, Bytes::from_static(b"key1"));
        assert_eq!(
            result1.value,
            ValueDeletable::Value(Bytes::from_static(b"value1_modified"))
        );

        // Tombstone should be unchanged (filter returns Keep for None value)
        let result2 = iter.next().await.unwrap().unwrap();
        assert_eq!(result2, entries[1]);

        let result3 = iter.next().await.unwrap();
        assert_eq!(result3, None);
    }

    #[tokio::test]
    async fn test_tombstone_decision() {
        let entries = vec![
            make_entry(b"delete:key1", b"value1", 1),
            make_entry(b"keep:key2", b"value2", 2),
            make_entry(b"delete:key3", b"value3", 3),
        ];
        let mock_iter = MockIterator::new(entries.clone());
        let filter = TombstoneFilter {
            prefix: b"delete:".to_vec(),
        };
        let mut iter = CompactionFilterIterator::new(mock_iter, Box::new(filter));
        iter.init().await.unwrap();

        // delete:key1 should be converted to tombstone
        let result1 = iter.next().await.unwrap().unwrap();
        assert_eq!(result1.key, Bytes::from_static(b"delete:key1"));
        assert!(result1.value.is_tombstone());
        assert_eq!(result1.seq, 1);

        // keep:key2 should be unchanged
        let result2 = iter.next().await.unwrap();
        assert_eq!(result2, Some(entries[1].clone()));

        // delete:key3 should be converted to tombstone
        let result3 = iter.next().await.unwrap().unwrap();
        assert_eq!(result3.key, Bytes::from_static(b"delete:key3"));
        assert!(result3.value.is_tombstone());
        assert_eq!(result3.seq, 3);

        let result4 = iter.next().await.unwrap();
        assert_eq!(result4, None);
    }

    #[tokio::test]
    async fn test_tombstone_on_merge_operand_converts_to_tombstone() {
        // When Tombstone is returned for a merge operand, it should be converted
        // to a tombstone, allowing the user to delete the entire key.
        let entries = vec![
            make_entry(b"delete:key1", b"value1", 1), // Value -> tombstone
            make_merge(b"delete:key2", b"merge2", 2), // Merge -> tombstone
            make_entry(b"keep:key3", b"value3", 3),   // Kept
        ];
        let mock_iter = MockIterator::new(entries.clone());
        let filter = TombstoneFilter {
            prefix: b"delete:".to_vec(),
        };
        let mut iter = CompactionFilterIterator::new(mock_iter, Box::new(filter));
        iter.init().await.unwrap();

        // delete:key1 (value) should be converted to tombstone
        let result1 = iter.next().await.unwrap().unwrap();
        assert_eq!(result1.key, Bytes::from_static(b"delete:key1"));
        assert!(result1.value.is_tombstone());

        // delete:key2 (merge) should also be converted to tombstone
        let result2 = iter.next().await.unwrap().unwrap();
        assert_eq!(result2.key, Bytes::from_static(b"delete:key2"));
        assert!(result2.value.is_tombstone());

        // keep:key3 should be unchanged
        let result3 = iter.next().await.unwrap().unwrap();
        assert_eq!(result3.key, Bytes::from_static(b"keep:key3"));
        assert!(!result3.value.is_tombstone());

        let result4 = iter.next().await.unwrap();
        assert_eq!(result4, None);
    }

    #[tokio::test]
    async fn test_on_compaction_end_called_on_completion() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        struct TrackingFilter {
            end_called: Arc<AtomicBool>,
        }

        #[async_trait]
        impl CompactionFilter for TrackingFilter {
            async fn filter(
                &mut self,
                _entry: &RowEntry,
            ) -> Result<CompactionFilterDecision, CompactionFilterError> {
                Ok(CompactionFilterDecision::Keep)
            }

            async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
                self.end_called.store(true, Ordering::SeqCst);
                Ok(())
            }
        }

        let entries = vec![
            make_entry(b"key1", b"value1", 1),
            make_entry(b"key2", b"value2", 2),
        ];
        let mock_iter = MockIterator::new(entries);
        let end_called = Arc::new(AtomicBool::new(false));
        let filter = TrackingFilter {
            end_called: end_called.clone(),
        };
        let mut iter = CompactionFilterIterator::new(mock_iter, Box::new(filter));
        iter.init().await.unwrap();

        // Consume all entries
        assert!(iter.next().await.unwrap().is_some());
        assert!(iter.next().await.unwrap().is_some());

        // on_compaction_end should not be called yet
        assert!(!end_called.load(Ordering::SeqCst));

        // This returns None and triggers on_compaction_end
        assert!(iter.next().await.unwrap().is_none());
        assert!(end_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_tombstone_clears_expire_ts() {
        // When an entry with expire_ts is converted to a tombstone,
        // the expire_ts should be cleared.
        let entries = vec![
            make_entry_with_expire_ts(b"delete:key1", b"value1", 1, 12345),
            make_entry_with_expire_ts(b"keep:key2", b"value2", 2, 67890),
        ];
        let mock_iter = MockIterator::new(entries);
        let filter = TombstoneFilter {
            prefix: b"delete:".to_vec(),
        };
        let mut iter = CompactionFilterIterator::new(mock_iter, Box::new(filter));
        iter.init().await.unwrap();

        // delete:key1 should be converted to tombstone with expire_ts cleared
        let result1 = iter.next().await.unwrap().unwrap();
        assert_eq!(result1.key, Bytes::from_static(b"delete:key1"));
        assert!(result1.value.is_tombstone());
        assert_eq!(result1.expire_ts, None); // expire_ts should be cleared

        // keep:key2 should be unchanged with expire_ts preserved
        let result2 = iter.next().await.unwrap().unwrap();
        assert_eq!(result2.key, Bytes::from_static(b"keep:key2"));
        assert!(!result2.value.is_tombstone());
        assert_eq!(result2.expire_ts, Some(67890)); // expire_ts preserved

        let result3 = iter.next().await.unwrap();
        assert_eq!(result3, None);
    }

    #[tokio::test]
    async fn test_modify_value_preserves_expire_ts() {
        // When an entry with expire_ts is modified to a new value,
        // the expire_ts should be preserved.
        let entries = vec![make_entry_with_expire_ts(b"key1", b"value1", 1, 12345)];
        let mock_iter = MockIterator::new(entries);
        let filter = ModifyValueFilter {
            suffix: b"_modified".to_vec(),
        };
        let mut iter = CompactionFilterIterator::new(mock_iter, Box::new(filter));
        iter.init().await.unwrap();

        let result = iter.next().await.unwrap().unwrap();
        assert_eq!(result.key, Bytes::from_static(b"key1"));
        assert_eq!(
            result.value,
            ValueDeletable::Value(Bytes::from_static(b"value1_modified"))
        );
        assert_eq!(result.expire_ts, Some(12345)); // expire_ts preserved
    }

    #[tokio::test]
    async fn test_filter_error_aborts_iteration() {
        /// A filter that fails on entries with keys starting with "fail:".
        struct FailingFilter;

        #[async_trait]
        impl CompactionFilter for FailingFilter {
            async fn filter(
                &mut self,
                entry: &RowEntry,
            ) -> Result<CompactionFilterDecision, CompactionFilterError> {
                if entry.key.starts_with(b"fail:") {
                    Err(CompactionFilterError::FilterError(
                        "intentional failure".into(),
                    ))
                } else {
                    Ok(CompactionFilterDecision::Keep)
                }
            }

            async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
                Ok(())
            }
        }

        let entries = vec![
            make_entry(b"keep:key1", b"value1", 1),
            make_entry(b"fail:key2", b"value2", 2),
            make_entry(b"keep:key3", b"value3", 3),
        ];
        let mock_iter = MockIterator::new(entries.clone());
        let mut iter = CompactionFilterIterator::new(mock_iter, Box::new(FailingFilter));
        iter.init().await.unwrap();

        // First entry should succeed
        let result1 = iter.next().await.unwrap();
        assert_eq!(result1, Some(entries[0].clone()));

        // Second entry should fail with filter error
        let result2 = iter.next().await;
        assert!(result2.is_err());
        let err = result2.unwrap_err();
        assert!(matches!(
            err,
            SlateDBError::CompactionFilterError(ref e) if matches!(e.as_ref(), CompactionFilterError::FilterError(_))
        ));
    }
}

#[cfg(test)]
#[path = "../tests/unit/compaction_filter_resume.rs"]
mod compaction_filter_resume;
