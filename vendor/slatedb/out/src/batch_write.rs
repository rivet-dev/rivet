//! # Batch Write
//!
//! This module adds batch write functionality to DbInner. Prior to this feature,
//! writes were performed directly in DbInner's `put_with_options` and
//! `delete_with_options` methods. For each operation, a lock was acquired on the
//! db_state to mutate the WAL or memtable. This worked fine for single writes,
//! but for batch writes, which take longer, it could create contention on the lock
//! because. This is dangerous in an async runtime because it can block the
//! threads, leading to starvation.
//!
//! This module spawns a separate task to handle batch writes. The task receives
//! a `WriteBatchMsg``, which contains a `WriteBatchRequest``. The `WriteBatchRequest`
//! contains a `WriteBatch` containing Put/Delete operations and a `oneshot::Sender`.
//! The `Sender` is used to send the table that the batch was written to back to the
//! caller so the caller can `.await` the result. The result is that callers safely
//! `.await` on their writes rather than holding a lock on the db_state.
//!
//! Centralizing the writes in a single event loop also provides a single location to
//! assign sequence numbers when we implement MVCC.
//!
//! [Pebble](https://github.com/cockroachdb/pebble) has a similar design and
//! [a good write-up](https://github.com/cockroachdb/pebble/blob/master/docs/rocksdb.md#commit-pipeline)
//! describing its benefits.
//!
//! _Note: The `write_batch` loop still holds a lock on the db_state. There can still
//! be contention between `get`s, which holds a lock, and the write loop._

use async_trait::async_trait;
use fail_parallel::fail_point;
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use std::sync::Arc;
use tracing::instrument;

use crate::config::WriteOptions;
use crate::db_state::DbState;
use crate::db_transaction::DbTransaction;
use crate::dispatcher::MessageHandler;
use crate::mem_table::KVTable;
use crate::types::RowEntry;
use crate::utils::WatchableOnceCellReader;
use crate::wal::{FlushResultFuture, WalWriter};
use crate::{batch::WriteBatch, db::DbInner, db::WriteHandle, error::SlateDBError};
use bytes::Bytes;
use parking_lot::RwLockWriteGuard;
use std::collections::BTreeSet;
use tokio::sync::oneshot;

pub(crate) const WRITE_BATCH_TASK_NAME: &str = "writer";

pub(crate) type WriteBatchResult = Result<WriteHandle, SlateDBError>;

/// A message processed by the batch writer event loop.
#[allow(clippy::large_enum_variant)]
pub(crate) enum BatchWriterMessage {
    /// Apply a write batch to the WAL and memtable. This may trigger freezing the memtable.
    WriteBatch(WriteBatchRequest),
    /// Flush the wal if enabled, and optionally freeze the current active memtable.
    Flush(BatchWriterFlush),
}

pub(crate) struct BatchWriterFlush {
    /// If true, then also freeze the current active memtable.
    freeze_memtable: bool,
    /// Sends a message when the writer has processed the flush message. On successful receipt
    /// of a message, the caller should wait on the received Receiver to get the result of the
    /// wal flush.
    done: oneshot::Sender<Result<FlushResultFuture, SlateDBError>>,
}

pub(crate) struct WriteBatchRequest {
    pub(crate) batch: WriteBatch,
    pub(crate) options: WriteOptions,
    pub(crate) done: oneshot::Sender<WriteBatchResult>,
    /// Holds the committing transaction once it is enqueued, ownership
    /// transfers from the caller to the writer. `None` for
    /// non-transactional writes. Fix for #1732.
    pub(crate) txn: Option<DbTransaction>,
}

impl std::fmt::Debug for BatchWriterMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchWriterMessage::WriteBatch(WriteBatchRequest { batch, options, .. }) => f
                .debug_struct("WriteBatch")
                .field("batch", batch)
                .field("options", options)
                .finish(),
            BatchWriterMessage::Flush(BatchWriterFlush {
                freeze_memtable, ..
            }) => f
                .debug_struct("Flush")
                .field("freeze_memtable", freeze_memtable)
                .finish(),
        }
    }
}

pub(crate) struct WriteBatchEventHandler {
    db_inner: Arc<DbInner>,
    wal_writer: Option<Box<dyn WalWriter>>,
}

impl WriteBatchEventHandler {
    pub(crate) fn new(db_inner: Arc<DbInner>, wal_writer: Option<Box<dyn WalWriter>>) -> Self {
        Self {
            db_inner,
            wal_writer,
        }
    }
}

#[async_trait]
impl MessageHandler<BatchWriterMessage> for WriteBatchEventHandler {
    async fn handle(&mut self, message: BatchWriterMessage) -> Result<(), SlateDBError> {
        match message {
            BatchWriterMessage::WriteBatch(WriteBatchRequest {
                batch,
                options,
                done,
                txn,
            }) => {
                let wal_writer = self.wal_writer.as_deref_mut();
                let result = self
                    .db_inner
                    .write_batch(batch, &options, txn.as_ref(), wal_writer)
                    .await;
                match result {
                    Ok(write_result) => {
                        let _ = done.send(write_result);
                        Ok(())
                    }
                    Err(error) => {
                        let _ = done.send(Err(error.clone()));
                        Err(error)
                    }
                }
            }
            BatchWriterMessage::Flush(flush_msg) => {
                let BatchWriterFlush {
                    freeze_memtable,
                    done,
                } = flush_msg;
                let result = self
                    .db_inner
                    .flush_batch_writer(freeze_memtable, self.wal_writer.as_mut())
                    .await;
                match result {
                    Ok(flush_result) => {
                        let _ = done.send(Ok(flush_result));
                        Ok(())
                    }
                    Err(error) => {
                        let _ = done.send(Err(error.clone()));
                        Err(error)
                    }
                }
            }
        }
    }

    async fn cleanup(
        &mut self,
        mut messages: BoxStream<'async_trait, BatchWriterMessage>,
        result: Result<(), SlateDBError>,
    ) -> Result<(), SlateDBError> {
        let error = result.clone().err().unwrap_or(SlateDBError::Closed);
        while let Some(msg) = messages.next().await {
            match msg {
                BatchWriterMessage::WriteBatch(req) => {
                    let _ = req.done.send(Err(error.clone()));
                }
                BatchWriterMessage::Flush(flush_msg) => {
                    let BatchWriterFlush {
                        freeze_memtable: _,
                        done,
                    } = flush_msg;
                    let _ = done.send(Err(error.clone()));
                }
            }
        }
        if let Some(wal_writer) = self.wal_writer.as_mut() {
            wal_writer.close().await?;
        }
        Ok(())
    }
}

impl DbInner {
    #[allow(clippy::panic)]
    #[instrument(level = "trace", skip_all, fields(batch_size = batch.op_count()))]
    async fn write_batch<'a>(
        &'a self,
        batch: WriteBatch,
        options: &WriteOptions,
        txn: Option<&DbTransaction>,
        mut wal_writer: Option<&mut (dyn WalWriter + 'static)>,
    ) -> Result<WriteBatchResult, SlateDBError> {
        let _options = options;
        #[cfg(not(dst))]
        let now = self.mono_clock.now().await?;
        #[cfg(dst)]
        // Force the current timestamp for DST operations. See #719 for details.
        let now = options.now;
        // If the user supplied a sequence number, validate that it's strictly greater
        // than the current max and advance the oracle. No CAS loop is needed here because
        // write_batch is always called from a single-writer event loop.
        let commit_seq = if options.seqnum > 0 {
            let current = self.oracle.last_seq();
            if options.seqnum <= current {
                return Ok(Err(SlateDBError::InvalidSequenceNumber {
                    provided: options.seqnum,
                    current,
                }));
            }
            self.oracle.advance_last_seq(options.seqnum);
            options.seqnum
        } else {
            self.oracle.next_seq()
        };

        // Check for transaction conflicts before proceeding with the write batch
        // if this batch is part of a transaction.
        if let Some(txn) = txn {
            if self.txn_manager.check_has_conflict(&txn.id()) {
                return Ok(Err(SlateDBError::TransactionConflict));
            }
        }

        // Count batch-local merge folding on the flush path so DB-side merge
        // resolution uses one metric for both write batches and memtable flushes.
        let (entries, touched_segments, entries_size) = match batch
            .extract_entries(
                commit_seq,
                now,
                self.settings.default_ttl_millis,
                self.flush_merge_operator.clone(),
                self.segment_extractor.as_deref(),
            )
            .await
        {
            Ok(extracted) => extracted,
            Err(error) => return Ok(Err(error)),
        };

        // RFC-0024 route-consistency: when a segment extractor is
        // configured, every write must extract a prefix that does
        // not nest with the current segment set. Runs before the
        // WAL append so a rejected batch produces no durable side
        // effects.
        if let Err(error) = self.validate_segment_antichain(&touched_segments) {
            return Ok(Err(error));
        }

        if let Some(wal_writer) = wal_writer.as_mut() {
            assert!(self.wal_enabled);
            // WAL entries must be appended to the wal buffer atomically. Otherwise,
            // the WAL buffer might flush the entries in the middle of the batch, which
            // would violate the guarantee that batches are written atomically. We do
            // this by appending the entire entry batch in a single call to the WAL buffer,
            // which holds a write lock during the append.
            wal_writer.append(&entries).await?;
            // TODO: handle sync here, if sync is enabled, we can call `flush` here. let's put this
            // in another Pull Request.
            self.write_entries_to_memtable(entries, touched_segments);
        } else {
            assert!(!self.wal_enabled);
            self.write_entries_to_memtable(entries, touched_segments);
        };
        // increment memtable_write_bytes by the size of the keys and values inserted into the memtable
        // after merge operators and overwrites are collapsed
        self.db_stats.memtable_write_bytes.increment(entries_size);

        // insert a fail point to make it easier to test the case where the last_committed_seq is not updated.
        // this is useful for testing the case where the reader is not able to see the writes.
        fail_point!(
            Arc::clone(&self.fp_registry),
            "write-batch-pre-commit",
            |_| { Err(SlateDBError::from(std::io::Error::other("oops"))) }
        );

        // track the recent committed txn for conflict check. if a txn is not supplied,
        // we still consider this as an transaction commit.
        if let Some(txn) = txn {
            self.txn_manager
                .track_recent_committed_txn(&txn.id(), commit_seq);
        } else {
            let write_keys = batch.keys();
            self.txn_manager
                .track_recent_committed_write_batch(&write_keys, commit_seq);
        }

        // insert a fail point to make it easier to test the case where the transaction is committed but
        // but remaining work hasn't been done. this is useful for testing that transaction commits and
        // commited seqnums get updated in lock-step. See #1301 for details.
        fail_point!(
            Arc::clone(&self.fp_registry),
            "write-batch-post-commit",
            |_| { Err(SlateDBError::from(std::io::Error::other("oops"))) }
        );

        // record the memtable sequence in the memtable's sequence tracker.
        self.record_memtable_sequence(commit_seq);

        // maybe freeze the memtable.
        self.maybe_freeze_current_memtable(wal_writer.as_deref())?;

        let write_handle =
            WriteHandle::new_with_waiter(commit_seq, now, self.status_manager.durability_waiter());

        Ok(Ok(write_handle))
    }

    fn maybe_freeze_current_memtable(
        &self,
        wal_writer: Option<&dyn WalWriter>,
    ) -> Result<(), SlateDBError> {
        // extract the information required to make a freeze decision under a read-lock first
        // so we don't call into `WalWriter` while holding a lock
        let (l0_sst_size_est, last_freeze_wal_id) = {
            let guard = self.state.read();
            let meta = guard.memtable().metadata();

            let last_freeze_wal_id = guard
                .state()
                .imm_memtable
                .front()
                .map(|imm| imm.recent_flushed_wal_id())
                .unwrap_or(guard.state().core().replay_after_wal_id);

            let l0_sst_size_est = self
                .table_store
                .estimate_encoded_size_compacted(meta.entry_num, meta.entries_size_in_bytes);

            (l0_sst_size_est, last_freeze_wal_id)
        };

        let wal_should_flush_memtable = wal_writer
            .is_some_and(|wal_writer| wal_writer.should_flush_memtable(last_freeze_wal_id));

        if !wal_should_flush_memtable && l0_sst_size_est < self.settings.l0_sst_size_bytes {
            return Ok(());
        }

        let replay_after_wal_id = self.wal_observer.status()?.last_flushed_wal_id;
        let mut guard = self.state.write();
        self.freeze_current_memtable_with_state_guard(&mut guard, replay_after_wal_id);
        Ok(())
    }

    async fn flush_batch_writer(
        &self,
        freeze_memtable: bool,
        wal_writer: Option<&mut Box<dyn WalWriter>>,
    ) -> Result<FlushResultFuture, SlateDBError> {
        let flush_rx = if let Some(wal_writer) = wal_writer {
            wal_writer.flush().await?
        } else {
            async { Ok(()) }.boxed()
        };
        if freeze_memtable {
            // Note that this likely won't reflect the result of the above flush call as we don't
            // block until the flush completes. That's fine, as any earlier wal is still a safe
            // replay point.
            let replay_after_wal_id = self.wal_observer.status()?.last_flushed_wal_id;
            let mut guard = self.state.write();
            self.freeze_current_memtable_with_state_guard(&mut guard, replay_after_wal_id);
        }
        Ok(flush_rx)
    }

    // TODO: this is only pub(crate) because currently the replay logic resides in db_common. We
    //       should consolidate replay and the write path into one module and make this private
    pub(crate) fn freeze_current_memtable_with_state_guard(
        &self,
        guard: &mut RwLockWriteGuard<'_, DbState>,
        replay_after_wal_id: u64,
    ) {
        if guard.memtable().is_empty() {
            return;
        }

        guard.freeze_memtable(replay_after_wal_id);
        let _ = self.memtable_flusher().notify_memtable_frozen();
    }

    /// Request a memtable freeze from the writer task. Sends a
    /// [`BatchWriterMessage::Flush`] to the writer event loop and waits for it to complete.
    #[instrument(level = "trace", skip_all, err(level = tracing::Level::DEBUG))]
    pub(crate) async fn request_batch_writer_flush(
        &self,
        freeze_memtable: bool,
    ) -> Result<(), SlateDBError> {
        let (done, rx) = oneshot::channel();
        self.write_notifier
            .send(BatchWriterMessage::Flush(BatchWriterFlush {
                freeze_memtable,
                done,
            }))?;
        Ok(rx.await??.await?)
    }

    /// RFC-0024 route-consistency check. Verifies that `batch_prefixes`,
    /// added to the union of every prefix the writer already knows
    /// about (manifest segments, in-flight imms, the active memtable),
    /// still forms an antichain — no prefix is a proper prefix of
    /// another.
    ///
    /// Optimized for the common case where the batch's prefixes are
    /// already recorded on the active memtable (writes concentrate on
    /// a small set of active segments). That case short-circuits
    /// without consulting older in-memory sources or the manifest.
    /// Only truly novel prefixes pay the manifest binary-search cost.
    ///
    /// Read-only and runs before any durable side-effect, so a
    /// rejection leaves no trace.
    fn validate_segment_antichain(
        &self,
        batch_prefixes: &BTreeSet<Bytes>,
    ) -> Result<(), SlateDBError> {
        if batch_prefixes.is_empty() {
            return Ok(());
        }

        check_batch_antichain(batch_prefixes)?;

        let mut remaining = batch_prefixes.clone();
        let guard = self.state.read();
        let memtable = guard.memtable().table();
        check_segment_prefix_antichain(&mut remaining, memtable)?;
        if remaining.is_empty() {
            return Ok(());
        }
        let cow = guard.state();
        for imm in cow.imm_memtable.iter() {
            check_segment_prefix_antichain(&mut remaining, &imm.table())?;
            if remaining.is_empty() {
                return Ok(());
            }
        }
        let core = cow.core();
        for c in &remaining {
            core.check_segment_prefix_antichain(c.as_ref())?;
        }
        Ok(())
    }

    /// Write entries to the currently active memtable and record
    /// the batch's touched-segment prefixes on it. Returns a durable
    /// watcher for the memtable. When no extractor is configured,
    /// `touched_segments` is empty and recording is a no-op.
    fn write_entries_to_memtable(
        &self,
        entries: Vec<RowEntry>,
        touched_segments: BTreeSet<Bytes>,
    ) -> WatchableOnceCellReader<Result<(), SlateDBError>> {
        let guard = self.state.read();
        let memtable = guard.memtable();
        self.status_manager.add_memtable_segments(&touched_segments);
        memtable.record_touched_segments(touched_segments.clone());
        entries.into_iter().for_each(|entry| memtable.put(entry));
        memtable.table().durable_watcher()
    }

    fn record_memtable_sequence(&self, seq: u64) {
        let ts = self.system_clock.now();
        let guard = self.state.read();
        guard.memtable().record_sequence(seq, ts);
    }
}

/// Verify that `prefixes` is itself an antichain — no element is a
/// proper prefix of another. The set is sorted (BTreeSet), so it
/// suffices to check every adjacent pair: in a sorted antichain any
/// nesting must surface at an adjacent boundary.
fn check_batch_antichain(prefixes: &BTreeSet<Bytes>) -> Result<(), SlateDBError> {
    let mut prev: Option<&Bytes> = None;
    for cur in prefixes {
        if let Some(p) = prev {
            if cur.starts_with(p.as_ref()) {
                return Err(SlateDBError::InvalidSegmentPrefix {
                    prefix: cur.clone(),
                    conflict: p.clone(),
                });
            }
        }
        prev = Some(cur);
    }
    Ok(())
}

/// Run `check` against every prefix in `remaining` and remove those it
/// reports as exact matches. Stops on the first error from `check`.
fn check_segment_prefix_antichain(
    remaining: &mut BTreeSet<Bytes>,
    table: &KVTable,
) -> Result<(), SlateDBError> {
    let mut to_remove: Vec<Bytes> = Vec::new();
    for c in remaining.iter() {
        if table.ensure_valid_segment(c)? {
            to_remove.push(c.clone());
        }
    }
    for k in to_remove {
        remaining.remove(&k);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object_store::memory::InMemory;
    use crate::wal::test_utils::FakeWalWriter;
    use crate::wal::{WalError, WalObserver, WalStatus};
    use crate::Db;

    enum FailingWalOperation {
        Append,
        Flush,
    }

    struct FailingWalWriter {
        inner: FakeWalWriter,
        operation: FailingWalOperation,
    }

    impl FailingWalWriter {
        fn new(operation: FailingWalOperation) -> Self {
            Self {
                inner: FakeWalWriter::new(0),
                operation,
            }
        }
    }

    #[async_trait]
    impl WalWriter for FailingWalWriter {
        async fn append(&mut self, write_batch: &[RowEntry]) -> Result<(), WalError> {
            if matches!(self.operation, FailingWalOperation::Append) {
                return Err(WalError::Fenced);
            }
            self.inner.append(write_batch).await
        }

        async fn flush(&mut self) -> Result<FlushResultFuture, WalError> {
            if matches!(self.operation, FailingWalOperation::Flush) {
                return Err(WalError::Fenced);
            }
            self.inner.flush().await
        }

        fn observer(&self) -> Box<dyn WalObserver> {
            self.inner.observer()
        }

        fn status(&self) -> Result<WalStatus, WalStatus> {
            self.inner.status()
        }

        async fn close(&mut self) -> Result<(), WalError> {
            self.inner.close().await
        }
    }

    /// Build a transaction-less `WriteBatchMessage` and its result receiver,
    /// keeping the `txn: None` and channel boilerplate out of individual tests.
    fn test_message(
        batch: WriteBatch,
        options: WriteOptions,
    ) -> (BatchWriterMessage, oneshot::Receiver<WriteBatchResult>) {
        let (done, rx) = oneshot::channel();
        (
            BatchWriterMessage::WriteBatch(WriteBatchRequest {
                batch,
                options,
                done,
                txn: None,
            }),
            rx,
        )
    }

    #[tokio::test]
    async fn test_append_error_notifies_caller_and_fails_handler() {
        let object_store = Arc::new(InMemory::new());
        let db = Db::open(
            "/tmp/test_append_error_notifies_caller_and_fails_handler",
            object_store,
        )
        .await
        .unwrap();
        let wal_writer = Box::new(FailingWalWriter::new(FailingWalOperation::Append));
        let mut handler = WriteBatchEventHandler::new(db.inner.clone(), Some(wal_writer));

        let mut batch = WriteBatch::new();
        batch.put(b"key", b"value");
        let (msg, done_rx) = test_message(batch, WriteOptions::default());

        let handler_error = handler.handle(msg).await.unwrap_err();
        assert!(matches!(handler_error, SlateDBError::Fenced));
        let caller_error = match done_rx.await.unwrap() {
            Ok(_) => panic!("append unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(matches!(caller_error, SlateDBError::Fenced));
        assert_eq!(db.get(b"key").await.unwrap(), None);

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_flush_error_notifies_caller_and_fails_handler() {
        let object_store = Arc::new(InMemory::new());
        let db = Db::open(
            "/tmp/test_flush_error_notifies_caller_and_fails_handler",
            object_store,
        )
        .await
        .unwrap();
        let wal_writer = Box::new(FailingWalWriter::new(FailingWalOperation::Flush));
        let mut handler = WriteBatchEventHandler::new(db.inner.clone(), Some(wal_writer));
        let (done, done_rx) = oneshot::channel();
        let msg = BatchWriterMessage::Flush(BatchWriterFlush {
            freeze_memtable: false,
            done,
        });

        let handler_error = handler.handle(msg).await.unwrap_err();
        assert!(matches!(handler_error, SlateDBError::Fenced));
        let caller_error = match done_rx.await.unwrap() {
            Ok(_) => panic!("flush unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(matches!(caller_error, SlateDBError::Fenced));

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_user_defined_seqnum() {
        let object_store = Arc::new(InMemory::new());
        let db = Db::open("/tmp/test_user_defined_seqnum", object_store)
            .await
            .unwrap();
        let wal_writer = Box::new(FakeWalWriter::new(0));
        let mut handler = WriteBatchEventHandler::new(db.inner.clone(), Some(wal_writer));

        // Write with a user-defined seqnum
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        let (msg, done_rx) = test_message(
            batch,
            WriteOptions {
                #[cfg(dst)]
                now: 0,
                seqnum: 42,
            },
        );
        handler.handle(msg).await.unwrap();
        let write_handle = done_rx.await.unwrap().unwrap();
        assert_eq!(write_handle.seqnum(), 42);

        // Write without a seqnum and verify auto-assigned is > 42
        let mut batch = WriteBatch::new();
        batch.put(b"key2", b"value2");
        let (msg, done_rx) = test_message(batch, WriteOptions::default());
        handler.handle(msg).await.unwrap();
        let write_handle = done_rx.await.unwrap().unwrap();
        assert!(write_handle.seqnum() > 42);
    }

    #[tokio::test]
    async fn test_user_defined_seqnum_rejects_lower_value() {
        let object_store = Arc::new(InMemory::new());
        let db = Db::open(
            "/tmp/test_user_defined_seqnum_rejects_lower_value",
            object_store,
        )
        .await
        .unwrap();
        let wal_writer = Box::new(FakeWalWriter::new(0));

        let mut handler = WriteBatchEventHandler::new(db.inner.clone(), Some(wal_writer));

        // First, do a normal write to advance the oracle
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        let (msg, done_rx) = test_message(batch, WriteOptions::default());
        handler.handle(msg).await.unwrap();
        let write_handle = done_rx.await.unwrap().unwrap();
        let first_seq = write_handle.seqnum();

        // Try to write with a seqnum <= the current max
        let mut batch = WriteBatch::new();
        batch.put(b"key2", b"value2");
        let (msg, done_rx) = test_message(
            batch,
            WriteOptions {
                #[cfg(dst)]
                now: 0,
                seqnum: 1,
            },
        );
        handler.handle(msg).await.unwrap();
        let result = done_rx.await.unwrap();
        assert!(matches!(
            result,
            Err(SlateDBError::InvalidSequenceNumber {
                provided: 1,
                current,
            }) if current == first_seq
        ));
    }

    fn batch(prefixes: &[&[u8]]) -> BTreeSet<Bytes> {
        prefixes.iter().map(|p| Bytes::copy_from_slice(p)).collect()
    }

    fn assert_invalid_segment_prefix(err: SlateDBError, prefix: &[u8], conflict: &[u8]) {
        match err {
            SlateDBError::InvalidSegmentPrefix {
                prefix: p,
                conflict: c,
            } => {
                assert_eq!(p.as_ref(), prefix);
                assert_eq!(c.as_ref(), conflict);
            }
            other => panic!("expected InvalidSegmentPrefix, got {other:?}"),
        }
    }

    #[test]
    fn check_batch_antichain_accepts_empty_set() {
        check_batch_antichain(&BTreeSet::new()).unwrap();
    }

    #[test]
    fn check_batch_antichain_accepts_singleton() {
        check_batch_antichain(&batch(&[b"abc"])).unwrap();
    }

    #[test]
    fn check_batch_antichain_accepts_disjoint_prefixes() {
        check_batch_antichain(&batch(&[b"aaa", b"bbb", b"ccc"])).unwrap();
    }

    #[test]
    fn check_batch_antichain_rejects_ancestor_descendant_pair() {
        // Sorted: ["abc", "abcd"]. Adjacent walk catches abcd extends abc.
        let err = check_batch_antichain(&batch(&[b"abc", b"abcd"])).unwrap_err();
        assert_invalid_segment_prefix(err, b"abcd", b"abc");
    }

    #[test]
    fn check_batch_antichain_rejects_when_nesting_is_not_at_input_adjacency() {
        // Sorted: ["abc", "abcd", "z"]. Adjacent (abc, abcd) catches it
        // even though the original input order placed "z" between them.
        let err = check_batch_antichain(&batch(&[b"abc", b"z", b"abcd"])).unwrap_err();
        assert_invalid_segment_prefix(err, b"abcd", b"abc");
    }

    /// Helper: open a fresh DB with the given segment extractor.
    async fn open_db_with_extractor(
        path: &str,
        extractor: Arc<dyn crate::prefix_extractor::PrefixExtractor>,
    ) -> Db {
        let object_store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());
        Db::builder(path, object_store)
            .with_segment_extractor(extractor)
            .build()
            .await
            .unwrap()
    }

    /// Sanity check: writes against an extractor-configured DB succeed and
    /// land in the memtable when every key produces a non-nesting prefix.
    /// Three sibling 3-byte prefixes in a single batch is the typical
    /// happy path — the unsegmented branch (legacy) should match.
    #[tokio::test]
    async fn write_with_extractor_accepts_disjoint_prefixes_in_one_batch() {
        let db = open_db_with_extractor(
            "/tmp/test_write_disjoint_prefixes",
            Arc::new(crate::test_utils::FixedThreeBytePrefixExtractor),
        )
        .await;

        let mut batch = WriteBatch::new();
        batch.put(b"aaa-1", b"v1");
        batch.put(b"bbb-1", b"v2");
        batch.put(b"ccc-1", b"v3");
        db.write(batch).await.unwrap();

        // Round-trip through the read path confirms the entries actually
        // landed somewhere.
        for (k, v) in [
            (&b"aaa-1"[..], &b"v1"[..]),
            (&b"bbb-1"[..], &b"v2"[..]),
            (&b"ccc-1"[..], &b"v3"[..]),
        ] {
            let got = db.get(k).await.unwrap().unwrap();
            assert_eq!(got.as_ref(), v);
        }
        db.close().await.unwrap();
    }

    /// A batch whose extracted prefixes nest with each other is rejected
    /// up front. The deliberately non-conforming test extractor returns
    /// `Some(2)` for `"ab*"` and `Some(3)` for `"abc*"`, so a batch with
    /// both yields prefixes `"ab"` and `"abc"` — a strict nest.
    #[tokio::test]
    async fn write_rejects_intra_batch_nesting_prefixes() {
        let db = open_db_with_extractor(
            "/tmp/test_write_intra_batch_nesting",
            Arc::new(crate::test_utils::NonAntichainTestPrefixExtractor),
        )
        .await;

        let mut batch = WriteBatch::new();
        batch.put(b"abc-x", b"v1"); // prefix "abc"
        batch.put(b"ab-y", b"v2"); // prefix "ab" — nests with "abc"
        let err = db.write(batch).await.unwrap_err();
        assert!(matches!(err.kind(), crate::error::ErrorKind::Invalid));
        // Nothing should be visible — the rejected batch left no trace.
        assert!(db.get(b"abc-x").await.unwrap().is_none());
        assert!(db.get(b"ab-y").await.unwrap().is_none());
        db.close().await.unwrap();
    }

    /// A new write is rejected when its extracted prefix would nest with a
    /// segment already present in the manifest. The first write succeeds
    /// and creates segment `"abc"`; the second tries to introduce `"ab"`
    /// (a strict ancestor) and is rejected.
    #[tokio::test]
    async fn write_rejects_when_new_prefix_nests_existing_segment() {
        let db = open_db_with_extractor(
            "/tmp/test_write_nests_existing_segment",
            Arc::new(crate::test_utils::NonAntichainTestPrefixExtractor),
        )
        .await;

        let mut batch = WriteBatch::new();
        batch.put(b"abc-1", b"v1"); // prefix "abc"
        db.write(batch).await.unwrap();
        // Force memtable → L0 → manifest update so the new segment is
        // visible in the next state read. The default `flush()` only
        // flushes the WAL when WAL is enabled.
        db.flush_with_options(crate::config::FlushOptions {
            flush_type: crate::config::FlushType::MemTable,
        })
        .await
        .unwrap();

        // Sanity: segment "abc" is now in the manifest.
        {
            let guard = db.inner.state.read();
            let cow = guard.state();
            let prefixes: Vec<&[u8]> = cow
                .core()
                .segments
                .iter()
                .map(|s| s.prefix.as_ref())
                .collect();
            assert!(
                prefixes.iter().any(|p| *p == &b"abc"[..]),
                "expected segment 'abc' to be persisted; got {:?}",
                prefixes
            );
        }

        // Second batch routes to "ab", which is a strict prefix of "abc".
        let mut batch = WriteBatch::new();
        batch.put(b"ab-1", b"v2");
        let err = db.write(batch).await.unwrap_err();
        assert!(matches!(err.kind(), crate::error::ErrorKind::Invalid));
        assert!(db.get(b"ab-1").await.unwrap().is_none());
        db.close().await.unwrap();
    }

    /// A new write is rejected when its prefix nests with one already
    /// recorded on the active memtable but not yet flushed. Pins down
    /// the per-source check: removing it would let the manifest-only
    /// fallback miss this conflict because the conflicting prefix
    /// has never been persisted.
    #[tokio::test]
    async fn write_rejects_when_new_prefix_nests_active_memtable_prefix() {
        let db = open_db_with_extractor(
            "/tmp/test_write_nests_memtable_prefix",
            Arc::new(crate::test_utils::NonAntichainTestPrefixExtractor),
        )
        .await;

        // First write lands in the active memtable, recording prefix "abc".
        // No flush — manifest still has no segments.
        let mut first = WriteBatch::new();
        first.put(b"abc-1", b"v1");
        db.write(first).await.unwrap();
        {
            let guard = db.inner.state.read();
            assert!(
                guard.state().core().segments.is_empty(),
                "manifest must not yet contain any segment for this test to be meaningful"
            );
        }

        // Second write routes to "ab" — a strict prefix of "abc" sitting
        // on the active memtable. Must be rejected.
        let mut second = WriteBatch::new();
        second.put(b"ab-2", b"v2");
        let err = db.write(second).await.unwrap_err();
        assert!(matches!(err.kind(), crate::error::ErrorKind::Invalid));
        assert!(db.get(b"ab-2").await.unwrap().is_none());
        assert_eq!(db.get(b"abc-1").await.unwrap().unwrap().as_ref(), b"v1");
        db.close().await.unwrap();
    }

    /// An extractor that returns `Some(0)` — even consistently — is a
    /// configuration error: it would route every key into the empty-
    /// prefix segment, blocking any subsequent named-segment write
    /// because the empty prefix nests with all of them. Reject the
    /// write up front so the user sees the cause clearly.
    #[tokio::test]
    async fn write_rejects_empty_extractor_prefix() {
        #[derive(Debug)]
        struct AlwaysEmptyExtractor;
        impl crate::prefix_extractor::PrefixExtractor for AlwaysEmptyExtractor {
            fn name(&self) -> &str {
                "always-empty"
            }
            fn prefix_len(&self, _target: &crate::prefix_extractor::PrefixTarget) -> Option<usize> {
                Some(0)
            }
        }
        let db = open_db_with_extractor(
            "/tmp/test_write_empty_prefix",
            Arc::new(AlwaysEmptyExtractor),
        )
        .await;

        let mut batch = WriteBatch::new();
        batch.put(b"any-key", b"v1");
        let err = db.write(batch).await.unwrap_err();
        assert!(matches!(err.kind(), crate::error::ErrorKind::Invalid));
        assert!(
            err.to_string().contains("empty prefix"),
            "expected empty-prefix error, got: {err}"
        );
        // Nothing got durable — the rejected batch left no trace.
        assert!(db.get(b"any-key").await.unwrap().is_none());
        db.close().await.unwrap();
    }

    /// No-extractor DBs continue to accept arbitrary keys with no
    /// validation — the segment-routing check is a no-op.
    #[tokio::test]
    async fn write_without_extractor_accepts_arbitrary_keys() {
        let object_store = Arc::new(InMemory::new());
        let db = Db::open("/tmp/test_write_no_extractor", object_store)
            .await
            .unwrap();
        let mut batch = WriteBatch::new();
        batch.put(b"a", b"v1");
        batch.put(b"abc", b"v2");
        batch.put(b"abcdef", b"v3");
        db.write(batch).await.unwrap();
        for (k, v) in [
            (&b"a"[..], &b"v1"[..]),
            (&b"abc"[..], &b"v2"[..]),
            (&b"abcdef"[..], &b"v3"[..]),
        ] {
            assert_eq!(db.get(k).await.unwrap().unwrap().as_ref(), v);
        }
        db.close().await.unwrap();
    }
}
