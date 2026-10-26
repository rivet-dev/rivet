//! WAL (Write-Ahead Log) SSTable encoding and building.
//!
//! This module provides a specialized SSTable builder for WAL entries. WAL SSTables
//! differ from regular SSTables in several key ways optimized for write-ahead logging:
//!
//! # Differences from Regular SSTables
//!
//! | Feature | Regular SSTable | WAL SSTable |
//! |---------|-----------------|-------------|
//! | Filter | Yes (optional) | No |
//! | Index key | First key per block | First sequence number per block |
//! | Key ordering | Sorted by key | Insertion order (by sequence number) |
//! | First key field in SST info  | First key | First sequence number |
//!
//! # WAL SSTable Format
//!
//! ```text
//! +------------------+
//! |   Data Blocks    |  <- Entries in insertion order, optionally compressed
//! +------------------+
//! |   Index Block    |  <- Block metadata with sequence numbers
//! +------------------+
//! |   SST Info       |  <- Table metadata
//! +------------------+
//! |  Metadata offset |  <- Metadata offset (8 bytes)
//! +------------------+
//! |     Version      |  <- Format version (2 bytes)
//! +------------------+
//! ```
//!
//! # Key Components
//!
//! - [`EncodedWalSsTableBuilder`]: Builder for constructing WAL SSTables from entries
//!
//! The builder reuses shared components from the [`crate::format::sst`] module:
//! - [`EncodedSsTableBlockBuilder`]: For encoding data blocks
//! - [`EncodedSsTableFooterBuilder`]: For encoding the footer
//! - [`BlockTransformer`]: Trait for custom block transformations (e.g., encryption)
//!
//! # Why No Filter?
//!
//! WAL SSTables are designed for sequential replay during recovery, not random
//! key lookups. Since entries are read sequentially by sequence number, a filter
//! would provide no benefit.
//!
//! # Sequence Number Index
//!
//! Instead of indexing by first key (as in regular SSTables), the WAL SSTable
//! index stores the first sequence number of each block. This enables efficient
//! seeking to a specific point in the write-ahead log during recovery.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::config::CompressionCodec;
use crate::db_state::{SsTableInfoCodec, SstType};
use crate::error::SlateDBError;
use crate::flatbuffer_types::{BlockMeta, BlockMetaArgs};
use crate::format::sst::{
    BlockBuilder, BlockTransformer, EncodedSsTable, EncodedSsTableBlock,
    EncodedSsTableBlockBuilder, EncodedSsTableFooterBuilder, SsTableFormat,
    SST_FORMAT_VERSION_LATEST,
};
use crate::types::RowEntry;
use bytes::Bytes;
use flatbuffers::DefaultAllocator;

// use as large of a block size as possible for the WAL. We don't need to support random seeks,
// so the size is optimized to minimize overhead.
const WAL_BLOCK_SIZE: usize = u16::MAX as usize;

impl SsTableFormat {
    pub(crate) fn wal_table_builder(&self) -> EncodedWalSsTableBuilder {
        let mut builder = EncodedWalSsTableBuilder::new(WAL_BLOCK_SIZE, self.sst_codec.clone());
        if let Some(codec) = self.compression_codec {
            builder = builder.with_compression_codec(codec);
        }
        if let Some(ref transformer) = self.block_transformer {
            builder = builder.with_block_transformer(transformer.clone());
        }
        builder
    }
}

/// Builds a WAL SSTable from entries.
///
/// This builder differs from the regular EncodedSsTableBuilder in that:
/// - It is assumed that the entries are added ordered by sequence number instead of key
/// - No filter is created
/// - The index tracks sequence number ranges per block instead of first keys
#[allow(unused)]
pub(crate) struct EncodedWalSsTableBuilder {
    block_builder: BlockBuilder,
    index_builder: flatbuffers::FlatBufferBuilder<'static, DefaultAllocator>,
    block_meta: Vec<flatbuffers::WIPOffset<BlockMeta<'static>>>,
    data_size: u64,
    blocks: VecDeque<EncodedSsTableBlock>,
    block_size_config: usize,
    sst_codec: Box<dyn SsTableInfoCodec>,
    compression_codec: Option<CompressionCodec>,
    block_transformer: Option<Arc<dyn BlockTransformer>>,
    sst_first_seq: Option<Bytes>,
    first_seq: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'static, u8>>>,
    entries_count: usize,
    entries_size_bytes: usize,
}

impl EncodedWalSsTableBuilder {
    /// Create a builder based on target block size.
    pub(crate) fn new(block_size: usize, sst_codec: Box<dyn SsTableInfoCodec>) -> Self {
        Self {
            data_size: 0,
            blocks: VecDeque::new(),
            block_meta: Vec::new(),
            block_size_config: block_size,
            block_builder: BlockBuilder::new_latest(block_size),
            index_builder: flatbuffers::FlatBufferBuilder::new(),
            sst_codec,
            compression_codec: None,
            block_transformer: None,
            sst_first_seq: None,
            first_seq: None,
            entries_count: 0,
            entries_size_bytes: 0,
        }
    }

    /// Sets the compression codec for compressing data blocks and index blocks
    pub(crate) fn with_compression_codec(mut self, codec: CompressionCodec) -> Self {
        self.compression_codec = Some(codec);
        self
    }

    /// Sets the block transformer for transforming data blocks and index blocks
    pub(crate) fn with_block_transformer(mut self, transformer: Arc<dyn BlockTransformer>) -> Self {
        self.block_transformer = Some(transformer);
        self
    }

    /// Adds an entry to the WAL SSTable and returns the size of the block that was finished if any.
    pub(crate) async fn add(&mut self, entry: RowEntry) -> Result<Option<usize>, SlateDBError> {
        // Track first sequence number for SST info
        let is_sst_first_seq = self.sst_first_seq.is_none();
        if is_sst_first_seq {
            self.sst_first_seq = Some(Bytes::copy_from_slice(&entry.seq.to_be_bytes()));
        }

        let mut block_size = None;
        if !self.block_builder.would_fit(&entry) {
            let first_seq_bytes = entry.seq.to_be_bytes();
            block_size = self.finish_block().await?;
            self.first_seq = Some(self.index_builder.create_vector(&first_seq_bytes));
        } else if is_sst_first_seq {
            let first_seq_bytes = entry.seq.to_be_bytes();
            self.first_seq = Some(self.index_builder.create_vector(&first_seq_bytes));
        }

        let entry_size = entry.estimated_size();
        self.block_builder.add(entry)?;
        self.entries_count += 1;
        self.entries_size_bytes += entry_size;

        Ok(block_size)
    }

    #[cfg(test)]
    pub(crate) async fn add_value(
        &mut self,
        key: &[u8],
        val: &[u8],
        seq: u64,
        create_ts: Option<i64>,
        expire_ts: Option<i64>,
    ) -> Result<Option<usize>, SlateDBError> {
        let entry = RowEntry::new(
            key.to_vec().into(),
            crate::types::ValueDeletable::Value(Bytes::copy_from_slice(val)),
            seq,
            create_ts,
            expire_ts,
        );
        self.add(entry).await
    }

    #[cfg(test)]
    pub(crate) fn next_block(&mut self) -> Option<EncodedSsTableBlock> {
        self.blocks.pop_front()
    }

    async fn finish_block(&mut self) -> Result<Option<usize>, SlateDBError> {
        if self.is_drained() {
            return Ok(None);
        }

        let new_builder = BlockBuilder::new_latest(self.block_size_config);
        let builder = std::mem::replace(&mut self.block_builder, new_builder);
        let mut block_builder = EncodedSsTableBlockBuilder::new(builder, self.data_size);
        if let Some(codec) = self.compression_codec {
            block_builder = block_builder.with_compression_codec(codec);
        }
        if let Some(transformer) = self.block_transformer.clone() {
            block_builder = block_builder.with_block_transformer(transformer);
        }
        let block = block_builder.build().await?;
        let block_meta = BlockMeta::create(
            &mut self.index_builder,
            &BlockMetaArgs {
                offset: block.offset,
                first_key: self.first_seq,
            },
        );
        self.block_meta.push(block_meta);

        let block_size = block.len();
        self.data_size += block_size as u64;
        self.blocks.push_back(block);
        self.first_seq = None;

        Ok(Some(block_size))
    }

    /// Builds the WAL SST from the current state.
    ///
    /// # Format
    ///
    /// +---------------------------------------------------+
    /// |                Data Blocks                        |
    /// |    (raw bytes produced by finish_block)           |
    /// +---------------------------------------------------+
    /// |                Index Block                        |
    /// |  +---------------------------------------------+  |
    /// |  | Index Data (compressed index block)         |  |
    /// |  +---------------------------------------------+  |
    /// |  | 4-byte Checksum (CRC32 of index data)       |  |
    /// |  +---------------------------------------------+  |
    /// +---------------------------------------------------+
    /// |                Metadata Block                     |
    /// |  +---------------------------------------------+  |
    /// |  | (SsTableInfo encoded with FlatBuffers)      |  |
    /// |  +---------------------------------------------+  |
    /// |  | 4-byte Checksum (CRC32 of metadata)         |  |
    /// |  +---------------------------------------------+  |
    /// +---------------------------------------------------+
    /// |             8-byte Metadata Offset                |
    /// +---------------------------------------------------+
    /// |                 2-byte Version                    |
    /// +---------------------------------------------------+
    ///
    /// Note: Unlike regular SSTs, WAL SSTs have no filter.
    /// The index first_key field contains the min sequence number (as bytes) for each block.
    /// SST info contains the first sequence number of the SST instead of the first key.
    pub(crate) async fn build(mut self) -> Result<EncodedSsTable, SlateDBError> {
        self.finish_block().await?;

        let format_version = SST_FORMAT_VERSION_LATEST;
        let mut footer_builder = EncodedSsTableFooterBuilder::new(
            self.data_size,
            self.sst_first_seq,
            None, // WAL SSTs don't have sorted keys, so no last_entry
            &*self.sst_codec,
            self.index_builder,
            self.block_meta,
            format_version,
            SstType::Wal,
        );
        if let Some(codec) = self.compression_codec {
            footer_builder = footer_builder.with_compression_codec(codec);
        }
        if let Some(transformer) = self.block_transformer.clone() {
            footer_builder = footer_builder.with_block_transformer(transformer);
        }
        let footer = footer_builder.build().await?;

        Ok(EncodedSsTable {
            format_version,
            info: footer.info,
            index: footer.index,
            filters: Arc::from([]),
            stats: None,
            unconsumed_blocks: self.blocks,
            footer: footer.encoded_bytes,
        })
    }

    pub(crate) fn is_drained(&self) -> bool {
        self.block_builder.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_iterator::BlockIteratorLatest;
    use crate::flatbuffer_types::FlatBufferSsTableInfoCodec;
    use crate::format::block::Block;
    use crate::test_utils::assert_iterator;
    use crate::types::ValueDeletable;

    fn next_block_to_iter(
        builder: &mut EncodedWalSsTableBuilder,
    ) -> BlockIteratorLatest<Arc<Block>> {
        let block = builder.next_block();
        assert!(block.is_some());
        let block = block.unwrap().block;
        BlockIteratorLatest::new_ascending(block)
    }

    #[tokio::test]
    async fn should_make_blocks_available_and_report_size() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(32, Box::new(FlatBufferSsTableInfoCodec {}));

        // When
        let result1 = builder
            .add_value(&[b'a'; 8], &[b'1'; 8], 1, Some(1), None)
            .await
            .unwrap();
        let result2 = builder
            .add_value(&[b'b'; 8], &[b'2'; 8], 2, Some(2), None)
            .await
            .unwrap();
        let result3 = builder
            .add_value(&[b'c'; 8], &[b'3'; 8], 3, Some(3), None)
            .await
            .unwrap();

        // Then
        assert!(result1.is_none(), "First entry should not finish a block");
        assert!(
            result2.is_some(),
            "Second entry should finish the first block"
        );
        assert!(
            result2.unwrap() > 0,
            "Block size of first block should be positive"
        );
        assert!(
            result3.is_some(),
            "Third entry should finish the second block"
        );

        let mut iter = next_block_to_iter(&mut builder);
        assert_iterator(
            &mut iter,
            vec![RowEntry::new_value(&[b'a'; 8], &[b'1'; 8], 1).with_create_ts(1)],
        )
        .await;

        let mut iter = next_block_to_iter(&mut builder);
        assert_iterator(
            &mut iter,
            vec![RowEntry::new_value(&[b'b'; 8], &[b'2'; 8], 2).with_create_ts(2)],
        )
        .await;

        assert!(builder.next_block().is_none());
    }

    #[tokio::test]
    async fn should_build_without_bloom_filter() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(b"key1", b"value1", 1, None, None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", 2, None, None)
            .await
            .unwrap();

        // When
        let encoded = builder.build().await.unwrap();

        // Then
        assert!(encoded.filters.is_empty());
        assert_eq!(encoded.info.filter_len, 0);
    }

    #[tokio::test]
    async fn should_store_entries_in_insertion_order() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(4096, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(b"zebra", b"val1", 5, Some(100), None)
            .await
            .unwrap();
        builder
            .add_value(b"apple", b"val2", 2, Some(200), None)
            .await
            .unwrap();
        builder
            .add_value(b"mango", b"val3", 10, Some(300), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        assert_eq!(
            encoded.info.first_entry.as_ref().unwrap().as_ref(),
            5u64.to_be_bytes()
        );
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_value(b"zebra", b"val1", 5).with_create_ts(100),
            RowEntry::new_value(b"apple", b"val2", 2).with_create_ts(200),
            RowEntry::new_value(b"mango", b"val3", 10).with_create_ts(300),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_handle_tombstones() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        let tombstone = RowEntry::new(
            Bytes::from_static(b"key1"),
            ValueDeletable::Tombstone,
            1,
            Some(100),
            None,
        );
        builder.add(tombstone).await.unwrap();
        builder
            .add_value(b"key2", b"value2", 2, Some(200), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_tombstone(b"key1", 1).with_create_ts(100),
            RowEntry::new_value(b"key2", b"value2", 2).with_create_ts(200),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_handle_merge() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        let merge = RowEntry::new(
            Bytes::from_static(b"key1"),
            ValueDeletable::Merge(Bytes::from_static(b"merge_value")),
            1,
            Some(100),
            None,
        );
        builder.add(merge).await.unwrap();
        builder
            .add_value(b"key2", b"value2", 2, Some(200), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_merge(b"key1", b"merge_value", 1).with_create_ts(100),
            RowEntry::new_value(b"key2", b"value2", 2).with_create_ts(200),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_store_first_seq_in_index() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(32, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(&[b'a'; 8], &[b'1'; 8], 10, None, None)
            .await
            .unwrap();
        builder
            .add_value(&[b'b'; 8], &[b'2'; 8], 11, None, None)
            .await
            .unwrap();
        builder
            .add_value(&[b'c'; 8], &[b'3'; 8], 12, None, None)
            .await
            .unwrap();

        // When
        let encoded = builder.build().await.unwrap();

        // Then
        let index = encoded.index.borrow();
        let block_metas = index.block_meta();
        assert_eq!(3, block_metas.len(), "Index should have three entries");
        assert_eq!(block_metas.get(0).first_key().bytes(), &10u64.to_be_bytes());
        assert_eq!(block_metas.get(1).first_key().bytes(), &11u64.to_be_bytes());
        assert_eq!(block_metas.get(2).first_key().bytes(), &12u64.to_be_bytes());
    }

    #[tokio::test]
    async fn should_build_empty_sst_when_no_entries() {
        // Given
        let builder = EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        assert!(builder.is_drained());

        // When
        let encoded = builder.build().await.unwrap();

        // Then
        assert!(encoded.unconsumed_blocks.is_empty());
        assert!(encoded.info.first_entry.is_none());
        assert!(encoded.filters.is_empty());
    }

    #[tokio::test]
    async fn should_handle_empty_value() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(b"key1", b"", 1, Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", 2, Some(2), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_value(b"key1", b"", 1).with_create_ts(1),
            RowEntry::new_value(b"key2", b"value2", 2).with_create_ts(2),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_handle_large_value() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(64, Box::new(FlatBufferSsTableInfoCodec {}));
        let large_value = vec![b'x'; 256];
        builder
            .add_value(b"key1", &large_value, 1, Some(1), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![RowEntry::new_value(b"key1", &large_value, 1).with_create_ts(1)];
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_handle_single_entry_sst() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(b"only_key", b"only_value", 42, Some(100), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        assert_eq!(
            encoded.info.first_entry.as_ref().unwrap().as_ref(),
            42u64.to_be_bytes()
        );
        assert_eq!(encoded.unconsumed_blocks.len(), 1);
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected =
            vec![RowEntry::new_value(b"only_key", b"only_value", 42).with_create_ts(100)];
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_handle_max_seq_number() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(b"key1", b"value1", u64::MAX, Some(1), None)
            .await
            .unwrap();

        // When
        let encoded = builder.build().await.unwrap();

        // Then
        let index = encoded.index.borrow();
        let block_metas = index.block_meta();
        assert_eq!(
            block_metas.get(0).first_key().bytes(),
            &u64::MAX.to_be_bytes()
        );
    }

    #[tokio::test]
    async fn should_handle_duplicate_keys_different_seqs() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(b"key1", b"value1", 1, Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key1", b"value2", 2, Some(2), None)
            .await
            .unwrap();
        builder
            .add_value(b"key1", b"value3", 3, Some(3), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_value(b"key1", b"value1", 1).with_create_ts(1),
            RowEntry::new_value(b"key1", b"value2", 2).with_create_ts(2),
            RowEntry::new_value(b"key1", b"value3", 3).with_create_ts(3),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_handle_entries_with_expire_ts() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(b"key1", b"value1", 1, Some(100), Some(200))
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", 2, Some(100), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_value(b"key1", b"value1", 1)
                .with_create_ts(100)
                .with_expire_ts(200),
            RowEntry::new_value(b"key2", b"value2", 2).with_create_ts(100),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_verify_block_checksums() {
        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(b"key1", b"value1", 1, None, None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let encoded_bytes = &block.encoded_bytes;

        // Then
        let checksum_offset = encoded_bytes.len() - size_of::<u32>();
        let stored_checksum =
            u32::from_be_bytes(encoded_bytes[checksum_offset..].try_into().unwrap());
        let computed_checksum = crc32fast::hash(&encoded_bytes[..checksum_offset]);
        assert_eq!(stored_checksum, computed_checksum);
    }

    #[cfg(feature = "snappy")]
    #[tokio::test]
    async fn should_compress_blocks_with_snappy() {
        use crate::format::sst::CHECKSUM_SIZE;

        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}))
                .with_compression_codec(CompressionCodec::Snappy);
        builder
            .add_value(b"key1", b"value1", 1, Some(100), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", 2, Some(200), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        assert_eq!(
            encoded.info.compression_codec,
            Some(CompressionCodec::Snappy)
        );
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let compressed_with_checksum = &block.encoded_bytes;
        let compressed =
            &compressed_with_checksum[..compressed_with_checksum.len() - CHECKSUM_SIZE];
        let decompressed = snap::raw::Decoder::new()
            .decompress_vec(compressed)
            .unwrap();
        assert_eq!(decompressed, block.block.encode().as_ref());
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_value(b"key1", b"value1", 1).with_create_ts(100),
            RowEntry::new_value(b"key2", b"value2", 2).with_create_ts(200),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[cfg(feature = "lz4")]
    #[tokio::test]
    async fn should_compress_blocks_with_lz4() {
        use crate::format::sst::CHECKSUM_SIZE;

        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}))
                .with_compression_codec(CompressionCodec::Lz4);
        builder
            .add_value(b"key1", b"value1", 1, Some(100), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", 2, Some(200), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        assert_eq!(encoded.info.compression_codec, Some(CompressionCodec::Lz4));
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let compressed_with_checksum = &block.encoded_bytes;
        let compressed =
            &compressed_with_checksum[..compressed_with_checksum.len() - CHECKSUM_SIZE];
        let decompressed = lz4_flex::block::decompress_size_prepended(compressed).unwrap();
        assert_eq!(decompressed, block.block.encode().as_ref());
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_value(b"key1", b"value1", 1).with_create_ts(100),
            RowEntry::new_value(b"key2", b"value2", 2).with_create_ts(200),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[cfg(feature = "zstd")]
    #[tokio::test]
    async fn should_compress_blocks_with_zstd() {
        use crate::format::sst::CHECKSUM_SIZE;

        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}))
                .with_compression_codec(CompressionCodec::Zstd);
        builder
            .add_value(b"key1", b"value1", 1, Some(100), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", 2, Some(200), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        assert_eq!(encoded.info.compression_codec, Some(CompressionCodec::Zstd));
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let compressed_with_checksum = &block.encoded_bytes;
        let compressed =
            &compressed_with_checksum[..compressed_with_checksum.len() - CHECKSUM_SIZE];
        let decompressed = zstd::stream::decode_all(compressed).unwrap();
        assert_eq!(decompressed, block.block.encode().as_ref());
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_value(b"key1", b"value1", 1).with_create_ts(100),
            RowEntry::new_value(b"key2", b"value2", 2).with_create_ts(200),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[cfg(feature = "zlib")]
    #[tokio::test]
    async fn should_compress_blocks_with_zlib() {
        use crate::format::sst::CHECKSUM_SIZE;
        use std::io::Read;

        // Given
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}))
                .with_compression_codec(CompressionCodec::Zlib);
        builder
            .add_value(b"key1", b"value1", 1, Some(100), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", 2, Some(200), None)
            .await
            .unwrap();

        // When
        let mut encoded = builder.build().await.unwrap();

        // Then
        assert_eq!(encoded.info.compression_codec, Some(CompressionCodec::Zlib));
        let block = encoded.unconsumed_blocks.pop_front().unwrap();
        let compressed_with_checksum = &block.encoded_bytes;
        let compressed =
            &compressed_with_checksum[..compressed_with_checksum.len() - CHECKSUM_SIZE];
        let mut decoder = flate2::read::ZlibDecoder::new(compressed);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert_eq!(decompressed, block.block.encode().as_ref());
        let mut iter = BlockIteratorLatest::new_ascending(block.block);
        let expected = vec![
            RowEntry::new_value(b"key1", b"value1", 1).with_create_ts(100),
            RowEntry::new_value(b"key2", b"value2", 2).with_create_ts(200),
        ];
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_set_sst_type_to_wal() {
        // given:
        let mut builder =
            EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}));
        builder
            .add_value(b"key1", b"value1", 1, None, None)
            .await
            .unwrap();

        // when:
        let encoded = builder.build().await.unwrap();

        // then:
        assert_eq!(encoded.info.sst_type, SstType::Wal,);
    }

    mod block_transformer_tests {
        use super::*;
        use async_trait::async_trait;
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// Mock block transformer that tracks encode/decode calls.
        struct MockBlockTransformer {
            encode_call_count: AtomicUsize,
            decode_call_count: AtomicUsize,
        }

        impl MockBlockTransformer {
            fn new() -> Arc<Self> {
                Arc::new(Self {
                    encode_call_count: AtomicUsize::new(0),
                    decode_call_count: AtomicUsize::new(0),
                })
            }

            fn encode_call_count(&self) -> usize {
                self.encode_call_count.load(Ordering::SeqCst)
            }

            #[allow(dead_code)]
            fn decode_call_count(&self) -> usize {
                self.decode_call_count.load(Ordering::SeqCst)
            }
        }

        #[async_trait]
        impl BlockTransformer for MockBlockTransformer {
            async fn encode(&self, data: Bytes) -> Result<Bytes, crate::error::Error> {
                self.encode_call_count.fetch_add(1, Ordering::SeqCst);
                Ok(data)
            }

            async fn decode(&self, data: Bytes) -> Result<Bytes, crate::error::Error> {
                self.decode_call_count.fetch_add(1, Ordering::SeqCst);
                Ok(data)
            }
        }

        #[tokio::test]
        async fn should_call_encode_for_data_block() {
            // Given
            let transformer = MockBlockTransformer::new();
            let mut builder =
                EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}))
                    .with_block_transformer(Arc::clone(&transformer) as Arc<dyn BlockTransformer>);
            builder
                .add_value(b"key1", b"value1", 1, Some(100), None)
                .await
                .unwrap();

            // When
            let _ = builder.build().await.unwrap();

            // Then - encode should be called for 1 data block + 1 index block = 2 calls
            assert_eq!(transformer.encode_call_count(), 2);
        }

        #[tokio::test]
        async fn should_call_encode_for_each_data_block() {
            // Given - use small block size to force multiple blocks
            let transformer = MockBlockTransformer::new();
            let mut builder =
                EncodedWalSsTableBuilder::new(32, Box::new(FlatBufferSsTableInfoCodec {}))
                    .with_block_transformer(Arc::clone(&transformer) as Arc<dyn BlockTransformer>);
            builder
                .add_value(&[b'a'; 8], &[b'1'; 8], 1, Some(1), None)
                .await
                .unwrap();
            builder
                .add_value(&[b'b'; 8], &[b'2'; 8], 2, Some(2), None)
                .await
                .unwrap();
            builder
                .add_value(&[b'c'; 8], &[b'3'; 8], 3, Some(3), None)
                .await
                .unwrap();

            // When
            let encoded = builder.build().await.unwrap();

            // Then - encode should be called for each data block + index block
            let data_block_count = encoded.unconsumed_blocks.len();
            assert!(data_block_count >= 2, "Should have multiple data blocks");
            // Each data block + 1 index block
            assert_eq!(transformer.encode_call_count(), data_block_count + 1);
        }

        #[tokio::test]
        async fn should_preserve_block_data_with_transformer() {
            // Given
            let transformer = MockBlockTransformer::new();
            let mut builder =
                EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}))
                    .with_block_transformer(transformer as Arc<dyn BlockTransformer>);
            builder
                .add_value(b"key1", b"value1", 1, Some(100), None)
                .await
                .unwrap();
            builder
                .add_value(b"key2", b"value2", 2, Some(200), None)
                .await
                .unwrap();

            // When
            let mut encoded = builder.build().await.unwrap();

            // Then - block data should still be readable
            let block = encoded.unconsumed_blocks.pop_front().unwrap();
            let mut iter = BlockIteratorLatest::new_ascending(block.block);
            let expected = vec![
                RowEntry::new_value(b"key1", b"value1", 1).with_create_ts(100),
                RowEntry::new_value(b"key2", b"value2", 2).with_create_ts(200),
            ];
            assert_iterator(&mut iter, expected).await;
        }

        #[cfg(feature = "lz4")]
        #[tokio::test]
        async fn should_call_encode_with_compression() {
            // Given
            let transformer = MockBlockTransformer::new();
            let mut builder =
                EncodedWalSsTableBuilder::new(1024, Box::new(FlatBufferSsTableInfoCodec {}))
                    .with_compression_codec(CompressionCodec::Lz4)
                    .with_block_transformer(Arc::clone(&transformer) as Arc<dyn BlockTransformer>);
            builder
                .add_value(b"key1", b"value1", 1, Some(100), None)
                .await
                .unwrap();

            // When
            let _ = builder.build().await.unwrap();

            // Then - encode should still be called (1 data block + 1 index block)
            assert_eq!(transformer.encode_call_count(), 2);
        }
    }
}
