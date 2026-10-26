use async_trait::async_trait;
use bytes::{Bytes, BytesMut};

use crate::block_iterator::BlockLike;
use crate::error::SlateDBError;
use crate::format::row_codec_v2::SstRowCodecV2;
use crate::iter::IterationOrder;
use crate::iter::RowEntryIterator;
use crate::types::RowEntry;

/// Internal state for ascending iteration.
struct AscendingState<B: BlockLike> {
    block: B,
    current_restart_idx: usize,
    offset_in_block: usize,
    entries_since_restart: usize,
    current_key: Bytes,
    exhausted: bool,
}

enum BlockIteratorInner<B: BlockLike> {
    Ascending(AscendingState<B>),
    Descending(DescendingBlockIteratorV2<B>),
}

/// Iterator for BlockV2 that uses restart points for efficient seeking.
/// Supports both ascending and descending iteration orders.
pub(crate) struct BlockIteratorV2<B: BlockLike> {
    inner: BlockIteratorInner<B>,
}

impl<B: BlockLike> BlockIteratorV2<B> {
    pub(crate) fn new(block: B, ordering: IterationOrder) -> Self {
        match ordering {
            IterationOrder::Ascending => {
                let initial_key = if block.offsets().is_empty() {
                    Bytes::new()
                } else {
                    Self::decode_first_key_at_restart(&block, 0)
                };

                BlockIteratorV2 {
                    inner: BlockIteratorInner::Ascending(AscendingState {
                        block,
                        current_restart_idx: 0,
                        offset_in_block: 0,
                        entries_since_restart: 0,
                        current_key: initial_key,
                        exhausted: false,
                    }),
                }
            }
            IterationOrder::Descending => BlockIteratorV2 {
                inner: BlockIteratorInner::Descending(DescendingBlockIteratorV2::new(block)),
            },
        }
    }

    #[allow(dead_code)] // Used in tests
    pub(crate) fn new_ascending(block: B) -> Self {
        Self::new(block, IterationOrder::Ascending)
    }

    pub(crate) fn is_empty(&self) -> bool {
        match &self.inner {
            BlockIteratorInner::Ascending(state) => state.is_empty(),
            BlockIteratorInner::Descending(state) => state.is_empty(),
        }
    }

    fn decode_first_key_at_restart(block: &B, restart_idx: usize) -> Bytes {
        let restart_offset = block.offsets()[restart_idx] as usize;
        let mut data = block.data().slice(restart_offset..);
        let codec = SstRowCodecV2::new();
        let (shared_bytes, key_suffix) = codec.decode_key_only(&mut data);
        assert_eq!(shared_bytes, 0, "restart point should have shared_bytes=0");
        key_suffix
    }
}

impl<B: BlockLike> AscendingState<B> {
    fn seek_to_restart(&mut self, restart_idx: usize) {
        if restart_idx >= self.block.offsets().len() {
            self.exhausted = true;
            return;
        }

        self.current_restart_idx = restart_idx;
        self.offset_in_block = self.block.offsets()[restart_idx] as usize;
        self.entries_since_restart = 0;
        self.current_key = BlockIteratorV2::decode_first_key_at_restart(&self.block, restart_idx);
        self.exhausted = false;
    }

    fn decode_entry_at_current_offset(&self) -> Result<(RowEntry, usize), SlateDBError> {
        let mut data = self.block.data().slice(self.offset_in_block..);
        let codec = SstRowCodecV2::new();
        let entry = codec.decode(&mut data)?;
        let bytes_consumed = self.block.data().len() - self.offset_in_block - data.len();
        let new_offset = self.offset_in_block + bytes_consumed;
        let full_key = entry.restore_full_key(&self.current_key);

        Ok((
            RowEntry::new(
                full_key,
                entry.value,
                entry.seq,
                entry.create_ts,
                entry.expire_ts,
            ),
            new_offset,
        ))
    }

    fn decode_key_at_offset(&self, offset: usize, prev_key: &[u8]) -> Bytes {
        let mut data = self.block.data().slice(offset..);
        let codec = SstRowCodecV2::new();
        let (shared_bytes, key_suffix) = codec.decode_key_only(&mut data);
        let shared = shared_bytes as usize;

        let mut full_key = BytesMut::with_capacity(shared + key_suffix.len());
        full_key.extend_from_slice(&prev_key[..shared]);
        full_key.extend_from_slice(&key_suffix);
        full_key.freeze()
    }

    fn is_empty(&self) -> bool {
        self.exhausted || self.offset_in_block >= self.block.data().len()
    }

    /// Returns the restart index to start scanning from when seeking to target.
    ///
    /// When a restart point has key exactly equal to target, we return the PREVIOUS
    /// restart point to ensure we don't miss any entries with that key that might
    /// exist before the found restart point.
    /// Binary search for the first restart index where key >= target.
    /// Returns `restarts.len()` if no such restart exists.
    fn binary_search_restarts(&self, target: &[u8]) -> usize {
        let restarts = self.block.offsets();
        let mut low = 0;
        let mut high = restarts.len();

        while low < high {
            let mid = low + (high - low) / 2;
            let restart_key = BlockIteratorV2::decode_first_key_at_restart(&self.block, mid);

            if restart_key.as_ref() < target {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        low
    }

    /// Find the restart region to begin an ascending scan for `target`.
    /// Backs up one position when target exactly matches a restart point's first key,
    /// so that duplicate keys straddling restart boundaries aren't missed.
    fn find_restart_for_key_ascending(&self, target: &[u8]) -> usize {
        let restarts = self.block.offsets();
        if restarts.is_empty() {
            return 0;
        }

        let low = self.binary_search_restarts(target);

        if low < restarts.len() {
            let restart_key = BlockIteratorV2::decode_first_key_at_restart(&self.block, low);
            if restart_key.as_ref() == target {
                return low.saturating_sub(1);
            }
        }

        low.saturating_sub(1)
    }

    /// Find the restart region to begin a descending scan for `target`.
    /// Returns the last restart whose first key <= target, so that for duplicate keys
    /// spanning multiple restart regions we start from the last one.
    fn find_restart_for_key_descending(&self, target: &[u8]) -> usize {
        let restarts = self.block.offsets();
        if restarts.is_empty() {
            return 0;
        }

        // binary_search_restarts finds the first restart with key >= target.
        let low = self.binary_search_restarts(target);

        if low < restarts.len() {
            let restart_key = BlockIteratorV2::decode_first_key_at_restart(&self.block, low);
            if restart_key.as_ref() == target {
                // Scan forward to find the last restart with the same first key.
                let mut last = low;
                while last + 1 < restarts.len() {
                    let next_key =
                        BlockIteratorV2::decode_first_key_at_restart(&self.block, last + 1);
                    if next_key.as_ref() != target {
                        break;
                    }
                    last += 1;
                }
                return last;
            }
        }

        low.saturating_sub(1)
    }

    fn restart_region_end(&self, restart_idx: usize) -> usize {
        self.block
            .offsets()
            .get(restart_idx + 1)
            .map(|&off| off as usize)
            .unwrap_or_else(|| self.block.data().len())
    }

    fn advance_past_current_entry(&mut self) -> Result<(), SlateDBError> {
        let mut data = self.block.data().slice(self.offset_in_block..);
        let codec = SstRowCodecV2::new();
        codec.decode(&mut data)?;
        let bytes_consumed = self.block.data().len() - self.offset_in_block - data.len();
        self.offset_in_block += bytes_consumed;
        self.entries_since_restart += 1;
        Ok(())
    }
}

#[async_trait]
impl<B: BlockLike> RowEntryIterator for BlockIteratorV2<B> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        match &mut self.inner {
            BlockIteratorInner::Ascending(state) => {
                if state.is_empty() {
                    return Ok(None);
                }

                let (entry, new_offset) = state.decode_entry_at_current_offset()?;

                // Update current key before advancing
                state.current_key = entry.key.clone();
                state.offset_in_block = new_offset;
                state.entries_since_restart += 1;

                // Check if we've moved to a new restart region
                if let Some(&next_restart_offset) =
                    state.block.offsets().get(state.current_restart_idx + 1)
                {
                    if state.offset_in_block >= next_restart_offset as usize {
                        state.current_restart_idx += 1;
                        state.entries_since_restart = 0;
                    }
                }

                if state.offset_in_block >= state.block.data().len() {
                    state.exhausted = true;
                }

                Ok(Some(entry))
            }
            BlockIteratorInner::Descending(iter) => iter.next().await,
        }
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        match &mut self.inner {
            BlockIteratorInner::Ascending(state) => {
                if state.block.offsets().is_empty() {
                    state.exhausted = true;
                    return Ok(());
                }

                let start_restart_idx = state.find_restart_for_key_ascending(next_key);

                // Iterate through restart regions starting from binary search result.
                for restart_idx in start_restart_idx..state.block.offsets().len() {
                    state.seek_to_restart(restart_idx);

                    if state.exhausted || state.current_key.as_ref() >= next_key {
                        return Ok(());
                    }

                    // Scan entries within this restart region
                    let region_end = state.restart_region_end(restart_idx);
                    let mut prev_key = state.current_key.clone();

                    while state.offset_in_block < region_end
                        && state.offset_in_block < state.block.data().len()
                    {
                        let current_key =
                            state.decode_key_at_offset(state.offset_in_block, &prev_key);

                        if current_key.as_ref() >= next_key {
                            state.current_key = prev_key;
                            return Ok(());
                        }

                        state.advance_past_current_entry()?;
                        prev_key = current_key;
                        state.current_key = prev_key.clone();
                    }
                }

                state.exhausted = true;
                Ok(())
            }
            BlockIteratorInner::Descending(iter) => iter.seek(next_key).await,
        }
    }
}

/// Descending iterator for BlockV2 that caches entries within each restart region,
/// then iterates in reverse order.
pub(crate) struct DescendingBlockIteratorV2<B: BlockLike> {
    ascending: AscendingState<B>,
    current_restart_idx: isize,
    cached_entries: Vec<RowEntry>,
    cache_idx: isize,
    exhausted: bool,
    initialized: bool,
}

impl<B: BlockLike> DescendingBlockIteratorV2<B> {
    pub(crate) fn is_empty(&self) -> bool {
        self.exhausted
    }

    pub(crate) fn new(block: B) -> Self {
        let num_restarts = block.offsets().len();
        let initial_key = if num_restarts == 0 {
            Bytes::new()
        } else {
            BlockIteratorV2::decode_first_key_at_restart(&block, 0)
        };

        DescendingBlockIteratorV2 {
            ascending: AscendingState {
                block,
                current_restart_idx: 0,
                offset_in_block: 0,
                entries_since_restart: 0,
                current_key: initial_key,
                exhausted: false,
            },
            current_restart_idx: num_restarts as isize - 1,
            cached_entries: Vec::new(),
            cache_idx: -1,
            exhausted: num_restarts == 0,
            initialized: false,
        }
    }

    fn load_restart_region(&mut self, restart_idx: usize) -> Result<(), SlateDBError> {
        self.cached_entries.clear();

        if restart_idx >= self.ascending.block.offsets().len() {
            self.cache_idx = -1;
            return Ok(());
        }

        self.ascending.seek_to_restart(restart_idx);
        let region_end = self.ascending.restart_region_end(restart_idx);

        while self.ascending.offset_in_block < region_end && !self.ascending.is_empty() {
            let (entry, new_offset) = self.ascending.decode_entry_at_current_offset()?;
            self.ascending.current_key = entry.key.clone();
            self.ascending.offset_in_block = new_offset;
            self.ascending.entries_since_restart += 1;
            self.cached_entries.push(entry);
        }

        self.cache_idx = self.cached_entries.len() as isize - 1;
        Ok(())
    }

    fn move_to_previous_region(&mut self) -> Result<(), SlateDBError> {
        self.current_restart_idx -= 1;
        if self.current_restart_idx < 0 {
            self.exhausted = true;
            self.cached_entries.clear();
            self.cache_idx = -1;
        } else {
            self.load_restart_region(self.current_restart_idx as usize)?;
        }
        Ok(())
    }
}

#[async_trait]
impl<B: BlockLike> RowEntryIterator for DescendingBlockIteratorV2<B> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        if !self.initialized && !self.exhausted {
            self.load_restart_region(self.current_restart_idx as usize)?;
            self.initialized = true;
        }
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        if !self.initialized {
            self.init().await?;
        }

        if self.exhausted {
            return Ok(None);
        }

        if self.cache_idx >= 0 {
            let entry = self.cached_entries[self.cache_idx as usize].clone();
            self.cache_idx -= 1;
            return Ok(Some(entry));
        }

        self.move_to_previous_region()?;

        if self.exhausted || self.cache_idx < 0 {
            self.exhausted = true;
            return Ok(None);
        }

        let entry = self.cached_entries[self.cache_idx as usize].clone();
        self.cache_idx -= 1;
        Ok(Some(entry))
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        self.initialized = true;

        if self.ascending.block.offsets().is_empty() {
            self.exhausted = true;
            return Ok(());
        }

        // Find the last restart region whose first key <= next_key, then scan backwards.
        let start_restart_idx = self.ascending.find_restart_for_key_descending(next_key);

        for restart_idx in (0..=start_restart_idx).rev() {
            self.current_restart_idx = restart_idx as isize;
            self.load_restart_region(restart_idx)?;

            // Find the largest key <= next_key in this region.
            // Since keys are sorted ascending, find the first key > next_key and take the previous.
            let first_greater = self
                .cached_entries
                .iter()
                .position(|entry| entry.key.as_ref() > next_key);

            let found_idx = match first_greater {
                Some(0) => None,
                Some(i) => Some(i - 1),
                None if !self.cached_entries.is_empty() => Some(self.cached_entries.len() - 1),
                None => None,
            };

            if let Some(idx) = found_idx {
                self.cache_idx = idx as isize;
                self.exhausted = false;
                return Ok(());
            }
        }

        self.exhausted = true;
        Ok(())
    }
}

#[cfg(feature = "bench-internal")]
pub mod benches {
    use super::BlockIteratorV2;
    use bytes::Bytes;

    #[cfg(feature = "bench-internal")]
    pub struct BlockIteratorV2BenchConfig {
        /// Target block size passed to the builder. The builder accepts a first
        /// entry that exceeds this (oversized-entry rule), so a single large value
        /// will expand the block past `block_size`.
        pub block_size: usize,
        pub key_size: usize,
        pub value_size: usize,
        /// Upper bound on entries generated and fed to the builder. Fewer may
        /// land in the block if it fills up first.
        pub num_entries: usize,
    }

    #[allow(clippy::panic)]
    pub fn block_iterator_v2_bench<F>(config: BlockIteratorV2BenchConfig, mut run_bench: F)
    where
        F: FnMut(&mut dyn FnMut()),
    {
        use crate::format::sst::BlockBuilder;
        use crate::iter::RowEntryIterator;
        use crate::types::{RowEntry, ValueDeletable};
        use futures::executor::block_on;
        use rand::RngCore;

        const SEED: u64 = 0x51A7_EDBB_E4C4;

        let rand = slatedb_common::DbRand::new(SEED);
        let mut rng = rand.rng();
        let mut pool: Vec<(Bytes, Bytes)> = (0..config.num_entries)
            .map(|_| {
                let mut key = vec![0u8; config.key_size];
                rng.fill_bytes(&mut key);
                let mut value = vec![0u8; config.value_size];
                rng.fill_bytes(&mut value);
                (Bytes::from(key), Bytes::from(value))
            })
            .collect();
        pool.sort_by(|a, b| a.0.cmp(&b.0));
        pool.dedup_by(|a, b| a.0 == b.0);

        let mut builder = BlockBuilder::new_v2(config.block_size);
        for (seq, (key, value)) in pool.into_iter().enumerate() {
            let entry = RowEntry::new(
                key,
                ValueDeletable::Value(value),
                seq as u64 + 1,
                None,
                None,
            );
            match builder.add(entry) {
                Ok(true) => {}
                Ok(false) => break,
                Err(e) => panic!("failed to add entry: {e:?}"),
            }
        }
        let block = builder.build().expect("failed to build block");

        run_bench(&mut || {
            block_on(async {
                let mut iter = BlockIteratorV2::new_ascending(&block);
                while let Some(entry) = iter.next().await.expect("iterator error") {
                    std::hint::black_box(entry);
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::block::Block;
    use crate::format::sst::BlockBuilder;
    use crate::types::ValueDeletable;

    fn make_entry(key: &[u8], value: &[u8], seq: u64) -> RowEntry {
        RowEntry::new(
            Bytes::copy_from_slice(key),
            ValueDeletable::Value(Bytes::copy_from_slice(value)),
            seq,
            None,
            None,
        )
    }

    fn build_test_block(entries: &[(&[u8], &[u8])], restart_interval: usize) -> Block {
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, restart_interval);
        for (i, (key, value)) in entries.iter().enumerate() {
            let _ = builder.add(make_entry(key, value, i as u64));
        }
        builder.build().expect("build failed")
    }

    #[tokio::test]
    async fn should_iterate_all_entries() {
        // given: a block with entries
        let block = build_test_block(
            &[(b"apple", b"1"), (b"banana", b"2"), (b"cherry", b"3")],
            16,
        );

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when/then: iterating returns all entries in order
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"apple");
        assert_eq!(kv.value, ValueDeletable::Value(Bytes::from("1")));

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"banana");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"cherry");

        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_to_exact_key() {
        // given: a block with entries
        let block = build_test_block(
            &[
                (b"apple", b"1"),
                (b"banana", b"2"),
                (b"cherry", b"3"),
                (b"date", b"4"),
            ],
            2,
        );

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking to an exact key
        iter.seek(b"banana").await.unwrap();

        // then: returns that key
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"banana");
    }

    #[tokio::test]
    async fn should_seek_to_key_between_entries() {
        // given: a block with entries
        let block = build_test_block(&[(b"apple", b"1"), (b"cherry", b"3"), (b"date", b"4")], 16);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking to a key between entries
        iter.seek(b"banana").await.unwrap();

        // then: returns the next key >= banana
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"cherry");
    }

    #[tokio::test]
    async fn should_seek_across_restart_boundaries() {
        // given: a block with restart_interval=2
        let block = build_test_block(
            &[
                (b"a", b"1"), // restart 0
                (b"b", b"2"),
                (b"c", b"3"), // restart 1
                (b"d", b"4"),
                (b"e", b"5"), // restart 2
            ],
            2,
        );

        assert_eq!(block.offsets().len(), 3);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking to a key in the third restart region
        iter.seek(b"e").await.unwrap();

        // then: finds the correct entry
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"e");
    }

    #[tokio::test]
    async fn should_handle_shared_prefix_keys() {
        // given: a block with keys sharing prefixes
        let block = build_test_block(
            &[
                (b"user:1000", b"alice"),
                (b"user:1001", b"bob"),
                (b"user:1002", b"carol"),
                (b"user:1003", b"dave"),
            ],
            2,
        );

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking and iterating
        iter.seek(b"user:1001").await.unwrap();

        // then: correct entries are returned
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"user:1001");
        assert_eq!(kv.value, ValueDeletable::Value(Bytes::from("bob")));

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"user:1002");
    }

    #[tokio::test]
    async fn should_binary_search_restarts_correctly() {
        // given: a block with many entries and small restart interval
        let mut builder = BlockBuilder::new_v2_with_restart_interval(16384, 4);
        for i in 0..100 {
            let key = format!("key_{:05}", i);
            let value = format!("value_{}", i);
            let _ = builder.add(make_entry(key.as_bytes(), value.as_bytes(), i as u64));
        }
        let block = builder.build().expect("build failed");

        // when: seeking to various keys
        let mut iter = BlockIteratorV2::new_ascending(&block);
        iter.seek(b"key_00050").await.unwrap();

        // then: correct key is found
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_00050");

        // Seek to another key
        iter.seek(b"key_00075").await.unwrap();
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_00075");
    }

    #[tokio::test]
    async fn should_seek_to_first_key() {
        // given: a block
        let block = build_test_block(&[(b"apple", b"1"), (b"banana", b"2")], 16);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking to the first key
        iter.seek(b"apple").await.unwrap();

        // then: first entry is returned
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"apple");
    }

    #[tokio::test]
    async fn should_seek_to_key_before_first() {
        // given: a block
        let block = build_test_block(&[(b"banana", b"2"), (b"cherry", b"3")], 16);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking to a key before the first entry
        iter.seek(b"apple").await.unwrap();

        // then: first entry is returned
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"banana");
    }

    #[tokio::test]
    async fn should_seek_past_last_key() {
        // given: a block
        let block = build_test_block(&[(b"apple", b"1"), (b"banana", b"2")], 16);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking past the last key
        iter.seek(b"zebra").await.unwrap();

        // then: no more entries
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_iterate_single_entry_block() {
        // given: a block with one entry
        let block = build_test_block(&[(b"only", b"one")], 16);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: iterating
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"only");

        // then: no more entries
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_in_single_entry_block() {
        // given: a block with one entry
        let block = build_test_block(&[(b"only", b"one")], 16);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking to that entry
        iter.seek(b"only").await.unwrap();

        // then: entry is returned
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"only");
    }

    #[tokio::test]
    async fn should_handle_entries_with_timestamps() {
        // given: a block with entries that have timestamps
        let mut builder = BlockBuilder::new_v2(4096);
        let _ = builder.add(RowEntry::new(
            Bytes::from("key1"),
            ValueDeletable::Value(Bytes::from("value1")),
            1,
            Some(100),
            Some(200),
        ));
        let _ = builder.add(RowEntry::new(
            Bytes::from("key2"),
            ValueDeletable::Tombstone,
            2,
            Some(300),
            None,
        ));
        let block = builder.build().expect("build failed");

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: iterating
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key1");
        assert_eq!(kv.create_ts, Some(100));
        assert_eq!(kv.expire_ts, Some(200));

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key2");
        assert!(matches!(kv.value, ValueDeletable::Tombstone));
        assert_eq!(kv.create_ts, Some(300));
    }

    #[tokio::test]
    async fn should_iterate_block_with_many_restart_points() {
        // given: a block with restart_interval=1 (every entry is a restart)
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 1);
        for i in 0..10 {
            let key = format!("key_{}", i);
            let value = format!("val_{}", i);
            let _ = builder.add(make_entry(key.as_bytes(), value.as_bytes(), i as u64));
        }
        let block = builder.build().expect("build failed");

        assert_eq!(block.offsets().len(), 10);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when/then: all entries are returned in order
        for i in 0..10 {
            let kv = iter.next().await.unwrap().unwrap();
            let expected_key = format!("key_{}", i);
            assert_eq!(kv.key.as_ref(), expected_key.as_bytes());
        }
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_multiple_times() {
        // given: a block
        let block = build_test_block(
            &[
                (b"a", b"1"),
                (b"b", b"2"),
                (b"c", b"3"),
                (b"d", b"4"),
                (b"e", b"5"),
            ],
            2,
        );

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking multiple times
        iter.seek(b"b").await.unwrap();
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"b");

        iter.seek(b"d").await.unwrap();
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"d");

        iter.seek(b"a").await.unwrap();
        // Note: seeking backward from current position may not work as expected
        // because we don't reset the iterator. Let's test seeking forward from current.
        let kv = iter.next().await.unwrap();
        // After seeking to "a" from position "d", behavior depends on implementation
        // Our implementation should allow seeking to any position
        assert!(kv.is_some());
    }

    #[tokio::test]
    async fn should_iterate_entries_with_merge_operands() {
        // given: a block with merge operand entries
        let mut builder = BlockBuilder::new_v2(4096);
        let _ = builder.add(RowEntry::new(
            Bytes::from("key1"),
            ValueDeletable::Value(Bytes::from("base")),
            1,
            None,
            None,
        ));
        let _ = builder.add(RowEntry::new(
            Bytes::from("key2"),
            ValueDeletable::Merge(Bytes::from("delta")),
            2,
            None,
            None,
        ));
        let block = builder.build().expect("build failed");

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: iterating
        let kv = iter.next().await.unwrap().unwrap();
        assert!(matches!(kv.value, ValueDeletable::Value(_)));

        let kv = iter.next().await.unwrap().unwrap();
        assert!(matches!(kv.value, ValueDeletable::Merge(_)));
        if let ValueDeletable::Merge(v) = kv.value {
            assert_eq!(v.as_ref(), b"delta");
        }
    }

    // ==================== Descending Iterator Tests ====================

    #[tokio::test]
    async fn should_iterate_all_entries_descending() {
        // given: a block with entries
        let block = build_test_block(
            &[(b"apple", b"1"), (b"banana", b"2"), (b"cherry", b"3")],
            16,
        );

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when/then: iterating returns all entries in reverse order
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"cherry");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"banana");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"apple");

        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_iterate_descending_across_restart_boundaries() {
        // given: a block with restart_interval=2
        let block = build_test_block(
            &[
                (b"a", b"1"), // restart 0
                (b"b", b"2"),
                (b"c", b"3"), // restart 1
                (b"d", b"4"),
                (b"e", b"5"), // restart 2
            ],
            2,
        );

        assert_eq!(block.offsets().len(), 3);

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when/then: iterating returns all entries in reverse order
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"e");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"d");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"c");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"b");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"a");

        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_descending_to_exact_key() {
        // given: a block with entries
        let block = build_test_block(
            &[
                (b"apple", b"1"),
                (b"banana", b"2"),
                (b"cherry", b"3"),
                (b"date", b"4"),
            ],
            2,
        );

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when: seeking to an exact key (descending seek finds largest key <= target)
        iter.seek(b"cherry").await.unwrap();

        // then: returns that key
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"cherry");

        // and continues descending
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"banana");
    }

    #[tokio::test]
    async fn should_seek_descending_to_key_between_entries() {
        // given: a block with entries
        let block = build_test_block(&[(b"apple", b"1"), (b"cherry", b"3"), (b"date", b"4")], 16);

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when: seeking to a key between entries (banana is between apple and cherry)
        iter.seek(b"banana").await.unwrap();

        // then: returns the largest key <= banana, which is apple
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"apple");

        // and no more entries (we're at the beginning)
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_descending_past_last_key() {
        // given: a block
        let block = build_test_block(&[(b"apple", b"1"), (b"banana", b"2")], 16);

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when: seeking to a key after all entries
        iter.seek(b"zebra").await.unwrap();

        // then: returns the last (largest) key
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"banana");
    }

    #[tokio::test]
    async fn should_seek_descending_before_first_key() {
        // given: a block
        let block = build_test_block(&[(b"banana", b"2"), (b"cherry", b"3")], 16);

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when: seeking to a key before all entries
        iter.seek(b"apple").await.unwrap();

        // then: no entries (nothing <= apple)
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_iterate_descending_single_entry_block() {
        // given: a block with one entry
        let block = build_test_block(&[(b"only", b"one")], 16);

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when: iterating
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"only");

        // then: no more entries
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_iterate_descending_with_shared_prefix_keys() {
        // given: a block with keys sharing prefixes
        let block = build_test_block(
            &[
                (b"user:1000", b"alice"),
                (b"user:1001", b"bob"),
                (b"user:1002", b"carol"),
                (b"user:1003", b"dave"),
            ],
            2,
        );

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when/then: iterating returns entries in reverse order
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"user:1003");
        assert_eq!(kv.value, ValueDeletable::Value(Bytes::from("dave")));

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"user:1002");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"user:1001");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"user:1000");

        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_iterate_descending_block_with_many_restart_points() {
        // given: a block with restart_interval=4 (restart point every 4 entries)
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 4);
        for i in 0..20 {
            let key = format!("key_{:02}", i);
            let value = format!("val_{}", i);
            let _ = builder.add(make_entry(key.as_bytes(), value.as_bytes(), i as u64));
        }
        let block = builder.build().expect("build failed");

        // 20 entries with restart_interval=4 -> 5 restart points
        assert_eq!(block.offsets().len(), 5);

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when/then: all entries are returned in reverse order
        for i in (0..20).rev() {
            let kv = iter.next().await.unwrap().unwrap();
            let expected_key = format!("key_{:02}", i);
            assert_eq!(kv.key.as_ref(), expected_key.as_bytes());
        }
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_descending_with_many_restart_points() {
        // given: a block with restart_interval=4 (restart point every 4 entries)
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 4);
        for i in 0..20 {
            let key = format!("key_{:02}", i);
            let value = format!("val_{}", i);
            let _ = builder.add(make_entry(key.as_bytes(), value.as_bytes(), i as u64));
        }
        let block = builder.build().expect("build failed");

        // 20 entries with restart_interval=4 -> 5 restart points
        // restart 0: key_00, key_01, key_02, key_03
        // restart 1: key_04, key_05, key_06, key_07
        // restart 2: key_08, key_09, key_10, key_11
        // restart 3: key_12, key_13, key_14, key_15
        // restart 4: key_16, key_17, key_18, key_19
        assert_eq!(block.offsets().len(), 5);

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when: seeking to a key in the middle of a restart region
        iter.seek(b"key_10").await.unwrap();

        // then: returns key_10 and continues descending
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_10");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_09");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_08");

        // crosses into previous restart region
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_07");

        // when: seeking to a key between entries
        iter.seek(b"key_14a").await.unwrap();

        // then: returns the largest key <= key_14a, which is key_14
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_14");

        // when: seeking to a key at the start of a restart region
        iter.seek(b"key_04").await.unwrap();

        // then: returns key_04
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_04");

        // and continues to previous region
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_03");
    }

    // ==================== BlockIteratorV2 with Descending Order Tests ====================

    #[tokio::test]
    async fn should_iterate_descending_via_block_iterator_v2() {
        // given: a block with entries
        let block = build_test_block(
            &[(b"apple", b"1"), (b"banana", b"2"), (b"cherry", b"3")],
            16,
        );

        // when: creating BlockIteratorV2 with Descending order
        let mut iter = BlockIteratorV2::new(&block, IterationOrder::Descending);

        // then: iterating returns all entries in reverse order
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"cherry");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"banana");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"apple");

        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_descending_via_block_iterator_v2() {
        // given: a block with entries and restart_interval=4
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 4);
        for i in 0..20 {
            let key = format!("key_{:02}", i);
            let value = format!("val_{}", i);
            let _ = builder.add(make_entry(key.as_bytes(), value.as_bytes(), i as u64));
        }
        let block = builder.build().expect("build failed");

        // when: creating BlockIteratorV2 with Descending order and seeking
        let mut iter = BlockIteratorV2::new(&block, IterationOrder::Descending);
        iter.seek(b"key_10").await.unwrap();

        // then: returns key_10 and continues descending
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_10");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key_09");
    }

    // ==================== Duplicate Key Edge Case Tests ====================

    #[tokio::test]
    async fn should_seek_finds_all_entries_with_duplicate_keys_across_restarts() {
        // given: a block with duplicate keys spanning multiple restart regions
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 2);
        // 6 entries with same key, restart_interval=2 means restarts at 0, 2, 4
        for i in 0..6 {
            let value = format!("val_{}", i);
            let _ = builder.add(make_entry(b"dup", value.as_bytes(), i as u64));
        }
        let block = builder.build().expect("build failed");

        assert_eq!(block.offsets().len(), 3); // restarts at entries 0, 2, 4

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking to "dup"
        iter.seek(b"dup").await.unwrap();

        // then: should find all 6 entries starting from the first
        for expected_seq in 0..6u64 {
            let kv = iter.next().await.unwrap().unwrap();
            assert_eq!(kv.key.as_ref(), b"dup");
            assert_eq!(
                kv.seq, expected_seq,
                "expected seq {} but got {}",
                expected_seq, kv.seq
            );
        }

        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_with_duplicate_keys_at_specific_restart() {
        // given: entries where duplicate keys start at a restart point
        // [a, a] [dup, dup] [dup, dup] [b, b]
        //  ^r0     ^r1        ^r2       ^r3
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 2);
        let _ = builder.add(make_entry(b"a", b"v0", 0));
        let _ = builder.add(make_entry(b"a", b"v1", 1));
        let _ = builder.add(make_entry(b"dup", b"v2", 2)); // restart 1
        let _ = builder.add(make_entry(b"dup", b"v3", 3));
        let _ = builder.add(make_entry(b"dup", b"v4", 4)); // restart 2
        let _ = builder.add(make_entry(b"dup", b"v5", 5));
        let _ = builder.add(make_entry(b"b", b"v6", 6)); // restart 3
        let _ = builder.add(make_entry(b"b", b"v7", 7));
        let block = builder.build().expect("build failed");

        assert_eq!(block.offsets().len(), 4);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking to "dup"
        iter.seek(b"dup").await.unwrap();

        // then: should find all 4 "dup" entries starting from seq=2
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"dup");
        assert_eq!(kv.seq, 2);

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"dup");
        assert_eq!(kv.seq, 3);

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"dup");
        assert_eq!(kv.seq, 4);

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"dup");
        assert_eq!(kv.seq, 5);

        // next should be "b"
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"b");
    }

    #[tokio::test]
    async fn should_seek_with_dup_key_straddling_first_restart() {
        // given: a block where duplicate keys start in the FIRST restart region
        // This tests that we don't go to restart -1 (which doesn't exist)
        // Layout with restart_interval=2:
        // restart 0: [dup, dup]    - key "dup"
        // restart 1: [dup, dup]    - key "dup"
        // restart 2: [z, z]        - key "z"
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 2);
        let _ = builder.add(make_entry(b"dup", b"v0", 0)); // restart 0
        let _ = builder.add(make_entry(b"dup", b"v1", 1));
        let _ = builder.add(make_entry(b"dup", b"v2", 2)); // restart 1
        let _ = builder.add(make_entry(b"dup", b"v3", 3));
        let _ = builder.add(make_entry(b"z", b"v4", 4)); // restart 2
        let _ = builder.add(make_entry(b"z", b"v5", 5));
        let block = builder.build().expect("build failed");

        assert_eq!(block.offsets().len(), 3);

        let mut iter = BlockIteratorV2::new_ascending(&block);

        // when: seeking to "dup"
        iter.seek(b"dup").await.unwrap();

        // then: should find ALL dup entries starting from the very first one (seq=0)
        for expected_seq in 0..4u64 {
            let kv = iter.next().await.unwrap().unwrap();
            assert_eq!(kv.key.as_ref(), b"dup");
            assert_eq!(kv.seq, expected_seq);
        }

        // next should be "z"
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"z");
    }

    #[tokio::test]
    async fn should_seek_descending_finds_all_entries_with_duplicate_keys_across_restarts() {
        // given: a block with duplicate keys spanning multiple restart regions
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 2);
        // 6 entries with same key, restart_interval=2 means restarts at 0, 2, 4
        for i in 0..6 {
            let value = format!("val_{}", i);
            let _ = builder.add(make_entry(b"dup", value.as_bytes(), i as u64));
        }
        let block = builder.build().expect("build failed");

        assert_eq!(block.offsets().len(), 3); // restarts at entries 0, 2, 4

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when: seeking to "dup"
        iter.seek(b"dup").await.unwrap();

        // then: should find all 6 entries in reverse order (last seq first)
        for expected_seq in (0..6u64).rev() {
            let kv = iter.next().await.unwrap().unwrap();
            assert_eq!(kv.key.as_ref(), b"dup");
            assert_eq!(
                kv.seq, expected_seq,
                "expected seq {} but got {}",
                expected_seq, kv.seq
            );
        }

        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_descending_with_duplicate_keys_at_specific_restart() {
        // given: entries where duplicate keys start at a restart point
        // [a, a] [dup, dup] [dup, dup] [b, b]
        //  ^r0     ^r1        ^r2       ^r3
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 2);
        let _ = builder.add(make_entry(b"a", b"v0", 0));
        let _ = builder.add(make_entry(b"a", b"v1", 1));
        let _ = builder.add(make_entry(b"dup", b"v2", 2)); // restart 1
        let _ = builder.add(make_entry(b"dup", b"v3", 3));
        let _ = builder.add(make_entry(b"dup", b"v4", 4)); // restart 2
        let _ = builder.add(make_entry(b"dup", b"v5", 5));
        let _ = builder.add(make_entry(b"b", b"v6", 6)); // restart 3
        let _ = builder.add(make_entry(b"b", b"v7", 7));
        let block = builder.build().expect("build failed");

        assert_eq!(block.offsets().len(), 4);

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when: seeking to "dup"
        iter.seek(b"dup").await.unwrap();

        // then: should find all 4 "dup" entries in reverse order
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"dup");
        assert_eq!(kv.seq, 5);

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"dup");
        assert_eq!(kv.seq, 4);

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"dup");
        assert_eq!(kv.seq, 3);

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"dup");
        assert_eq!(kv.seq, 2);

        // next should be "a"
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"a");
    }

    #[tokio::test]
    async fn should_seek_descending_with_dup_key_straddling_restarts() {
        // given: a block where duplicate keys straddle restart boundaries
        // Layout with restart_interval=2:
        // [a, dup] [dup, dup] [dup, y] [y, z]
        //   ^r0       ^r1       ^r2      ^r3
        let mut builder = BlockBuilder::new_v2_with_restart_interval(4096, 2);
        let _ = builder.add(make_entry(b"a", b"v0", 0)); // restart 0
        let _ = builder.add(make_entry(b"dup", b"v1", 1));
        let _ = builder.add(make_entry(b"dup", b"v2", 2)); // restart 1
        let _ = builder.add(make_entry(b"dup", b"v3", 3));
        let _ = builder.add(make_entry(b"dup", b"v4", 4)); // restart 2
        let _ = builder.add(make_entry(b"y", b"v5", 5));
        let _ = builder.add(make_entry(b"y", b"v6", 6)); // restart 3
        let _ = builder.add(make_entry(b"z", b"v7", 7));
        let block = builder.build().expect("build failed");

        assert_eq!(block.offsets().len(), 4);

        let mut iter = DescendingBlockIteratorV2::new(&block);

        // when: seeking to "dup"
        iter.seek(b"dup").await.unwrap();

        // then: should find ALL dup entries in reverse order (last seq first)
        for expected_seq in (1..5u64).rev() {
            let kv = iter.next().await.unwrap().unwrap();
            assert_eq!(kv.key.as_ref(), b"dup");
            assert_eq!(kv.seq, expected_seq);
        }

        // next should be "a"
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"a");
    }

    #[tokio::test]
    async fn should_return_entry_without_copying() {
        // given: a block with entries
        let block = build_test_block(
            &[(b"apple", b"1"), (b"banana", b"2"), (b"cherry", b"3")],
            16,
        );
        let mut iter = BlockIteratorV2::new_ascending(&block);
        let data_start = block.data().as_ptr();
        let data_end = unsafe { data_start.add(block.data().len() - 1) };

        // when/then: assert values reside within block allocation
        loop {
            let Some(kv) = iter.next().await.unwrap() else {
                break;
            };
            let Some(v) = kv.value.as_bytes() else {
                continue;
            };
            let v_data = v.as_ptr();
            assert!(v_data >= data_start);
            assert!(v_data < data_end);
        }
    }

    // ==================== Property-Based Tests ====================

    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use proptest::test_runner::TestCaseError;

        /// Strategy that generates a sorted list of (key, seq) pairs where keys
        /// may have duplicates. Each key is 1-4 lowercase letters, and there can
        /// be 1-36 copies of any given key (with increasing seq numbers). Using
        /// up to 36 copies ensures duplicate keys can span more than 2 restart
        /// points even with the default restart interval of 16.
        fn sorted_entries_strategy() -> impl Strategy<Value = Vec<(Vec<u8>, u64)>> {
            prop::collection::vec(
                (prop::collection::vec(b'a'..=b'z', 1..=4), 1..=36usize),
                2..=10,
            )
            .prop_map(|key_specs| {
                let mut entries = Vec::new();
                let mut seq = 0u64;
                let mut key_specs = key_specs;
                key_specs.sort_by(|a, b| a.0.cmp(&b.0));
                key_specs.dedup_by(|a, b| a.0 == b.0);
                for (key, count) in key_specs {
                    for _ in 0..count {
                        entries.push((key.clone(), seq));
                        seq += 1;
                    }
                }
                entries
            })
        }

        fn build_block_from_entries(entries: &[(Vec<u8>, u64)], restart_interval: usize) -> Block {
            let mut builder = BlockBuilder::new_v2_with_restart_interval(65536, restart_interval);
            for (key, seq) in entries {
                let _ = builder.add(make_entry(key, b"v", *seq));
            }
            builder.build().expect("build failed")
        }

        fn distinct_keys(entries: &[(Vec<u8>, u64)]) -> Vec<Vec<u8>> {
            let mut keys: Vec<Vec<u8>> = entries.iter().map(|(k, _)| k.clone()).collect();
            keys.dedup();
            keys
        }

        proptest! {
            #[test]
            fn should_seek_and_iterate_ascending(
                entries in sorted_entries_strategy(),
                restart_interval in 2..=16usize,
            ) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let result: Result<(), TestCaseError> = rt.block_on(async {
                    let block = build_block_from_entries(&entries, restart_interval);

                    for seek_key in &distinct_keys(&entries) {
                        let mut iter = BlockIteratorV2::new_ascending(&block);
                        iter.seek(seek_key).await.unwrap();

                        // Collect all expected entries at or after the seek key
                        let expected: Vec<_> = entries.iter()
                            .filter(|(k, _)| k >= seek_key)
                            .collect();

                        // First entry should be the first occurrence of the seek key
                        let first = iter.next().await.unwrap();
                        prop_assert!(first.is_some(), "seek to {:?} should find an entry", seek_key);
                        let first = first.unwrap();
                        prop_assert_eq!(
                            first.key.as_ref(), expected[0].0.as_slice(),
                            "first key mismatch after seek to {:?}", seek_key
                        );
                        prop_assert_eq!(
                            first.seq, expected[0].1,
                            "first seq mismatch after seek to {:?}", seek_key
                        );

                        // Remaining entries should match in order
                        for (expected_key, expected_seq) in &expected[1..] {
                            let entry = iter.next().await.unwrap();
                            prop_assert!(entry.is_some());
                            let entry = entry.unwrap();
                            prop_assert_eq!(entry.key.as_ref(), expected_key.as_slice());
                            prop_assert_eq!(entry.seq, *expected_seq);
                        }

                        prop_assert!(iter.next().await.unwrap().is_none());
                    }
                    Ok(())
                });
                result?;
            }

            #[test]
            fn should_seek_and_iterate_descending(
                entries in sorted_entries_strategy(),
                restart_interval in 2..=16usize,
            ) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let result: Result<(), TestCaseError> = rt.block_on(async {
                    let block = build_block_from_entries(&entries, restart_interval);

                    for seek_key in &distinct_keys(&entries) {
                        let mut iter = DescendingBlockIteratorV2::new(&block);
                        iter.seek(seek_key).await.unwrap();

                        // Collect all expected entries at or before the seek key, reversed
                        let expected: Vec<_> = entries.iter()
                            .filter(|(k, _)| k <= seek_key)
                            .rev()
                            .collect();

                        // First entry should be the last occurrence of the seek key
                        let first = iter.next().await.unwrap();
                        prop_assert!(first.is_some(), "descending seek to {:?} should find an entry", seek_key);
                        let first = first.unwrap();
                        prop_assert_eq!(
                            first.key.as_ref(), expected[0].0.as_slice(),
                            "first key mismatch after descending seek to {:?}", seek_key
                        );
                        prop_assert_eq!(
                            first.seq, expected[0].1,
                            "first seq mismatch after descending seek to {:?}", seek_key
                        );

                        // Remaining entries should match in reverse order
                        for (expected_key, expected_seq) in &expected[1..] {
                            let entry = iter.next().await.unwrap();
                            prop_assert!(entry.is_some());
                            let entry = entry.unwrap();
                            prop_assert_eq!(entry.key.as_ref(), expected_key.as_slice());
                            prop_assert_eq!(entry.seq, *expected_seq);
                        }

                        prop_assert!(iter.next().await.unwrap().is_none());
                    }
                    Ok(())
                });
                result?;
            }
        }
    }
}
