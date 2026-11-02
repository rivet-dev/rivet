use crate::blob::ReadOnlyBlob;
use crate::config::CompressionCodec;
use crate::db_state::{FilterFormat, SsTableInfo, SsTableInfoCodec, SstType};
use crate::error::SlateDBError;
use crate::filter_policy::{BloomFilterPolicy, FilterPolicy, NamedFilter};
use crate::flatbuffer_types::{
    BlockMeta, FlatBufferSsTableInfoCodec, SsTableIndex, SsTableIndexArgs, SsTableIndexOwned,
};
use crate::format::block::{Block, BlockBuilderV1};
use crate::format::block_v2::BlockBuilderV2;
use crate::format::row;
use crate::sst_stats::{BlockStats, SstStats};
use async_trait::async_trait;
use bytes::{Buf, BufMut, Bytes};
use flatbuffers::DefaultAllocator;
use futures::future::try_join_all;
use log::warn;
use std::collections::VecDeque;
#[cfg(feature = "zlib")]
use std::io::Read;
#[cfg(feature = "zlib")]
use std::io::Write;
use std::ops::Range;
use std::sync::Arc;

// 8 bytes for the metadata offset + 2 bytes for the version
const NUM_FOOTER_BYTES: usize = 10;
const NUM_FOOTER_BYTES_LONG: u64 = NUM_FOOTER_BYTES as u64;
const SEQNUM_SIZE: usize = size_of::<u64>();
pub(crate) const SST_FORMAT_VERSION: u16 = 1;
pub(crate) const SST_FORMAT_VERSION_V2: u16 = 2;
pub(crate) const SST_FORMAT_VERSION_LATEST: u16 = SST_FORMAT_VERSION_V2;

fn is_supported_version(version: u16) -> bool {
    matches!(version, SST_FORMAT_VERSION | SST_FORMAT_VERSION_V2)
}

#[allow(private_interfaces)]
pub(crate) enum BlockBuilder {
    V1(BlockBuilderV1),
    V2(BlockBuilderV2),
}

impl BlockBuilder {
    pub(crate) fn new_v1(block_size: usize) -> Self {
        Self::V1(BlockBuilderV1::new(block_size))
    }

    pub(crate) fn new_v2(block_size: usize) -> Self {
        Self::V2(BlockBuilderV2::new(block_size))
    }

    pub(crate) fn new_latest(block_size: usize) -> Self {
        Self::new_v2(block_size)
    }

    #[cfg(test)]
    pub(crate) fn new_v2_with_restart_interval(block_size: usize, restart_interval: usize) -> Self {
        Self::V2(BlockBuilderV2::new_with_restart_interval(
            block_size,
            restart_interval,
        ))
    }

    pub(crate) fn would_fit(&self, entry: &crate::types::RowEntry) -> bool {
        match self {
            Self::V1(builder) => builder.would_fit(entry),
            Self::V2(builder) => builder.would_fit(entry),
        }
    }

    pub(crate) fn add(&mut self, entry: crate::types::RowEntry) -> Result<bool, SlateDBError> {
        match self {
            Self::V1(builder) => builder.add(entry),
            Self::V2(builder) => builder.add(entry),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::V1(builder) => builder.is_empty(),
            Self::V2(builder) => builder.is_empty(),
        }
    }

    pub(crate) fn build(self) -> Result<Block, SlateDBError> {
        match self {
            Self::V1(builder) => builder.build(),
            Self::V2(builder) => builder.build(),
        }
    }

    #[cfg(test)]
    pub(crate) fn add_value(
        &mut self,
        key: &[u8],
        value: &[u8],
        ts: Option<i64>,
        expire_ts: Option<i64>,
    ) -> bool {
        match self {
            Self::V1(builder) => builder.add_value(key, value, ts, expire_ts),
            Self::V2(builder) => builder.add_value(key, value, ts, expire_ts),
        }
    }
}

pub(crate) struct BlockBuilderWithStats {
    builder: BlockBuilder,
    stats: BlockStats,
}

impl BlockBuilderWithStats {
    pub(crate) fn new(builder: BlockBuilder) -> Self {
        Self {
            builder,
            stats: BlockStats::default(),
        }
    }

    pub(crate) fn would_fit(&self, entry: &crate::types::RowEntry) -> bool {
        self.builder.would_fit(entry)
    }

    pub(crate) fn add(&mut self, entry: crate::types::RowEntry) -> Result<bool, SlateDBError> {
        match &entry.value {
            crate::types::ValueDeletable::Value(_) => self.stats.num_puts += 1,
            crate::types::ValueDeletable::Merge(_) => self.stats.num_merges += 1,
            crate::types::ValueDeletable::Tombstone => self.stats.num_deletes += 1,
        }
        self.builder.add(entry)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.builder.is_empty()
    }

    pub(crate) fn into_parts(self) -> (BlockBuilder, BlockStats) {
        (self.builder, self.stats)
    }
}

pub(crate) const SIZEOF_U8: usize = size_of::<u8>();
pub(crate) const SIZEOF_U16: usize = size_of::<u16>();
pub(crate) const SIZEOF_U32: usize = size_of::<u32>();
pub(crate) const SIZEOF_U64: usize = size_of::<u64>();
pub(crate) const CHECKSUM_SIZE: usize = SIZEOF_U32;
pub(crate) const OFFSET_SIZE: usize = SIZEOF_U16;
pub(crate) const METADATA_OFFSET_SIZE: usize = SIZEOF_U64;
pub(crate) const VERSION_SIZE: usize = SIZEOF_U16;

/// Trait for transforming block data before storage and after retrieval.
///
/// This can be used to implement encryption, custom encoding, or other
/// transformations on block data. The transformer is applied after compression
/// on write and before decompression on read.
///
/// The trait is async to allow implementations to use `spawn_blocking` for
/// CPU-intensive operations (like encryption) or to call external services
/// (like a KMS).
///
/// # Example
///
/// ```
/// use async_trait::async_trait;
/// use bytes::Bytes;
/// use slatedb::{BlockTransformer, Error};
///
/// struct XorTransformer {
///     key: u8,
/// }
///
/// #[async_trait]
/// impl BlockTransformer for XorTransformer {
///     async fn encode(&self, data: Bytes) -> Result<Bytes, Error> {
///         let transformed: Vec<u8> = data.iter().map(|b| b ^ self.key).collect();
///         Ok(Bytes::from(transformed))
///     }
///
///     async fn decode(&self, data: Bytes) -> Result<Bytes, Error> {
///         self.encode(data).await
///     }
/// }
/// ```
#[async_trait]
pub trait BlockTransformer: Send + Sync {
    /// Encode (transform) block data before storage.
    async fn encode(&self, data: Bytes) -> Result<Bytes, crate::error::Error>;

    /// Decode (inverse transform) block data after retrieval.
    async fn decode(&self, data: Bytes) -> Result<Bytes, crate::error::Error>;
}

impl SsTableInfo {
    pub(crate) fn encode(info: &SsTableInfo, buf: &mut Vec<u8>, sst_codec: &dyn SsTableInfoCodec) {
        let data = &sst_codec.encode(info);
        buf.extend_from_slice(data);
        buf.put_u32(crc32fast::hash(data));
    }

    pub(crate) fn decode(
        raw_info: Bytes,
        sst_codec: &dyn SsTableInfoCodec,
    ) -> Result<SsTableInfo, SlateDBError> {
        if raw_info.len() <= 4 {
            return Err(SlateDBError::EmptyBlockMeta);
        }
        let data = raw_info.slice(..raw_info.len() - 4).clone();
        let checksum = raw_info.slice(raw_info.len() - 4..).get_u32();
        if checksum != crc32fast::hash(&data) {
            return Err(SlateDBError::ChecksumMismatch { path: None });
        }

        let info = sst_codec.decode(&data)?;
        Ok(info)
    }
}

/// Encoded SST data blocks
pub(crate) struct EncodedSsTableBlock {
    /// offset of the block within the SST
    pub(crate) offset: u64,
    /// uncompressed and untransformed block
    pub(crate) block: Arc<Block>,
    /// compressed and transformed block
    pub(crate) encoded_bytes: Bytes,
    /// first and last key of the block. None when the producer does not track
    /// keys (WAL blocks, whose index tracks sequence numbers instead)
    pub(crate) key_span: Option<(Bytes, Bytes)>,
}

impl EncodedSsTableBlock {
    pub(crate) fn len(&self) -> usize {
        self.encoded_bytes.len()
    }
}

/// Builder for encoding a single SST block with compression and transformation.
pub(crate) struct EncodedSsTableBlockBuilder {
    /// builder for data blocks
    block_builder: BlockBuilder,
    /// offset of the block within the SST
    offset: u64,
    /// first and last key of the block
    key_span: Option<(Bytes, Bytes)>,
    /// codec for compressing the data block
    compression_codec: Option<CompressionCodec>,
    /// transformer for transforming the data block (e.g. encryption)
    block_transformer: Option<Arc<dyn BlockTransformer>>,
}

impl EncodedSsTableBlockBuilder {
    pub(crate) fn new(block_builder: BlockBuilder, offset: u64) -> Self {
        Self {
            block_builder,
            offset,
            key_span: None,
            compression_codec: None,
            block_transformer: None,
        }
    }

    /// Sets the first and last key of the block
    pub(crate) fn with_key_span(mut self, first_key: Bytes, last_key: Bytes) -> Self {
        self.key_span = Some((first_key, last_key));
        self
    }

    /// Sets the compression codec for compressing the data block
    pub(crate) fn with_compression_codec(mut self, codec: CompressionCodec) -> Self {
        self.compression_codec = Some(codec);
        self
    }

    /// Sets the block transformer for transforming the data block
    pub(crate) fn with_block_transformer(mut self, transformer: Arc<dyn BlockTransformer>) -> Self {
        self.block_transformer = Some(transformer);
        self
    }

    pub(crate) async fn build(self) -> Result<EncodedSsTableBlock, SlateDBError> {
        let block = self.block_builder.build()?;
        let encoded_block = block.encode();
        let mut compressed_and_transformed_block = Vec::new();
        compress_and_transform(
            &mut compressed_and_transformed_block,
            encoded_block,
            self.compression_codec,
            self.block_transformer.as_ref(),
        )
        .await?;
        Ok(EncodedSsTableBlock {
            offset: self.offset,
            block: Arc::new(block),
            encoded_bytes: Bytes::from(compressed_and_transformed_block),
            key_span: self.key_span,
        })
    }
}

/// The encoded footer of an SSTable, containing filter, index, stats, info, and metadata.
pub(crate) struct EncodedSsTableFooter {
    pub(crate) info: SsTableInfo,
    pub(crate) index: SsTableIndexOwned,
    pub(crate) filters: Arc<[NamedFilter]>,
    #[allow(dead_code)]
    pub(crate) stats: Option<SstStats>,
    pub(crate) encoded_bytes: Bytes,
}

/// Builder for encoding the SSTable footer (filter, index, info, and metadata).
pub(crate) struct EncodedSsTableFooterBuilder<'a, 'b> {
    /// size of all data blocks in the SST
    blocks_size: u64,
    /// first entry in the SST, key for compacted data, sequence number for WAL
    first_entry: Option<Bytes>,
    /// last entry (key) in the SST for compacted data, None for WAL SSTs
    last_entry: Option<Bytes>,
    /// codec for compressing data blocks, index blocks, and filter blocks
    compression_codec: Option<CompressionCodec>,
    /// transformer for transforming data blocks, index blocks, and filter blocks
    block_transformer: Option<Arc<dyn BlockTransformer>>,
    /// codec for the SST info
    sst_info_codec: &'a dyn SsTableInfoCodec,
    /// builder for the index block
    index_builder: flatbuffers::FlatBufferBuilder<'b, DefaultAllocator>,
    /// metadata block
    block_meta: Vec<flatbuffers::WIPOffset<BlockMeta<'b>>>,
    /// filter blocks
    filters: Arc<[NamedFilter]>,
    /// stats block
    stats: Option<SstStats>,
    /// SST format version
    sst_format_version: u16,
    /// type of SST (Compacted or Wal)
    sst_type: SstType,
}

impl<'a, 'b> EncodedSsTableFooterBuilder<'a, 'b> {
    pub(crate) fn new(
        blocks_len: u64,
        sst_first_entry: Option<Bytes>,
        sst_last_entry: Option<Bytes>,
        sst_codec: &'a dyn SsTableInfoCodec,
        index_builder: flatbuffers::FlatBufferBuilder<'b, DefaultAllocator>,
        block_meta: Vec<flatbuffers::WIPOffset<BlockMeta<'b>>>,
        sst_format_version: u16,
        sst_type: SstType,
    ) -> Self {
        Self {
            blocks_size: blocks_len,
            first_entry: sst_first_entry,
            last_entry: sst_last_entry,
            compression_codec: None,
            block_transformer: None,
            sst_info_codec: sst_codec,
            index_builder,
            block_meta,
            filters: Arc::from([]),
            stats: None,
            sst_format_version,
            sst_type,
        }
    }

    /// Sets an optional compression codec to the footer.
    pub(crate) fn with_compression_codec(mut self, codec: CompressionCodec) -> Self {
        self.compression_codec = Some(codec);
        self
    }

    /// Sets an optional block transformer to the footer.
    pub(crate) fn with_block_transformer(mut self, transformer: Arc<dyn BlockTransformer>) -> Self {
        self.block_transformer = Some(transformer);
        self
    }

    /// Adds stats to the footer.
    pub(crate) fn with_stats(mut self, stats: SstStats) -> Self {
        self.stats = Some(stats);
        self
    }

    /// Adds filters to the footer as a composite block.
    pub(crate) fn with_filters(mut self, filters: Arc<[NamedFilter]>) -> Self {
        self.filters = filters;
        self
    }

    /// Builds the footer with the index, optional filter and optional stats.
    pub(crate) async fn build(mut self) -> Result<EncodedSsTableFooter, SlateDBError> {
        let mut buf = Vec::new();

        // Write composite filter block if any filters are present
        let filter_offset = self.blocks_size + buf.len() as u64;
        let filters = std::mem::take(&mut self.filters);
        let filter_len = if filters.is_empty() {
            0u64
        } else {
            // Encode composite filter block:
            // [num_filters: u16][name_len: u16][name_bytes...][data_len: u64][data_bytes...]...
            let mut composite = Vec::new();
            composite.put_u16(u16::try_from(filters.len()).expect("too many filters"));
            for nf in filters.iter() {
                let name_bytes = nf.name.as_bytes();
                composite
                    .put_u16(u16::try_from(name_bytes.len()).expect("filter policy name too long"));
                composite.put_slice(name_bytes);
                let mut encoded_data = Vec::new();
                nf.filter.encode(&mut encoded_data);
                composite.put_u64(encoded_data.len() as u64);
                composite.put_slice(&encoded_data);
            }
            compress_and_transform(
                &mut buf,
                Bytes::from(composite),
                self.compression_codec,
                self.block_transformer.as_ref(),
            )
            .await? as u64
        };

        let vector = self.index_builder.create_vector(&self.block_meta);
        let index_wip = SsTableIndex::create(
            &mut self.index_builder,
            &SsTableIndexArgs {
                block_meta: Some(vector),
            },
        );
        self.index_builder.finish(index_wip, None);
        let index_data = Bytes::from(self.index_builder.finished_data().to_vec());
        let index = SsTableIndexOwned::new(index_data.clone())?;
        let index_offset = self.blocks_size + buf.len() as u64;
        let index_len = compress_and_transform(
            &mut buf,
            index_data,
            self.compression_codec,
            self.block_transformer.as_ref(),
        )
        .await? as u64;

        // Write stats block if present
        let maybe_stats = self.stats.take();
        let (stats_offset, stats_len) = match &maybe_stats {
            Some(stats) => {
                let offset = self.blocks_size + buf.len() as u64;
                let len = compress_and_transform(
                    &mut buf,
                    stats.encode(),
                    self.compression_codec,
                    self.block_transformer.as_ref(),
                )
                .await? as u64;
                (offset, len)
            }
            None => (0u64, 0u64),
        };

        let meta_offset = self.blocks_size + buf.len() as u64;
        let filter_format = FilterFormat::Composite;
        let info = SsTableInfo {
            first_entry: self.first_entry,
            last_entry: self.last_entry,
            index_offset,
            index_len,
            filter_offset,
            filter_len,
            compression_codec: self.compression_codec,
            sst_type: self.sst_type,
            stats_offset,
            stats_len,
            filter_format,
        };
        SsTableInfo::encode(&info, &mut buf, self.sst_info_codec);

        buf.put_u64(meta_offset);
        buf.put_u16(self.sst_format_version);

        Ok(EncodedSsTableFooter {
            info,
            index,
            filters,
            stats: maybe_stats,
            encoded_bytes: Bytes::from(buf),
        })
    }
}

pub(crate) struct EncodedSsTable {
    pub(crate) format_version: u16,
    pub(crate) info: SsTableInfo,
    pub(crate) index: SsTableIndexOwned,
    pub(crate) filters: Arc<[NamedFilter]>,
    pub(crate) stats: Option<SstStats>,
    pub(crate) unconsumed_blocks: VecDeque<EncodedSsTableBlock>,
    pub(crate) footer: Bytes,
}

impl EncodedSsTable {
    pub(crate) fn remaining_len(&self) -> usize {
        self.unconsumed_blocks
            .iter()
            .map(|chunk| chunk.encoded_bytes.len())
            .sum::<usize>()
            + self.footer.len()
    }

    pub(crate) fn put_remaining<T: BufMut>(&self, buf: &mut T) {
        for chunk in self.unconsumed_blocks.iter() {
            buf.put_slice(chunk.encoded_bytes.as_ref())
        }
        buf.put_slice(self.footer.as_ref());
    }

    pub(crate) fn remaining_as_bytes(&self) -> Bytes {
        let total_size = self.remaining_len();
        let mut data = Vec::<u8>::with_capacity(total_size);
        self.put_remaining(&mut data);
        Bytes::from(data)
    }
}

/// Compresses and transforms the data using the specified
/// compression codec and the block transformer.
pub(crate) async fn compress_and_transform(
    buf: &mut Vec<u8>,
    data: Bytes,
    compression_codec: Option<CompressionCodec>,
    block_transformer: Option<&Arc<dyn BlockTransformer>>,
) -> Result<usize, SlateDBError> {
    let compressed = match compression_codec {
        None => data,
        Some(c) => {
            let compressed = compress(data, c)?;
            // Account for CPU-only compression work.
            tokio::task::coop::consume_budget().await;
            compressed
        }
    };
    let transformed = transform(compressed, block_transformer).await?;
    let checksum = crc32fast::hash(&transformed);
    let len = transformed.len() + CHECKSUM_SIZE;
    // Reserve the exact number of bytes we are about to append. Without this,
    // callers that pass an empty or undersized `Vec` (e.g.
    // `EncodedSsTableBlockBuilder::build`, or the footer builder which calls
    // this function multiple times on a shared `Vec`) pay `~log2(len)`
    // reallocs per invocation as the backing buffer geometrically doubles.
    // For workloads that build many blocks per SST (every write burst), those
    // transient over-provisioned buffers dominate peak `inuse_space`.
    buf.reserve(len);
    buf.put(transformed);
    buf.put_u32(checksum);
    Ok(len)
}

/// Compresses the data using the specified compression codec.
pub(crate) fn compress(
    #[allow(unused_variables)] data: Bytes,
    c: CompressionCodec,
) -> Result<Bytes, SlateDBError> {
    match c {
        #[cfg(feature = "snappy")]
        CompressionCodec::Snappy => {
            let compressed = snap::raw::Encoder::new()
                .compress_vec(&data)
                .map_err(|_| SlateDBError::BlockCompressionError)?;
            Ok(Bytes::from(compressed))
        }
        #[cfg(feature = "zlib")]
        CompressionCodec::Zlib => {
            let mut encoder =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            encoder
                .write_all(&data)
                .map_err(|_| SlateDBError::BlockCompressionError)?;
            Ok(Bytes::from(
                encoder
                    .finish()
                    .map_err(|_| SlateDBError::BlockCompressionError)?,
            ))
        }
        #[cfg(feature = "lz4")]
        CompressionCodec::Lz4 => {
            let compressed = lz4_flex::block::compress_prepend_size(&data);
            Ok(Bytes::from(compressed))
        }
        #[cfg(feature = "zstd")]
        CompressionCodec::Zstd => {
            let compressed =
                zstd::bulk::compress(&data, 3).map_err(|_| SlateDBError::BlockCompressionError)?;
            Ok(Bytes::from(compressed))
        }
    }
}

/// Transforms the data using the specified block transformer.
pub(crate) async fn transform(
    data: Bytes,
    block_transformer: Option<&Arc<dyn BlockTransformer>>,
) -> Result<Bytes, SlateDBError> {
    let transformed = match block_transformer {
        Some(t) => {
            let transformed = t
                .encode(data)
                .await
                .map_err(|_| SlateDBError::BlockTransformError)?;
            // Account for CPU-only transformation work.
            tokio::task::coop::consume_budget().await;
            transformed
        }
        None => data,
    };
    Ok(transformed)
}

pub(crate) type LengthOffsetAndVersion = (u64, u64, u16);

pub(crate) type TableInfoAndVersion = (SsTableInfo, u16);

#[derive(Clone)]
pub(crate) struct SsTableFormat {
    pub(crate) block_size: usize,
    pub(crate) min_filter_keys: u32,
    pub(crate) sst_codec: Box<dyn SsTableInfoCodec>,
    pub(crate) filter_policies: Vec<Arc<dyn FilterPolicy>>,
    pub(crate) compression_codec: Option<CompressionCodec>,
    pub(crate) block_transformer: Option<Arc<dyn BlockTransformer>>,
    pub(crate) block_format: Option<crate::sst_builder::BlockFormat>,
}

impl Default for SsTableFormat {
    fn default() -> Self {
        Self {
            block_size: 4096,
            min_filter_keys: 0,
            sst_codec: Box::new(FlatBufferSsTableInfoCodec {}),
            filter_policies: vec![Arc::new(BloomFilterPolicy::new(10))],
            compression_codec: None,
            block_transformer: None,
            block_format: None,
        }
    }
}

impl SsTableFormat {
    async fn read_length_and_metadata_offset_and_version(
        &self,
        obj: &impl ReadOnlyBlob,
    ) -> Result<LengthOffsetAndVersion, SlateDBError> {
        let obj_len = obj.len().await?;
        if obj_len <= NUM_FOOTER_BYTES_LONG {
            return Err(SlateDBError::EmptySSTable);
        }
        let header = obj
            .read_range((obj_len - NUM_FOOTER_BYTES_LONG)..obj_len)
            .await?;
        assert_eq!(header.len(), NUM_FOOTER_BYTES);

        let version = header.slice(8..NUM_FOOTER_BYTES).get_u16();
        let sst_metadata_offset = header.slice(0..8).get_u64();
        Ok((obj_len, sst_metadata_offset, version))
    }

    fn validate_version(&self, version: u16) -> Result<(), SlateDBError> {
        if !is_supported_version(version) {
            return Err(SlateDBError::InvalidVersion {
                format_name: "SST",
                supported_versions: vec![SST_FORMAT_VERSION, SST_FORMAT_VERSION_V2],
                actual_version: version,
            });
        }
        Ok(())
    }

    pub(crate) async fn read_info_and_version(
        &self,
        obj: &impl ReadOnlyBlob,
    ) -> Result<TableInfoAndVersion, SlateDBError> {
        let (obj_len, sst_metadata_offset, version) = self
            .read_length_and_metadata_offset_and_version(obj)
            .await?;
        self.validate_version(version)?;
        let sst_metadata_bytes = obj
            .read_range(sst_metadata_offset..obj_len - NUM_FOOTER_BYTES_LONG)
            .await?;
        SsTableInfo::decode(sst_metadata_bytes, &*self.sst_codec).map(|info| (info, version))
    }

    pub(crate) async fn read_filters(
        &self,
        info: &SsTableInfo,
        obj: &impl ReadOnlyBlob,
    ) -> Result<Arc<[NamedFilter]>, SlateDBError> {
        if self.filter_policies.is_empty() || info.filter_len == 0 {
            return Ok(Arc::from([]));
        }
        let filter_end = info.filter_offset + info.filter_len;
        let filter_offset_range = info.filter_offset..filter_end;
        let filter_bytes = obj.read_range(filter_offset_range).await?;
        let compression_codec = info.compression_codec;
        self.decode_filters(info, filter_bytes, compression_codec)
            .await
    }

    async fn decode_filters(
        &self,
        info: &SsTableInfo,
        bytes: Bytes,
        compression_codec: Option<CompressionCodec>,
    ) -> Result<Arc<[NamedFilter]>, SlateDBError> {
        let filter_bytes = self.validate_checksum(bytes)?;

        let untransformed_bytes = match &self.block_transformer {
            Some(t) => t
                .decode(filter_bytes)
                .await
                .map_err(|_| SlateDBError::BlockTransformError)?,
            None => filter_bytes,
        };
        let decompressed_bytes = match compression_codec {
            Some(c) => Self::decompress(untransformed_bytes, c)?,
            None => untransformed_bytes,
        };

        match info.filter_format {
            FilterFormat::Legacy => {
                // Legacy pre-composite SST: raw filter bytes are decoded
                // against the bloom filter policy if one is configured.
                let legacy: Vec<NamedFilter> = self
                    .filter_policies
                    .iter()
                    .find(|p| p.name() == BloomFilterPolicy::NAME)
                    .map(|policy| NamedFilter {
                        name: policy.name().to_string(),
                        filter: policy.decode(&decompressed_bytes),
                    })
                    .into_iter()
                    .collect();
                Ok(legacy.into())
            }
            FilterFormat::Composite => self.decode_composite_filter_block(&decompressed_bytes),
        }
    }

    /// Decodes a composite filter block.
    ///
    /// Format: `[num_filters: u16][name_len: u16][name_bytes...][data_len: u64][data_bytes...]...`
    ///
    /// Each filter's name is matched against the configured `filter_policies`.
    /// Filters whose policy is not found are dropped to support filter migration.
    fn decode_composite_filter_block(
        &self,
        data: &[u8],
    ) -> Result<Arc<[NamedFilter]>, SlateDBError> {
        let mut cursor = data;
        if cursor.len() < 2 {
            return Err(SlateDBError::InvalidFilterBlock);
        }
        let num_filters = cursor.get_u16() as usize;
        let mut filters: Vec<NamedFilter> = Vec::with_capacity(num_filters);

        for _ in 0..num_filters {
            if cursor.len() < 2 {
                return Err(SlateDBError::InvalidFilterBlock);
            }
            let name_len = cursor.get_u16() as usize;
            if cursor.len() < name_len {
                return Err(SlateDBError::InvalidFilterBlock);
            }
            let name = std::str::from_utf8(&cursor[..name_len])
                .map_err(|_| SlateDBError::InvalidFilterBlock)?;
            cursor.advance(name_len);

            if cursor.len() < 8 {
                return Err(SlateDBError::InvalidFilterBlock);
            }
            let data_len =
                usize::try_from(cursor.get_u64()).map_err(|_| SlateDBError::InvalidFilterBlock)?;
            if cursor.len() < data_len {
                return Err(SlateDBError::InvalidFilterBlock);
            }
            let filter_data = &cursor[..data_len];
            cursor.advance(data_len);

            match self.filter_policies.iter().find(|p| p.name() == name) {
                Some(policy) => filters.push(NamedFilter {
                    name: policy.name().to_string(),
                    filter: policy.decode(filter_data),
                }),
                None => {
                    warn!("unknown filter policy '{}' in SST, skipping", name);
                }
            }
        }

        Ok(filters.into())
    }

    pub(crate) async fn read_index(
        &self,
        info: &SsTableInfo,
        obj: &impl ReadOnlyBlob,
    ) -> Result<SsTableIndexOwned, SlateDBError> {
        let index_off = info.index_offset;
        let index_end = index_off + info.index_len;
        let index_bytes = obj.read_range(index_off..index_end).await?;
        let compression_codec = info.compression_codec;
        self.decode_index(index_bytes, compression_codec).await
    }

    #[cfg(test)]
    pub(crate) async fn read_index_raw(
        &self,
        info: &SsTableInfo,
        sst_bytes: &Bytes,
    ) -> Result<SsTableIndexOwned, SlateDBError> {
        let index_off = info.index_offset as usize;
        let index_end = index_off + info.index_len as usize;
        let index_bytes: Bytes = sst_bytes.slice(index_off..index_end);
        let compression_codec = info.compression_codec;
        self.decode_index(index_bytes, compression_codec).await
    }

    async fn decode_index(
        &self,
        bytes: Bytes,
        compression_codec: Option<CompressionCodec>,
    ) -> Result<SsTableIndexOwned, SlateDBError> {
        let index_bytes = self.validate_checksum(bytes)?;

        let untransformed_bytes = match &self.block_transformer {
            Some(t) => t
                .decode(index_bytes)
                .await
                .map_err(|_| SlateDBError::BlockTransformError)?,
            None => index_bytes,
        };
        let decompressed_bytes = match compression_codec {
            Some(c) => Self::decompress(untransformed_bytes, c)?,
            None => untransformed_bytes,
        };

        Ok(SsTableIndexOwned::new(decompressed_bytes)?)
    }

    pub(crate) async fn read_stats(
        &self,
        info: &SsTableInfo,
        obj: &impl ReadOnlyBlob,
    ) -> Result<Option<SstStats>, SlateDBError> {
        if info.stats_len == 0 {
            return Ok(None);
        }
        let stats_end = info.stats_offset + info.stats_len;
        let stats_bytes = obj.read_range(info.stats_offset..stats_end).await?;
        let compression_codec = info.compression_codec;
        Ok(Some(
            self.decode_stats(stats_bytes, compression_codec).await?,
        ))
    }

    #[allow(dead_code)]
    async fn decode_stats(
        &self,
        bytes: Bytes,
        compression_codec: Option<CompressionCodec>,
    ) -> Result<SstStats, SlateDBError> {
        let stats_bytes = self.validate_checksum(bytes)?;
        let untransformed_bytes = match &self.block_transformer {
            Some(t) => t
                .decode(stats_bytes)
                .await
                .map_err(|_| SlateDBError::BlockTransformError)?,
            None => stats_bytes,
        };
        let decompressed_bytes = match compression_codec {
            Some(c) => Self::decompress(untransformed_bytes, c)?,
            None => untransformed_bytes,
        };
        SstStats::decode(decompressed_bytes)
    }

    /// Decompresses the compressed data using the specified compression codec.
    fn decompress(
        #[allow(unused_variables)] compressed_data: Bytes,
        compression_option: CompressionCodec,
    ) -> Result<Bytes, SlateDBError> {
        match compression_option {
            #[cfg(feature = "snappy")]
            CompressionCodec::Snappy => Ok(Bytes::from(
                snap::raw::Decoder::new()
                    .decompress_vec(&compressed_data)
                    .map_err(|_| SlateDBError::BlockDecompressionError)?,
            )),
            #[cfg(feature = "zlib")]
            CompressionCodec::Zlib => {
                let mut decoder = flate2::read::ZlibDecoder::new(&compressed_data[..]);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|_| SlateDBError::BlockDecompressionError)?;
                Ok(Bytes::from(decompressed))
            }
            #[cfg(feature = "lz4")]
            CompressionCodec::Lz4 => {
                let decompressed = lz4_flex::block::decompress_size_prepended(&compressed_data)
                    .map_err(|_| SlateDBError::BlockDecompressionError)?;
                Ok(Bytes::from(decompressed))
            }
            #[cfg(feature = "zstd")]
            CompressionCodec::Zstd => {
                let decompressed = zstd::stream::decode_all(&compressed_data[..])
                    .map_err(|_| SlateDBError::BlockDecompressionError)?;
                Ok(Bytes::from(decompressed))
            }
        }
    }

    pub(crate) fn block_range(
        &self,
        blocks: Range<usize>,
        info: &SsTableInfo,
        index: &SsTableIndex,
    ) -> Range<u64> {
        let mut end_offset = if info.filter_len > 0 {
            info.filter_offset
        } else {
            info.index_offset
        };
        if blocks.end < index.block_meta().len() {
            let next_block_meta = index.block_meta().get(blocks.end);
            end_offset = next_block_meta.offset();
        }
        let start_offset = index.block_meta().get(blocks.start).offset();
        start_offset..end_offset
    }

    pub(crate) async fn read_blocks(
        &self,
        info: &SsTableInfo,
        index_owned: &SsTableIndexOwned,
        blocks: Range<usize>,
        obj: &impl ReadOnlyBlob,
    ) -> Result<VecDeque<Block>, SlateDBError> {
        let index = index_owned.borrow();
        assert!(blocks.start <= blocks.end);
        assert!(blocks.end <= index.block_meta().len());
        if blocks.start == blocks.end {
            return Ok(VecDeque::new());
        }
        let range = self.block_range(blocks.clone(), info, &index);
        let start_range = range.start;
        let bytes: Bytes = obj.read_range(range).await?;
        let compression_codec = info.compression_codec;
        let decode_futures: Vec<_> = blocks
            .map(|block| {
                let block_meta = index.block_meta().get(block);
                let block_bytes_start = usize::try_from(block_meta.offset() - start_range).expect(
                    "attempted to read byte data with size \
                        larger than 32 bits on a 32-bit system",
                );
                let block_bytes = if block == index.block_meta().len() - 1 {
                    bytes.slice(block_bytes_start..)
                } else {
                    let next_block_meta = index.block_meta().get(block + 1);
                    let block_bytes_end = usize::try_from(next_block_meta.offset() - start_range)
                        .expect(
                            "attempted to read byte data with size \
                            larger than 32 bits on a 32-bit system",
                        );
                    bytes.slice(block_bytes_start..block_bytes_end)
                };
                self.decode_block(block_bytes, compression_codec)
            })
            .collect();
        let decoded_blocks = try_join_all(decode_futures).await?;
        Ok(VecDeque::from(decoded_blocks))
    }

    async fn decode_block(
        &self,
        bytes: Bytes,
        compression_codec: Option<CompressionCodec>,
    ) -> Result<Block, SlateDBError> {
        let block_bytes = self.validate_checksum(bytes)?;
        let untransformed_bytes = match &self.block_transformer {
            Some(t) => t
                .decode(block_bytes)
                .await
                .map_err(|_| SlateDBError::BlockTransformError)?,
            None => block_bytes,
        };
        let decompressed_bytes = match compression_codec {
            Some(c) => Self::decompress(untransformed_bytes, c)?,
            None => untransformed_bytes,
        };

        Ok(Block::decode(decompressed_bytes))
    }

    pub(crate) async fn read_block(
        &self,
        info: &SsTableInfo,
        index: &SsTableIndexOwned,
        block: usize,
        obj: &impl ReadOnlyBlob,
    ) -> Result<Block, SlateDBError> {
        let mut blocks = self.read_blocks(info, index, block..block + 1, obj).await?;
        Ok(blocks.pop_front().expect("expected a block to be returned"))
    }

    #[cfg(test)]
    pub(crate) async fn read_block_raw(
        &self,
        info: &SsTableInfo,
        index_owned: &SsTableIndexOwned,
        block: usize,
        sst_bytes: &Bytes,
    ) -> Result<Block, SlateDBError> {
        let index = index_owned.borrow();
        let range = self.block_range(block..block + 1, info, &index);
        let range = range.start as usize..range.end as usize;
        let bytes: Bytes = sst_bytes.slice(range);
        let compression_codec = info.compression_codec;
        self.decode_block(bytes, compression_codec).await
    }

    /// validate checksum and return the actual data bytes
    pub(crate) fn validate_checksum(&self, bytes: Bytes) -> Result<Bytes, SlateDBError> {
        let data_bytes = bytes.slice(..bytes.len() - CHECKSUM_SIZE);
        let mut checksum_bytes = bytes.slice(bytes.len() - CHECKSUM_SIZE..);
        let checksum = crc32fast::hash(&data_bytes);
        let stored_checksum = checksum_bytes.get_u32();
        if checksum != stored_checksum {
            return Err(SlateDBError::ChecksumMismatch { path: None });
        }
        Ok(data_bytes)
    }

    /// The function estimates the size of the SST (Sorted String Table) without considering compression effects.
    pub(crate) fn estimate_encoded_size_compacted(
        &self,
        entry_num: usize,
        estimated_entries_size: usize,
    ) -> usize {
        if entry_num == 0 {
            return 0;
        }
        let guess_at_average_first_key_size_bytes = 12usize;
        let (entries_size_encoded, number_of_blocks) = self
            .estimate_entry_size_encoded_and_number_of_blocks(entry_num, estimated_entries_size);

        let mut ans = self.estimate_encoded_size_data_index_metadata(
            entry_num,
            entries_size_encoded,
            number_of_blocks,
            guess_at_average_first_key_size_bytes,
        );
        ans += self.estimate_encoded_size_filter(entry_num);

        ans += self.estimate_encoded_size_stats(number_of_blocks);

        ans
    }

    pub(crate) fn estimate_encoded_size_wal(
        &self,
        entry_num: usize,
        estimated_entries_size: usize,
    ) -> usize {
        if entry_num == 0 {
            return 0;
        }
        let (entries_size_encoded, number_of_blocks) = self
            .estimate_entry_size_encoded_and_number_of_blocks(entry_num, estimated_entries_size);

        self.estimate_encoded_size_data_index_metadata(
            entry_num,
            entries_size_encoded,
            number_of_blocks,
            SEQNUM_SIZE,
        )
    }

    fn estimate_entry_size_encoded_and_number_of_blocks(
        &self,
        entry_num: usize,
        estimated_entries_size: usize,
    ) -> (usize, usize) {
        let entries_size_encoded =
            row::SstRowCodecV0::estimate_encoded_size(entry_num, estimated_entries_size);
        let number_of_blocks = usize::div_ceil(entries_size_encoded, self.block_size);

        (entries_size_encoded, number_of_blocks)
    }

    fn estimate_encoded_size_data_index_metadata(
        &self,
        entry_num: usize,
        entries_size_encoded: usize,
        number_of_blocks: usize,
        average_first_key_size: usize,
    ) -> usize {
        let mut ans =
            Block::estimate_encoded_size(entry_num, entries_size_encoded, number_of_blocks);

        // estimate sum of Index
        let guess_at_average_first_key_size_bytes = average_first_key_size;
        ans += number_of_blocks * (guess_at_average_first_key_size_bytes + OFFSET_SIZE)
            + CHECKSUM_SIZE;

        // estimate sum of Metadata (SsTableInfo)
        let first_entry_size = guess_at_average_first_key_size_bytes;
        let index_offset_length_size = 2 * SIZEOF_U64;
        let filter_offset_length_size = 2 * SIZEOF_U64;
        let compression_format_size = SIZEOF_U8;
        let sst_type_size = SIZEOF_U8;
        let last_entry_size = guess_at_average_first_key_size_bytes;
        let stats_offset_length_size = 2 * SIZEOF_U64;
        let filter_format_size = SIZEOF_U8;
        ans += first_entry_size
            + index_offset_length_size
            + filter_offset_length_size
            + compression_format_size
            + sst_type_size
            + last_entry_size
            + stats_offset_length_size
            + filter_format_size
            + CHECKSUM_SIZE;

        ans += METADATA_OFFSET_SIZE + VERSION_SIZE;

        ans
    }

    fn estimate_encoded_size_filter(&self, entry_num: usize) -> usize {
        if entry_num >= self.min_filter_keys as usize && !self.filter_policies.is_empty() {
            // Composite block framing: [num_filters: u16]
            let mut size = 2;
            for p in &self.filter_policies {
                // Per-filter framing: [name_len: u16][name_bytes][data_len: u64]
                size += 2 + p.name().len() + 8;
                size += p.estimate_size(entry_num);
            }
            // One CRC32 checksum appended to the entire composite block by
            // compress_and_transform.
            size += CHECKSUM_SIZE;
            size
        } else {
            0
        }
    }

    fn estimate_encoded_size_stats(&self, number_of_blocks: usize) -> usize {
        let ops_stats = 3 * SIZEOF_U64;
        let key_value_stats = 2 * SIZEOF_U64;
        let ops_stats_per_block = 3 * SIZEOF_U16;

        ops_stats + key_value_stats + (number_of_blocks * ops_stats_per_block) + CHECKSUM_SIZE
    }
}
