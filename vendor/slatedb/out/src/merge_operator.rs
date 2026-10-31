use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use slatedb_common::metrics::CounterFn;
use thiserror::Error;

use crate::{
    error::SlateDBError,
    iter::{RowEntryIterator, TrackedRowEntryIterator},
    types::{RowEntry, ValueDeletable},
    utils::merge_options,
};

#[non_exhaustive]
#[derive(Clone, Debug, Error)]
pub enum MergeOperatorError {
    #[error("merge_batch called with empty operands and no existing value")]
    EmptyBatch,

    #[error("{message}")]
    Callback { message: String },
}

/// A trait for implementing custom merge operations in SlateDB.
///
/// The MergeOperator allows applications to bypass the traditional read/modify/update cycle
/// in performance-critical situations where computation can be expressed using an associative operator.
/// This is particularly useful for implementing:
/// - Aggregations (e.g., counters, sums)
/// - Buffering (e.g., append-only lists)
///
/// # Associativity Requirement
/// The merge operation MUST be associative, meaning that for any values a, b, and c:
/// merge(merge(a, b), c) == merge(a, merge(b, c))
///
/// # Examples
/// Here's an example of a counter merge operator:
/// ```
/// use bytes::Bytes;
/// use slatedb::{MergeOperator, MergeOperatorError};
///
/// struct CounterMergeOperator;
///
/// impl MergeOperator for CounterMergeOperator {
///     fn merge(&self, _key: &Bytes, existing_value: Option<Bytes>, operand: Bytes) -> Result<Bytes, MergeOperatorError> {
///         let existing = existing_value
///             .map(|v| u64::from_le_bytes(v.as_ref().try_into().unwrap()))
///             .unwrap_or(0);
///         let increment = u64::from_le_bytes(operand.as_ref().try_into().unwrap());
///         Ok(Bytes::copy_from_slice(&(existing + increment).to_le_bytes()))
///     }
///
///     fn merge_batch(&self, _key: &Bytes, existing_value: Option<Bytes>, operands: &[Bytes]) -> Result<Bytes, MergeOperatorError> {
///         let mut total = existing_value
///             .map(|v| u64::from_le_bytes(v.as_ref().try_into().unwrap()))
///             .unwrap_or(0);
///
///         for operand in operands {
///             let increment = u64::from_le_bytes(operand.as_ref().try_into().unwrap());
///             total += increment;
///         }
///
///         Ok(Bytes::copy_from_slice(&total.to_le_bytes()))
///     }
/// }
/// ```
pub trait MergeOperator {
    /// Merges the existing value with a new value to produce a combined result.
    ///
    /// This method is called during reads and compactions to combine multiple merge operands
    /// into a single value. The implementation must be associative to ensure correct behavior.
    ///
    /// # Arguments
    /// * `key` - The key of the entry
    /// * `existing_value` - The current accumulated value
    /// * `value` - The new value to merge with the existing value
    ///
    /// # Returns
    /// * `Ok(Bytes)` - The merged result as bytes
    /// * `Err(MergeOperatorError)` - If the merge operation fails
    fn merge(
        &self,
        key: &Bytes,
        existing_value: Option<Bytes>,
        value: Bytes,
    ) -> Result<Bytes, MergeOperatorError>;

    /// Merges a batch of operands with an optional existing value.
    ///
    /// This method allows for more efficient batch processing of merge operands.
    /// The default implementation applies pairwise merging, but implementations
    /// can override this for better performance (e.g., a counter can sum all values at once).
    ///
    /// # Arguments
    /// * `key` - The key of the entry
    /// * `existing_value` - The current accumulated value (if any)
    /// * `operands` - A slice of operands to merge, ordered from oldest to newest
    ///
    /// # Returns
    /// * `Ok(Bytes)` - The merged result as bytes
    /// * `Err(MergeOperatorError)` - If the merge operation fails
    fn merge_batch(
        &self,
        key: &Bytes,
        existing_value: Option<Bytes>,
        operands: &[Bytes],
    ) -> Result<Bytes, MergeOperatorError> {
        let mut result = existing_value;
        for operand in operands {
            result = Some(self.merge(key, result, operand.clone())?);
        }
        result.ok_or(MergeOperatorError::EmptyBatch)
    }
}

pub(crate) type MergeOperatorType = Arc<dyn MergeOperator + Send + Sync>;

/// Counter for merge operands resolved across all paths.
///
/// Incremented on each successful `merge_batch` call by the number of
/// operands in that call. Because merge resolution batches operands and then
/// re-merges intermediate results, the counter can exceed the number of raw
/// user-written merge operands.
pub const MERGE_OPERATOR_OPERANDS: &str = "slatedb.merge_operator_operands";
pub(crate) const MERGE_OPERATOR_OPERANDS_DESCRIPTION: &str = "Merge operator operands resolved";
pub(crate) const MERGE_OPERATOR_PATH_LABEL: &str = "path";
pub(crate) const MERGE_OPERATOR_READ_PATH: &str = "read";
pub(crate) const MERGE_OPERATOR_FLUSH_PATH: &str = "flush";
pub(crate) const MERGE_OPERATOR_COMPACT_PATH: &str = "compact";

pub(crate) fn instrument_merge_operator(
    merge_operator: MergeOperatorType,
    merge_batch_operands: Arc<dyn CounterFn>,
) -> MergeOperatorType {
    Arc::new(InstrumentedMergeOperator {
        merge_operator,
        merge_batch_operands,
    })
}

struct InstrumentedMergeOperator {
    merge_operator: MergeOperatorType,
    merge_batch_operands: Arc<dyn CounterFn>,
}

impl MergeOperator for InstrumentedMergeOperator {
    fn merge(
        &self,
        key: &Bytes,
        existing_value: Option<Bytes>,
        value: Bytes,
    ) -> Result<Bytes, MergeOperatorError> {
        // We count operands in `merge_batch` because that is the production API.
        // The default `merge_batch` implementation already calls `merge` once per
        // operand, so counting here would double-count the default path.
        self.merge_operator.merge(key, existing_value, value)
    }

    fn merge_batch(
        &self,
        key: &Bytes,
        existing_value: Option<Bytes>,
        operands: &[Bytes],
    ) -> Result<Bytes, MergeOperatorError> {
        let result = self
            .merge_operator
            .merge_batch(key, existing_value, operands)?;
        // This counts the exact operand slices that successfully reach
        // `merge_batch`, including intermediate results produced by batched
        // merge resolution.
        self.merge_batch_operands.increment(operands.len() as u64);
        Ok(result)
    }
}

/// Maximum number of merge operands to collect before calling `merge_batch`.
///
/// This controls the streaming batch size when merging entries with the same key.
/// The iterator collects up to this many merge operands, then calls `merge_batch`
/// to combine them into a single intermediate result. This process repeats until
/// all operands are processed or a base value is found.
///
/// Benefits of batching:
/// - Reduces function call overhead (N calls → N/BATCH_SIZE calls)
/// - Enables batch-optimized merge implementations (e.g., sum all at once)
/// - Reduces intermediate memory allocations
///
/// Trade-offs:
/// - Larger batches: Better performance, higher peak memory per key
/// - Smaller batches: Lower memory usage, more function calls
///
/// TODO: Make this configurable based on system memory constraints
const MERGE_BATCH_SIZE: usize = 100;

/// An iterator that ensures merge operands are not returned when no merge operator is configured.
pub(crate) struct MergeOperatorRequiredIterator<T: RowEntryIterator> {
    delegate: T,
}

impl<T: RowEntryIterator> MergeOperatorRequiredIterator<T> {
    pub(crate) fn new(delegate: T) -> Self {
        Self { delegate }
    }
}

#[async_trait]
impl<T: RowEntryIterator> RowEntryIterator for MergeOperatorRequiredIterator<T> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        self.delegate.init().await
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        let next = self.delegate.next().await?;
        if let Some(entry) = next {
            match &entry.value {
                ValueDeletable::Merge(_) => {
                    return Err(SlateDBError::MergeOperatorMissing);
                }
                _ => return Ok(Some(entry)),
            }
        }
        Ok(None)
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        self.delegate.seek(next_key).await
    }
}

impl<T: TrackedRowEntryIterator> TrackedRowEntryIterator for MergeOperatorRequiredIterator<T> {
    fn bytes_processed(&self) -> u64 {
        self.delegate.bytes_processed()
    }
}

/// An iterator that merges mergeable entries into a single value.
///
/// It is expected that this is the top level iterator in a merge scan, and therefore
/// return a ValueDeletable::Value entry (instead of a Merge even if the resolved value
/// is a merge operand).
pub(crate) struct MergeOperatorIterator<T: RowEntryIterator> {
    merge_operator: MergeOperatorType,
    delegate: T,
    /// Entry from the delegate that we've peeked ahead and buffered.
    buffered_entry: Option<RowEntry>,
    /// Whether to merge entries with different expire timestamps.
    merge_different_expire_ts: bool,
    /// A barrier sequence number that supports snapshot reads using this iterator. If not None,
    /// the iterator will not merge entries with sequence number greater than this value.
    snapshot_barrier_seq: Option<u64>,
}

/// Tracks metadata across multiple entries during merge operations.
///
/// When merging entries for the same key, we need to aggregate metadata from all
/// entries to produce the final merged entry. This struct accumulates:
/// - The maximum creation timestamp (for tracking when data was first written)
/// - The minimum expiration timestamp (most restrictive TTL)
/// - The maximum sequence number (for ordering and validation)
#[derive(Debug, Clone)]
struct MergeTracker {
    /// Maximum creation timestamp seen across all merged entries.
    /// Used to track when the data was first created.
    max_create_ts: Option<i64>,

    /// Minimum expiration timestamp seen across all merged entries.
    /// The merged result expires at the earliest expiration time.
    min_expire_ts: Option<i64>,

    /// Maximum sequence number seen across all merged entries.
    /// Also used to validate that entries are processed in descending order.
    seq: u64,
}

impl MergeTracker {
    /// Updates the tracker with metadata from a new entry.
    ///
    /// This method:
    /// - Takes the maximum of creation timestamps (latest creation)
    /// - Takes the minimum of expiration timestamps (earliest expiration)
    /// - Validates that sequence numbers are in descending order
    /// - Updates the maximum sequence number
    ///
    /// # Errors
    ///
    /// Returns `SlateDBError::InvalidSequenceOrder` if the entry's sequence number
    /// is greater than the current maximum, indicating entries are out of order.
    fn update(&mut self, entry: &RowEntry) -> Result<(), SlateDBError> {
        self.max_create_ts = merge_options(self.max_create_ts, entry.create_ts, i64::max);
        self.min_expire_ts = merge_options(self.min_expire_ts, entry.expire_ts, i64::min);

        // sequence numbers should be descending
        if self.seq < entry.seq {
            return Err(SlateDBError::InvalidSequenceOrder {
                current_seq: self.seq,
                next_seq: entry.seq,
            });
        }

        self.seq = std::cmp::max(self.seq, entry.seq);
        Ok(())
    }
}

#[allow(unused)]
impl<T: RowEntryIterator> MergeOperatorIterator<T> {
    pub(crate) fn new(
        merge_operator: MergeOperatorType,
        delegate: T,
        merge_different_expire_ts: bool,
        snapshot_barrier_seq: Option<u64>,
    ) -> Self {
        Self {
            merge_operator,
            delegate,
            buffered_entry: None,
            merge_different_expire_ts,
            snapshot_barrier_seq,
        }
    }
}

impl<T: RowEntryIterator> MergeOperatorIterator<T> {
    /// Checks if an entry matches the current key and expiration timestamp.
    /// Returns true if keys match and either `merge_different_expire_ts` is enabled
    /// or the expire_ts matches.
    fn is_matching_entry(
        &self,
        entry: &RowEntry,
        key: &Bytes,
        first_expire_ts: Option<i64>,
    ) -> bool {
        entry.key == *key && (self.merge_different_expire_ts || first_expire_ts == entry.expire_ts)
    }

    /// Processes a batch of merge operands into a single intermediate result.
    ///
    /// Reverses the batch (oldest to newest), extracts operand values, updates metadata
    /// tracking, calls `merge_batch`, and clears the batch to free memory.
    fn process_batch(
        &self,
        key: &Bytes,
        batch: &mut Vec<RowEntry>,
        merge_tracker: &mut MergeTracker,
    ) -> Result<Bytes, SlateDBError> {
        batch.reverse();
        let mut operands: Vec<Bytes> = Vec::with_capacity(batch.len());
        for entry in &*batch {
            merge_tracker.update(entry)?;
            if let Some(v) = entry.value.as_bytes() {
                operands.push(v);
            }
        }

        let batch_result = self.merge_operator.merge_batch(key, None, &operands)?;
        batch.clear();
        Ok(batch_result)
    }

    /// Merges a sequence of entries with the same key into a single entry.
    ///
    /// Collects merge operands in batches of `MERGE_BATCH_SIZE`, processes each batch
    /// into intermediate results, and continues until a base value (Value/Tombstone) is
    /// found. Finally combines all intermediate results with the base value.
    ///
    /// Returns `Some(RowEntry)` with merged value and aggregated metadata, or `None` if
    /// only a tombstone with no operands was found.
    async fn merge_with_older_entries(
        &mut self,
        first_entry: RowEntry,
    ) -> Result<Option<RowEntry>, SlateDBError> {
        let key = first_entry.key.clone();
        let first_expire_ts = first_entry.expire_ts;

        let mut merge_tracker = MergeTracker {
            max_create_ts: None,
            min_expire_ts: None,
            seq: first_entry.seq,
        };

        let mut results = Vec::new();
        let mut batch = Vec::with_capacity(MERGE_BATCH_SIZE);

        let mut next = Some(first_entry);

        // this loop returns the "base value" (non merge operand) if it exists
        let base = loop {
            if let Some(entry) = next {
                if !self.is_matching_entry(&entry, &key, first_expire_ts) {
                    self.buffered_entry = Some(entry);
                    break None;
                } else if !matches!(entry.value, ValueDeletable::Merge(_)) {
                    // found a Value or Tombstone, this is the base value
                    break Some(entry);
                } else {
                    batch.push(entry);
                }

                // if the batch is full, merge it and add the result to the results vector
                if batch.len() >= MERGE_BATCH_SIZE {
                    results.push(self.process_batch(&key, &mut batch, &mut merge_tracker)?);
                }

                next = self.delegate.next().await?;
                // Keep cached operand scans cooperative.
                tokio::task::coop::consume_budget().await;
            } else {
                break None;
            }
        };

        // handle leftovers from the last batch
        if !batch.is_empty() {
            results.push(self.process_batch(&key, &mut batch, &mut merge_tracker)?);
        }

        let base_value = base.as_ref().and_then(|b| b.value.as_bytes());
        let found_base = base.is_some();

        // Fold the base entry's metadata into the tracker so that its
        // create_ts and expire_ts are reflected in the merged result.
        if let Some(ref base_entry) = base {
            merge_tracker.update(base_entry)?;
        }

        // If we have no results and either no base or a tombstone base, return None
        if results.is_empty() && base_value.is_none() {
            return Ok(None);
        }

        results.reverse();
        let final_result = self
            .merge_operator
            .merge_batch(&key, base_value, &results)?;

        Ok(Some(RowEntry {
            key: key.clone(),
            value: if found_base {
                ValueDeletable::Value(final_result)
            } else {
                ValueDeletable::Merge(final_result)
            },
            seq: merge_tracker.seq,
            create_ts: merge_tracker.max_create_ts,
            expire_ts: merge_tracker.min_expire_ts,
        }))
    }
}

#[async_trait]
impl<T: RowEntryIterator> RowEntryIterator for MergeOperatorIterator<T> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        self.delegate.init().await
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        let next = match self.buffered_entry.take() {
            Some(entry) => Some(entry),
            None => self.delegate.next().await?,
        };
        if let Some(entry) = next {
            match &entry.value {
                ValueDeletable::Merge(_) => {
                    if let Some(snapshot_barrier_seq) = self.snapshot_barrier_seq {
                        if entry.seq > snapshot_barrier_seq {
                            return Ok(Some(entry));
                        }
                    }
                    // A mergeable entry, we need to accumulate all mergeable entries
                    // ahead for the same key and merge them into a single value.
                    return self.merge_with_older_entries(entry).await;
                }
                // Not a mergeable entry, just return it.
                _ => return Ok(Some(entry)),
            }
        }
        Ok(None)
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        self.buffered_entry = None;
        self.delegate.seek(next_key).await
    }
}

impl<T: TrackedRowEntryIterator> TrackedRowEntryIterator for MergeOperatorIterator<T> {
    fn bytes_processed(&self) -> u64 {
        self.delegate.bytes_processed()
    }
}

#[cfg(test)]
mod tests {
    use std::{cmp::Ordering, collections::VecDeque, fmt::Debug};

    use rstest::rstest;
    use slatedb_common::metrics::{lookup_metric, test_recorder_helper};

    use crate::test_utils::assert_iterator;

    use super::*;

    struct MockMergeOperator;

    impl MergeOperator for MockMergeOperator {
        fn merge(
            &self,
            _key: &Bytes,
            existing_value: Option<Bytes>,
            value: Bytes,
        ) -> Result<Bytes, MergeOperatorError> {
            match existing_value {
                Some(existing) => {
                    let mut merged = existing.to_vec();
                    merged.extend_from_slice(&value);
                    Ok(Bytes::from(merged))
                }
                None => Ok(value),
            }
        }
    }

    /// Mock merge operator that tracks whether merge_batch is called
    struct MockBatchedMergeOperator {
        merge_batch_call_count: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl MockBatchedMergeOperator {
        fn new() -> (Self, Arc<std::sync::atomic::AtomicUsize>) {
            let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            (
                Self {
                    merge_batch_call_count: counter.clone(),
                },
                counter,
            )
        }
    }

    impl MergeOperator for MockBatchedMergeOperator {
        fn merge(
            &self,
            _key: &Bytes,
            existing_value: Option<Bytes>,
            value: Bytes,
        ) -> Result<Bytes, MergeOperatorError> {
            // Same as MockMergeOperator - concatenate bytes
            match existing_value {
                Some(existing) => {
                    let mut merged = existing.to_vec();
                    merged.extend_from_slice(&value);
                    Ok(Bytes::from(merged))
                }
                None => Ok(value),
            }
        }

        fn merge_batch(
            &self,
            _key: &Bytes,
            existing_value: Option<Bytes>,
            operands: &[Bytes],
        ) -> Result<Bytes, MergeOperatorError> {
            // Increment counter to track that merge_batch was called
            self.merge_batch_call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            // Efficiently concatenate all operands at once
            let mut result = existing_value.unwrap_or_default().to_vec();
            for operand in operands {
                result.extend_from_slice(operand);
            }
            Ok(Bytes::from(result))
        }
    }

    struct FailingMergeBatchOperator;

    impl MergeOperator for FailingMergeBatchOperator {
        fn merge(
            &self,
            _key: &Bytes,
            _existing_value: Option<Bytes>,
            value: Bytes,
        ) -> Result<Bytes, MergeOperatorError> {
            Ok(value)
        }

        fn merge_batch(
            &self,
            key: &Bytes,
            _existing_value: Option<Bytes>,
            _operands: &[Bytes],
        ) -> Result<Bytes, MergeOperatorError> {
            Err(MergeOperatorError::Callback {
                message: format!("failed to merge {key:?}"),
            })
        }
    }

    #[test]
    fn test_instrumented_merge_operator_counts_successful_batch_operands() {
        const TEST_COUNTER: &str = "test.merge_operator.operands";

        let (metrics_recorder, recorder) = test_recorder_helper();
        let counter = recorder.counter(TEST_COUNTER).register();
        let merge_operator = instrument_merge_operator(Arc::new(MockMergeOperator), counter);

        let result = merge_operator
            .merge_batch(
                &Bytes::from_static(b"key1"),
                None,
                &[Bytes::from_static(b"a"), Bytes::from_static(b"b")],
            )
            .unwrap();

        assert_eq!(result, Bytes::from_static(b"ab"));
        assert_eq!(
            lookup_metric(metrics_recorder.as_ref(), TEST_COUNTER),
            Some(2)
        );
    }

    #[test]
    fn test_instrumented_merge_operator_does_not_count_failed_batch_operands() {
        const TEST_COUNTER: &str = "test.merge_operator.operands";

        let (metrics_recorder, recorder) = test_recorder_helper();
        let counter = recorder.counter(TEST_COUNTER).register();
        let merge_operator =
            instrument_merge_operator(Arc::new(FailingMergeBatchOperator), counter);

        let result = merge_operator.merge_batch(
            &Bytes::from_static(b"key1"),
            None,
            &[Bytes::from_static(b"a"), Bytes::from_static(b"b")],
        );

        assert!(result.is_err());
        assert_eq!(
            lookup_metric(metrics_recorder.as_ref(), TEST_COUNTER),
            Some(0)
        );
    }

    #[tokio::test]
    async fn test_merge_operator_iterator() {
        let merge_operator = Arc::new(MockMergeOperator {});
        let data = vec![
            RowEntry::new_merge(b"key1", b"1", 1),
            RowEntry::new_merge(b"key1", b"2", 2),
            RowEntry::new_merge(b"key1", b"3", 3),
            RowEntry::new_merge(b"key1", b"4", 4),
            RowEntry::new_value(b"key2", b"1", 5),
            RowEntry::new_value(b"key3", b"1", 6),
            RowEntry::new_merge(b"key3", b"2", 7),
            RowEntry::new_merge(b"key3", b"3", 8),
        ];
        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );
        assert_iterator(
            &mut iterator,
            vec![
                RowEntry::new_merge(b"key1", b"1234", 4),
                RowEntry::new_value(b"key2", b"1", 5),
                RowEntry::new_value(b"key3", b"123", 8),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_seek_clears_buffered_entry() {
        let merge_operator = Arc::new(MockMergeOperator {});
        let data = vec![
            RowEntry::new_merge(b"key1", b"1", 1),
            RowEntry::new_merge(b"key1", b"2", 2),
            RowEntry::new_value(b"key2", b"v", 3),
        ];
        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        iterator.init().await.unwrap();
        let first = iterator.next().await.unwrap().unwrap();
        assert_eq!(first.key.as_ref(), b"key1");
        assert_eq!(first.value.as_bytes().unwrap().as_ref(), b"12");

        iterator.seek(b"key3").await.unwrap();
        assert!(
            iterator.next().await.unwrap().is_none(),
            "seek should not return a stale buffered entry"
        );
    }

    #[derive(Debug)]
    struct TestCase {
        unsorted_data: Vec<RowEntry>,
        expected: Vec<RowEntry>,
        merge_different_expire_ts: bool,
        snapshot_barrier_seq: Option<u64>,
    }

    impl Default for TestCase {
        fn default() -> Self {
            Self {
                unsorted_data: vec![],
                expected: vec![],
                merge_different_expire_ts: true,
                snapshot_barrier_seq: None,
            }
        }
    }

    #[rstest]
    #[case::different_expire_ts_read_path(TestCase {
        unsorted_data: vec![
            RowEntry::new_merge(b"key1", b"1", 1).with_expire_ts(1),
            RowEntry::new_merge(b"key1", b"2", 2).with_expire_ts(2),
            RowEntry::new_merge(b"key1", b"3", 3).with_expire_ts(3),
            RowEntry::new_value(b"key2", b"1", 4),
            RowEntry::new_merge(b"key3", b"1", 5).with_expire_ts(1),
            RowEntry::new_merge(b"key3", b"2", 6).with_expire_ts(1),
            RowEntry::new_merge(b"key3", b"3", 7).with_expire_ts(2),
        ],
        expected: vec![
            RowEntry::new_merge(b"key1", b"123", 3).with_expire_ts(1),
            RowEntry::new_value(b"key2", b"1", 4),
            RowEntry::new_merge(b"key3", b"123", 7).with_expire_ts(1),
        ],
        ..TestCase::default()
    })]
    #[case::different_expire_ts_write_path(TestCase {
        unsorted_data: vec![
            RowEntry::new_merge(b"key1", b"1", 1).with_expire_ts(1),
            RowEntry::new_merge(b"key1", b"2", 2).with_expire_ts(2),
            RowEntry::new_merge(b"key1", b"3", 3).with_expire_ts(3),
            RowEntry::new_value(b"key2", b"1", 4),
            RowEntry::new_merge(b"key3", b"1", 5).with_expire_ts(1),
            RowEntry::new_merge(b"key3", b"2", 6).with_expire_ts(1),
            RowEntry::new_merge(b"key3", b"3", 7).with_expire_ts(2),
        ],
        expected: vec![
            RowEntry::new_merge(b"key1", b"3", 3).with_expire_ts(3),
            RowEntry::new_merge(b"key1", b"2", 2).with_expire_ts(2),
            RowEntry::new_merge(b"key1", b"1", 1).with_expire_ts(1),
            RowEntry::new_value(b"key2", b"1", 4),
            RowEntry::new_merge(b"key3", b"3", 7).with_expire_ts(2),
            RowEntry::new_merge(b"key3", b"12", 6).with_expire_ts(1),
        ],
        // On write path (compaction, memtable), we don't merge entries
        // with different expire timestamps to allow per-element expiration.
        merge_different_expire_ts: false,
        ..TestCase::default()
    })]
    #[case::merge_with_tombstone(TestCase {
        unsorted_data: vec![
            RowEntry::new_merge(b"key1", b"1", 1),
            RowEntry::new_merge(b"key1", b"2", 2),
            RowEntry::new_tombstone(b"key1", 3),
            RowEntry::new_merge(b"key1", b"3", 4),
            RowEntry::new_value(b"key2", b"1", 5)
        ],
        expected: vec![
            // Merge + Tombstone becomes a value to invalidate older entries.
            RowEntry::new_value(b"key1", b"3", 4),
            RowEntry::new_merge(b"key1", b"12", 2),
            RowEntry::new_value(b"key2", b"1", 5)
        ],
        ..TestCase::default()
    })]
    #[case::multiple_values(TestCase {
        unsorted_data: vec![
            RowEntry::new_value(b"key1", b"1", 1),
            RowEntry::new_value(b"key1", b"2", 2),
        ],
        expected: vec![
            RowEntry::new_value(b"key1", b"2", 2),
            RowEntry::new_value(b"key1", b"1", 1),
        ],
        ..TestCase::default()
    })]
    #[case::merge_with_snapshot_barrier(TestCase {
        unsorted_data: vec![
            RowEntry::new_merge(b"key1", b"1", 1),
            RowEntry::new_merge(b"key1", b"2", 2),
            RowEntry::new_merge(b"key1", b"3", 3),
            RowEntry::new_merge(b"key1", b"4", 4),
            RowEntry::new_value(b"key1", b"5", 5)
        ],
        expected: vec![
            RowEntry::new_value(b"key1", b"5", 5),
            RowEntry::new_merge(b"key1", b"4", 4),
            RowEntry::new_merge(b"key1", b"123", 3),
        ],
        snapshot_barrier_seq: Some(3),
        ..TestCase::default()
    })]
    #[tokio::test]
    async fn test(#[case] test_case: TestCase) {
        let merge_operator = Arc::new(MockMergeOperator {});
        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            test_case.unsorted_data.into(),
            test_case.merge_different_expire_ts,
            test_case.snapshot_barrier_seq,
        );
        assert_iterator(&mut iterator, test_case.expected).await;
    }

    struct MockRowEntryIterator {
        values: VecDeque<RowEntry>,
    }

    #[async_trait]
    impl RowEntryIterator for MockRowEntryIterator {
        async fn init(&mut self) -> Result<(), SlateDBError> {
            Ok(())
        }

        async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
            Ok(self.values.pop_front())
        }

        async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
            self.values.retain(|entry| entry.key == next_key);
            Ok(())
        }
    }

    impl From<Vec<RowEntry>> for MockRowEntryIterator {
        /// Converts a vector of RowEntries into a MockRowEntryIterator. The vector is sorted
        /// by key and reverse sequence number.
        fn from(values: Vec<RowEntry>) -> Self {
            let mut sorted_values = values;
            sorted_values.sort_by(|left, right| {
                let ord = left.key.cmp(&right.key);
                if ord == Ordering::Equal {
                    right.seq.cmp(&left.seq)
                } else {
                    ord
                }
            });
            Self {
                values: sorted_values.into(),
            }
        }
    }

    /// A merge operator that routes to different merge strategies based on key prefix
    struct KeyPrefixMergeOperator;

    impl MergeOperator for KeyPrefixMergeOperator {
        fn merge(
            &self,
            key: &Bytes,
            existing_value: Option<Bytes>,
            value: Bytes,
        ) -> Result<Bytes, MergeOperatorError> {
            if key.starts_with(b"sum:") {
                // Sum merge for sum keys
                match existing_value {
                    Some(existing) => {
                        let existing_num =
                            u64::from_le_bytes(existing.as_ref().try_into().unwrap());
                        let new_num = u64::from_le_bytes(value.as_ref().try_into().unwrap());
                        Ok(Bytes::copy_from_slice(
                            &(existing_num + new_num).to_le_bytes(),
                        ))
                    }
                    None => Ok(value),
                }
            } else if key.starts_with(b"max:") {
                // Max merge for max keys
                match existing_value {
                    Some(existing) => {
                        let existing_num =
                            u64::from_le_bytes(existing.as_ref().try_into().unwrap());
                        let new_num = u64::from_le_bytes(value.as_ref().try_into().unwrap());
                        Ok(Bytes::copy_from_slice(
                            &existing_num.max(new_num).to_le_bytes(),
                        ))
                    }
                    None => Ok(value),
                }
            } else {
                // Default to concat for unknown prefixes
                match existing_value {
                    Some(existing) => {
                        let mut merged = existing.to_vec();
                        merged.extend_from_slice(&value);
                        Ok(Bytes::from(merged))
                    }
                    None => Ok(value),
                }
            }
        }

        // Override merge_batch to handle batches efficiently with key-based routing
        fn merge_batch(
            &self,
            key: &Bytes,
            existing_value: Option<Bytes>,
            operands: &[Bytes],
        ) -> Result<Bytes, MergeOperatorError> {
            if key.starts_with(b"max:") {
                // For max operator, find the maximum value across all operands
                let mut max_val = existing_value
                    .map(|v| u64::from_le_bytes(v.as_ref().try_into().unwrap()))
                    .unwrap_or(0);

                for operand in operands {
                    let val = u64::from_le_bytes(operand.as_ref().try_into().unwrap());
                    max_val = max_val.max(val);
                }

                Ok(Bytes::copy_from_slice(&max_val.to_le_bytes()))
            } else {
                // For other prefixes, use pairwise merge
                let mut result = existing_value;
                for operand in operands {
                    result = Some(self.merge(key, result, operand.clone())?);
                }
                result.ok_or(MergeOperatorError::EmptyBatch)
            }
        }
    }

    #[tokio::test]
    async fn should_route_merge_based_on_key_prefix() {
        let merge_operator = Arc::new(KeyPrefixMergeOperator {});

        let data = vec![
            // Sum key
            RowEntry::new_merge(b"sum:counter", &5u64.to_le_bytes(), 1),
            RowEntry::new_merge(b"sum:counter", &3u64.to_le_bytes(), 2),
            RowEntry::new_merge(b"sum:counter", &7u64.to_le_bytes(), 3),
            // Max key
            RowEntry::new_merge(b"max:score", &5u64.to_le_bytes(), 4),
            RowEntry::new_merge(b"max:score", &10u64.to_le_bytes(), 5),
            RowEntry::new_merge(b"max:score", &3u64.to_le_bytes(), 6),
        ];

        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        // Expected: max should return 10, sum should return 15
        let max_expected = 10u64.to_le_bytes();
        let sum_expected = 15u64.to_le_bytes();

        assert_iterator(
            &mut iterator,
            vec![
                RowEntry::new_merge(b"max:score", &max_expected, 6),
                RowEntry::new_merge(b"sum:counter", &sum_expected, 3),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_batched_merge_with_many_operands() {
        let merge_operator = Arc::new(MockMergeOperator {});

        let mut data = vec![];
        for i in 1..=250 {
            data.push(RowEntry::new_merge(b"key1", &[i as u8], i));
        }

        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        let expected_bytes: Vec<u8> = (1..=250).map(|i| i as u8).collect();
        let expected = vec![RowEntry::new_merge(b"key1", &expected_bytes, 250)];

        assert_iterator(&mut iterator, expected).await;
    }

    #[tokio::test]
    async fn test_batched_merge_with_base_value() {
        let merge_operator = Arc::new(MockMergeOperator {});

        let mut data = vec![];
        data.push(RowEntry::new_value(b"key1", b"BASE", 0));
        for i in 1..=150 {
            data.push(RowEntry::new_merge(b"key1", &[i as u8], i));
        }

        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        let mut expected_bytes = b"BASE".to_vec();
        expected_bytes.extend((1..=150).map(|i| i as u8));
        let expected = vec![RowEntry::new_value(b"key1", &expected_bytes, 150)];

        assert_iterator(&mut iterator, expected).await;
    }

    #[tokio::test]
    async fn test_merge_batch_is_actually_called() {
        let (merge_operator, call_count) = MockBatchedMergeOperator::new();
        let merge_operator = Arc::new(merge_operator);

        let mut data = vec![];
        for i in 1..=250 {
            data.push(RowEntry::new_merge(b"key1", &[i as u8], i));
        }

        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        let expected_bytes: Vec<u8> = (1..=250).map(|i| i as u8).collect();
        let expected = vec![RowEntry::new_merge(b"key1", &expected_bytes, 250)];
        assert_iterator(&mut iterator, expected).await;

        let actual_calls = call_count.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            actual_calls, 4,
            "Expected merge_batch to be called 4 times for 250 operands (3 batches + 1 final merge), but was called {} times",
            actual_calls
        );
    }

    #[tokio::test]
    async fn test_merge_batch_with_base_value_call_count() {
        let (merge_operator, call_count) = MockBatchedMergeOperator::new();
        let merge_operator = Arc::new(merge_operator);

        // Create base value + 150 merge operands (will require 2 batches: 100 + 50)
        let mut data = vec![];
        data.push(RowEntry::new_value(b"key1", b"BASE", 0));
        for i in 1..=150 {
            data.push(RowEntry::new_merge(b"key1", &[i as u8], i));
        }

        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        let mut expected_bytes = b"BASE".to_vec();
        expected_bytes.extend((1..=150).map(|i| i as u8));
        let expected = vec![RowEntry::new_value(b"key1", &expected_bytes, 150)];
        assert_iterator(&mut iterator, expected).await;

        let actual_calls = call_count.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            actual_calls, 3,
            "Expected merge_batch to be called 3 times for 150 operands (2 batches + 1 final merge), but was called {} times",
            actual_calls
        );
    }

    #[tokio::test]
    async fn test_merge_operator_merges_entries_with_mixed_expire_ts() {
        let merge_operator = Arc::new(MockMergeOperator {});

        // Create entries with different expiration times
        let data = vec![
            RowEntry::new_merge(b"key1", b"4", 4).with_expire_ts(300),
            RowEntry::new_merge(b"key1", b"2", 2).with_expire_ts(150),
            RowEntry::new_merge(b"key1", b"1", 1).with_expire_ts(200),
            RowEntry::new_merge(b"key1", b"5", 5).with_expire_ts(100),
            RowEntry::new_merge(b"key1", b"3", 3).with_expire_ts(50),
        ];

        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        // Entries sorted by reverse seq: seq=5, 4, 3, 2, 1
        // Reversed to seq asc: b"1", b"2", b"3", b"4", b"5" -> merged to "12345"
        // min_expire_ts = min(200, 150, 50, 300, 100) = 50
        assert_iterator(
            &mut iterator,
            vec![RowEntry::new_merge(b"key1", b"12345", 5).with_expire_ts(50)],
        )
        .await;
    }

    #[tokio::test]
    async fn test_merge_operator_merges_entries_with_base_value_and_mixed_expire_ts() {
        let merge_operator = Arc::new(MockMergeOperator {});

        // Create entries with a base value and merge operands with different expire_ts
        let data = vec![
            RowEntry::new_merge(b"key1", b"4", 4).with_expire_ts(300),
            RowEntry::new_merge(b"key1", b"2", 2).with_expire_ts(250),
            RowEntry::new_merge(b"key1", b"1", 1).with_expire_ts(150),
            RowEntry::new_merge(b"key1", b"3", 3).with_expire_ts(50),
            RowEntry::new_value(b"key1", b"BASE", 0).with_expire_ts(200),
        ];

        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        // Sorted by reverse seq: seq=4, 3, 2, 1, 0(BASE)
        // Merge operands [4, 3, 2, 1] reversed to [1, 2, 3, 4], merged to "1234", then merged with BASE
        // min_expire_ts = min(300, 50, 250, 150, 200) = 50
        assert_iterator(
            &mut iterator,
            vec![RowEntry::new_value(b"key1", b"BASE1234", 4).with_expire_ts(50)],
        )
        .await;
    }

    #[tokio::test]
    async fn test_merge_operator_merges_all_entries_with_expire_ts() {
        let merge_operator = Arc::new(MockMergeOperator {});

        // All merge operands have expire_ts set
        let data = vec![
            RowEntry::new_merge(b"key1", b"1", 1).with_expire_ts(50),
            RowEntry::new_merge(b"key1", b"2", 2).with_expire_ts(80),
            RowEntry::new_merge(b"key1", b"3", 3).with_expire_ts(90),
        ];

        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        // Sorted by reverse seq: seq=3, 2, 1
        // Reversed to [1, 2, 3], merged to "123"
        // min_expire_ts = min(50, 80, 90) = 50
        assert_iterator(
            &mut iterator,
            vec![RowEntry::new_merge(b"key1", b"123", 3).with_expire_ts(50)],
        )
        .await;
    }

    #[tokio::test]
    async fn should_use_base_expire_ts_when_it_is_the_earliest() {
        // given: a base value with the earliest expire_ts
        let merge_operator = Arc::new(MockMergeOperator {});
        let data = vec![
            RowEntry::new_merge(b"key1", b"2", 2).with_expire_ts(300),
            RowEntry::new_merge(b"key1", b"1", 1).with_expire_ts(200),
            RowEntry::new_value(b"key1", b"BASE", 0).with_expire_ts(50),
        ];

        let mut iterator = MergeOperatorIterator::<MockRowEntryIterator>::new(
            merge_operator,
            data.into(),
            true,
            None,
        );

        // then: min_expire_ts should come from the base value (50)
        assert_iterator(
            &mut iterator,
            vec![RowEntry::new_value(b"key1", b"BASE12", 2).with_expire_ts(50)],
        )
        .await;
    }
}
