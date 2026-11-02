use bytes::Bytes;
use uuid::Uuid;

use crate::batch::WriteBatch;
use crate::bytes_range::{ByteRangeBounds, BytesRange};
use crate::config::{
    FlushOptions, MergeOptions, PutOptions, ReadOptions, ScanOptions, WriteOptions,
};
use crate::db::WriteHandle;
use crate::db_cache::CacheTarget;
use crate::db_state::{SsTableHandle, SsTableId};
use crate::db_status::DbStatus;
use crate::manifest::VersionedManifest;
use crate::transaction_manager::IsolationLevel;
use crate::types::KeyValue;
use crate::DbIterator;

/// Trait for read-only database operations.
///
/// This trait defines the interface for reading data from SlateDB,
/// and can be implemented by `Db`, `DbReader` and `DbSnapshot`
/// to provide a unified interface for read-only operations.
#[async_trait::async_trait]
pub trait DbReadOps {
    /// Get a value from the database with default read options.
    ///
    /// The `Bytes` object returned contains a slice of an entire
    /// 4 KiB block. The block will be held in memory as long as the
    /// caller holds a reference to the `Bytes` object. Consider
    /// copying the data if you need to hold it for a long time.
    ///
    /// ## Arguments
    /// - `key`: the key to get
    ///
    /// ## Returns
    /// - `Result<Option<Bytes>, Error>`:
    ///     - `Some(Bytes)`: the value if it exists
    ///     - `None`: if the value does not exist
    ///
    /// ## Errors
    /// - `Error`: if there was an error getting the value
    async fn get<K: AsRef<[u8]> + Send>(&self, key: K) -> Result<Option<Bytes>, crate::Error> {
        self.get_with_options(key, &ReadOptions::default()).await
    }

    /// Get a value from the database with custom read options.
    ///
    /// The `Bytes` object returned contains a slice of an entire
    /// 4 KiB block. The block will be held in memory as long as the
    /// caller holds a reference to the `Bytes` object. Consider
    /// copying the data if you need to hold it for a long time.
    ///
    /// ## Arguments
    /// - `key`: the key to get
    /// - `options`: the read options to use
    ///
    /// ## Returns
    /// - `Result<Option<Bytes>, Error>`:
    ///   - `Some(Bytes)`: the value if it exists
    ///   - `None`: if the value does not exist
    ///
    /// ## Errors
    /// - `Error`: if there was an error getting the value
    async fn get_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<Bytes>, crate::Error>;

    /// Get a key-value pair from the database with default read options.
    ///
    /// Returns the key along with its value and metadata (sequence number,
    /// creation timestamp, expiration timestamp). Unlike [`get`](Self::get),
    /// which returns only the value bytes, this method returns a [`KeyValue`]
    /// that includes row metadata.
    ///
    /// ## Arguments
    /// - `key`: the key to look up
    ///
    /// ## Returns
    /// - `Ok(Some(KeyValue))`: if the key exists and is not deleted/expired
    /// - `Ok(None)`: if the key does not exist or is deleted/expired
    ///
    /// ## Errors
    /// - `Error`: if there was an error reading from the database
    async fn get_key_value<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
    ) -> Result<Option<KeyValue>, crate::Error> {
        self.get_key_value_with_options(key, &ReadOptions::default())
            .await
    }

    /// Get a key-value pair from the database with custom read options.
    ///
    /// Returns the key along with its value and metadata (sequence number,
    /// creation timestamp, expiration timestamp). Unlike
    /// [`get_with_options`](Self::get_with_options), which returns only the
    /// value bytes, this method returns a [`KeyValue`] that includes row
    /// metadata.
    ///
    /// ## Arguments
    /// - `key`: the key to look up
    /// - `options`: the read options to use
    ///
    /// ## Returns
    /// - `Ok(Some(KeyValue))`: if the key exists and is not deleted/expired
    /// - `Ok(None)`: if the key does not exist or is deleted/expired
    ///
    /// ## Errors
    /// - `Error`: if there was an error reading from the database
    async fn get_key_value_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<KeyValue>, crate::Error>;

    /// Scan a range of keys using the default scan options.
    ///
    /// returns a `DbIterator`
    ///
    /// ## Arguments
    /// - `range`: the range of keys to scan
    ///
    /// ## Errors
    /// - `Error`: if there was an error scanning the range of keys
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    async fn scan<T>(&self, range: T) -> Result<DbIterator, crate::Error>
    where
        T: ByteRangeBounds + Send,
    {
        self.scan_with_options(range, &ScanOptions::default()).await
    }

    /// Scan a range of keys with the provided options.
    ///
    /// returns a `DbIterator`
    ///
    /// ## Arguments
    /// - `range`: the range of keys to scan
    /// - `options`: the scan options to use
    ///
    /// ## Errors
    /// - `Error`: if there was an error scanning the range of keys
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    async fn scan_with_options<T>(
        &self,
        range: T,
        options: &ScanOptions,
    ) -> Result<DbIterator, crate::Error>
    where
        T: ByteRangeBounds + Send;

    /// Scan keys that share the provided prefix, restricted to `subrange`,
    /// using the default scan options.
    ///
    /// The subrange bounds are key *suffixes* interpreted relative to the
    /// prefix: a bound `s` selects the full key `prefix ++ s`. Pass `..` to
    /// scan the prefix's entire keyspace.
    ///
    /// ## Arguments
    /// - `prefix`: the key prefix to scan
    /// - `subrange`: the range of key suffixes (relative to `prefix`) to
    ///   scan; `..` scans all keys with the prefix
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    async fn scan_prefix<P, T>(&self, prefix: P, subrange: T) -> Result<DbIterator, crate::Error>
    where
        P: AsRef<[u8]> + Send,
        T: ByteRangeBounds + Send,
    {
        self.scan_prefix_with_options(prefix, subrange, &ScanOptions::default())
            .await
    }

    /// Scan keys that share the provided prefix, restricted to `subrange`,
    /// with custom options. See [`Self::scan_prefix`] for the subrange
    /// semantics.
    ///
    /// ## Arguments
    /// - `prefix`: the key prefix to scan
    /// - `subrange`: the range of key suffixes (relative to `prefix`) to
    ///   scan; `..` scans all keys with the prefix
    /// - `options`: the scan options to use
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    async fn scan_prefix_with_options<P, T>(
        &self,
        prefix: P,
        subrange: T,
        options: &ScanOptions,
    ) -> Result<DbIterator, crate::Error>
    where
        P: AsRef<[u8]> + Send,
        T: ByteRangeBounds + Send,
    {
        let range = BytesRange::from_prefix_and_subrange(prefix.as_ref(), subrange);
        self.scan_with_options(range, options).await
    }
}

/// Trait for write-side database operations.
///
/// This trait defines the asynchronous write API exposed by [`Db`](crate::Db),
/// allowing consumers to write generic code or test doubles over the writer
/// surface without depending on the concrete `Db` type.
///
/// Write methods return after updating the in-memory WAL and MemTable. They do
/// not wait for the write to become durable in object storage. Call
/// [`WriteHandle::await_durable`] on the returned handle to wait for one write,
/// or [`Self::flush`] to flush all pending writes.
#[async_trait::async_trait]
pub trait DbWriteOps {
    /// The transaction type returned by [`Self::begin`]. Stub
    /// implementations supply their own [`DbTransactionOps`] type, while
    /// the real `Db` returns a `DbTransaction`.
    type Transaction: DbTransactionOps + Send;

    /// Write a value into the database with default `PutOptions` and
    /// `WriteOptions`.
    ///
    /// This method does not wait for durability. See [`DbWriteOps`] for
    /// details.
    ///
    /// ## Arguments
    /// - `key`: the key to write
    /// - `value`: the value to write
    ///
    /// ## Errors
    /// - `Error`: if there was an error writing the value.
    async fn put<K, V>(&self, key: K, value: V) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]> + Send,
        V: AsRef<[u8]> + Send,
    {
        self.put_with_options(key, value, &PutOptions::default(), &WriteOptions::default())
            .await
    }

    /// Write a value into the database with custom `PutOptions` and
    /// `WriteOptions`.
    ///
    /// This method does not wait for durability. See [`DbWriteOps`] for
    /// details.
    ///
    /// ## Arguments
    /// - `key`: the key to write
    /// - `value`: the value to write
    /// - `put_opts`: the put options to use
    /// - `write_opts`: the write options to use
    ///
    /// ## Errors
    /// - `Error`: if there was an error writing the value.
    async fn put_with_options<K, V>(
        &self,
        key: K,
        value: V,
        put_opts: &PutOptions,
        write_opts: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]> + Send,
        V: AsRef<[u8]> + Send;

    /// Delete a key from the database with default `WriteOptions`.
    ///
    /// This method does not wait for durability. See [`DbWriteOps`] for
    /// details.
    ///
    /// ## Arguments
    /// - `key`: the key to delete
    ///
    /// ## Errors
    /// - `Error`: if there was an error deleting the key.
    async fn delete<K: AsRef<[u8]> + Send>(&self, key: K) -> Result<WriteHandle, crate::Error> {
        self.delete_with_options(key, &WriteOptions::default())
            .await
    }

    /// Delete a key from the database with custom `WriteOptions`.
    ///
    /// This method does not wait for durability. See [`DbWriteOps`] for
    /// details.
    ///
    /// ## Arguments
    /// - `key`: the key to delete
    /// - `options`: the write options to use
    ///
    /// ## Errors
    /// - `Error`: if there was an error deleting the key.
    async fn delete_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error>;

    /// Merge a value into the database with default `MergeOptions` and
    /// `WriteOptions`.
    ///
    /// This method does not wait for durability. See [`DbWriteOps`] for
    /// details.
    ///
    /// Merge operations allow applications to bypass the traditional
    /// read/modify/write cycle by expressing partial updates using an
    /// associative operator. The merge operator must be configured when
    /// opening the database.
    ///
    /// ## Arguments
    /// - `key`: the key to merge into
    /// - `value`: the merge operand to apply
    ///
    /// ## Errors
    /// - `Error`: if there was an error merging the value, or if no merge
    ///   operator is configured.
    async fn merge<K, V>(&self, key: K, value: V) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]> + Send,
        V: AsRef<[u8]> + Send,
    {
        self.merge_with_options(
            key,
            value,
            &MergeOptions::default(),
            &WriteOptions::default(),
        )
        .await
    }

    /// Merge a value into the database with custom `MergeOptions` and
    /// `WriteOptions`.
    ///
    /// This method does not wait for durability. See [`DbWriteOps`] for
    /// details.
    ///
    /// ## Arguments
    /// - `key`: the key to merge into
    /// - `value`: the merge operand to apply
    /// - `merge_opts`: the merge options to use
    /// - `write_opts`: the write options to use
    ///
    /// ## Errors
    /// - `Error`: if there was an error merging the value, or if no merge
    ///   operator is configured.
    async fn merge_with_options<K, V>(
        &self,
        key: K,
        value: V,
        merge_opts: &MergeOptions,
        write_opts: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]> + Send,
        V: AsRef<[u8]> + Send;

    /// Write a batch of put/delete operations atomically to the database.
    ///
    /// This method does not wait for durability. See [`DbWriteOps`] for
    /// details.
    ///
    /// ## Arguments
    /// - `batch`: the batch of operations to write
    ///
    /// ## Errors
    /// - `Error`: if there was an error writing the batch.
    async fn write(&self, batch: WriteBatch) -> Result<WriteHandle, crate::Error> {
        self.write_with_options(batch, &WriteOptions::default())
            .await
    }

    /// Write a batch of put/delete operations atomically to the database with
    /// custom `WriteOptions`.
    ///
    /// This method does not wait for durability. See [`DbWriteOps`] for
    /// details.
    ///
    /// ## Arguments
    /// - `batch`: the batch of operations to write
    /// - `options`: the write options to use
    ///
    /// ## Errors
    /// - `Error`: if there was an error writing the batch.
    async fn write_with_options(
        &self,
        batch: WriteBatch,
        options: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error>;

    /// Flush in-memory writes to object storage. This function blocks until
    /// the in-memory data has been durably written.
    ///
    /// ## Errors
    /// - `Error`: if there was an error flushing the database.
    async fn flush(&self) -> Result<(), crate::Error>;

    /// Flush in-memory writes to disk with custom options.
    ///
    /// An error will be returned if `options.flush_type` is `FlushType::Wal`
    /// and the WAL is disabled.
    ///
    /// ## Arguments
    /// - `options`: the flush options
    ///
    /// ## Errors
    /// - `Error`: if there was an error flushing the database.
    async fn flush_with_options(&self, options: FlushOptions) -> Result<(), crate::Error>;

    /// Begin a new transaction with the specified isolation level.
    ///
    /// ## Arguments
    /// - `isolation_level`: the isolation level for the transaction
    ///
    /// ## Returns
    /// - `Result<Self::Transaction, Error>`: the transaction handle
    async fn begin(
        &self,
        isolation_level: IsolationLevel,
    ) -> Result<Self::Transaction, crate::Error>;
}

/// Trait for transactional database operations.
///
/// This trait defines the synchronous write API and lifecycle operations
/// exposed by [`DbTransaction`](crate::DbTransaction), and extends
/// [`DbReadOps`] so consumers can write generic code or test doubles over
/// the full transaction surface without depending on the concrete
/// `DbTransaction` type.
#[async_trait::async_trait]
pub trait DbTransactionOps: DbReadOps {
    /// Put a key-value pair into the transaction with default `PutOptions`.
    /// The write is buffered in the transaction's write batch until commit.
    fn put<K, V>(&self, key: K, value: V) -> Result<(), crate::Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put_with_options(key, value, &PutOptions::default())
    }

    /// Put a key-value pair into the transaction with custom `PutOptions`.
    /// The write is buffered in the transaction's write batch until commit.
    fn put_with_options<K, V>(
        &self,
        key: K,
        value: V,
        options: &PutOptions,
    ) -> Result<(), crate::Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>;

    /// Delete a key from the transaction. The delete is buffered in the
    /// transaction's write batch until commit.
    fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<(), crate::Error>;

    /// Merge a key-value pair into the transaction with default
    /// `MergeOptions`.
    ///
    /// ## Errors
    /// - `Error`: if no merge operator is configured for the database.
    fn merge<K, V>(&self, key: K, value: V) -> Result<(), crate::Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_with_options(key, value, &MergeOptions::default())
    }

    /// Merge a key-value pair into the transaction with custom `MergeOptions`.
    ///
    /// ## Errors
    /// - `Error`: if no merge operator is configured for the database.
    fn merge_with_options<K, V>(
        &self,
        key: K,
        value: V,
        options: &MergeOptions,
    ) -> Result<(), crate::Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>;

    /// Mark keys as read for conflict detection.
    ///
    /// When keys are marked as read, the transaction will detect conflicts
    /// if another transaction modifies any of those keys after this
    /// transaction started, regardless of the isolation level.
    fn mark_read<K, I>(&self, keys: I) -> Result<(), crate::Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>;

    /// Mark written keys as untracked for conflict detection.
    ///
    /// Keys marked with this method are still written atomically with the
    /// rest of the transaction, but are excluded from transaction conflict
    /// detection on commit for both this transaction and other transactions.
    fn unmark_write<K, I>(&self, keys: I) -> Result<(), crate::Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>;

    /// Get the sequence number this transaction was started at.
    fn seqnum(&self) -> u64;

    /// Get the unique transaction ID assigned by the transaction manager.
    fn id(&self) -> Uuid;

    /// Commit the transaction with default `WriteOptions`.
    ///
    /// A successful commit applies the write atomically but does not wait for
    /// durability. Call [`WriteHandle::await_durable`] on the returned handle
    /// when the result is `Some`.
    ///
    /// ## Returns
    /// - `Ok(Some(WriteHandle))` if the commit is successful and there are
    ///   writes in the batch.
    /// - `Ok(None)` if the commit is successful but the write batch is empty.
    ///
    /// ## Errors
    /// - `Error`: if the commit operation fails (I/O errors or conflict
    ///   detection).
    async fn commit(self) -> Result<Option<WriteHandle>, crate::Error>
    where
        Self: Sized + Send,
    {
        self.commit_with_options(&WriteOptions::default()).await
    }

    /// Commit the transaction with custom `WriteOptions`.
    ///
    /// A successful commit applies the write atomically but does not wait for
    /// durability. Call [`WriteHandle::await_durable`] on the returned handle
    /// when the result is `Some`.
    ///
    async fn commit_with_options(
        self,
        options: &WriteOptions,
    ) -> Result<Option<WriteHandle>, crate::Error>
    where
        Self: Sized + Send;

    /// Rollback the transaction by discarding all buffered operations.
    fn rollback(self)
    where
        Self: Sized;
}

/// Trait for database metadata operations.
///
/// This trait provides access to database status and manifest information,
/// implemented by [`Db`](crate::Db) and [`DbReader`](crate::DbReader) to
/// provide a unified interface for metadata access.
///
/// The trait is object-safe, allowing for dynamic dispatch when needed.
pub trait DbMetadataOps {
    /// Get the current manifest state.
    ///
    /// Returns the current manifest snapshot known to this handle, paired
    /// with its manifest version ID.
    fn manifest(&self) -> VersionedManifest;

    /// Subscribe to database state changes.
    ///
    /// Returns a [`tokio::sync::watch::Receiver<DbStatus>`] that always
    /// reflects the latest database status. The status includes the latest durable
    /// sequence number and the current manifest snapshot observed by this
    /// handle. For [`Db`](crate::Db) is the current in-memory snapshot and
    /// for [`DbReader`](crate::DbReader) it is the latest manifest polled from object storage.
    /// For example, you can wait for a specific sequence number to
    /// become durable:
    ///
    /// ```ignore
    /// let seq = 42; // sequence number from a write operation
    /// let mut rx = db.subscribe();
    /// rx.wait_for(|s| s.durable_seq >= seq).await.expect("db dropped");
    /// ```
    ///
    /// The status also exposes the segment prefixes (RFC-0024) known to the
    /// handle via [`DbStatus::list_segments`](crate::DbStatus::list_segments),
    /// which returns all segments — the union of those in the manifest and those
    /// touched in the memtables but not yet flushed. For example, you can wait
    /// for a segment to appear:
    ///
    /// ```ignore
    /// let want = b"prefix".to_vec();
    /// let mut rx = db.subscribe();
    /// rx.wait_for(|s| s.list_segments().iter().any(|seg| seg.prefix == want))
    ///     .await
    ///     .expect("db dropped");
    /// ```
    ///
    /// # Deadlock risk
    ///
    /// The returned receiver holds a read lock on the current value while
    /// borrowed (via [`borrow`](tokio::sync::watch::Receiver::borrow),
    /// [`borrow_and_update`](tokio::sync::watch::Receiver::borrow_and_update),
    /// or the guard returned by [`wait_for`](tokio::sync::watch::Receiver::wait_for)).
    /// The database must acquire a write lock to publish new status updates.
    /// Holding the read guard for an extended period will block all database status
    /// updates and may cause a deadlock. See the [deadlock warning in
    /// `Receiver::borrow`](https://docs.rs/tokio/latest/tokio/sync/watch/struct.Receiver.html#method.borrow)
    /// for details. Always clone or copy the data you need:
    ///
    /// ```ignore
    /// // Good: clone the status and release the lock immediately.
    /// let status = rx.borrow().clone();
    /// some_async_fn(status.durable_seq).await;
    /// some_other_async_fn(status.current_manifest.clone()).await;
    ///
    /// // Good: copy the durable seq and release the lock immediately.
    /// let durable_seq = rx.borrow().durable_seq; // uses Copy trait
    /// some_async_fn(durable_seq).await;
    ///
    /// // Bad: holding the status across an await blocks all senders.
    /// let status = rx.borrow();
    /// some_async_fn(status.durable_seq).await; // deadlock!
    /// ```
    fn subscribe(&self) -> tokio::sync::watch::Receiver<DbStatus>;

    /// Returns the latest database status.
    ///
    /// This is a snapshot of the current state and will not update automatically.
    /// Use [`subscribe`](DbMetadataOps::subscribe) to receive real-time updates.
    fn status(&self) -> DbStatus;
}

/// Trait for block-cache warming and eviction operations.
#[async_trait::async_trait]
pub trait DbCacheManagerOps {
    /// Warms selected cache content for one SST.
    ///
    /// Callers fan out over SSTs themselves (for example with
    /// `FuturesUnordered`) to get the concurrency they want. Per-target
    /// outcomes are reflected in cache-manager metrics, not the return value.
    ///
    /// Warming [`CacheTarget::Data`] also warms the SST index, since block
    /// planning depends on it.
    ///
    /// Returns `Err` on the first failing target. If no block cache is
    /// configured, or if the SST is not reachable from the current manifest,
    /// the call is a no-op that returns `Ok(())`.
    async fn warm_sst(
        &self,
        sst_id: SsTableId,
        targets: &[CacheTarget],
    ) -> Result<(), crate::Error>;

    /// Best-effort removal of the cache entries of an SST named by `targets`.
    ///
    /// A [`CacheTarget::Data`] target removes the blocks whose key span
    /// overlaps its range, which reads the SST index. The other targets each
    /// remove one entry. [`CacheTarget::all`] removes everything.
    ///
    /// The handle names the entries, so the SST need not be in the current
    /// manifest. Callers own the decision of what is safe to evict.
    async fn evict_cached_sst(
        &self,
        sst: &SsTableHandle,
        targets: &[CacheTarget],
    ) -> Result<(), crate::Error>;

    /// Send this instance's cached data to disk.
    ///
    /// This moves data for this instance's `db_cache_id` from memory to disk.
    /// It frees memory now, and protects the data from an ungraceful
    /// process exit later. A later instance with the same `db_cache_id` can
    /// read the data back from disk.
    ///
    /// This affects the whole scope, not only this instance's own reads
    /// and writes. If another instance uses the same `db_cache_id`, this call
    /// also flushes that instance's data.
    ///
    /// Does nothing if no block cache is set, or if the cache has no disk
    /// storage.
    async fn flush_cache_to_disk(&self) -> Result<(), crate::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compile-time check: the trait is object-safe.
    fn _assert_object_safe(_: &dyn DbMetadataOps) {}
    fn _assert_cache_manager_ops_object_safe(_: &dyn DbCacheManagerOps) {}
}
