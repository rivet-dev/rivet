use std::cmp::Ordering;
use std::sync::Arc;

use crate::format::block::Block;
use crate::format::row::SstRowCodecV0;
use crate::iter::IterationOrder;
use crate::iter::IterationOrder::Ascending;
use crate::{error::SlateDBError, iter::RowEntryIterator, types::RowEntry};
use async_trait::async_trait;
use bytes::{Buf, Bytes, BytesMut};
use IterationOrder::Descending;

pub(crate) trait BlockLike: Send + Sync {
    fn data(&self) -> &Bytes;
    fn offsets(&self) -> &[u16];
}

impl BlockLike for Block {
    fn data(&self) -> &Bytes {
        &self.data
    }

    fn offsets(&self) -> &[u16] {
        &self.offsets
    }
}

impl BlockLike for &Block {
    fn data(&self) -> &Bytes {
        &self.data
    }

    fn offsets(&self) -> &[u16] {
        &self.offsets
    }
}

impl BlockLike for Arc<Block> {
    fn data(&self) -> &Bytes {
        &self.data
    }

    fn offsets(&self) -> &[u16] {
        &self.offsets
    }
}

/// Type alias for the latest block iterator version.
/// Note: B is constrained to BlockLike by BlockIteratorV2's definition.
#[cfg(test)]
pub(crate) type BlockIteratorLatest<B> = crate::block_iterator_v2::BlockIteratorV2<B>;

/// Block iterator that dispatches on the SST format version.
pub(crate) enum DataBlockIterator<B: BlockLike> {
    V1(BlockIterator<B>),
    V2(crate::block_iterator_v2::BlockIteratorV2<B>),
}

impl<B: BlockLike> DataBlockIterator<B> {
    pub(crate) fn new(
        block: B,
        sst_version: u16,
        order: IterationOrder,
    ) -> Result<Self, SlateDBError> {
        use crate::format::sst::{SST_FORMAT_VERSION, SST_FORMAT_VERSION_V2};
        match sst_version {
            SST_FORMAT_VERSION => Ok(Self::V1(BlockIterator::new(block, order))),
            SST_FORMAT_VERSION_V2 => Ok(Self::V2(crate::block_iterator_v2::BlockIteratorV2::new(
                block, order,
            ))),
            _ => Err(SlateDBError::InvalidVersion {
                format_name: "SST",
                supported_versions: vec![SST_FORMAT_VERSION, SST_FORMAT_VERSION_V2],
                actual_version: sst_version,
            }),
        }
    }

    pub(crate) async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        match self {
            Self::V1(iter) => iter.next().await,
            Self::V2(iter) => iter.next().await,
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        match self {
            Self::V1(iter) => iter.seek(next_key).await,
            Self::V2(iter) => iter.seek(next_key).await,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::V1(iter) => iter.is_empty(),
            Self::V2(iter) => iter.is_empty(),
        }
    }
}

pub(crate) struct BlockIterator<B: BlockLike> {
    block: B,
    off_off: usize,
    // first key in the block, because slateDB does not support multi version of keys
    // so we use `Bytes` temporarily
    first_key: Bytes,
    ordering: IterationOrder,
}

#[async_trait]
impl<B: BlockLike> RowEntryIterator for BlockIterator<B> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        let result = self.load_at_current_off();
        match result {
            Ok(None) => Ok(None),
            Ok(key_value) => {
                self.advance();
                Ok(key_value)
            }
            Err(e) => Err(e),
        }
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        let num_entries = self.block.offsets().len();
        if num_entries == 0 {
            return Ok(());
        }

        match self.ordering {
            Ascending => {
                // Binary search to find the first key >= next_key
                // Search entire block (bidirectional seeking)
                let mut low = 0;
                let mut high = num_entries;

                while low < high {
                    let mid = low + (high - low) / 2;
                    let mid_key = self.decode_key_at_index(mid)?;

                    match mid_key.as_ref().cmp(next_key) {
                        Ordering::Less => {
                            low = mid + 1;
                        }
                        Ordering::Equal | Ordering::Greater => {
                            high = mid;
                        }
                    }
                }

                self.off_off = low;
            }
            Descending => {
                // Binary search to find the last key <= next_key
                // Strategy: find first physical index where key > next_key, then go back one
                // Search entire block (bidirectional seeking)
                let mut low = 0;
                let mut high = num_entries;

                while low < high {
                    let mid = low + (high - low) / 2;
                    let mid_key = self.decode_key_at_index(mid)?;

                    if mid_key.as_ref() <= next_key {
                        low = mid + 1;
                    } else {
                        high = mid;
                    }
                }

                // low is now the first physical index where key > next_key
                // (or num_entries if all keys <= next_key)
                if low > 0 {
                    // There's at least one key <= next_key
                    // The last such key is at physical index (low - 1)
                    // Convert to descending off_off: off_off = num_entries - 1 - physical_idx
                    let physical_idx = low - 1;
                    self.off_off = num_entries - 1 - physical_idx;
                } else {
                    // All keys are > next_key, position past the end (empty)
                    self.off_off = num_entries;
                }
            }
        }
        Ok(())
    }
}

impl<B: BlockLike> BlockIterator<B> {
    pub(crate) fn new(block: B, ordering: IterationOrder) -> Self {
        BlockIterator {
            first_key: BlockIterator::decode_first_key(&block),
            block,
            off_off: 0,
            ordering,
        }
    }

    #[allow(dead_code)] // Used in other modules
    pub(crate) fn new_ascending(block: B) -> Self {
        Self::new(block, Ascending)
    }

    fn advance(&mut self) {
        self.off_off += 1;
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.off_off >= self.block.offsets().len()
    }

    fn load_at_current_off(&self) -> Result<Option<RowEntry>, SlateDBError> {
        if self.is_empty() {
            return Ok(None);
        }
        let off_off = match self.ordering {
            Ascending => self.off_off,
            Descending => self.block.offsets().len() - 1 - self.off_off,
        };

        let off = self.block.offsets()[off_off];
        let off_usz = off as usize;
        // TODO: bounds checks to avoid panics? (paulgb)
        let mut cursor = self.block.data().slice(off_usz..);
        let codec = SstRowCodecV0::new();
        let sst_row = codec.decode(&mut cursor)?;
        Ok(Some(RowEntry::new(
            sst_row.restore_full_key(&self.first_key),
            sst_row.value,
            sst_row.seq,
            sst_row.create_ts,
            sst_row.expire_ts,
        )))
    }

    fn decode_first_key(block: &B) -> Bytes {
        let mut buf = block.data().slice(..);
        let overlap_len = buf.get_u16() as usize;
        assert_eq!(overlap_len, 0, "first key overlap should be 0");
        let key_len = buf.get_u16() as usize;
        let first_key = &buf[..key_len];
        Bytes::copy_from_slice(first_key)
    }

    /// Decodes just the key at the given offset index without parsing the full row.
    /// This is more efficient for binary search where we only need to compare keys.
    fn decode_key_at_index(&self, index: usize) -> Result<Bytes, SlateDBError> {
        let off = self.block.offsets()[index] as usize;
        let mut cursor = self.block.data().slice(off..);

        let key_prefix_len = cursor.get_u16() as usize;
        let key_suffix_len = cursor.get_u16() as usize;
        let key_suffix = &cursor[..key_suffix_len];

        // Reconstruct the full key from first_key prefix + suffix
        let mut full_key = BytesMut::with_capacity(key_prefix_len + key_suffix_len);
        full_key.extend_from_slice(&self.first_key[..key_prefix_len]);
        full_key.extend_from_slice(key_suffix);
        Ok(full_key.freeze())
    }
}

#[cfg(test)]
mod tests {
    use crate::block_iterator::BlockIterator;
    use crate::bytes_range::BytesRange;
    use crate::format::sst::BlockBuilder;
    use crate::iter::IterationOrder::Descending;
    use crate::iter::RowEntryIterator;
    use crate::proptest_util::{arbitrary, sample};
    use crate::test_utils::{assert_iterator, assert_next};
    use crate::types::{KeyValue, RowEntry};
    use crate::{proptest_util, test_utils};
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn test_iterator() {
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"donkey", b"kong", Some(1), None));
        assert!(block_builder.add_value(b"kratos", b"atreus", Some(2), None));
        assert!(block_builder.add_value(b"super", b"mario", Some(3), None));
        let block = block_builder.build().unwrap();
        let mut iter = BlockIterator::new_ascending(&block);
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"donkey", b"kong");
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"kratos", b"atreus");
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"super", b"mario");
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_seek_to_existing_key() {
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"donkey", b"kong", Some(1), None));
        assert!(block_builder.add_value(b"kratos", b"atreus", Some(2), None));
        assert!(block_builder.add_value(b"super", b"mario", Some(3), None));
        let block = block_builder.build().unwrap();
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"kratos").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"kratos", b"atreus");
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"super", b"mario");
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_seek_to_nonexisting_key() {
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"donkey", b"kong", Some(1), None));
        assert!(block_builder.add_value(b"kratos", b"atreus", Some(2), None));
        assert!(block_builder.add_value(b"super", b"mario", Some(3), None));
        let block = block_builder.build().unwrap();
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"ka").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"kratos", b"atreus");
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"super", b"mario");
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_seek_to_key_beyond_last_key() {
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"donkey", b"kong", Some(1), None));
        assert!(block_builder.add_value(b"kratos", b"atreus", Some(2), None));
        assert!(block_builder.add_value(b"super", b"mario", Some(3), None));
        let block = block_builder.build().unwrap();
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"zzz").await.unwrap();
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_seek_to_key_skips_records_prior_to_next_key() {
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"donkey", b"kong", None, None));
        assert!(block_builder.add_value(b"kratos", b"atreus", None, None));
        assert!(block_builder.add_value(b"super", b"mario", None, None));
        let block = block_builder.build().unwrap();
        let mut iter = BlockIterator::new_ascending(block);
        assert_next(&mut iter, &RowEntry::new_value(b"donkey", b"kong", 0)).await;
        iter.seek(b"s").await.unwrap();
        assert_iterator(&mut iter, vec![RowEntry::new_value(b"super", b"mario", 0)]).await;
    }

    #[tokio::test]
    async fn test_seek_to_key_with_iterator_at_seek_point() {
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"donkey", b"kong", None, None));
        assert!(block_builder.add_value(b"kratos", b"atreus", None, None));
        assert!(block_builder.add_value(b"super", b"mario", None, None));
        let block = block_builder.build().unwrap();
        let mut iter = BlockIterator::new_ascending(block);
        assert_next(&mut iter, &RowEntry::new_value(b"donkey", b"kong", 0)).await;
        iter.seek(b"kratos").await.unwrap();
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(b"kratos", b"atreus", 0),
                RowEntry::new_value(b"super", b"mario", 0),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_seek_to_key_beyond_last_key_in_block() {
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"donkey", b"kong", Some(1), None));
        assert!(block_builder.add_value(b"kratos", b"atreus", Some(2), None));
        assert!(block_builder.add_value(b"super", b"mario", Some(3), None));
        let block = block_builder.build().unwrap();
        let mut iter = BlockIterator::new_ascending(block);
        iter.seek(b"zelda".as_ref()).await.unwrap();
        assert_iterator(&mut iter, Vec::new()).await;
    }

    #[test]
    fn should_iterate_arbitrary_range() {
        let mut runner = proptest_util::runner::new(file!(), None);
        let runtime = Runtime::new().unwrap();
        let sample_table = sample::table(runner.rng(), 5, 10);

        let mut block_builder = BlockBuilder::new_v1(1024);
        for (key, value) in &sample_table {
            block_builder.add_value(key, value, None, None);
        }
        let block = Arc::new(block_builder.build().unwrap());

        runner
            .run(&arbitrary::iteration_order(), |ordering| {
                let mut iter = BlockIterator::new(block.clone(), ordering);
                runtime.block_on(test_utils::assert_ranged_kv_scan(
                    &sample_table,
                    &BytesRange::from(..),
                    ordering,
                    &mut iter,
                ));
                Ok(())
            })
            .unwrap();
    }

    // ----- Binary search tests -----

    #[tokio::test]
    async fn should_binary_search_in_large_block() {
        // given: a block with many entries
        let mut block_builder = BlockBuilder::new_v1(16384);
        for i in 0..100u32 {
            let key = format!("key_{:05}", i);
            let value = format!("value_{}", i);
            assert!(block_builder.add_value(key.as_bytes(), value.as_bytes(), None, None));
        }
        let block = block_builder.build().unwrap();

        // when: seeking to various keys
        // then: the correct entries are returned
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"key_00050").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"key_00050", b"value_50");

        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"key_00099").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"key_00099", b"value_99");

        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"key_00000").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"key_00000", b"value_0");
    }

    #[tokio::test]
    async fn should_seek_to_first_key_in_block() {
        // given: a block with multiple entries
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"apple", b"1", None, None));
        assert!(block_builder.add_value(b"banana", b"2", None, None));
        assert!(block_builder.add_value(b"cherry", b"3", None, None));
        let block = block_builder.build().unwrap();

        // when: seeking to the first key
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"apple").await.unwrap();

        // then: the first entry is returned
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"apple", b"1");
    }

    #[tokio::test]
    async fn should_seek_to_last_key_in_block() {
        // given: a block with multiple entries
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"apple", b"1", None, None));
        assert!(block_builder.add_value(b"banana", b"2", None, None));
        assert!(block_builder.add_value(b"cherry", b"3", None, None));
        let block = block_builder.build().unwrap();

        // when: seeking to the last key
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"cherry").await.unwrap();

        // then: the last entry is returned and iteration ends
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"cherry", b"3");
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_to_key_before_first() {
        // given: a block with entries
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"banana", b"2", None, None));
        assert!(block_builder.add_value(b"cherry", b"3", None, None));
        let block = block_builder.build().unwrap();

        // when: seeking to a key before the first entry
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"apple").await.unwrap();

        // then: the first entry is returned
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"banana", b"2");
    }

    #[tokio::test]
    async fn should_seek_with_shared_prefix_keys() {
        // given: a block with keys that share prefixes (tests prefix encoding interaction)
        let mut block_builder = BlockBuilder::new_v1(4096);
        assert!(block_builder.add_value(b"user:1000", b"alice", None, None));
        assert!(block_builder.add_value(b"user:1001", b"bob", None, None));
        assert!(block_builder.add_value(b"user:1002", b"carol", None, None));
        assert!(block_builder.add_value(b"user:1010", b"dave", None, None));
        assert!(block_builder.add_value(b"user:1020", b"eve", None, None));
        let block = block_builder.build().unwrap();

        // when: seeking to various keys with shared prefixes
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"user:1001").await.unwrap();

        // then: correct entry is found
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"user:1001", b"bob");

        // when: seeking to a key between entries
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"user:1005").await.unwrap();

        // then: the next entry is returned
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"user:1010", b"dave");
    }

    #[tokio::test]
    async fn should_seek_multiple_times_sequentially() {
        // given: a block with multiple entries
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"a", b"1", None, None));
        assert!(block_builder.add_value(b"b", b"2", None, None));
        assert!(block_builder.add_value(b"c", b"3", None, None));
        assert!(block_builder.add_value(b"d", b"4", None, None));
        assert!(block_builder.add_value(b"e", b"5", None, None));
        let block = block_builder.build().unwrap();
        let mut iter = BlockIterator::new_ascending(block);

        // when/then: multiple sequential seeks work correctly
        iter.seek(b"b").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"b", b"2");

        iter.seek(b"d").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"d", b"4");

        iter.seek(b"e").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"e", b"5");
    }

    #[tokio::test]
    async fn should_seek_bidirectionally_ascending() {
        // given: a block with entries and an iterator advanced past the first entry
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"a", b"1", None, None));
        assert!(block_builder.add_value(b"b", b"2", None, None));
        assert!(block_builder.add_value(b"c", b"3", None, None));
        assert!(block_builder.add_value(b"d", b"4", None, None));
        let block = block_builder.build().unwrap();
        let mut iter = BlockIterator::new_ascending(block);

        // advance past "a" and "b"
        iter.next().await.unwrap();
        iter.next().await.unwrap();

        // when: seeking to a key before current position (backward seek)
        iter.seek(b"a").await.unwrap();

        // then: seek goes backwards, returns "a"
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"a", b"1");

        // Verify we can continue forward
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"b", b"2");

        // Seek forward
        iter.seek(b"d").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"d", b"4");
    }

    #[tokio::test]
    async fn should_seek_in_single_entry_block() {
        // given: a block with only one entry
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"only", b"one", None, None));
        let block = block_builder.build().unwrap();

        // when: seeking to the exact key
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"only").await.unwrap();

        // then: the entry is returned
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"only", b"one");

        // when: seeking to a key before it
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"aaa").await.unwrap();

        // then: the entry is returned
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"only", b"one");

        // when: seeking to a key after it
        let mut iter = BlockIterator::new_ascending(&block);
        iter.seek(b"zzz").await.unwrap();

        // then: no entries remain
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_decode_key_at_index_correctly() {
        // given: a block with entries that have shared prefixes
        let mut block_builder = BlockBuilder::new_v1(4096);
        assert!(block_builder.add_value(b"prefix_aaa", b"1", None, None));
        assert!(block_builder.add_value(b"prefix_bbb", b"2", None, None));
        assert!(block_builder.add_value(b"prefix_ccc", b"3", None, None));
        let block = block_builder.build().unwrap();
        let iter = BlockIterator::new_ascending(&block);

        // when: decoding keys at each index
        // then: full keys are correctly reconstructed
        let key0 = iter.decode_key_at_index(0).unwrap();
        assert_eq!(key0.as_ref(), b"prefix_aaa");

        let key1 = iter.decode_key_at_index(1).unwrap();
        assert_eq!(key1.as_ref(), b"prefix_bbb");

        let key2 = iter.decode_key_at_index(2).unwrap();
        assert_eq!(key2.as_ref(), b"prefix_ccc");
    }

    // ----- Descending iteration seek tests -----

    #[tokio::test]
    async fn should_seek_descending_to_exact_key() {
        // given: a block with multiple entries
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"apple", b"1", None, None));
        assert!(block_builder.add_value(b"banana", b"2", None, None));
        assert!(block_builder.add_value(b"cherry", b"3", None, None));
        let block = block_builder.build().unwrap();

        // when: seeking to exact key in descending order
        let mut iter = BlockIterator::new(&block, Descending);
        iter.seek(b"banana").await.unwrap();

        // then: should iterate backwards from banana
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"banana", b"2");
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"apple", b"1");
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_descending_beyond_last_key() {
        // given: a block with keys apple, banana, cherry
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"apple", b"1", None, None));
        assert!(block_builder.add_value(b"banana", b"2", None, None));
        assert!(block_builder.add_value(b"cherry", b"3", None, None));
        let block = block_builder.build().unwrap();

        // when: seeking to key beyond last key (zzz > cherry)
        let mut iter = BlockIterator::new(&block, Descending);
        iter.seek(b"zzz").await.unwrap();

        // then: should start from the last key and iterate backwards
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"cherry", b"3");
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"banana", b"2");
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"apple", b"1");
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_descending_to_key_before_first() {
        // given: a block with keys banana, cherry, dragon
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"banana", b"2", None, None));
        assert!(block_builder.add_value(b"cherry", b"3", None, None));
        assert!(block_builder.add_value(b"dragon", b"4", None, None));
        let block = block_builder.build().unwrap();

        // when: seeking to key before first key (apple < banana)
        let mut iter = BlockIterator::new(&block, Descending);
        iter.seek(b"apple").await.unwrap();

        // then: should be empty (no keys <= apple)
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_descending_to_key_between_entries() {
        // given: a block with keys apple, cherry, dragon
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"apple", b"1", None, None));
        assert!(block_builder.add_value(b"cherry", b"3", None, None));
        assert!(block_builder.add_value(b"dragon", b"4", None, None));
        let block = block_builder.build().unwrap();

        // when: seeking to "banana" which is between apple and cherry
        let mut iter = BlockIterator::new(&block, Descending);
        iter.seek(b"banana").await.unwrap();

        // then: should position at last key <= banana, which is apple
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"apple", b"1");
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_full_descending_iteration_work() {
        // given: a block with multiple entries
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"apple", b"1", None, None));
        assert!(block_builder.add_value(b"banana", b"2", None, None));
        assert!(block_builder.add_value(b"cherry", b"3", None, None));
        let block = block_builder.build().unwrap();

        // when: iterating in descending order without seek
        let mut iter = BlockIterator::new(block, Descending);

        // then: should iterate from last to first
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"cherry", b"3");
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"banana", b"2");
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"apple", b"1");
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_descending_bidirectionally() {
        // given: a block with multiple entries
        let mut block_builder = BlockBuilder::new_v1(1024);
        assert!(block_builder.add_value(b"a", b"v1", Some(1), None));
        assert!(block_builder.add_value(b"b", b"v2", Some(2), None));
        assert!(block_builder.add_value(b"c", b"v3", Some(3), None));
        assert!(block_builder.add_value(b"d", b"v4", Some(4), None));
        assert!(block_builder.add_value(b"e", b"v5", Some(5), None));
        let block = block_builder.build().unwrap();

        // when: iterating in descending
        let mut iter = BlockIterator::new(block, Descending);

        // First, advance past some entries
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"e", b"v5");

        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"d", b"v4");

        // Seek forward to "b"
        iter.seek(b"b").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"b", b"v2");

        // Seek backward to "d" (bidirectional)
        iter.seek(b"d").await.unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"d", b"v4");

        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"c", b"v3");

        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"b", b"v2");

        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        test_utils::assert_kv(&kv, b"a", b"v1");

        assert!(iter.next().await.unwrap().is_none());
    }
}
