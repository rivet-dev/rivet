use async_trait::async_trait;

use crate::error::SlateDBError;
use crate::iter::{IterationOrder, RowEntryIterator, TrackedRowEntryIterator};
use crate::types::{RowEntry, ValueDeletable};
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, VecDeque};

struct MergeIteratorHeapEntry<'a> {
    next_kv: RowEntry,
    index: usize,
    iterator: Box<dyn RowEntryIterator + 'a>,
    order: IterationOrder,
}

impl<'a> MergeIteratorHeapEntry<'a> {
    /// Seek the iterator and return a new heap entry
    async fn seek(
        mut self,
        next_key: &[u8],
    ) -> Result<Option<MergeIteratorHeapEntry<'a>>, SlateDBError> {
        if self.next_kv.key >= next_key {
            Ok(Some(self))
        } else {
            self.iterator.seek(next_key).await?;
            if let Some(next_kv) = self.iterator.next().await? {
                Ok(Some(MergeIteratorHeapEntry {
                    next_kv,
                    index: self.index,
                    iterator: self.iterator,
                    order: self.order,
                }))
            } else {
                Ok(None)
            }
        }
    }
}

impl Eq for MergeIteratorHeapEntry<'_> {}

impl PartialEq<Self> for MergeIteratorHeapEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.next_kv == other.next_kv
    }
}

impl PartialOrd<Self> for MergeIteratorHeapEntry<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MergeIteratorHeapEntry<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Wrapped in Reverse in the BinaryHeap, so the smallest entry here gets
        // popped first. Within a key, always return highest seqnum first.
        // Across keys, the direction depends on the iteration order.
        let key_ord = match self.order {
            IterationOrder::Ascending => self.next_kv.key.cmp(&other.next_kv.key),
            IterationOrder::Descending => other.next_kv.key.cmp(&self.next_kv.key),
        };
        match key_ord {
            Ordering::Equal => other.next_kv.seq.cmp(&self.next_kv.seq),
            ord => ord,
        }
    }
}

pub(crate) struct MergeIterator<'a> {
    /// The current entry popped from the heap.
    current: Option<MergeIteratorHeapEntry<'a>>,
    /// Use a heap to perform merge sort.
    iterators: BinaryHeap<Reverse<MergeIteratorHeapEntry<'a>>>,
    /// Iterators that have not yet been initialized and seeded.
    pending_iterators: Vec<(usize, Box<dyn RowEntryIterator + 'a>)>,
    /// Whether to deduplicate entries of multiple versions with the same key. It's enabled by
    /// default, but it is useful to disable when we want to have some merge logics during
    /// compaction.
    dedup: bool,
    /// Tracks whether the iterator has performed its heavy initialization step.
    initialized: bool,
    /// Counter to track bytes processed (key + value length) for progress reporting.
    bytes_processed: u64,
    /// The iteration order for key comparison in the merge heap.
    order: IterationOrder,
}

impl<'a> MergeIterator<'a> {
    pub(crate) fn new<T: RowEntryIterator + 'a>(
        iterators: impl IntoIterator<Item = T>,
    ) -> Result<Self, SlateDBError> {
        Self::new_with_order(iterators, IterationOrder::Ascending)
    }

    pub(crate) fn new_with_order<T: RowEntryIterator + 'a>(
        iterators: impl IntoIterator<Item = T>,
        order: IterationOrder,
    ) -> Result<Self, SlateDBError> {
        Ok(Self {
            current: None,
            iterators: BinaryHeap::new(),
            order,
            pending_iterators: iterators
                .into_iter()
                .enumerate()
                .map(|(index, iterator)| {
                    (index, Box::new(iterator) as Box<dyn RowEntryIterator + 'a>)
                })
                .collect(),
            dedup: true,
            initialized: false,
            bytes_processed: 0,
        })
    }

    pub(crate) fn with_dedup(mut self, dedup: bool) -> Self {
        self.dedup = dedup;
        self
    }

    async fn initialize(&mut self) -> Result<(), SlateDBError> {
        if self.initialized {
            return Ok(());
        }

        for (index, mut iterator) in self.pending_iterators.drain(..) {
            iterator.init().await?;
            if let Some(next_kv) = iterator.next().await? {
                self.iterators.push(Reverse(MergeIteratorHeapEntry {
                    next_kv,
                    index,
                    iterator,
                    order: self.order,
                }));
            }
        }
        self.current = self.iterators.pop().map(|r| r.0);
        self.initialized = true;
        Ok(())
    }

    async fn ensure_initialized(&mut self) -> Result<(), SlateDBError> {
        if !self.initialized {
            self.initialize().await?;
        }
        Ok(())
    }

    fn peek(&self) -> Option<&RowEntry> {
        self.current.as_ref().map(|c| &c.next_kv)
    }

    async fn advance(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        self.ensure_initialized().await?;
        if let Some(mut iterator_state) = self.current.take() {
            let current_kv = iterator_state.next_kv;
            if let Some(kv) = iterator_state.iterator.next().await? {
                iterator_state.next_kv = kv;
                self.iterators.push(Reverse(iterator_state));
            }
            self.current = self.iterators.pop().map(|r| r.0);

            // Track bytes processed for progress reporting
            let entry_bytes = current_kv.key.len() as u64 + current_kv.value.len() as u64;
            self.bytes_processed += entry_bytes;

            return Ok(Some(current_kv));
        }
        Ok(None)
    }
}

#[async_trait]
impl RowEntryIterator for MergeIterator<'_> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        self.initialize().await
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        if !self.initialized {
            return Err(SlateDBError::IteratorNotInitialized);
        }
        if !self.dedup {
            return self.advance().await;
        }

        let current_kv = match self.advance().await? {
            Some(kv) => kv,
            None => return Ok(None),
        };

        // the iterators are stored in order of increasing key and decreasing
        // seqnum, which means that the first entry in the heap is the one with
        // the highest seqnum for a given key. we want to advance other iterators
        // to skip their current value if the current value is not a merge oepration
        // (we can ignore merge values after seeing the first non-merge value
        // because tombstones/values serve as "barriers" in the merge operation)
        if !matches!(current_kv.value, ValueDeletable::Merge(_)) {
            while let Some(peeked_entry) = self.peek() {
                if peeked_entry.key != current_kv.key {
                    break;
                }
                self.advance().await?;
            }
        }

        Ok(Some(current_kv))
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        if !self.initialized {
            return Err(SlateDBError::IteratorNotInitialized);
        }
        self.ensure_initialized().await?;
        let mut seek_futures = VecDeque::new();
        if let Some(iterator) = self.current.take() {
            seek_futures.push_back(iterator.seek(next_key))
        }

        for iterator in self.iterators.drain() {
            seek_futures.push_back(iterator.0.seek(next_key));
        }

        for seek_result in futures::future::join_all(seek_futures).await {
            if let Some(seeked_iterator) = seek_result? {
                self.iterators.push(Reverse(seeked_iterator));
            }
        }

        self.current = self.iterators.pop().map(|r| r.0);
        Ok(())
    }
}

impl TrackedRowEntryIterator for MergeIterator<'_> {
    fn bytes_processed(&self) -> u64 {
        self.bytes_processed
    }
}

#[cfg(test)]
mod tests {
    use crate::iter::{IterationOrder, RowEntryIterator};
    use crate::merge_iterator::MergeIterator;
    use crate::test_utils::{assert_iterator, assert_next, TestIterator};
    use crate::types::RowEntry;
    use std::collections::VecDeque;
    use std::vec;

    #[tokio::test]
    async fn test_merge_iterator_should_include_entries_in_order() {
        let mut iters: VecDeque<TestIterator> = VecDeque::new();
        iters.push_back(
            TestIterator::new()
                .with_entry(b"aaaa", b"1111", 0)
                .with_entry(b"cccc", b"3333", 0)
                .with_entry(b"zzzz", b"26262626", 0),
        );
        iters.push_back(
            TestIterator::new()
                .with_entry(b"bbbb", b"2222", 0)
                .with_entry(b"xxxx", b"24242424", 0)
                .with_entry(b"yyyy", b"25252525", 0),
        );
        iters.push_back(
            TestIterator::new()
                .with_entry(b"dddd", b"4444", 0)
                .with_entry(b"eeee", b"5555", 0)
                .with_entry(b"gggg", b"7777", 0),
        );

        let mut merge_iter = MergeIterator::new(iters).unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"aaaa", b"1111", 0),
                RowEntry::new_value(b"bbbb", b"2222", 0),
                RowEntry::new_value(b"cccc", b"3333", 0),
                RowEntry::new_value(b"dddd", b"4444", 0),
                RowEntry::new_value(b"eeee", b"5555", 0),
                RowEntry::new_value(b"gggg", b"7777", 0),
                RowEntry::new_value(b"xxxx", b"24242424", 0),
                RowEntry::new_value(b"yyyy", b"25252525", 0),
                RowEntry::new_value(b"zzzz", b"26262626", 0),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_merge_iterator_should_write_one_entry_with_given_key() {
        let mut iters: VecDeque<TestIterator> = VecDeque::new();
        iters.push_back(
            TestIterator::new()
                .with_entry(b"aaaa", b"0000", 6)
                .with_entry(b"aaaa", b"1111", 5)
                .with_entry(b"cccc", b"use this one c", 5),
        );
        iters.push_back(
            TestIterator::new()
                .with_entry(b"cccc", b"badc1", 1)
                .with_entry(b"xxxx", b"use this one x", 4),
        );
        iters.push_back(
            TestIterator::new()
                .with_entry(b"bbbb", b"2222", 3)
                .with_entry(b"cccc", b"badc2", 3)
                .with_entry(b"xxxx", b"badx1", 3),
        );

        let mut merge_iter = MergeIterator::new(iters).unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"aaaa", b"0000", 6),
                RowEntry::new_value(b"bbbb", b"2222", 3),
                RowEntry::new_value(b"cccc", b"use this one c", 5),
                RowEntry::new_value(b"xxxx", b"use this one x", 4),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_two_iterator_should_include_entries_in_order() {
        let iter1 = TestIterator::new()
            .with_entry(b"aaaa", b"1111", 0)
            .with_entry(b"cccc", b"3333", 0)
            .with_entry(b"zzzz", b"26262626", 0);
        let iter2 = TestIterator::new()
            .with_entry(b"bbbb", b"2222", 0)
            .with_entry(b"xxxx", b"24242424", 0)
            .with_entry(b"yyyy", b"25252525", 0);

        let mut merge_iter = MergeIterator::new([iter1, iter2]).unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"aaaa", b"1111", 0),
                RowEntry::new_value(b"bbbb", b"2222", 0),
                RowEntry::new_value(b"cccc", b"3333", 0),
                RowEntry::new_value(b"xxxx", b"24242424", 0),
                RowEntry::new_value(b"yyyy", b"25252525", 0),
                RowEntry::new_value(b"zzzz", b"26262626", 0),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_two_iterator_should_write_one_entry_with_given_key() {
        let iter1 = TestIterator::new()
            .with_entry(b"aaaa", b"1111", 0)
            .with_entry(b"cccc", b"use this one c", 5);
        let iter2 = TestIterator::new()
            .with_entry(b"cccc", b"badc1", 2)
            .with_entry(b"xxxx", b"24242424", 3);

        let mut merge_iter = MergeIterator::new([iter1, iter2]).unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"aaaa", b"1111", 0),
                RowEntry::new_value(b"cccc", b"use this one c", 5),
                RowEntry::new_value(b"xxxx", b"24242424", 3),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_seek_merge_iter() {
        let mut iters: VecDeque<TestIterator> = VecDeque::new();
        iters.push_back(
            TestIterator::new()
                .with_entry(b"aa", b"aa1", 0)
                .with_entry(b"bb", b"bb1", 0),
        );
        iters.push_back(
            TestIterator::new()
                .with_entry(b"aa", b"aa2", 0)
                .with_entry(b"bb", b"bb2", 0)
                .with_entry(b"cc", b"cc2", 0),
        );

        let mut merge_iter = MergeIterator::new(iters).unwrap();
        merge_iter.init().await.unwrap();
        merge_iter.seek(b"bb".as_ref()).await.unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"bb", b"bb1", 0),
                RowEntry::new_value(b"cc", b"cc2", 0),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_seek_merge_iter_to_current_key() {
        let mut iters: VecDeque<TestIterator> = VecDeque::new();
        iters.push_back(
            TestIterator::new()
                .with_entry(b"aa", b"aa1", 0)
                .with_entry(b"bb", b"bb1", 0),
        );
        iters.push_back(
            TestIterator::new()
                .with_entry(b"aa", b"aa2", 0)
                .with_entry(b"bb", b"bb2", 0)
                .with_entry(b"cc", b"cc2", 0),
        );

        let mut merge_iter = MergeIterator::new(iters).unwrap();
        assert_next(&mut merge_iter, &RowEntry::new_value(b"aa", b"aa1", 0)).await;

        merge_iter.seek(b"bb".as_ref()).await.unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"bb", b"bb1", 0),
                RowEntry::new_value(b"cc", b"cc2", 0),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_two_merge_seek() {
        let iter1 = TestIterator::new()
            .with_entry(b"aa", b"aa1", 1)
            .with_entry(b"bb", b"bb0", 2)
            .with_entry(b"bb", b"bb1", 1)
            .with_entry(b"dd", b"dd1", 3);
        let iter2 = TestIterator::new()
            .with_entry(b"aa", b"aa2", 4)
            .with_entry(b"bb", b"bb2", 5)
            .with_entry(b"cc", b"cc0", 6)
            .with_entry(b"cc", b"cc2", 5)
            .with_entry(b"ee", b"ee2", 7);

        let mut merge_iter = MergeIterator::new([iter1, iter2]).unwrap();
        merge_iter.init().await.unwrap();
        merge_iter.seek(b"b".as_ref()).await.unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"bb", b"bb2", 5),
                RowEntry::new_value(b"cc", b"cc0", 6),
                RowEntry::new_value(b"dd", b"dd1", 3),
                RowEntry::new_value(b"ee", b"ee2", 7),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_dedup_disabled() {
        let iter1 = TestIterator::new()
            .with_entry(b"key1", b"value1", 1)
            .with_entry(b"key2", b"value2", 2);
        let iter2 = TestIterator::new()
            .with_entry(b"key1", b"value1_updated", 3)
            .with_entry(b"key3", b"value3", 4);

        let mut merge_iter = MergeIterator::new([iter1, iter2])
            .unwrap()
            .with_dedup(false);

        // With dedup disabled, should return all entries in order
        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"key1", b"value1_updated", 3), // second occurrence
                RowEntry::new_value(b"key1", b"value1", 1),         // first occurrence
                RowEntry::new_value(b"key2", b"value2", 2),
                RowEntry::new_value(b"key3", b"value3", 4),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn should_not_dedup_valid_merge_entries() {
        let mut iters: VecDeque<TestIterator> = VecDeque::new();
        iters.push_back(
            TestIterator::new()
                .with_row_entry(RowEntry::new_merge(b"k1", b"b", 2))
                .with_row_entry(RowEntry::new_merge(b"k1", b"a", 1)),
        );
        iters.push_back(TestIterator::new().with_row_entry(RowEntry::new_merge(b"k1", b"c", 3)));

        let mut merge_iter = MergeIterator::new(iters).unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_merge(b"k1", b"c", 3),
                RowEntry::new_merge(b"k1", b"b", 2),
                RowEntry::new_merge(b"k1", b"a", 1),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn should_advance_past_old_merge_entries() {
        let mut iters: VecDeque<TestIterator> = VecDeque::new();
        iters.push_back(
            TestIterator::new()
                .with_row_entry(RowEntry::new_merge(b"k1", b"a", 2))
                .with_row_entry(RowEntry::new_merge(b"k1", b"b", 1)),
        );
        iters.push_back(TestIterator::new().with_row_entry(RowEntry::new_value(
            b"k1",
            b"new_value",
            3,
        )));

        let mut merge_iter = MergeIterator::new(iters).unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![RowEntry::new_value(b"k1", b"new_value", 3)],
        )
        .await;
    }

    #[tokio::test]
    async fn test_merge_iterator_descending_should_include_entries_in_reverse_order() {
        let mut iters: VecDeque<TestIterator> = VecDeque::new();
        // Each sub-iterator provides entries in descending key order
        iters.push_back(
            TestIterator::new()
                .with_entry(b"zzzz", b"26262626", 0)
                .with_entry(b"cccc", b"3333", 0)
                .with_entry(b"aaaa", b"1111", 0),
        );
        iters.push_back(
            TestIterator::new()
                .with_entry(b"yyyy", b"25252525", 0)
                .with_entry(b"xxxx", b"24242424", 0)
                .with_entry(b"bbbb", b"2222", 0),
        );

        let mut merge_iter =
            MergeIterator::new_with_order(iters, IterationOrder::Descending).unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"zzzz", b"26262626", 0),
                RowEntry::new_value(b"yyyy", b"25252525", 0),
                RowEntry::new_value(b"xxxx", b"24242424", 0),
                RowEntry::new_value(b"cccc", b"3333", 0),
                RowEntry::new_value(b"bbbb", b"2222", 0),
                RowEntry::new_value(b"aaaa", b"1111", 0),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_merge_iterator_descending_dedup() {
        let iter1 = TestIterator::new()
            .with_entry(b"cccc", b"use this one c", 5)
            .with_entry(b"aaaa", b"1111", 0);
        let iter2 = TestIterator::new()
            .with_entry(b"xxxx", b"use this one x", 4)
            .with_entry(b"cccc", b"badc1", 1);

        let mut merge_iter =
            MergeIterator::new_with_order([iter1, iter2], IterationOrder::Descending).unwrap();

        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_value(b"xxxx", b"use this one x", 4),
                RowEntry::new_value(b"cccc", b"use this one c", 5),
                RowEntry::new_value(b"aaaa", b"1111", 0),
            ],
        )
        .await;
    }
}
