//! SSTable (Sorted String Table) encoding and building.
//!
//! This module provides the core data structures and builders for creating and reading
//! SSTables, the persistent on-disk format used by SlateDB for storing sorted key-value
//! data.
//!
//! # SSTable Format
//!
//! An SSTable consists of the following components, written sequentially:
//!
//! ```text
//! +------------------+
//! |   Data Blocks    |  <- Key-value entries, optionally compressed
//! +------------------+
//! |   Filter Block   |  <- Filter for efficient key lookups (optional)
//! +------------------+
//! |   Index Block    |  <- Block metadata (offsets, first keys)
//! +------------------+
//! |   Stats Block    |  <- Per-SST statistics (num_puts, num_deletes, etc.)
//! +------------------+
//! |   SST Info       |  <- Table metadata (key range, compression, etc.)
//! +------------------+
//! |  Metadata offset |  <- Metadata offset (8 bytes)
//! +------------------+
//! |     Version      |  <- Format version (2 bytes)
//! +------------------+
//! ```
//!
//! Each block is followed by a CRC32 checksum for data integrity verification.
//!
//! # Key Components
//!
//! - [`EncodedSsTableBuilder`]: Builder for constructing SSTables from entries
//!
//! The builder reuses shared components from the [`crate::format::sst`] module:
//! - [`EncodedSsTableBlockBuilder`]: For encoding data blocks
//! - [`EncodedSsTableFooterBuilder`]: For encoding the footer
//! - [`BlockTransformer`]: Trait for custom block transformations (e.g., encryption)
//!
//! # Compression
//!
//! SSTables support optional compression via the [`CompressionCodec`] enum:
//! - Snappy (feature: `snappy`)
//! - LZ4 (feature: `lz4`)
//! - Zstd (feature: `zstd`)
//! - Zlib (feature: `zlib`)
//!
//! Compression is applied per-block before the checksum is computed.
//!
//! # Block Transformation
//!
//! The [`BlockTransformer`] trait allows custom transformations on block data,
//! such as encryption. Transformations are applied after compression on write
//! and before decompression on read.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::config::CompressionCodec;
use crate::db_state::{SsTableInfoCodec, SstType};
use crate::error::SlateDBError;
use crate::filter_policy::{FilterBuilder, FilterPolicy, NamedFilter};
use crate::flatbuffer_types::{BlockMeta, BlockMetaArgs};
use crate::format::sst::{
    BlockBuilder, BlockBuilderWithStats, EncodedSsTable, EncodedSsTableBlock,
    EncodedSsTableBlockBuilder, EncodedSsTableFooterBuilder, SsTableFormat, SST_FORMAT_VERSION,
    SST_FORMAT_VERSION_LATEST, SST_FORMAT_VERSION_V2,
};
use crate::sst_stats::SstStats;
use crate::types::RowEntry;
use crate::utils::compute_index_key;
use crate::BlockTransformer;
use bytes::Bytes;
use flatbuffers::DefaultAllocator;

/// SST block format version.
///
/// This enum is only available under `#[cfg(test)]` for integration tests
/// to verify backward compatibility between V1 and V2 formats.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(unreachable_pub)] // Exported conditionally under #[cfg(test)] in lib.rs
pub enum BlockFormat {
    /// Original block format (V1) with per-entry offsets.
    #[allow(dead_code)] // Used in tests
    V1,
    /// Prefix compression block format (V2) with restart points.
    #[allow(dead_code)] // Used in tests
    V2,
    /// Latest block format (V2) with restart points.
    Latest,
}

impl BlockFormat {
    /// Returns the SST format version corresponding to this block format.
    fn sst_format_version(self) -> u16 {
        match self {
            BlockFormat::V1 => SST_FORMAT_VERSION,
            BlockFormat::V2 => SST_FORMAT_VERSION_V2,
            BlockFormat::Latest => SST_FORMAT_VERSION_LATEST,
        }
    }
}

impl SsTableFormat {
    pub(crate) fn table_builder(&self) -> EncodedSsTableBuilder {
        let mut builder = EncodedSsTableBuilder::new(
            self.block_size,
            self.min_filter_keys,
            self.sst_codec.clone(),
            &self.filter_policies,
        );
        if let Some(block_format) = self.block_format {
            builder = builder.with_block_format(block_format);
        }
        if let Some(codec) = self.compression_codec {
            builder = builder.with_compression_codec(codec);
        }
        if let Some(ref transformer) = self.block_transformer {
            builder = builder.with_block_transformer(transformer.clone());
        }
        builder
    }
}

/// Builds an SSTable from key-value pairs.
pub(crate) struct EncodedSsTableBuilder {
    builder: BlockBuilderWithStats,
    index_builder: flatbuffers::FlatBufferBuilder<'static, DefaultAllocator>,
    first_key: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'static, u8>>>,
    sst_first_key: Option<Bytes>,
    sst_last_key: Option<Bytes>,
    current_block_first_key: Option<Bytes>,
    current_block_max_key: Option<Bytes>,
    block_meta: Vec<flatbuffers::WIPOffset<BlockMeta<'static>>>,
    current_len: u64,
    blocks: VecDeque<EncodedSsTableBlock>,
    block_size: usize,
    block_format: BlockFormat,
    sst_format_version: u16,
    min_filter_keys: u32,
    stats: SstStats,
    filter_builders: Vec<(String, Box<dyn FilterBuilder>)>,
    sst_codec: Box<dyn SsTableInfoCodec>,
    compression_codec: Option<CompressionCodec>,
    block_transformer: Option<Arc<dyn BlockTransformer>>,
}

impl EncodedSsTableBuilder {
    /// Create a builder based on target block size.
    pub(crate) fn new(
        block_size: usize,
        min_filter_keys: u32,
        sst_codec: Box<dyn SsTableInfoCodec>,
        filter_policies: &[Arc<dyn FilterPolicy>],
    ) -> Self {
        let filter_builders: Vec<(String, Box<dyn FilterBuilder>)> = filter_policies
            .iter()
            .map(|p| (p.name().to_string(), p.builder()))
            .collect();
        Self {
            current_len: 0,
            blocks: VecDeque::new(),
            block_meta: Vec::new(),
            first_key: None,
            sst_first_key: None,
            sst_last_key: None,
            current_block_first_key: None,
            current_block_max_key: None,
            block_size,
            block_format: BlockFormat::Latest,
            builder: BlockBuilderWithStats::new(BlockBuilder::new_latest(block_size)),
            sst_format_version: SST_FORMAT_VERSION_LATEST,
            min_filter_keys,
            stats: SstStats::default(),
            filter_builders,
            index_builder: flatbuffers::FlatBufferBuilder::new(),
            sst_codec,
            compression_codec: None,
            block_transformer: None,
        }
    }

    fn new_block_builder(&self) -> BlockBuilderWithStats {
        let builder = match self.block_format {
            BlockFormat::V1 => BlockBuilder::new_v1(self.block_size),
            BlockFormat::V2 => BlockBuilder::new_v2(self.block_size),
            BlockFormat::Latest => BlockBuilder::new_latest(self.block_size),
        };
        BlockBuilderWithStats::new(builder)
    }

    /// Sets the compression codec for compressing the blocks
    fn with_compression_codec(mut self, codec: CompressionCodec) -> Self {
        self.compression_codec = Some(codec);
        self
    }

    /// Sets the block transformer for transforming the blocks
    fn with_block_transformer(mut self, transformer: Arc<dyn BlockTransformer>) -> Self {
        self.block_transformer = Some(transformer);
        self
    }

    /// Sets the block format and derives the SST format version from it.
    /// This ensures consistency between block format and SST version.
    ///
    /// # Panics
    /// Panics if called after data has been added to the builder, as this would
    /// result in mixed block types within the SST.
    pub(crate) fn with_block_format(mut self, block_format: BlockFormat) -> Self {
        assert!(
            self.sst_first_key.is_none(),
            "cannot change block format after data has been added"
        );
        self.block_format = block_format;
        self.sst_format_version = block_format.sst_format_version();
        self.builder = self.new_block_builder();
        self
    }

    /// Adds an entry to the SSTable and returns the size of the block that was finished if any.
    /// The block size is calculated after applying any compression if enabled.
    /// The block size is None if the builder has not finished compacting a block yet.
    pub(crate) async fn add(&mut self, entry: RowEntry) -> Result<Option<usize>, SlateDBError> {
        self.stats.raw_key_size += entry.key.len() as u64;
        self.stats.raw_val_size += entry.value.len() as u64;

        let index_key = compute_index_key(self.current_block_max_key.clone(), &entry.key);
        let is_sst_first_key = self.sst_first_key.is_none();

        let mut block_size = None;
        if !self.builder.would_fit(&entry) {
            block_size = self.finish_block().await?;
            self.first_key = Some(self.index_builder.create_vector(&index_key));
        } else if is_sst_first_key {
            self.first_key = Some(self.index_builder.create_vector(&index_key));
        }

        for (_, fb) in &mut self.filter_builders {
            fb.add_entry(&entry);
        }
        if is_sst_first_key {
            self.sst_first_key = Some(entry.key.clone());
        }
        self.sst_last_key = Some(entry.key.clone());
        if self.builder.is_empty() {
            self.current_block_first_key = Some(entry.key.clone());
        }
        self.current_block_max_key = Some(entry.key.clone());

        self.builder.add(entry)?;

        Ok(block_size)
    }

    #[cfg(test)]
    pub(crate) async fn add_value(
        &mut self,
        key: &[u8],
        val: &[u8],
        ts: Option<i64>,
        expire_ts: Option<i64>,
    ) -> Result<Option<usize>, SlateDBError> {
        let entry = RowEntry::new(
            key.to_vec().into(),
            crate::types::ValueDeletable::Value(Bytes::copy_from_slice(val)),
            0,
            ts,
            expire_ts,
        );
        self.add(entry).await
    }

    pub(crate) fn next_block(&mut self) -> Option<EncodedSsTableBlock> {
        self.blocks.pop_front()
    }

    #[cfg(test)]
    pub(crate) fn num_blocks(&self) -> usize {
        // use block_meta since blocks can be consumed as sst is being built
        self.block_meta.len()
    }

    async fn finish_block(&mut self) -> Result<Option<usize>, SlateDBError> {
        if self.is_drained() {
            return Ok(None);
        }

        let new_builder = self.new_block_builder();
        let old_builder = std::mem::replace(&mut self.builder, new_builder);
        let (builder, block_stats) = old_builder.into_parts();
        let mut block_builder = EncodedSsTableBlockBuilder::new(builder, self.current_len);
        if let Some((first_key, last_key)) = self
            .current_block_first_key
            .take()
            .zip(self.current_block_max_key.take())
        {
            block_builder = block_builder.with_key_span(first_key, last_key);
        }
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
                first_key: self.first_key,
            },
        );
        self.block_meta.push(block_meta);
        self.stats.num_puts += block_stats.num_puts as u64;
        self.stats.num_deletes += block_stats.num_deletes as u64;
        self.stats.num_merges += block_stats.num_merges as u64;
        self.stats.block_stats.push(block_stats);

        let block_size = block.len();
        self.current_len += block_size as u64;
        self.blocks.push_back(block);
        self.first_key = None;

        Ok(Some(block_size))
    }

    /// Builds the SST from the current state.
    ///
    /// # Format
    ///
    /// +---------------------------------------------------+
    /// |                Data Blocks                        |
    /// |    (raw bytes produced by finish_block)           |
    /// +---------------------------------------------------+
    /// |                Filter Block*                      |
    /// |  +---------------------------------------------+  |
    /// |  | Filter Data (compressed encoded filter)     |  |
    /// |  +---------------------------------------------+  |
    /// |  | 4-byte Checksum (CRC32 of filter data)      |  |
    /// |  +---------------------------------------------+  |
    /// +---------------------------------------------------+
    /// |                Index Block                        |
    /// |  +---------------------------------------------+  |
    /// |  | Index Data (compressed index block)         |  |
    /// |  +---------------------------------------------+  |
    /// |  | 4-byte Checksum (CRC32 of index data)       |  |
    /// |  +---------------------------------------------+  |
    /// +---------------------------------------------------+
    /// |                Stats Block                        |
    /// |  +---------------------------------------------+  |
    /// |  | Stats Data (compressed encoded stats)       |  |
    /// |  +---------------------------------------------+  |
    /// |  | 4-byte Checksum (CRC32 of stats data)       |  |
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
    /// * Only present if num_keys >= min_filter_keys.
    ///
    pub(crate) async fn build(mut self) -> Result<EncodedSsTable, SlateDBError> {
        self.finish_block().await?;

        // Build footer (includes index building)
        let mut footer_builder = EncodedSsTableFooterBuilder::new(
            self.current_len,
            self.sst_first_key,
            self.sst_last_key,
            &*self.sst_codec,
            self.index_builder,
            self.block_meta,
            self.sst_format_version,
            SstType::Compacted,
        );
        if let Some(codec) = self.compression_codec {
            footer_builder = footer_builder.with_compression_codec(codec);
        }
        if let Some(transformer) = self.block_transformer.clone() {
            footer_builder = footer_builder.with_block_transformer(transformer);
        }
        // Build all filters if enough keys
        if self.stats.num_rows() >= self.min_filter_keys as u64 && !self.filter_builders.is_empty()
        {
            let filters: Arc<[NamedFilter]> = self
                .filter_builders
                .iter_mut()
                .map(|(name, fb)| NamedFilter {
                    name: name.clone(),
                    filter: fb.build(),
                })
                .collect::<Vec<_>>()
                .into();
            footer_builder = footer_builder.with_filters(filters);
        }
        footer_builder = footer_builder.with_stats(self.stats);

        let footer = footer_builder.build().await?;

        Ok(EncodedSsTable {
            format_version: self.sst_format_version,
            info: footer.info,
            index: footer.index,
            filters: footer.filters,
            stats: footer.stats,
            unconsumed_blocks: self.blocks,
            footer: footer.encoded_bytes,
        })
    }

    pub(crate) fn is_drained(&self) -> bool {
        self.builder.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use std::ops::Range;

    use async_trait::async_trait;
    use bytes::{BufMut, BytesMut};
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use std::sync::Arc;
    use std::vec;

    use super::*;
    use crate::blob::ReadOnlyBlob;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::block_iterator::{BlockIteratorLatest, BlockLike};
    use crate::bytes_range::BytesRange;
    use crate::db_state::{SsTableId, SsTableView};
    use crate::filter_policy::{BloomFilterPolicy, FilterQuery};
    use crate::format::block::Block;
    use crate::prefix_extractor::PrefixExtractor;
    use crate::reader::ReadTrace;
    use crate::sst_iter::{SstIterator, SstIteratorOptions};
    use crate::tablestore::{TableStore, TableStoreKind};
    use crate::test_utils::{assert_iterator, build_test_sst};
    use crate::wal::slatedb::store::{WalFileId, WalTableStore};

    fn test_sst_id(id: u64) -> SsTableId {
        SsTableId::from(ulid::Ulid::from_parts(id, 0))
    }

    #[test]
    fn test_estimate_encoded_size() {
        let mut format = SsTableFormat::default();

        // Test with zero entries
        assert_eq!(format.estimate_encoded_size_compacted(0, 0), 0);

        // Test with one entry
        let encoded_entry_size = 100;
        let size = format.estimate_encoded_size_compacted(1, encoded_entry_size);
        assert!(size > 0);

        // Test with multiple entries that don't trigger filter
        format.min_filter_keys = 1000;
        let num_entries = 100;
        let total_size = encoded_entry_size * num_entries;
        let size = format.estimate_encoded_size_compacted(num_entries, total_size);
        assert!(size > total_size); // Should be larger due to overhead

        // Test with entries that should trigger filter
        let num_entries = format.min_filter_keys as usize * 10;
        let total_size = encoded_entry_size * num_entries;
        let size_with_filter = format.estimate_encoded_size_compacted(num_entries, total_size);
        format.min_filter_keys = format.min_filter_keys * 10 + 1;
        let size_without_filter =
            format.estimate_encoded_size_compacted(num_entries, encoded_entry_size * num_entries);
        assert!(size_with_filter > size_without_filter); // Should be larger due to filter
    }

    /// If this test fails the size of encoded SSTs changed.
    /// This test is a reminder to update
    /// [`SsTableFormat::estimate_encoded_size_compacted`] and
    /// [`SsTableFormat::estimate_encoded_size_wal`].
    #[tokio::test]
    async fn test_estimate_vs_actual_encoded_size() {
        use crate::paths::PathResolver;
        use crate::types::ValueDeletable;
        use object_store::ObjectStoreExt;

        let format = SsTableFormat {
            block_size: 1024,
            min_filter_keys: 0, // always build a filter
            ..SsTableFormat::default()
        };
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let table_store = TableStore::new(
            object_store.clone(),
            format.clone(),
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let wal_store = WalTableStore::new(
            object_store.clone(),
            format.clone(),
            root_path.clone(),
            TableStoreKind::Main,
        );
        let path_resolver = PathResolver::from_root(root_path);

        // 16-byte keys/values, no timestamps. Keys are spread across the
        // keyspace (bit-reversed counter in the leading bytes) so adjacent keys
        // share almost no prefix — the estimate does not model the block's
        // prefix compression, so highly-similar keys would skew the comparison.
        let num_entries = 500usize;
        let mut keys: Vec<Bytes> = (0..num_entries as u64)
            .map(|i| {
                let mut k = [0u8; 16];
                k[..8].copy_from_slice(&i.reverse_bits().to_be_bytes());
                k[8..].copy_from_slice(&i.to_be_bytes());
                Bytes::copy_from_slice(&k)
            })
            .collect();
        keys.sort();
        let entries: Vec<RowEntry> = keys
            .into_iter()
            .enumerate()
            .map(|(i, key)| {
                RowEntry::new(
                    key,
                    ValueDeletable::Value(Bytes::from(format!("val{i:013}").into_bytes())),
                    (i + 1) as u64,
                    None,
                    None,
                )
            })
            .collect();
        let estimated_entries_size: usize = entries.iter().map(|e| e.estimated_size()).sum();

        let actual_size = |id: &SsTableId| {
            let object_store = object_store.clone();
            let path = path_resolver.sst_path(id);
            async move { object_store.head(&path).await.unwrap().size as usize }
        };

        let report = |label: &str, estimate: usize, actual: usize, expected_diff: usize| {
            assert_eq!(
                estimate as i64 - actual as i64,
                expected_diff as i64,
                "If this test fails the size of {label} SSTs changed. \
                This is a reminder to update SsTableFormat::estimate_encoded_size_{label}() \
                and/or this test, if needed."
            );
        };

        // --- compacted ---
        let mut builder = table_store.table_builder();
        for entry in &entries {
            builder.add(entry.clone()).await.unwrap();
        }
        let encoded = builder.build().await.unwrap();
        let compacted_id = SsTableId::from(ulid::Ulid::new());
        table_store
            .write_sst(&compacted_id, &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        report(
            "compacted",
            format.estimate_encoded_size_compacted(num_entries, estimated_entries_size),
            actual_size(&compacted_id).await,
            3065,
        );

        // --- wal ---
        let mut wal_builder = format.wal_table_builder();
        for entry in &entries {
            wal_builder.add(entry.clone()).await.unwrap();
        }
        let wal_encoded = wal_builder.build().await.unwrap();
        let wal_id = WalFileId::from(1);
        wal_store.write_sst(wal_id, &wal_encoded).await.unwrap();
        let wal_actual_size = object_store
            .head(&path_resolver.wal_sst_path(&wal_id))
            .await
            .unwrap()
            .size as usize;
        report(
            "wal",
            format.estimate_encoded_size_wal(num_entries, estimated_entries_size),
            wal_actual_size,
            3917,
        );
    }

    fn next_block_to_iter(builder: &mut EncodedSsTableBuilder) -> BlockIteratorLatest<Arc<Block>> {
        let block = builder.next_block();
        assert!(block.is_some());
        let block = block.unwrap().block;
        BlockIteratorLatest::new_ascending(block)
    }

    #[tokio::test]
    async fn test_builder_should_make_blocks_available() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 32,
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(&[b'a'; 8], &[b'1'; 8], Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(&[b'b'; 8], &[b'2'; 8], Some(2), None)
            .await
            .unwrap();
        builder
            .add_value(&[b'c'; 8], &[b'3'; 8], Some(3), None)
            .await
            .unwrap();

        // when:
        let mut iter = next_block_to_iter(&mut builder);
        assert_iterator(
            &mut iter,
            vec![RowEntry::new_value(&[b'a'; 8], &[b'1'; 8], 0).with_create_ts(1)],
        )
        .await;
        let mut iter = next_block_to_iter(&mut builder);
        assert_iterator(
            &mut iter,
            vec![RowEntry::new_value(&[b'b'; 8], &[b'2'; 8], 0).with_create_ts(2)],
        )
        .await;
        assert!(builder.next_block().is_none());
        builder
            .add_value(&[b'd'; 8], &[b'4'; 8], Some(1), None)
            .await
            .unwrap();
        let mut iter = next_block_to_iter(&mut builder);
        assert_iterator(
            &mut iter,
            vec![RowEntry::new_value(&[b'c'; 8], &[b'3'; 8], 0).with_create_ts(3)],
        )
        .await;
        assert!(builder.next_block().is_none());
    }

    #[tokio::test]
    async fn test_builder_should_return_unconsumed_blocks() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 32,
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format.clone(),
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(&[b'a'; 8], &[b'1'; 8], Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(&[b'b'; 8], &[b'2'; 8], Some(2), None)
            .await
            .unwrap();
        builder
            .add_value(&[b'c'; 8], &[b'3'; 8], Some(3), None)
            .await
            .unwrap();
        let first_block = builder.next_block();

        let encoded = builder.build().await.unwrap();

        let mut raw_sst = Vec::<u8>::new();
        raw_sst.put_slice(first_block.unwrap().encoded_bytes.as_ref());
        assert_eq!(encoded.unconsumed_blocks.len(), 2);
        encoded.put_remaining(&mut raw_sst);
        let raw_sst = Bytes::copy_from_slice(raw_sst.as_slice());
        let index = format
            .read_index_raw(&encoded.info, &raw_sst)
            .await
            .unwrap();
        let block = format
            .read_block_raw(&encoded.info, &index, 0, &raw_sst)
            .await
            .unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block);
        assert_iterator(
            &mut iter,
            vec![RowEntry::new_value(&[b'a'; 8], &[b'1'; 8], 0).with_create_ts(1)],
        )
        .await;
        let block = format
            .read_block_raw(&encoded.info, &index, 1, &raw_sst)
            .await
            .unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block);
        assert_iterator(
            &mut iter,
            vec![RowEntry::new_value(&[b'b'; 8], &[b'2'; 8], 0).with_create_ts(2)],
        )
        .await;
        let block = format
            .read_block_raw(&encoded.info, &index, 2, &raw_sst)
            .await
            .unwrap();
        let mut iter = BlockIteratorLatest::new_ascending(block);
        assert_iterator(
            &mut iter,
            vec![RowEntry::new_value(&[b'c'; 8], &[b'3'; 8], 0).with_create_ts(3)],
        )
        .await;
    }

    #[tokio::test]
    async fn test_builder_should_return_blocks_with_correct_data_and_offsets() {
        let format = SsTableFormat::default();

        let sst = build_test_sst(&format, 3).await;

        let bytes = sst.remaining_as_bytes();
        let index = format.read_index_raw(&sst.info, &bytes).await.unwrap();
        let block_metas = index.borrow().block_meta();
        assert_eq!(block_metas.len(), sst.unconsumed_blocks.len());
        for i in 0..block_metas.len() {
            let encoded_block = sst.unconsumed_blocks.get(i).unwrap();
            assert_eq!(block_metas.get(i).offset(), encoded_block.offset);
            let read_block = format
                .read_block_raw(&sst.info, &index, i, &bytes)
                .await
                .unwrap();
            assert!(*encoded_block.block == read_block);
        }
    }

    #[tokio::test]
    async fn test_builder_should_track_block_key_spans() {
        // one entry per block
        let format = SsTableFormat {
            block_size: 32,
            ..SsTableFormat::default()
        };
        let mut builder = format.table_builder();
        for i in 0..4u8 {
            builder
                .add_value(&[b'a' + i; 16], &[i; 16], None, None)
                .await
                .unwrap();
        }
        let sst = builder.build().await.unwrap();
        assert_eq!(sst.unconsumed_blocks.len(), 4);
        for (i, block) in sst.unconsumed_blocks.iter().enumerate() {
            let key = Bytes::copy_from_slice(&[b'a' + i as u8; 16]);
            assert_eq!(block.key_span, Some((key.clone(), key)));
        }

        // all entries in one block
        let mut builder = SsTableFormat::default().table_builder();
        for i in 0..4u8 {
            builder
                .add_value(&[b'a' + i; 16], &[i; 16], None, None)
                .await
                .unwrap();
        }
        let sst = builder.build().await.unwrap();
        assert_eq!(sst.unconsumed_blocks.len(), 1);
        assert_eq!(
            sst.unconsumed_blocks[0].key_span,
            Some((
                Bytes::copy_from_slice(&[b'a'; 16]),
                Bytes::copy_from_slice(&[b'd'; 16])
            ))
        );
    }

    #[rstest]
    #[case::default_sst(SsTableFormat::default(), 0, true)]
    #[case::sst_with_no_filter(SsTableFormat { min_filter_keys: 9, ..SsTableFormat::default() }, 0, false)]
    #[case::sst_builds_filter_with_10_bits_per_key(SsTableFormat { filter_policies: vec![Arc::new(BloomFilterPolicy::new(10))], ..SsTableFormat::default() }, 0, true)]
    #[case::sst_builds_filter_with_20_bits_per_key(SsTableFormat { filter_policies: vec![Arc::new(BloomFilterPolicy::new(20))], ..SsTableFormat::default() }, 0, true)]
    #[tokio::test]
    async fn test_sstable(
        #[case] format: SsTableFormat,
        #[case] table_id_seed: u64,
        #[case] should_have_filter: bool,
    ) {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let table_store = TableStore::new(
            object_store,
            format.clone(),
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        for k in 1..=8 {
            builder
                .add_value(
                    format!("key{}", k).as_bytes(),
                    format!("value{}", k).as_bytes(),
                    Some(k),
                    None,
                )
                .await
                .unwrap();
        }
        let encoded = builder.build().await.unwrap();
        let encoded_info = encoded.info.clone();

        for nf in encoded.filters.iter() {
            let mut encoded_filter = Vec::new();
            nf.filter.encode(&mut encoded_filter);
            // encoded data includes the 2-byte num_probes header
            assert_eq!(encoded_filter.len(), 2 + nf.filter.size());
        }

        // write sst and validate that the handle returned has the correct content.
        let sst_handle = table_store
            .write_sst(&test_sst_id(table_id_seed), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        assert_eq!(encoded_info, sst_handle.info);
        let sst_info = sst_handle.info;
        assert_eq!(
            b"key1",
            sst_info.first_entry.unwrap().as_ref(),
            "first entry in sst info should be correct"
        );

        // construct sst info from the raw bytes and validate that it matches the original info.
        let sst_handle_from_store = table_store
            .open_sst(&test_sst_id(table_id_seed), Some(Bytes::new()))
            .await
            .unwrap();
        assert_eq!(encoded_info, sst_handle_from_store.info);
        let index = table_store
            .read_index(
                &sst_handle_from_store,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        let sst_info_from_store = sst_handle_from_store.info;
        assert_eq!(1, index.borrow().block_meta().len());
        assert_eq!(
            b"key1",
            sst_info_from_store.first_entry.unwrap().as_ref(),
            "first entry in sst info should be correct after reading from store"
        );
        assert_eq!(
            b"",
            index.borrow().block_meta().get(0).first_key().bytes(),
            "index key in block meta should be correct after reading from store"
        );

        // Validate filter presence
        if should_have_filter {
            assert!(sst_info.filter_len > 0);
        } else {
            assert_eq!(sst_info.filter_len, 0);
        }
    }

    #[rstest]
    #[case::none(None)]
    #[cfg_attr(feature = "snappy", case::snappy(Some(CompressionCodec::Snappy)))]
    #[cfg_attr(feature = "zlib", case::zlib(Some(CompressionCodec::Zlib)))]
    #[cfg_attr(feature = "lz4", case::lz4(Some(CompressionCodec::Lz4)))]
    #[cfg_attr(feature = "zstd", case::zstd(Some(CompressionCodec::Zstd)))]
    #[tokio::test]
    async fn test_sstable_with_compression(#[case] compression: Option<CompressionCodec>) {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

        let format = SsTableFormat {
            compression_codec: compression,
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let encoded_info = encoded.info.clone();
        table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let sst_handle = table_store
            .open_sst(&test_sst_id(0), Some(Bytes::new()))
            .await
            .unwrap();
        let index = table_store
            .read_index(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        let filters = table_store
            .read_filters(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert!(!filters.is_empty());
        let filter = &filters[0].filter;

        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"key1"))));
        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"key2"))));
        assert_eq!(encoded_info, sst_handle.info);
        assert_eq!(1, index.borrow().block_meta().len());
        assert_eq!(
            b"key1",
            sst_handle.info.first_entry.unwrap().as_ref(),
            "first entry in sst info should be correct"
        );
    }

    #[rstest]
    #[case::none(None, None)]
    #[cfg_attr(
        all(feature = "snappy", feature = "zlib"),
        case::snappy_zlib(Some(CompressionCodec::Snappy), Some(CompressionCodec::Zlib))
    )]
    #[cfg_attr(
        all(feature = "zlib", feature = "lz4"),
        case::zlib_lz4(Some(CompressionCodec::Zlib), Some(CompressionCodec::Lz4))
    )]
    #[cfg_attr(
        all(feature = "lz4", feature = "zstd"),
        case::lz4_zstd(Some(CompressionCodec::Lz4), Some(CompressionCodec::Zstd))
    )]
    #[cfg_attr(
        all(feature = "zstd", feature = "snappy"),
        case::zstd_snappy(Some(CompressionCodec::Zstd), Some(CompressionCodec::Snappy))
    )]
    #[tokio::test]
    async fn test_sstable_with_compression_using_sst_info(
        #[case] compression: Option<CompressionCodec>,
        #[case] dummy_codec: Option<CompressionCodec>,
    ) {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            compression_codec: compression,
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store.clone(),
            format,
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let encoded_info = encoded.info.clone();
        table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();

        // decompression is independent of TableFormat. It uses the CompressionFormat from SSTable Info to decompress sst.
        let format = SsTableFormat {
            compression_codec: dummy_codec,
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let sst_handle = table_store
            .open_sst(&test_sst_id(0), Some(Bytes::new()))
            .await
            .unwrap();
        let index = table_store
            .read_index(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        let filters = table_store
            .read_filters(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert!(!filters.is_empty());
        let filter = &filters[0].filter;

        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"key1"))));
        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"key2"))));
        assert_eq!(encoded_info, sst_handle.info);
        assert_eq!(1, index.borrow().block_meta().len());
        assert_eq!(
            b"key1",
            sst_handle.info.first_entry.unwrap().as_ref(),
            "first entry in sst info should be correct"
        );
    }

    #[rstest]
    #[case::partial_blocks(0..2, vec![
        vec![
            RowEntry::new_value(&[b'a'; 2], &[1u8; 2], 0),
            RowEntry::new_value(&[b'b'; 2], &[2u8; 2], 0),
        ],
        vec![
            RowEntry::new_value(&[b'c'; 20], &[3u8; 20], 0).with_create_ts(3),
        ],
    ])]
    #[case::all_blocks(0..3, vec![
        vec![
            RowEntry::new_value(&[b'a'; 2], &[1u8; 2], 0),
            RowEntry::new_value(&[b'b'; 2], &[2u8; 2], 0),
        ],
        vec![
            RowEntry::new_value(&[b'c'; 20], &[3u8; 20], 0).with_create_ts(3),
        ],
        vec![
            RowEntry::new_value(&[b'd'; 20], &[4u8; 20], 0).with_create_ts(4),
        ],
    ])]
    #[tokio::test]
    async fn test_read_blocks(
        #[case] block_range: Range<usize>,
        #[case] expected_blocks: Vec<Vec<RowEntry>>,
    ) {
        // given:
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 48,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format.clone(),
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(&[b'a'; 2], &[1u8; 2], None, None)
            .await
            .unwrap();
        builder
            .add_value(&[b'b'; 2], &[2u8; 2], None, None)
            .await
            .unwrap();
        builder
            .add_value(&[b'c'; 20], &[3u8; 20], Some(3), None)
            .await
            .unwrap();
        builder
            .add_value(&[b'd'; 20], &[4u8; 20], Some(4), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let info = encoded.info.clone();
        let bytes = encoded.remaining_as_bytes();
        let index = format.read_index_raw(&encoded.info, &bytes).await.unwrap();
        let blob = BytesBlob { bytes };

        // when:
        let mut blocks = format
            .read_blocks(&info, &index, block_range, &blob)
            .await
            .unwrap();

        // then:
        for expected_entries in expected_blocks {
            let mut iter = BlockIteratorLatest::new_ascending(blocks.pop_front().unwrap());
            assert_iterator(&mut iter, expected_entries).await;
        }
        assert!(blocks.is_empty())
    }

    #[tokio::test]
    async fn test_sstable_index_size() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 32,
            ..SsTableFormat::default()
        };

        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let encoded_info = encoded.info.clone();

        // write sst and validate that the handle returned has the correct content.
        let sst_handle = table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        assert_eq!(encoded_info, sst_handle.info);
        let sst_info = sst_handle.info;
        assert_eq!(
            b"key1",
            sst_info.first_entry.unwrap().as_ref(),
            "first entry in sst info should be correct"
        );

        // construct sst info from the raw bytes and validate that it matches the original info.
        let sst_handle_from_store = table_store
            .open_sst(&test_sst_id(0), Some(Bytes::new()))
            .await
            .unwrap();
        assert_eq!(encoded_info, sst_handle_from_store.info);
        let index = table_store
            .read_index(
                &sst_handle_from_store,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();

        assert_eq!(88, index.size());
    }

    #[tokio::test]
    async fn test_checksum_mismatch() {
        let format = SsTableFormat::default();
        // create corrupted data by modifying bytes but keeping same checksum
        let mut corrupted_bytes = BytesMut::new();
        let bytes = &b"something"[..];
        corrupted_bytes.put(bytes);
        corrupted_bytes.put_u32(crc32fast::hash(bytes)); // original checksum
        corrupted_bytes[0] ^= 1; // corrupt one byte

        assert!(matches!(
            format.validate_checksum(corrupted_bytes.into()),
            Err(SlateDBError::ChecksumMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn test_version_checking() {
        // Create a valid SST
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 32,
            ..SsTableFormat::default()
        };

        let table_store = TableStore::new(
            object_store,
            format.clone(),
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let bytes = encoded.remaining_as_bytes();

        // Test valid version decodes properly through read_info
        let valid_blob = BytesBlob {
            bytes: bytes.clone(),
        };
        let result = format.read_info_and_version(&valid_blob).await;
        match result {
            Ok(_) => {}
            Err(e) => {
                panic!("Expected Ok result, but got error: {:?}", e);
            }
        }

        let mut invalid_bytes = BytesMut::from(bytes.clone());
        // Corrupt the version
        invalid_bytes[bytes.len() - 1] ^= 1;
        let invalid_blob = BytesBlob {
            bytes: invalid_bytes.freeze(),
        };
        assert!(matches!(
            format.read_info_and_version(&invalid_blob).await,
            Err(SlateDBError::InvalidVersion { .. })
        ));
    }

    #[rstest]
    #[case::owned(true)]
    #[case::borrowed(false)]
    #[tokio::test]
    async fn test_sst_handle_with_visible_ranges(
        #[case] is_owned: bool,
    ) -> Result<(), SlateDBError> {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 1024,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let mut builder = table_store.table_builder();
        for key in 'a'..='z' {
            let key_bytes = [key as u8];
            builder.add_value(&key_bytes, b"value", None, None).await?;
        }
        let encoded = builder.build().await?;

        let sst_id = test_sst_id(0);
        let sst_handle = SsTableView::identity(
            table_store
                .write_sst(&sst_id, &encoded, Some(Bytes::new()))
                .await?,
        )
        .with_visible_range(BytesRange::from_ref("c"..="f"));

        let expected_entries = vec![
            RowEntry::new_value(b"c", b"value", 0),
            RowEntry::new_value(b"d", b"value", 0),
            RowEntry::new_value(b"e", b"value", 0),
            RowEntry::new_value(b"f", b"value", 0),
        ];

        if is_owned {
            // scan the entire sst and validate that the visible range is respected.
            let mut iter = SstIterator::new_owned_initialized(
                ..,
                sst_handle.clone(),
                table_store.clone(),
                SstIteratorOptions::default(),
            )
            .await?
            .expect("Expected Some(iter) but got None");

            assert_iterator(&mut iter, expected_entries).await;

            // scan range outside of visible range and validate that it returns empty iterator.
            let iter = SstIterator::new_owned_initialized(
                Bytes::from_static(b"m")..Bytes::from_static(b"p"),
                sst_handle,
                table_store,
                SstIteratorOptions::default(),
            )
            .await?;

            assert!(iter.is_none());
        } else {
            // scan the entire sst and validate that the visible range is respected.
            let mut iter = SstIterator::new_borrowed_initialized(
                ..,
                &sst_handle,
                table_store.clone(),
                SstIteratorOptions::default(),
            )
            .await?
            .expect("Expected Some(iter) but got None");

            assert_iterator(&mut iter, expected_entries).await;

            // scan range outside of visible range and validate that it returns empty iterator.
            let iter = SstIterator::new_borrowed_initialized(
                Bytes::from_static(b"m")..Bytes::from_static(b"p"),
                &sst_handle,
                table_store,
                SstIteratorOptions::default(),
            )
            .await?;

            assert!(iter.is_none());
        }

        Ok(())
    }

    struct BytesBlob {
        bytes: Bytes,
    }

    impl ReadOnlyBlob for BytesBlob {
        async fn len(&self) -> Result<u64, SlateDBError> {
            Ok(self.bytes.len() as u64)
        }

        async fn read_range(&self, range: Range<u64>) -> Result<Bytes, SlateDBError> {
            Ok(self.bytes.slice(range.start as usize..range.end as usize))
        }

        async fn read(&self) -> Result<Bytes, SlateDBError> {
            Ok(self.bytes.clone())
        }
    }

    struct XorTransformer {
        key: u8,
    }

    #[async_trait]
    impl BlockTransformer for XorTransformer {
        async fn encode(&self, data: Bytes) -> Result<Bytes, crate::error::Error> {
            let transformed: Vec<u8> = data.iter().map(|b| b ^ self.key).collect();
            Ok(Bytes::from(transformed))
        }

        async fn decode(&self, data: Bytes) -> Result<Bytes, crate::error::Error> {
            self.encode(data).await
        }
    }

    #[tokio::test]
    async fn test_sstable_with_block_transformer() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let transformer = Arc::new(XorTransformer { key: 0x42 });

        let format = SsTableFormat {
            block_transformer: Some(transformer.clone()),
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let encoded_info = encoded.info.clone();
        table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();

        let sst_handle = table_store
            .open_sst(&test_sst_id(0), Some(Bytes::new()))
            .await
            .unwrap();
        let index = table_store
            .read_index(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        let filters = table_store
            .read_filters(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert!(!filters.is_empty());
        let filter = &filters[0].filter;

        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"key1"))));
        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"key2"))));
        assert_eq!(encoded_info, sst_handle.info);
        assert_eq!(1, index.borrow().block_meta().len());
        assert_eq!(
            b"key1",
            sst_handle.info.first_entry.unwrap().as_ref(),
            "first entry in sst info should be correct"
        );
    }

    #[tokio::test]
    async fn test_block_transformer_with_compression() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let transformer = Arc::new(XorTransformer { key: 0xAB });

        #[cfg(feature = "snappy")]
        let compression = Some(CompressionCodec::Snappy);
        #[cfg(not(feature = "snappy"))]
        let compression = None;

        let format = SsTableFormat {
            block_transformer: Some(transformer),
            compression_codec: compression,
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();

        let sst_handle = table_store
            .open_sst(&test_sst_id(0), Some(Bytes::new()))
            .await
            .unwrap();
        let index = table_store
            .read_index(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();

        assert_eq!(1, index.borrow().block_meta().len());
        assert_eq!(
            b"key1",
            sst_handle.info.first_entry.unwrap().as_ref(),
            "first entry in sst info should be correct with compression + transformer"
        );
    }

    #[test]
    fn should_allow_with_block_format_before_adding_data() {
        // given: a builder with no data
        let format = SsTableFormat::default();
        let builder = format.table_builder();

        // when/then: with_block_format should succeed before adding any data
        let _ = builder.with_block_format(BlockFormat::V2);
    }

    #[tokio::test]
    #[should_panic(expected = "cannot change block format after data has been added")]
    async fn should_panic_with_block_format_after_adding_data() {
        // given: a builder that has had data added
        let format = SsTableFormat::default();
        let mut builder = format.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();

        // when/then: with_block_format should panic
        let _ = builder.with_block_format(BlockFormat::V2);
    }

    #[test]
    fn should_default_to_latest_block_format() {
        // given/when: create a new builder
        let format = SsTableFormat::default();
        let builder = format.table_builder();

        // then: it should default to latest block format (currently V2)
        assert_eq!(builder.block_format, BlockFormat::Latest);
        assert_eq!(builder.sst_format_version, SST_FORMAT_VERSION_LATEST);
    }

    #[test]
    fn should_use_v1_format_version_when_explicitly_configured() {
        // given: a builder configured to use V1 format
        let format = SsTableFormat::default();
        let builder = format.table_builder().with_block_format(BlockFormat::V1);

        // then: the builder should have V1 format settings
        assert_eq!(builder.block_format, BlockFormat::V1);
        assert_eq!(builder.sst_format_version, SST_FORMAT_VERSION);
    }

    #[tokio::test]
    async fn should_read_v1_sst_written_with_explicit_config() {
        // given: an SST built with explicit V1 configuration and enough entries
        // to distinguish V1 (per-entry offsets) from V2 (restart point offsets)
        let num_entries = 20;
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat::default();
        let table_store = TableStore::new(
            object_store,
            format.clone(),
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store
            .table_builder()
            .with_block_format(BlockFormat::V1);
        let mut expected = Vec::new();
        for i in 0..num_entries {
            let key = format!("key_{:03}", i);
            let value = format!("value_{}", i);
            builder
                .add_value(key.as_bytes(), value.as_bytes(), Some(i as i64), None)
                .await
                .unwrap();
            expected.push(
                RowEntry::new_value(key.as_bytes(), value.as_bytes(), 0).with_create_ts(i as i64),
            );
        }
        let encoded = builder.build().await.unwrap();
        let sst_handle = table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();

        // then: the stored SST should be V1 format
        let version = table_store
            .read_sst_version(&sst_handle.id, Some(Bytes::new()))
            .await
            .unwrap();
        assert_eq!(version, SST_FORMAT_VERSION);
        assert_eq!(sst_handle.format_version, SST_FORMAT_VERSION);

        // then: V1 blocks should have one offset per entry
        let blocks = table_store
            .read_blocks(&sst_handle, 0..1, Some(Bytes::new()))
            .await
            .unwrap();
        let block = &blocks[0];
        // V1: offsets.len() == number of entries in the block, which should be
        // much larger than 1 (unlike V2 which would have ~1 restart point)
        assert_eq!(
            block.offsets().len(),
            num_entries,
            "V1 blocks should have one offset per entry"
        );

        // when: reading the SST back
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            SsTableView::identity(sst_handle),
            Arc::new(table_store),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .unwrap();

        // then: all data should be readable
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_read_v2_sst_written_with_default() {
        // given: an SST built with default settings (V2) and enough entries
        // to distinguish V2 (restart point offsets) from V1 (per-entry offsets)
        let num_entries = 20;
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat::default();
        let table_store = TableStore::new(
            object_store,
            format.clone(),
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        let mut expected = Vec::new();
        for i in 0..num_entries {
            let key = format!("key_{:03}", i);
            let value = format!("value_{}", i);
            builder
                .add_value(key.as_bytes(), value.as_bytes(), Some(i as i64), None)
                .await
                .unwrap();
            expected.push(
                RowEntry::new_value(key.as_bytes(), value.as_bytes(), 0).with_create_ts(i as i64),
            );
        }
        let encoded = builder.build().await.unwrap();
        let sst_handle = table_store
            .write_sst(&test_sst_id(1), &encoded, Some(Bytes::new()))
            .await
            .unwrap();

        // then: the stored SST should be V2 format
        let version = table_store
            .read_sst_version(&sst_handle.id, Some(Bytes::new()))
            .await
            .unwrap();
        assert_eq!(version, SST_FORMAT_VERSION_LATEST);
        assert_eq!(sst_handle.format_version, SST_FORMAT_VERSION_LATEST);

        // then: V2 blocks should have fewer offsets than entries (restart points only)
        let blocks = table_store
            .read_blocks(&sst_handle, 0..1, Some(Bytes::new()))
            .await
            .unwrap();
        let block = &blocks[0];
        // V2 default restart interval is 16, so 20 entries -> 2 restart points
        assert!(
            block.offsets().len() < num_entries,
            "V2 blocks should have fewer offsets (restart points) than entries: {} offsets vs {} entries",
            block.offsets().len(),
            num_entries,
        );

        // when: reading the SST back
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            SsTableView::identity(sst_handle),
            Arc::new(table_store),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .unwrap();

        // then: all data should be readable
        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn test_stats_block_with_mixed_entry_types() {
        use crate::types::ValueDeletable;

        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat::default();
        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();

        // Add a put
        builder
            .add(RowEntry::new(
                Bytes::from_static(b"key1"),
                ValueDeletable::Value(Bytes::from_static(b"val1")),
                0,
                None,
                None,
            ))
            .await
            .unwrap();
        // Add a delete
        builder
            .add(RowEntry::new(
                Bytes::from_static(b"key2"),
                ValueDeletable::Tombstone,
                0,
                None,
                None,
            ))
            .await
            .unwrap();
        // Add a merge
        builder
            .add(RowEntry::new(
                Bytes::from_static(b"key3"),
                ValueDeletable::Merge(Bytes::from_static(b"merge_val")),
                0,
                None,
                None,
            ))
            .await
            .unwrap();
        // Add another put
        builder
            .add(RowEntry::new(
                Bytes::from_static(b"key4"),
                ValueDeletable::Value(Bytes::from_static(b"val4")),
                0,
                None,
                None,
            ))
            .await
            .unwrap();

        let encoded = builder.build().await.unwrap();
        let sst_handle = table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let stats = table_store
            .read_stats(&sst_handle, true, Some(Bytes::new()))
            .await
            .unwrap()
            .expect("stats should be present");

        assert_eq!(stats.num_puts, 2);
        assert_eq!(stats.num_deletes, 1);
        assert_eq!(stats.num_merges, 1);
        assert_eq!(
            stats.raw_key_size,
            (b"key1".len() + b"key2".len() + b"key3".len() + b"key4".len()) as u64
        );
        // Tombstone has 0 value size
        assert_eq!(
            stats.raw_val_size,
            (b"val1".len() + b"merge_val".len() + b"val4".len()) as u64
        );
    }

    #[tokio::test]
    async fn test_stats_block_backward_compat() {
        // Old SSTs without stats block should return None
        let info = crate::db_state::SsTableInfo {
            stats_offset: 0,
            stats_len: 0,
            ..Default::default()
        };
        let format = SsTableFormat::default();
        let bytes = Bytes::from(vec![0u8; 100]);
        let blob = BytesBlob { bytes };
        let stats = format.read_stats(&info, &blob).await.unwrap();
        assert!(stats.is_none());
    }

    #[rstest]
    #[case::none(None)]
    #[cfg_attr(feature = "snappy", case::snappy(Some(CompressionCodec::Snappy)))]
    #[cfg_attr(feature = "zlib", case::zlib(Some(CompressionCodec::Zlib)))]
    #[cfg_attr(feature = "lz4", case::lz4(Some(CompressionCodec::Lz4)))]
    #[cfg_attr(feature = "zstd", case::zstd(Some(CompressionCodec::Zstd)))]
    #[tokio::test]
    async fn test_stats_block_with_compression(#[case] compression: Option<CompressionCodec>) {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            compression_codec: compression,
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let sst_handle = table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let stats = table_store
            .read_stats(&sst_handle, true, Some(Bytes::new()))
            .await
            .unwrap()
            .expect("stats should be present");

        assert_eq!(stats.num_puts, 2);
        assert_eq!(stats.num_deletes, 0);
        assert_eq!(stats.num_merges, 0);
        assert_eq!(stats.raw_key_size, 8);
        assert_eq!(stats.raw_val_size, 12);
    }

    #[tokio::test]
    async fn test_stats_block_with_block_transformer() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let transformer = Arc::new(XorTransformer { key: 0x42 });
        let format = SsTableFormat {
            block_transformer: Some(transformer),
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let sst_handle = table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let stats = table_store
            .read_stats(&sst_handle, true, Some(Bytes::new()))
            .await
            .unwrap()
            .expect("stats should be present");

        assert_eq!(stats.num_puts, 2);
        assert_eq!(stats.num_deletes, 0);
        assert_eq!(stats.num_merges, 0);
        assert_eq!(stats.raw_key_size, 8);
        assert_eq!(stats.raw_val_size, 12);
    }

    #[tokio::test]
    async fn test_block_stats_multi_block() {
        use crate::types::ValueDeletable;
        // Use a small block_size (32) with large entries (16-byte keys + 16-byte
        // values) so every entry exceeds the block capacity and lands in its own
        // block regardless of value type.
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 32,
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = table_store.table_builder();
        // Block 0: put
        builder
            .add_value(&[b'a'; 16], &[b'1'; 16], Some(1), None)
            .await
            .unwrap();
        // Block 1: delete
        builder
            .add(RowEntry::new(
                Bytes::from_static(&[b'b'; 16]),
                ValueDeletable::Tombstone,
                0,
                None,
                None,
            ))
            .await
            .unwrap();
        // Block 2: merge
        builder
            .add(RowEntry::new(
                Bytes::from_static(&[b'c'; 16]),
                ValueDeletable::Merge(Bytes::from_static(&[b'3'; 16])),
                0,
                None,
                None,
            ))
            .await
            .unwrap();

        let encoded = builder.build().await.unwrap();
        let sst_handle = table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let stats = table_store
            .read_stats(&sst_handle, true, Some(Bytes::new()))
            .await
            .unwrap()
            .expect("stats should be present");

        // Aggregate totals
        assert_eq!(stats.num_puts, 1);
        assert_eq!(stats.num_deletes, 1);
        assert_eq!(stats.num_merges, 1);

        // Block stats: one block per entry, each with the correct type.
        assert_eq!(stats.block_stats.len(), 3);
        // Block 0: put
        assert_eq!(stats.block_stats[0].num_puts, 1);
        assert_eq!(stats.block_stats[0].num_deletes, 0);
        assert_eq!(stats.block_stats[0].num_merges, 0);
        // Block 1: delete
        assert_eq!(stats.block_stats[1].num_puts, 0);
        assert_eq!(stats.block_stats[1].num_deletes, 1);
        assert_eq!(stats.block_stats[1].num_merges, 0);
        // Block 2: merge
        assert_eq!(stats.block_stats[2].num_puts, 0);
        assert_eq!(stats.block_stats[2].num_deletes, 0);
        assert_eq!(stats.block_stats[2].num_merges, 1);
    }

    #[tokio::test]
    async fn test_multi_policy_composite_filter_round_trip() {
        // Minimal fixed-prefix extractor defined inline so this test has no
        // dependency on the private test helper in filter_policy.rs.
        struct Fixed3Extractor;
        impl PrefixExtractor for Fixed3Extractor {
            fn name(&self) -> &str {
                "fixed3"
            }
            fn prefix_len(&self, target: &crate::prefix_extractor::PrefixTarget) -> Option<usize> {
                let input = match target {
                    crate::prefix_extractor::PrefixTarget::Point(k) => k.as_ref(),
                    crate::prefix_extractor::PrefixTarget::Prefix(p) => p.as_ref(),
                };
                (input.len() >= 3).then_some(3)
            }
        }

        let policy1 = Arc::new(BloomFilterPolicy::new(10)); // name "_bf"
        let policy2 = Arc::new(
            BloomFilterPolicy::new(10).with_prefix_extractor(Arc::new(Fixed3Extractor)), // name "_bf:p=fixed3"
        );

        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 1,
            filter_policies: vec![policy1.clone(), policy2.clone()],
            ..SsTableFormat::default()
        };
        let table_store = TableStore::new(
            object_store.clone(),
            format,
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );

        // Write keys whose 3-byte prefix is "key".
        let mut builder = table_store.table_builder();
        for i in 0u32..5 {
            let key = format!("key{:04}", i);
            builder
                .add_value(key.as_bytes(), b"value", Some(i as i64), None)
                .await
                .unwrap();
        }
        let encoded = builder.build().await.unwrap();
        table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let handle = table_store
            .open_sst(&test_sst_id(0), Some(Bytes::new()))
            .await
            .unwrap();

        // --- Both sub-filters decoded correctly ---
        let filters = table_store
            .read_filters(
                &handle,
                false,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            filters.len(),
            2,
            "composite block should contain two sub-filters"
        );

        let f1 = filters
            .iter()
            .find(|nf| nf.name == "_bf")
            .expect("policy1 filter missing");
        let f2 = filters
            .iter()
            .find(|nf| nf.name == "_bf:p=fixed3")
            .expect("policy2 filter missing");

        // policy1 answers point queries for inserted keys.
        let q = FilterQuery::point(Bytes::from_static(b"key0000"));
        assert!(f1.filter.might_match(&q));

        // policy2 answers 3-byte prefix queries for inserted keys.
        let q = FilterQuery::prefix(Bytes::from_static(b"key"));
        assert!(f2.filter.might_match(&q));

        // --- Graceful skip of unrecognized policy ---
        // A reader that only knows policy1 should silently drop policy2's
        // sub-filter and return exactly one filter.
        let format_partial = SsTableFormat {
            min_filter_keys: 1,
            filter_policies: vec![policy1.clone()],
            ..SsTableFormat::default()
        };
        let store_partial = TableStore::new(
            object_store,
            format_partial,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let handle_partial = store_partial
            .open_sst(&test_sst_id(0), Some(Bytes::new()))
            .await
            .unwrap();
        let partial = store_partial
            .read_filters(
                &handle_partial,
                false,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            partial.len(),
            1,
            "unknown policy sub-filter should be silently dropped"
        );
        assert_eq!(partial[0].name, "_bf");
    }
}
