//! Parallel L0 memtable flusher pipeline.
//!
//! Coordinates the flush of immutable memtables to L0 SSTs in object storage.
//! The pipeline consists of three components:
//! - **tracker**: event loop for flush requests, dispatch, and waiter tracking
//! - **uploader**: parallel worker pool for SST build and upload
//! - **manifest_writer**: ordered manifest retirement and checkpoint creation

mod manifest_writer;
mod tracker;
mod uploader;

pub(crate) use manifest_writer::FlushResult;
#[cfg(test)]
pub(crate) use tracker::MANIFEST_REFRESH_COUNT;

use crate::checkpoint::CheckpointCreateResult;
use crate::config::CheckpointOptions;
use crate::db::DbInner;
use crate::db_status::ClosedResultWriter;
use crate::dispatcher::MessageHandlerExecutor;
use crate::error::SlateDBError;
use crate::manifest::store::FenceableManifest;
use crate::memtable_flusher::manifest_writer::ManifestWriter;
use crate::memtable_flusher::tracker::{FlushTracker, TrackerMessage};
use crate::memtable_flusher::uploader::Uploader;
use crate::utils::SafeSender;
use log::warn;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::oneshot;

const TRACKER_TASK_NAME: &str = "l0_flush_tracker";

/// Flush request target exposed by the memtable flusher.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FlushTarget {
    /// Return the current durability frontier without initiating new flush work.
    /// Used for `CheckpointScope::Durable` when the caller just needs a consistent
    /// snapshot of what is already durable.
    CurrentDurable,
    /// Wait until all currently known immutable memtables are durably flushed. Used
    /// for explicit `flush()` calls and checkpoint creation, where full durability
    /// is required before proceeding.
    All,
}

/// Parallel L0 memtable flusher subsystem.
pub(crate) struct MemtableFlusher {
    messages_tx: SafeSender<TrackerMessage>,
    messages_rx: async_channel::Receiver<TrackerMessage>,
}

impl MemtableFlusher {
    /// Creates a new memtable flusher.
    /// Call [`start`](Self::start) to register background tasks with the executor.
    pub(crate) fn new(closed_result: &dyn ClosedResultWriter) -> Self {
        let (messages_tx, messages_rx) =
            SafeSender::unbounded_channel(closed_result.result_reader());
        Self {
            messages_tx,
            messages_rx,
        }
    }

    pub(crate) fn start(
        &self,
        inner: Arc<DbInner>,
        manifest: FenceableManifest,
        tokio_handle: &Handle,
        executor: &MessageHandlerExecutor,
        closed_result: &dyn ClosedResultWriter,
    ) -> Result<(), SlateDBError> {
        let uploader = Uploader::start(
            Arc::clone(&inner),
            closed_result,
            self.messages_tx.clone(),
            executor,
            tokio_handle,
        )?;

        let manifest_writer = ManifestWriter::start(
            Arc::clone(&inner),
            manifest,
            inner.settings.manifest_poll_interval,
            closed_result,
            executor,
            tokio_handle,
            self.messages_tx.clone(),
        )?;

        let tracker = FlushTracker::new(inner, uploader, manifest_writer);
        executor.add_handler(
            TRACKER_TASK_NAME.to_string(),
            Box::new(tracker),
            self.messages_rx.clone(),
            tokio_handle,
        )?;

        Ok(())
    }

    /// Enqueues a manifest refresh and waits until it completes.
    pub(crate) async fn refresh_manifest(&self) -> Result<(), SlateDBError> {
        let (tx, rx) = oneshot::channel();
        self.messages_tx
            .send(TrackerMessage::PollManifest { sender: tx })?;
        rx.await.map_err(SlateDBError::ReadChannelError)?
    }

    /// Processes one flush request using the requested target.
    pub(crate) async fn flush(&self, target: FlushTarget) -> Result<FlushResult, SlateDBError> {
        let (tx, rx) = oneshot::channel();
        self.messages_tx
            .send(TrackerMessage::FlushRequest { target, sender: tx })?;
        rx.await.map_err(SlateDBError::ReadChannelError)?
    }

    /// Notifies the flusher that a memtable may have been frozen.
    /// Triggers reconcile and dispatch without waiting for a result.
    pub(crate) fn notify_memtable_frozen(&self) -> Result<(), SlateDBError> {
        self.messages_tx.send(TrackerMessage::MemtableFrozen)
    }

    /// Creates a checkpoint using the memtable flusher's flush semantics.
    pub(crate) async fn create_checkpoint(
        &self,
        target: FlushTarget,
        options: CheckpointOptions,
    ) -> Result<CheckpointCreateResult, SlateDBError> {
        let (tx, rx) = oneshot::channel();
        self.messages_tx.send(TrackerMessage::CheckpointRequest {
            target,
            options,
            sender: tx,
        })?;
        rx.await.map_err(SlateDBError::ReadChannelError)?
    }

    /// Closes the flusher and its subsystems via the executor.
    /// On clean shutdown, the caller is expected to have already flushed
    /// (via `Db::flush`), so the pipeline should be idle. The ordering
    /// here matters mainly for error shutdown: shutting down the uploader
    /// and manifest writer first ensures their final completion messages
    /// reach the tracker before it drains.
    pub(crate) async fn shutdown(executor: &MessageHandlerExecutor) {
        Uploader::shutdown(executor).await;
        ManifestWriter::shutdown(executor).await;
        if let Err(e) = executor.shutdown_task(TRACKER_TASK_NAME).await {
            warn!("failed to shutdown l0 flush tracker [error={:?}]", e);
        }
    }
}
