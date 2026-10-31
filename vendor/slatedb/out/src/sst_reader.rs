//! Read-only, metadata-only API for inspecting compacted SST files.
//!
//! The main types are:
//!
//! - [`SstReader`]: opens compacted SSTs by ULID or from an existing handle.
//! - [`SstFile`]: one compacted SST with accessors for metadata, stats, and index.
//!
//! # Example
//!
//! ```
//! use slatedb::config::{FlushOptions, FlushType, PutOptions, WriteOptions};
//! use slatedb::object_store::memory::InMemory;
//! use slatedb::{Db, SstReader};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), slatedb::Error> {
//!     let object_store = Arc::new(InMemory::new());
//!     let path = "/sst_reader_example";
//!     let db = Db::open(path, object_store.clone()).await?;
//!
//!     db.put_with_options(
//!         b"k1",
//!         b"v1",
//!         &PutOptions::default(),
//!         &WriteOptions::default(),
//!     )
//!     .await?;
//!     db.flush_with_options(FlushOptions {
//!         flush_type: FlushType::MemTable,
//!     })
//!     .await?;
//!
//!     let manifest = db.manifest();
//!     let reader = SstReader::new(path, object_store, None, None);
//!
//!     // Inspect L0 SSTs
//!     for view in manifest.l0() {
//!         let sst_file = reader.open_with_handle(view.sst.clone())?;
//!         if let Some(stats) = sst_file.stats().await? {
//!             let _ = stats.num_puts;
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

use bytes::Bytes;
use object_store::path::Path;
use object_store::ObjectStore;
use ulid::Ulid;

use crate::block_cache_policy::BlockCachePolicy;
use crate::block_iterator::DataBlockIterator;
use crate::db_cache::DbCache;
use crate::db_state::{SsTableHandle, SsTableId, SsTableInfo};
use crate::flatbuffer_types::SsTableIndexOwned;
use crate::format::sst::{BlockTransformer, SsTableFormat};
use crate::iter::IterationOrder;
use crate::partitioned_keyspace::{partition_point, RangePartitionedKeySpace};
use crate::sst_stats::SstStats;
use crate::tablestore::{TableStore, TableStoreKind};
use crate::types::RowEntry;
use crate::IdentifiedObjectMetadata;

/// Opens compacted SST files for read-only inspection.
///
/// `SstReader` wraps the read-only functionality needed to open individual
/// SSTs and inspect their metadata, stats, and index data. SST paths are
/// resolved as `{root}/compacted/{ulid}.sst`.
pub struct SstReader {
    table_store: Arc<TableStore>,
}

impl SstReader {
    /// Creates a new SST reader for the database at the given path.
    ///
    /// The `object_store` should point at the same store used by the `Db`.
    /// An optional `DbCache` can be provided for in-memory caching of index
    /// and stats blocks. If the database was opened with a `BlockTransformer`
    /// (e.g. for encryption), the same transformer must be provided here.
    pub fn new<P: Into<Path>>(
        root_path: P,
        object_store: Arc<dyn ObjectStore>,
        cache: Option<Arc<dyn DbCache>>,
        block_transformer: Option<Arc<dyn BlockTransformer>>,
    ) -> Self {
        let sst_format = SsTableFormat {
            block_transformer,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store,
            sst_format,
            root_path.into(),
            cache,
            TableStoreKind::Reader,
            BlockCachePolicy::default(),
        ));
        Self { table_store }
    }

    /// Opens a compacted SST by its ULID.
    ///
    /// This reads the SST footer and metadata from object storage.
    ///
    /// ## Errors
    ///
    /// Returns an error if the SST file does not exist (e.g. it was GC'd)
    /// or if there is an issue reading from object storage.
    pub async fn open(&self, id: Ulid) -> Result<SstFile, crate::Error> {
        let handle = self
            .table_store
            .open_sst(&SsTableId::from(id), Some(Bytes::new()))
            .await?;
        Ok(SstFile {
            id,
            handle,
            table_store: Arc::clone(&self.table_store),
        })
    }

    /// Creates an `SstFile` from an existing `SsTableHandle` (no I/O needed).
    ///
    pub fn open_with_handle(&self, handle: SsTableHandle) -> Result<SstFile, crate::Error> {
        let id = handle.id.value();
        Ok(SstFile {
            id,
            handle,
            table_store: Arc::clone(&self.table_store),
        })
    }
}

/// A single compacted SST file with accessors for metadata, stats, and index.
pub struct SstFile {
    id: Ulid,
    handle: SsTableHandle,
    table_store: Arc<TableStore>,
}

/// A zero-copy view of an SST's block index.
///
/// The view owns a reference to the cached index data. Keys returned by its
/// accessors borrow directly from that data without allocation or copying.
#[derive(Clone)]
pub struct SstIndex {
    inner: Arc<SsTableIndexOwned>,
}

impl SstIndex {
    /// Returns the number of data blocks described by the index.
    pub fn len(&self) -> usize {
        self.inner.borrow().block_meta().len()
    }

    /// Returns whether the index contains no data blocks.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the block offset and first key at `index`.
    pub fn get(&self, index: usize) -> Option<(u64, &[u8])> {
        let block_meta = self.inner.borrow().block_meta();
        if index >= block_meta.len() {
            return None;
        }
        let meta = block_meta.get(index);
        Some((meta.offset(), meta.first_key().bytes()))
    }

    /// Iterates over block offsets and first keys in index order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (u64, &[u8])> + '_ {
        let block_meta = self.inner.borrow().block_meta();
        (0..block_meta.len()).map(move |index| {
            let meta = block_meta.get(index);
            (meta.offset(), meta.first_key().bytes())
        })
    }

    /// Returns the first index for which `pred` is false.
    ///
    /// The index first keys are sorted, so `pred` must return `true` for a
    /// contiguous prefix of the index, matching [`slice::partition_point`].
    pub fn partition_point<P>(&self, pred: P) -> usize
    where
        P: Fn(&[u8]) -> bool,
    {
        partition_point(self, pred)
    }
}

impl RangePartitionedKeySpace for SstIndex {
    fn partitions(&self) -> usize {
        self.len()
    }

    fn partition_first_key(&self, partition: usize) -> &[u8] {
        self.get(partition)
            .expect("partition index should be in range")
            .1
    }
}

impl SstFile {
    /// Returns the SST's ULID identifier.
    pub fn id(&self) -> Ulid {
        self.id
    }

    /// Returns the SST file metadata from `SsTableInfo`.
    ///
    /// This is available without any I/O since the info is cached in the handle.
    pub fn info(&self) -> &SsTableInfo {
        &self.handle.info
    }

    /// Returns object store metadata for this SST file.
    ///
    /// This performs one HEAD request to the object store.
    ///
    /// ## Errors
    ///
    /// Returns an error if the SST file does not exist or if there is an
    /// issue reading from object storage.
    pub async fn metadata(&self) -> Result<IdentifiedObjectMetadata<Ulid>, crate::Error> {
        let metadata = self
            .table_store
            .metadata(&self.handle.id, Some(Bytes::new()))
            .await
            .map_err(crate::Error::from)?;
        Ok(IdentifiedObjectMetadata::new(self.id, metadata))
    }

    /// Reads the stats block from object storage.
    ///
    /// Returns `None` for old SSTs that were written before the stats block
    /// was added (i.e. `stats_offset == 0 && stats_len == 0`).
    ///
    /// ## Errors
    ///
    /// Returns an error if there is an issue reading from object storage.
    pub async fn stats(&self) -> Result<Option<SstStats>, crate::Error> {
        self.table_store
            .read_stats(&self.handle, true, Some(Bytes::new()))
            .await
            .map_err(Into::into)
    }

    /// Returns a zero-copy view of the SST index block.
    ///
    /// The returned [`SstIndex`] contains one entry for each data block in the
    /// SST, in block order. Each entry contains the on-disk byte offset of
    /// the block and the first key stored in that block. The index keeps
    /// the cached index data alive, and keys returned by its accessors borrow
    /// directly from that data without allocation or copying.
    ///
    /// ## Errors
    ///
    /// Returns an error if there is an issue reading from object storage.
    pub async fn index(&self) -> Result<SstIndex, crate::Error> {
        let inner = self
            .table_store
            .read_index(
                &self.handle,
                true,
                Some(Bytes::new()),
                &crate::reader::ReadTrace::new(None),
                None,
            )
            .await?;
        Ok(SstIndex { inner })
    }

    /// Reads a single data block by its index and returns the decoded rows.
    ///
    /// The `block` parameter is the zero-based index of the block within the
    /// SST, parallel to the entries returned by [`SstFile::index`]. Each
    /// [`RowEntry`] carries the key, value (or tombstone/merge operand),
    /// sequence number, and optional timestamps as stored on disk — no MVCC
    /// filtering is applied.
    ///
    /// ## Errors
    ///
    /// Returns an error if `block` is out of range for this SST, or if there
    /// is an issue reading or decoding the block.
    pub async fn read_block(&self, block: usize) -> Result<Vec<RowEntry>, crate::Error> {
        let index = self
            .table_store
            .read_index(
                &self.handle,
                true,
                Some(Bytes::new()),
                &crate::reader::ReadTrace::new(None),
                None,
            )
            .await?;
        let num_blocks = index.borrow().block_meta().len();
        if block >= num_blocks {
            return Err(crate::Error::invalid(format!(
                "block index {block} out of range for SST with {num_blocks} block(s)"
            )));
        }
        let mut blocks = self
            .table_store
            .read_blocks_using_index(
                &self.handle,
                index,
                block..block + 1,
                true,
                Some(Bytes::new()),
            )
            .await?;
        let block = blocks
            .pop_front()
            .expect("read_blocks_using_index returned no block for a valid index");
        let mut iter =
            DataBlockIterator::new(block, self.handle.format_version, IterationOrder::Ascending)?;
        let mut rows = Vec::new();
        while let Some(row) = iter.next().await? {
            rows.push(row);
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FlushOptions, FlushType, PutOptions, SstBlockSize, WriteOptions};
    use crate::paths::PathResolver;
    use crate::test_utils::StringConcatMergeOperator;
    use crate::types::ValueDeletable;
    use crate::Db;
    use bytes::Bytes;
    use object_store::memory::InMemory;

    /// Helper: create a DB with 10 puts, 3 deletes, and 2 merges, flush to
    /// L0, and return the object store + path + manifest for inspection.
    async fn setup_db_with_l0() -> (
        Arc<dyn ObjectStore>,
        &'static str,
        crate::manifest::VersionedManifest,
    ) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_sst_reader";
        let db = Db::builder(path, object_store.clone())
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .with_sst_block_size(SstBlockSize::Other(64))
            .build()
            .await
            .unwrap();

        // 10 puts
        for i in 0..10u8 {
            db.put_with_options(
                &[b'k', i],
                &[b'v', i],
                &PutOptions::default(),
                &WriteOptions::default(),
            )
            .await
            .unwrap();
        }

        // 3 deletes
        for i in 10..13u8 {
            db.delete(&[b'k', i]).await.unwrap();
        }

        // 2 merges
        for i in 13..15u8 {
            db.merge(&[b'k', i], &[b'm', i]).await.unwrap();
        }

        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        let manifest = db.manifest();
        db.close().await.unwrap();
        (object_store, path, manifest)
    }

    #[tokio::test]
    async fn test_open_and_info() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        assert!(
            !manifest.manifest.core.tree.l0.is_empty(),
            "expected at least one L0 SST"
        );
        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open(view.sst.id.value()).await.unwrap();

        let info = sst_file.info();
        assert!(info.first_entry.is_some());
        assert!(info.index_offset > 0);
    }

    #[tokio::test]
    async fn test_open_with_handle() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();

        assert_eq!(sst_file.id(), view.sst.id.value());
        assert_eq!(sst_file.info(), &view.sst.info);
    }

    #[tokio::test]
    async fn test_stats() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();
        let stats = sst_file.stats().await.unwrap();

        let stats = stats.expect("expected stats block to be present");
        assert_eq!(stats.num_puts, 10);
        assert_eq!(stats.num_deletes, 3);
        assert_eq!(stats.num_merges, 2);
        assert_eq!(stats.num_rows(), 15);
        assert!(stats.raw_key_size > 0);
        assert!(stats.raw_val_size > 0);
    }

    #[tokio::test]
    async fn test_index() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();
        let index = sst_file.index().await.unwrap();

        assert!(!index.is_empty());
        // First index key should be <= the SST's first entry (it may be a
        // shortened separator key rather than the exact first key).
        if let Some(first_entry) = sst_file.info().first_entry.as_ref() {
            let (_, first_key) = index.get(0).expect("index should not be empty");
            assert!(first_key <= first_entry.as_ref());
        }
        // Offsets should be monotonically increasing
        let mut entries = index.iter();
        let mut previous_offset = entries.next().expect("index should not be empty").0;
        for (offset, _) in entries {
            assert!(previous_offset < offset);
            previous_offset = offset;
        }
        assert_eq!(index.get(index.len()), None);
    }

    #[tokio::test]
    async fn test_index_partition_point_matches_slice() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();
        let index = sst_file.index().await.unwrap();
        let owned = index
            .iter()
            .map(|(offset, first_key)| (offset, Bytes::copy_from_slice(first_key)))
            .collect::<Vec<_>>();

        for key in [b"".as_slice(), b"k00", b"k05", b"k99"] {
            assert_eq!(
                index.partition_point(|candidate| candidate < key),
                owned.partition_point(|(_, candidate)| candidate.as_ref() < key)
            );
            assert_eq!(
                index.partition_point(|candidate| candidate <= key),
                owned.partition_point(|(_, candidate)| candidate.as_ref() <= key)
            );
        }
    }

    #[tokio::test]
    async fn test_block_stats_parallel_to_index() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();

        let stats = sst_file
            .stats()
            .await
            .unwrap()
            .expect("expected stats block");
        let index = sst_file.index().await.unwrap();

        assert_eq!(
            stats.block_stats.len(),
            index.len(),
            "block_stats should be parallel to the index"
        );

        // Sum of per-block puts should equal aggregate puts
        let sum_puts: u64 = stats.block_stats.iter().map(|b| b.num_puts as u64).sum();
        assert_eq!(sum_puts, stats.num_puts);
    }

    #[tokio::test]
    async fn test_read_block_returns_all_kinds() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();
        let index = sst_file.index().await.unwrap();
        assert!(!index.is_empty());

        // Collect all rows across all blocks.
        let mut all_rows: Vec<RowEntry> = Vec::new();
        for i in 0..index.len() {
            let rows = sst_file.read_block(i).await.unwrap();
            assert!(!rows.is_empty(), "block {i} should have at least one row");
            // Rows within a block are in ascending key order.
            for window in rows.windows(2) {
                assert!(window[0].key <= window[1].key);
            }
            all_rows.extend(rows);
        }

        let puts = all_rows
            .iter()
            .filter(|r| matches!(r.value, ValueDeletable::Value(_)))
            .count();
        let merges = all_rows
            .iter()
            .filter(|r| matches!(r.value, ValueDeletable::Merge(_)))
            .count();
        let tombstones = all_rows
            .iter()
            .filter(|r| matches!(r.value, ValueDeletable::Tombstone))
            .count();
        assert_eq!(puts, 10);
        assert_eq!(merges, 2);
        assert_eq!(tombstones, 3);
    }

    #[tokio::test]
    async fn test_read_block_out_of_range() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();
        let num_blocks = sst_file.index().await.unwrap().len();

        let err = sst_file.read_block(num_blocks).await.unwrap_err();
        assert!(format!("{err}").contains("out of range"));
    }

    #[tokio::test]
    async fn test_read_block_first_key_matches_index() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();
        let index = sst_file.index().await.unwrap();

        // The index entry's first_key may be a shortened separator, so the
        // block's actual first key is always >= the index's separator key.
        for (i, (_, first_key)) in index.iter().enumerate() {
            let rows = sst_file.read_block(i).await.unwrap();
            assert!(rows[0].key >= *first_key);
        }
    }

    #[tokio::test]
    async fn test_metadata() {
        let (store, path, manifest) = setup_db_with_l0().await;
        let reader = SstReader::new(path, store, None, None);

        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();
        let metadata = sst_file.metadata().await.unwrap();

        assert_eq!(metadata.id, view.sst.id.value());
        assert!(metadata.metadata.size > 0);
        assert_eq!(
            metadata.metadata.location,
            PathResolver::from_root(path).sst_path(&view.sst.id)
        );
    }

    #[tokio::test]
    async fn test_open_missing_sst() {
        let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let reader = SstReader::new("/nonexistent", store, None, None);

        let result = reader.open(Ulid::new()).await;
        assert!(result.is_err());
    }

    struct XorTransformer {
        key: u8,
    }

    #[async_trait::async_trait]
    impl BlockTransformer for XorTransformer {
        async fn encode(&self, data: Bytes) -> Result<Bytes, crate::Error> {
            let transformed: Vec<u8> = data.iter().map(|b| b ^ self.key).collect();
            Ok(Bytes::from(transformed))
        }

        async fn decode(&self, data: Bytes) -> Result<Bytes, crate::Error> {
            self.encode(data).await
        }
    }

    #[tokio::test]
    async fn test_stats_with_block_transformer() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_sst_reader_transformer";
        let transformer: Arc<dyn BlockTransformer> = Arc::new(XorTransformer { key: 0x42 });

        // Write data with a block transformer
        let db = Db::builder(path, object_store.clone())
            .with_block_transformer(transformer.clone())
            .build()
            .await
            .unwrap();

        for i in 0..5u8 {
            db.put_with_options(
                &[b'k', i],
                &[b'v', i],
                &PutOptions::default(),
                &WriteOptions::default(),
            )
            .await
            .unwrap();
        }
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        let manifest = db.manifest();
        db.close().await.unwrap();

        // Reading with the correct transformer should succeed
        let reader = SstReader::new(path, object_store.clone(), None, Some(transformer));
        let view = &manifest.manifest.core.tree.l0[0];
        let sst_file = reader.open_with_handle(view.sst.clone()).unwrap();
        let stats = sst_file.stats().await.unwrap().expect("expected stats");
        assert_eq!(stats.num_puts, 5);

        // Reading without the transformer should fail
        let reader_no_transformer = SstReader::new(path, object_store, None, None);
        let sst_file = reader_no_transformer
            .open_with_handle(view.sst.clone())
            .unwrap();
        assert!(sst_file.stats().await.is_err());
    }
}
