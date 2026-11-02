//! This module provides a read-only API for inspecting WAL SST files that were
//! flushed by SlateDB writers.
//!
//! The API has the following main types:
//!
//! - [`WalReader`]: opens a WAL namespace and lists WAL files.
//! - [`WalFile`]: one WAL file (`id`) plus accessors for metadata and contents.
//! - [`IdentifiedObjectMetadata`]: object-store metadata for one WAL file plus its WAL id.
//! - [`WalFileIterator`]: entry-level iterator over a WAL file.
//!
//! WAL files returned by [`WalReader::list`] are ordered by WAL ID in ascending
//! order. Iterating each file with [`WalFile::iterator`] yields [`RowEntry`]
//! values in the order they are stored in that WAL SST.
//!
//! `WalFileIterator` intentionally exposes `next`, not `next`. This keeps
//! the API at [`RowEntry`] level and preserves tombstones and merge rows exactly
//! as written to the WAL.
//!
//! # Listing costs and polling strategy
//!
//! The `list()` API can become expensive when WAL retention is high or GC is not
//! keeping up. If the GC is not running, listings can grow without bound. Even
//! with GC, CDC often needs higher retention. Retaining WAL files for just
//! 1 hour can yield tens of thousands of files, which is expensive to list in
//! both cost and time.
//!
//! If you plan to poll frequently, use `list()` once to establish a baseline,
//! then poll with `WalReader::get(latest_id + 1)` to avoid repeated large
//! listings.
//!
//! # Example
//!
//! ```
//! use slatedb::config::{FlushOptions, FlushType, PutOptions, WriteOptions};
//! use slatedb::object_store::memory::InMemory;
//! use slatedb::{Db, Error, WalReader};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let object_store = Arc::new(InMemory::new());
//!     let path = "/wal_reader_example";
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
//!         flush_type: FlushType::Wal,
//!     })
//!     .await?;
//!
//!     let reader = WalReader::new(path, object_store);
//!     for wal_file in reader.list(..).await? {
//!         let mut iter = wal_file.iterator().await?;
//!         while let Some(entry) = iter.next().await? {
//!             let _ = entry;
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

use object_store::path::Path;
use object_store::ObjectStore;

use crate::format::sst::SsTableFormat;
use crate::iter::{EmptyIterator, RowEntryIterator};
use crate::tablestore::TableStoreKind;
use crate::types::RowEntry;
use crate::wal::slatedb::sst_iterator::{WalSstIterator, WalSstIteratorOptions};
use crate::wal::slatedb::store::{WalFileId, WalTableStore};
use crate::IdentifiedObjectMetadata;

/// Iterator over entries in a WAL file.
pub struct WalFileIterator {
    iter: WalFileIteratorInner,
}

#[allow(clippy::large_enum_variant)]
enum WalFileIteratorInner {
    Empty(EmptyIterator),
    Sst(WalSstIterator),
}

impl WalFileIterator {
    fn new(iter: WalFileIteratorInner) -> Self {
        Self { iter }
    }

    /// Returns the next entry in the WAL file.
    pub async fn next(&mut self) -> Result<Option<RowEntry>, crate::Error> {
        match &mut self.iter {
            WalFileIteratorInner::Empty(iter) => iter.next().await,
            WalFileIteratorInner::Sst(iter) => iter.next().await,
        }
        .map_err(Into::into)
    }
}

/// Represents a single WAL file stored in object storage and provides methods
/// to inspect and read its contents.
pub struct WalFile {
    /// The unique identifier for this WAL file. Corresponds to the SST filename without
    /// the extension. For example, file `000123.sst` would have id `123`.
    pub id: u64,

    wal_store: Arc<WalTableStore>,
}

impl WalFile {
    /// Returns metadata for this WAL file.
    ///
    /// ## Errors
    ///
    /// Raises an error if the metadata could not be read from object storage. This can
    /// happen if the file was deleted after listing or if there was an issue with the
    /// object store. If the file is missing, a [`crate::Error`] with
    /// [`crate::ErrorKind::Data`] is returned, and its source contains an
    /// `object_store::Error::NotFound`.
    pub async fn metadata(&self) -> Result<IdentifiedObjectMetadata<u64>, crate::Error> {
        let metadata = self
            .wal_store
            .metadata(WalFileId::from(self.id))
            .await
            .map_err(crate::Error::from)?;
        Ok(IdentifiedObjectMetadata::new(self.id, metadata))
    }

    /// Returns an iterator over `RowEntry`s in this WAL file. Raises an error if the
    /// WAL file could not be read.
    ///
    /// ## Errors
    ///
    /// Raises an error if the data could not be read from object storage. This can
    /// happen if the file was deleted after listing or if there was an issue with the
    /// object store. If the file is missing, a [`crate::Error`] with
    /// [`crate::ErrorKind::Data`] is returned, and its source contains an
    /// `object_store::Error::NotFound`.
    pub async fn iterator(&self) -> Result<WalFileIterator, crate::Error> {
        let sst = match self.wal_store.open_sst(WalFileId::from(self.id)).await {
            Ok(sst) => sst,
            Err(crate::error::SlateDBError::EmptySSTable) => {
                // Zero-byte WAL files are fencing markers, not corrupt WALs.
                return Ok(WalFileIterator::new(WalFileIteratorInner::Empty(
                    EmptyIterator::new(),
                )));
            }
            Err(err) => return Err(err.into()),
        };
        let iter = WalSstIterator::new(
            sst,
            Arc::clone(&self.wal_store),
            WalSstIteratorOptions {
                // Optimize for throughput. Go for 256MiB per fetch.
                target_bytes_to_fetch: 256 * 1024 * 1024,
                ..Default::default()
            },
        )
        .await?;
        Ok(WalFileIterator::new(WalFileIteratorInner::Sst(iter)))
    }

    /// Returns the WAL ID immediately following this file's ID.
    pub fn next_id(&self) -> u64 {
        self.id + 1
    }

    /// Returns a [`WalFile`] handle for the next WAL file after this one.
    ///
    /// This does not check whether the next WAL file actually exists in object storage.
    pub fn next_file(&self) -> Self {
        Self {
            id: self.next_id(),
            wal_store: Arc::clone(&self.wal_store),
        }
    }
}

/// Reads WAL files in object storage for a specific database.
pub struct WalReader {
    wal_store: Arc<WalTableStore>,
}

impl WalReader {
    /// Creates a new WAL reader for the database at the given path.
    ///
    /// If the database was configured with a separate WAL object store, pass that
    /// object store here.
    pub fn new<P: Into<Path>>(path: P, object_store: Arc<dyn ObjectStore>) -> Self {
        let sst_format = SsTableFormat::default();
        let wal_store = Arc::new(WalTableStore::new(
            object_store,
            sst_format,
            path.into(),
            TableStoreKind::Reader,
        ));
        Self { wal_store }
    }

    /// Lists WAL files in ascending order by their ID within the specified range.
    /// If `range` is unbounded, all WAL files are returned.
    pub async fn list<R: RangeBounds<u64>>(&self, range: R) -> Result<Vec<WalFile>, crate::Error> {
        let start = match range.start_bound() {
            Bound::Included(id) => Bound::Included(WalFileId::from(*id)),
            Bound::Excluded(id) => Bound::Excluded(WalFileId::from(*id)),
            Bound::Unbounded => Bound::Unbounded,
        };
        let end = match range.end_bound() {
            Bound::Included(id) => Bound::Included(WalFileId::from(*id)),
            Bound::Excluded(id) => Bound::Excluded(WalFileId::from(*id)),
            Bound::Unbounded => Bound::Unbounded,
        };
        let result = self.wal_store.list_wal_ssts((start, end)).await;
        Ok(result?
            .into_iter()
            .map(|wal_file| WalFile {
                id: wal_file.id.value(),
                wal_store: Arc::clone(&self.wal_store),
            })
            .collect())
    }

    /// Creates a [`WalFile`] handle for a WAL ID.
    pub fn get(&self, id: u64) -> WalFile {
        WalFile {
            id,
            wal_store: Arc::clone(&self.wal_store),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::config::{FlushOptions, FlushType, PutOptions, WriteOptions};
    use crate::test_utils::StringConcatMergeOperator;
    use crate::types::ValueDeletable;
    use crate::Db;
    use object_store::memory::InMemory;
    use object_store::ObjectStoreExt;

    fn has_not_found_object_store_source(err: &crate::Error) -> bool {
        err.source()
            .and_then(|source| source.downcast_ref::<object_store::Error>())
            .is_some_and(|source| matches!(source, object_store::Error::NotFound { .. }))
            || err
                .source()
                .and_then(|source| source.downcast_ref::<Arc<object_store::Error>>())
                .is_some_and(|source| {
                    matches!(source.as_ref(), object_store::Error::NotFound { .. })
                })
    }

    #[tokio::test]
    async fn test_list_and_iterator() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_reader";
        let db = Db::open(path, main_store.clone()).await.unwrap();
        db.put_with_options(
            b"k2",
            b"v2",
            &PutOptions::default(),
            &WriteOptions::default(),
        )
        .await
        .unwrap();
        db.put_with_options(
            b"k1",
            b"v1",
            &PutOptions::default(),
            &WriteOptions::default(),
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        let wal_reader = WalReader::new(path, main_store.clone());
        let wal_files = wal_reader.list(..).await.unwrap();
        assert!(!wal_files.is_empty());
        let mut rows = Vec::new();
        for wal_file in wal_files {
            let mut iter = wal_file.iterator().await.unwrap();
            while let Some(entry) = iter.next().await.unwrap() {
                rows.push(entry);
            }
        }
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].key.as_ref(), b"k2");
        assert!(matches!(
            &rows[0].value,
            ValueDeletable::Value(value) if value.as_ref() == b"v2"
        ));
        assert_eq!(rows[1].key.as_ref(), b"k1");
        assert!(matches!(
            &rows[1].value,
            ValueDeletable::Value(value) if value.as_ref() == b"v1"
        ));
    }

    #[tokio::test]
    async fn test_reads_from_wal_object_store() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let wal_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_store";
        let db = Db::builder(path, main_store.clone())
            .with_wal_object_store(wal_store.clone())
            .build()
            .await
            .unwrap();
        db.put_with_options(
            b"k1",
            b"v1",
            &PutOptions::default(),
            &WriteOptions::default(),
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        let wal_reader = WalReader::new(path, wal_store.clone());
        let wal_files = wal_reader.list(..).await.unwrap();
        assert!(!wal_files.is_empty());
        let mut rows = Vec::new();
        for wal_file in wal_files {
            let mut iter = wal_file.iterator().await.unwrap();
            while let Some(entry) = iter.next().await.unwrap() {
                rows.push(entry);
            }
        }
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key.as_ref(), b"k1");
        assert!(matches!(
            &rows[0].value,
            ValueDeletable::Value(value) if value.as_ref() == b"v1"
        ));
    }

    #[tokio::test]
    async fn test_wal_file_metadata_matches_object_store_metadata() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_reader_metadata";
        let db = Db::open(path, main_store.clone()).await.unwrap();
        db.put_with_options(
            b"k1",
            b"v1",
            &PutOptions::default(),
            &WriteOptions::default(),
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        let wal_reader = WalReader::new(path, main_store.clone());
        let wal_files = wal_reader.list(..).await.unwrap();
        assert!(!wal_files.is_empty());

        for wal_file in wal_files {
            let wal_metadata = wal_file.metadata().await.unwrap();
            let object_metadata = main_store
                .head(&wal_metadata.metadata.location)
                .await
                .unwrap();
            assert_eq!(wal_metadata.id, wal_file.id);
            assert_eq!(
                wal_metadata.metadata.last_modified,
                object_metadata.last_modified
            );
            assert_eq!(wal_metadata.metadata.size, object_metadata.size);
            assert_eq!(wal_metadata.metadata.location, object_metadata.location);
        }
    }

    #[tokio::test]
    async fn test_zero_byte_wal_file_has_empty_iterator() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_reader_zero_byte";
        let wal_store = WalTableStore::new(
            main_store.clone(),
            SsTableFormat::default(),
            path,
            TableStoreKind::Reader,
        );
        let wal_reader = WalReader::new(path, main_store);
        wal_store.write_wal_fence(1.into()).await.unwrap();

        let wal_files = wal_reader.list(..).await.unwrap();
        assert_eq!(wal_files.len(), 1);
        assert_eq!(wal_files[0].id, 1);

        let wal_metadata = wal_files[0].metadata().await.unwrap();
        assert_eq!(wal_metadata.id, 1);
        assert_eq!(wal_metadata.metadata.size, 0);

        let mut iter = wal_files[0].iterator().await.unwrap();
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_returns_wal_file_for_id() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_reader_get_missing";
        let wal_reader = WalReader::new(path, main_store);
        let wal_file = wal_reader.get(42);
        assert_eq!(wal_file.id, 42);
    }

    #[test]
    fn test_wal_file_next_id_and_next_file() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_reader_next_id";
        let wal_reader = WalReader::new(path, main_store);
        let wal_file = wal_reader.get(41);

        assert_eq!(wal_file.next_id(), 42);

        let next = wal_file.next_file();
        assert_eq!(next.id, 42);
        assert!(Arc::ptr_eq(&wal_file.wal_store, &next.wal_store));
    }

    #[tokio::test]
    async fn test_metadata_returns_error_when_file_deleted() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_reader_missing_metadata";
        let db = Db::open(path, main_store.clone()).await.unwrap();
        db.put_with_options(
            b"k1",
            b"v1",
            &PutOptions::default(),
            &WriteOptions::default(),
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        let wal_reader = WalReader::new(path, main_store.clone());
        let wal_file = wal_reader
            .list(..)
            .await
            .unwrap()
            .into_iter()
            .next()
            .expect("expected at least one WAL file");
        let wal_metadata = wal_file.metadata().await.unwrap();

        main_store
            .delete(&wal_metadata.metadata.location)
            .await
            .unwrap();
        let err = wal_file
            .metadata()
            .await
            .expect_err("expected metadata() to fail after deleting WAL file");
        assert_eq!(err.kind(), crate::ErrorKind::Data);
        assert!(has_not_found_object_store_source(&err));
    }

    #[tokio::test]
    async fn test_iterator_returns_error_when_file_deleted() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_reader_missing_iterator";
        let db = Db::open(path, main_store.clone()).await.unwrap();
        db.put_with_options(
            b"k1",
            b"v1",
            &PutOptions::default(),
            &WriteOptions::default(),
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        let wal_reader = WalReader::new(path, main_store.clone());
        let wal_file = wal_reader
            .list(..)
            .await
            .unwrap()
            .into_iter()
            .next()
            .expect("expected at least one WAL file");
        let wal_metadata = wal_file.metadata().await.unwrap();

        main_store
            .delete(&wal_metadata.metadata.location)
            .await
            .unwrap();
        let err = match wal_file.iterator().await {
            Ok(_) => panic!("expected iterator() to fail after deleting WAL file"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), crate::ErrorKind::Data);
        assert!(has_not_found_object_store_source(&err));
    }

    #[tokio::test]
    async fn test_iterator_returns_tombstones() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_reader_tombstones";
        let db = Db::open(path, main_store.clone()).await.unwrap();

        db.delete(b"k_tombstone").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        let wal_reader = WalReader::new(path, main_store);
        let wal_files = wal_reader.list(..).await.unwrap();
        assert!(!wal_files.is_empty());

        let mut rows = Vec::new();
        for wal_file in wal_files {
            let mut iter = wal_file.iterator().await.unwrap();
            while let Some(entry) = iter.next().await.unwrap() {
                rows.push(entry);
            }
        }

        let tombstone_entry = rows
            .iter()
            .find(|entry| entry.key.as_ref() == b"k_tombstone")
            .expect("expected deleted key in WAL iterator output");
        assert!(matches!(tombstone_entry.value, ValueDeletable::Tombstone));
    }

    #[tokio::test]
    async fn test_iterator_returns_merge_operands() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test_wal_reader_merges";
        let db = Db::builder(path, main_store.clone())
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        db.merge(b"k_merge", b"merge_operand").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        let wal_reader = WalReader::new(path, main_store);
        let wal_files = wal_reader.list(..).await.unwrap();
        assert!(!wal_files.is_empty());

        let mut rows = Vec::new();
        for wal_file in wal_files {
            let mut iter = wal_file.iterator().await.unwrap();
            while let Some(entry) = iter.next().await.unwrap() {
                rows.push(entry);
            }
        }

        let merge_entry = rows
            .iter()
            .find(|entry| entry.key.as_ref() == b"k_merge")
            .expect("expected merge key in WAL iterator output");
        assert!(matches!(
            &merge_entry.value,
            ValueDeletable::Merge(value) if value.as_ref() == b"merge_operand"
        ));
    }
}
