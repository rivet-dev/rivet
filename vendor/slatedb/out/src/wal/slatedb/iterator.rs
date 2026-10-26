use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use log::error;
use slatedb_common::clock::SystemClock;
use tokio::sync::watch;
use tokio::task;
use tokio::task::JoinHandle;

use crate::db_status::DbStatus;
use crate::error::SlateDBError;
use crate::iter::{EmptyIterator, RowEntryIterator};
use crate::manifest::store::ManifestStore;
use crate::manifest::VersionedManifest;
use crate::utils::panic_string;
use crate::wal::{WalError, WalIterator as WalIteratorTrait, WalRows};
use crate::RowEntry;

use super::resource_limiter::ResourceLimiter;
use super::sst_iterator::{SpawnFetchResult, WalSstIterator, WalSstIteratorOptions};
use super::store::{WalFileId, WalTableStore};

#[async_trait]
pub(crate) trait ManifestReader: Send + Sync + 'static {
    async fn manifest(&self) -> Result<VersionedManifest, SlateDBError>;
}

#[async_trait]
impl ManifestReader for watch::Receiver<DbStatus> {
    async fn manifest(&self) -> Result<VersionedManifest, SlateDBError> {
        Ok(self.borrow().current_manifest.clone())
    }
}

#[async_trait]
impl ManifestReader for ManifestStore {
    async fn manifest(&self) -> Result<VersionedManifest, SlateDBError> {
        self.read_latest_manifest().await
    }
}

pub(crate) struct SlateDbWalIteratorOptions {
    /// Target bytes per block-fetch request.
    pub(crate) target_bytes_to_fetch: usize,
    /// Shared soft limit on bytes reserved by all WAL SST iterators.
    pub(crate) max_buffered_bytes: usize,
    /// Shared soft limit on in-flight fetch tasks across all WAL SST iterators.
    pub(crate) max_fetch_tasks: usize,
}

impl Default for SlateDbWalIteratorOptions {
    fn default() -> Self {
        Self {
            target_bytes_to_fetch: 4 * 1024 * 1024,
            max_buffered_bytes: 128 * 1024 * 1024,
            max_fetch_tasks: 128,
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum WalFileIterator {
    Empty(EmptyIterator),
    Sst(Box<WalSstIterator>),
}

impl WalFileIterator {
    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        match self {
            Self::Empty(iter) => iter.next().await,
            Self::Sst(iter) => iter.next().await,
        }
    }

    fn spawn_fetches(&mut self) -> SpawnFetchResult {
        match self {
            Self::Empty(_) => SpawnFetchResult::default(),
            Self::Sst(iter) => iter.spawn_fetches(),
        }
    }
}

struct WalRowsCollector {
    wal_id: u64,
    iter: WalFileIterator,
    rows: Vec<RowEntry>,
    drained: bool,
}

impl WalRowsCollector {
    fn new(wal_id: u64, iter: WalFileIterator) -> Self {
        Self {
            wal_id,
            iter,
            rows: vec![],
            drained: false,
        }
    }

    fn spawn_fetches(&mut self) -> SpawnFetchResult {
        self.iter.spawn_fetches()
    }

    async fn collect(&mut self) -> Result<(), WalError> {
        loop {
            match self.iter.next().await {
                Ok(Some(row)) => self.rows.push(row),
                Ok(None) => {
                    self.drained = true;
                    break Ok(());
                }
                Err(err) if err.has_object_store_not_found() => {
                    break Err(WalError::WalTruncated(self.wal_id));
                }
                Err(err) => {
                    break Err(err.into());
                }
            }
        }
    }
}

impl From<WalRowsCollector> for WalRows {
    fn from(reader: WalRowsCollector) -> Self {
        assert!(reader.drained);
        WalRows {
            last_consumed_wal_file_id: reader.wal_id,
            rows: reader.rows,
        }
    }
}

struct CurrentWalFile {
    initialized: bool,
    collector: Option<WalRowsCollector>,
}

impl CurrentWalFile {
    fn initial() -> Self {
        Self {
            initialized: false,
            collector: None,
        }
    }

    fn initialized(&self) -> bool {
        self.initialized
    }

    async fn collect(&mut self) -> Result<Option<WalRows>, WalError> {
        assert!(self.initialized);
        let Some(collector) = &mut self.collector else {
            return Ok(None);
        };
        collector.collect().await?;
        let collector = self.collector.take().expect("unreachable");
        self.initialized = false;
        Ok(Some(collector.into()))
    }

    fn advance(&mut self, collector: WalRowsCollector) {
        assert!(!self.initialized);
        self.initialized = true;
        self.collector = Some(collector);
    }

    fn finish(&mut self) {
        self.initialized = true;
        self.collector = None;
    }
}

/// Iterates over the writes in a range of WAL files. Returns the rows of one WAL
/// file per [`WalRows`], and verifies that files carry strictly increasing seq
/// ranges — the ordering callers rely on to split and tag memtables safely.
///
/// Preloading opens each WAL SST and loads its index, then schedules block
/// fetches under one shared task and byte budget. A file's rows are read out
/// only when it is returned from [`Self::next`], so at most one file's rows are
/// materialized at a time. For an unbounded end, open tasks poll their assigned
/// future WAL IDs until the files appear or the manifest proves that a missing
/// file was truncated.
pub(crate) struct SlateDbWalIterator {
    sst_iter_options: WalSstIteratorOptions,
    preload_limit: usize,
    end_bound: WalIteratorEndBound,
    wal_store: Arc<WalTableStore>,
    next_files: VecDeque<JoinHandle<Result<WalRowsCollector, WalError>>>,
    loading_files: VecDeque<Result<WalRowsCollector, WalError>>,
    next_wal_id: Option<u64>,
    /// The greatest seq returned so far, used to verify that WAL files arrive
    /// with strictly increasing seq ranges.
    last_seq: Option<u64>,
    /// Set once iteration has ended, either because the range was exhausted or
    /// because an error was returned.
    terminal_result: Option<Result<Option<WalRows>, WalError>>,
    current_file: CurrentWalFile,
}

#[derive(Clone)]
pub(crate) enum WalIteratorEndBound {
    Exclusive(u64),
    Unbounded {
        manifest_reader: Arc<dyn ManifestReader>,
        poll_interval: Duration,
        system_clock: Arc<dyn SystemClock>,
    },
}

impl WalIteratorEndBound {
    fn contains(&self, wal_id: u64) -> bool {
        match self {
            Self::Exclusive(end_wal_id) => wal_id < *end_wal_id,
            Self::Unbounded { .. } => true,
        }
    }
}

impl std::fmt::Debug for WalIteratorEndBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exclusive(end_wal_id) => f.debug_tuple("Exclusive").field(end_wal_id).finish(),
            Self::Unbounded { poll_interval, .. } => f
                .debug_struct("Unbounded")
                .field("poll_interval", poll_interval)
                .finish_non_exhaustive(),
        }
    }
}

impl SlateDbWalIterator {
    pub(crate) fn range(
        from_wal_id: u64,
        to_bound: WalIteratorEndBound,
        options: SlateDbWalIteratorOptions,
        wal_store: Arc<WalTableStore>,
    ) -> Result<Self, SlateDBError> {
        let preload_limit = options.max_fetch_tasks.max(1);
        let sst_iter_options = WalSstIteratorOptions {
            target_bytes_to_fetch: options.target_bytes_to_fetch,
            fetch_limiter: ResourceLimiter::new(options.max_fetch_tasks),
            buffer_limiter: ResourceLimiter::new(options.max_buffered_bytes),
        };

        Ok(Self {
            sst_iter_options,
            preload_limit,
            end_bound: to_bound,
            wal_store,
            next_files: VecDeque::new(),
            loading_files: VecDeque::new(),
            next_wal_id: Some(from_wal_id),
            last_seq: None,
            terminal_result: None,
            current_file: CurrentWalFile::initial(),
        })
    }

    fn spawn_opens(&mut self) {
        while self.maybe_spawn_open() {}
    }

    fn maybe_spawn_open(&mut self) -> bool {
        let Some(next_wal_id) = self.next_wal_id else {
            return false;
        };
        if !self.end_bound.contains(next_wal_id)
            || self.next_files.len() + self.loading_files.len() >= self.preload_limit
        {
            return false;
        }

        self.next_wal_id = next_wal_id.checked_add(1);

        async fn try_open_file_iter(
            wal_id: u64,
            sst_iter_options: WalSstIteratorOptions,
            wal_store: Arc<WalTableStore>,
        ) -> Result<WalRowsCollector, SlateDBError> {
            let sst = match wal_store.open_sst(WalFileId::from(wal_id)).await {
                Ok(sst) => sst,
                Err(SlateDBError::EmptySSTable) => {
                    // Zero-byte WAL files are fence markers; replay them as empty WALs
                    // so the last replayed WAL ID still advances past the marker.
                    return Ok(WalRowsCollector::new(
                        wal_id,
                        WalFileIterator::Empty(EmptyIterator::new()),
                    ));
                }
                Err(err) => return Err(err),
            };
            let iter = WalSstIterator::new(sst, Arc::clone(&wal_store), sst_iter_options).await?;
            Ok(WalRowsCollector::new(
                wal_id,
                WalFileIterator::Sst(Box::new(iter)),
            ))
        }

        async fn open_file_iter(
            wal_id: u64,
            sst_iter_options: WalSstIteratorOptions,
            wal_store: Arc<WalTableStore>,
            end_bound: WalIteratorEndBound,
        ) -> Result<WalRowsCollector, WalError> {
            loop {
                match try_open_file_iter(wal_id, sst_iter_options.clone(), Arc::clone(&wal_store))
                    .await
                {
                    Ok(iter) => return Ok(iter),
                    Err(err) if err.has_object_store_not_found() => {
                        let WalIteratorEndBound::Unbounded {
                            manifest_reader,
                            poll_interval,
                            system_clock,
                        } = &end_bound
                        else {
                            return Err(WalError::WalTruncated(wal_id));
                        };

                        let manifest = manifest_reader.manifest().await?;
                        if wal_id < manifest.next_wal_sst_id() {
                            // This WAL is known to have been written durably in the past,
                            // so it must have been deleted by GC.
                            return Err(WalError::WalTruncated(wal_id));
                        }
                        system_clock.sleep(*poll_interval).await;
                    }
                    Err(err) => return Err(err.into()),
                }
            }
        }

        let handle = task::spawn(open_file_iter(
            next_wal_id,
            self.sst_iter_options.clone(),
            Arc::clone(&self.wal_store),
            self.end_bound.clone(),
        ));
        self.next_files.push_back(handle);
        true
    }

    fn open_task_result(
        end_bound: &WalIteratorEndBound,
        result: Result<Result<WalRowsCollector, WalError>, task::JoinError>,
    ) -> Result<WalRowsCollector, WalError> {
        match result {
            Ok(result) => result,
            Err(join_err) => {
                let task_name = format!("wal_replay[end_bound={end_bound:?}]");
                let msg = if let Ok(panic_err) = join_err.try_into_panic() {
                    format!(
                        "wal_replay task panicked unexpectedly. [task_name={}, panic={}]",
                        task_name,
                        panic_string(&panic_err),
                    )
                } else {
                    format!("wal_replay task cancelled. [task_name={}]", task_name)
                };
                error!("{}", msg);
                let error = Arc::from(Box::<dyn std::error::Error + Send + Sync>::from(msg));
                Err(WalError::InternalError(error))
            }
        }
    }

    /// Moves every completed open from the task queue into the metadata-loaded queue without
    /// waiting for an unfinished open.
    async fn move_finished_files(&mut self) {
        while self.next_files.front().is_some_and(JoinHandle::is_finished) {
            let result = self
                .next_files
                .front_mut()
                .expect("a finished open must exist")
                .await;
            self.next_files.pop_front();
            self.loading_files
                .push_back(Self::open_task_result(&self.end_bound, result));
        }
    }

    /// Waits for the oldest open while preserving `next` cancellation safety.
    async fn await_next_file(&mut self) -> bool {
        // Await a mutable reference so cancellation leaves the handle in the queue.
        // See https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html#cancel-safety.
        let Some(join_handle) = self.next_files.front_mut() else {
            return false;
        };
        let result = join_handle.await;
        self.next_files.pop_front();
        self.loading_files
            .push_back(Self::open_task_result(&self.end_bound, result));
        true
    }

    fn spawn_loading_file_fetches(&mut self) {
        for file in &mut self.loading_files {
            let Ok(file) = file else {
                break;
            };
            if file.spawn_fetches().resource_exhausted {
                break;
            }
        }
    }

    /// Advances completed opens through metadata loading and speculative block fetching, then
    /// promotes the oldest file when a current file is needed.
    async fn load_next_file(&mut self) -> Result<(), WalError> {
        self.spawn_opens();
        self.move_finished_files().await;

        if !self.current_file.initialized() && self.loading_files.is_empty() {
            if !self.await_next_file().await {
                self.current_file.finish();
                return Ok(());
            }
            self.spawn_opens();
            self.move_finished_files().await;
        }

        self.spawn_loading_file_fetches();

        if !self.current_file.initialized() {
            let Some(file) = self.loading_files.pop_front() else {
                self.current_file.finish();
                return Ok(());
            };
            self.current_file.advance(file?);
        }

        self.spawn_opens();
        Ok(())
    }

    fn terminate(
        &mut self,
        result: Result<Option<WalRows>, WalError>,
    ) -> Result<Option<WalRows>, WalError> {
        self.terminal_result = Some(result.clone());
        for task in self.next_files.drain(..) {
            task.abort();
        }
        self.loading_files.clear();
        self.current_file.collector = None;
        result
    }
}

#[async_trait]
impl WalIteratorTrait for SlateDbWalIterator {
    /// Get the next set of writes from the WAL files in the range. Each returned
    /// [`WalRows`] holds the rows of one WAL file; a WAL file with no rows
    /// yields a batch with empty `rows`. A bounded iterator returns `None` once
    /// its range has been read; an iterator with an unbounded end polls future
    /// WAL files instead.
    async fn next(&mut self) -> Result<Option<WalRows>, WalError> {
        if let Some(result) = self.terminal_result.clone() {
            return result;
        }

        if let Err(err) = self.load_next_file().await {
            return self.terminate(Err(err));
        }
        match self.current_file.collect().await {
            Err(err) => self.terminate(Err(err)),
            Ok(None) => self.terminate(Ok(None)),
            Ok(Some(rows)) => {
                // Verify that WAL files carry strictly increasing seq ranges. Replay
                // relies on this ordering to split and tag memtables safely: a commit seq
                // spanning two WAL files, or files with overlapping seq ranges, would
                // break recovery's (wal_id, seq) watermark filtering.
                if let Some(min_seq) = rows.rows.iter().map(|row| row.seq).min() {
                    if let Some(last_seq) = self.last_seq {
                        if min_seq <= last_seq {
                            let msg = format!(
                                "WAL replay saw out-of-order seqs across WAL files. \
                                [wal_id={}, min_seq={}, last_seq={}]",
                                rows.last_consumed_wal_file_id, min_seq, last_seq,
                            );
                            error!("{}", &msg);
                            let error =
                                Arc::from(Box::<dyn std::error::Error + Send + Sync>::from(msg));
                            return self.terminate(Err(WalError::InternalError(error)));
                        }
                    }
                    let max_seq = rows
                        .rows
                        .iter()
                        .map(|row| row.seq)
                        .max()
                        .expect("non-empty rows have a max seq");
                    self.last_seq = Some(max_seq);
                }
                Ok(Some(rows))
            }
        }
    }
}

impl Drop for SlateDbWalIterator {
    fn drop(&mut self) {
        for task in self.next_files.drain(..) {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;
    use std::time::Duration;

    use bytes::Bytes;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::DefaultSystemClock;

    use super::{
        SlateDbWalIterator, SlateDbWalIteratorOptions, WalFileIterator, WalIteratorEndBound,
        WalRowsCollector,
    };
    use crate::db_status::DbStatusManager;
    use crate::format::sst::SsTableFormat;
    use crate::iter::EmptyIterator;
    use crate::manifest::{Manifest, ManifestCore, VersionedManifest};
    use crate::object_store_tag::TableStoreKind;
    use crate::types::RowEntry;
    use crate::wal::slatedb::sst_iterator::WalSstIterator;
    use crate::wal::slatedb::store::WalTableStore;
    use crate::wal::{WalError, WalIterator as _};

    fn versioned_manifest(id: u64, next_wal_id: u64) -> VersionedManifest {
        let mut core = ManifestCore::new();
        core.next_wal_sst_id = next_wal_id;
        VersionedManifest::from_manifest(id, Manifest::initial(core))
    }

    fn status_manager(next_wal_id: u64) -> DbStatusManager {
        DbStatusManager::new_with_initial_values(
            0,
            versioned_manifest(1, next_wal_id),
            BTreeSet::new(),
        )
    }

    #[tokio::test]
    async fn should_repeat_terminal_error_for_wal_iterator() {
        let table_store = test_table_store();
        let mut wal_iter = SlateDbWalIterator::range(
            1,
            WalIteratorEndBound::Exclusive(2),
            SlateDbWalIteratorOptions::default(),
            Arc::clone(&table_store),
        )
        .unwrap();

        assert!(matches!(
            wal_iter.next().await,
            Err(WalError::WalTruncated(1))
        ));
        assert!(matches!(
            wal_iter.next().await,
            Err(WalError::WalTruncated(1))
        ));
    }

    #[tokio::test]
    async fn should_repeat_terminal_none_for_wal_iterator() {
        let table_store = test_table_store();
        let mut wal_iter = SlateDbWalIterator::range(
            1,
            WalIteratorEndBound::Exclusive(1),
            SlateDbWalIteratorOptions::default(),
            Arc::clone(&table_store),
        )
        .unwrap();

        assert!(wal_iter.next().await.unwrap().is_none());
        assert!(wal_iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_honor_an_exclusive_end_bound() {
        let table_store = test_table_store();
        table_store.write_wal_fence(1.into()).await.unwrap();
        table_store.write_wal_fence(2.into()).await.unwrap();
        let mut wal_iter = SlateDbWalIterator::range(
            1,
            WalIteratorEndBound::Exclusive(2),
            SlateDbWalIteratorOptions::default(),
            table_store,
        )
        .unwrap();

        let batch = wal_iter.next().await.unwrap().unwrap();
        assert_eq!(batch.last_consumed_wal_file_id, 1);
        assert!(batch.rows.is_empty());
        assert!(wal_iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_continue_spawning_fetches_after_an_empty_wal() {
        let table_store = test_table_store();
        let mut builder = table_store.table_builder();
        builder
            .add(RowEntry::new_value(b"key", b"value", 1))
            .await
            .unwrap();
        let encoded_sst = builder.build().await.unwrap();
        let table = table_store.write_sst(2.into(), &encoded_sst).await.unwrap();
        let mut wal_iter = SlateDbWalIterator::range(
            1,
            WalIteratorEndBound::Exclusive(3),
            SlateDbWalIteratorOptions {
                target_bytes_to_fetch: usize::MAX,
                max_buffered_bytes: usize::MAX,
                max_fetch_tasks: 2,
            },
            Arc::clone(&table_store),
        )
        .unwrap();
        let sst_iter = WalSstIterator::new(table, table_store, wal_iter.sst_iter_options.clone())
            .await
            .unwrap();
        wal_iter.loading_files.push_back(Ok(WalRowsCollector::new(
            1,
            WalFileIterator::Empty(EmptyIterator::new()),
        )));
        wal_iter.loading_files.push_back(Ok(WalRowsCollector::new(
            2,
            WalFileIterator::Sst(Box::new(sst_iter)),
        )));

        let empty_result = wal_iter
            .loading_files
            .front_mut()
            .unwrap()
            .as_mut()
            .unwrap()
            .spawn_fetches();
        assert_eq!(empty_result.spawned, 0);
        assert!(!empty_result.resource_exhausted);

        wal_iter.spawn_loading_file_fetches();

        let already_scheduled = wal_iter
            .loading_files
            .back_mut()
            .unwrap()
            .as_mut()
            .unwrap()
            .spawn_fetches();
        assert_eq!(already_scheduled.spawned, 0);
        assert!(!already_scheduled.resource_exhausted);
    }

    #[tokio::test(start_paused = true)]
    async fn should_poll_future_wals_in_an_unbounded_range() {
        let table_store = test_table_store();
        let status_manager = status_manager(1);
        let mut wal_iter = SlateDbWalIterator::range(
            1,
            WalIteratorEndBound::Unbounded {
                manifest_reader: Arc::new(status_manager.subscribe()),
                poll_interval: Duration::from_millis(10),
                system_clock: Arc::new(DefaultSystemClock::new()),
            },
            SlateDbWalIteratorOptions::default(),
            Arc::clone(&table_store),
        )
        .unwrap();

        assert!(
            tokio::time::timeout(Duration::from_millis(30), wal_iter.next())
                .await
                .is_err(),
            "an unbounded iterator returned before WAL 1 existed"
        );

        table_store.write_wal_fence(1.into()).await.unwrap();
        let first = tokio::time::timeout(Duration::from_millis(100), wal_iter.next())
            .await
            .expect("iterator did not observe WAL 1")
            .unwrap()
            .expect("unbounded iterator returned None");
        assert!(first.rows.is_empty());
        assert_eq!(first.last_consumed_wal_file_id, 1);

        assert!(
            tokio::time::timeout(Duration::from_millis(30), wal_iter.next())
                .await
                .is_err(),
            "an unbounded iterator returned before WAL 2 existed"
        );

        table_store.write_wal_fence(2.into()).await.unwrap();
        let second = tokio::time::timeout(Duration::from_millis(100), wal_iter.next())
            .await
            .expect("iterator did not observe WAL 2")
            .unwrap()
            .expect("unbounded iterator returned None");
        assert!(second.rows.is_empty());
        assert_eq!(second.last_consumed_wal_file_id, 2);
    }

    #[tokio::test(start_paused = true)]
    async fn should_report_truncation_when_manifest_advances_past_a_missing_wal() {
        let table_store = test_table_store();
        let status_manager = status_manager(1);
        let mut wal_iter = SlateDbWalIterator::range(
            1,
            WalIteratorEndBound::Unbounded {
                manifest_reader: Arc::new(status_manager.subscribe()),
                poll_interval: Duration::from_millis(10),
                system_clock: Arc::new(DefaultSystemClock::new()),
            },
            SlateDbWalIteratorOptions::default(),
            table_store,
        )
        .unwrap();

        assert!(
            tokio::time::timeout(Duration::from_millis(30), wal_iter.next())
                .await
                .is_err(),
            "the iterator did not poll a future WAL"
        );

        status_manager.report_manifest(versioned_manifest(2, 2));
        let result = tokio::time::timeout(Duration::from_millis(100), wal_iter.next())
            .await
            .expect("iterator did not react to the manifest update");
        assert!(matches!(result, Err(WalError::WalTruncated(1))));
    }

    #[tokio::test]
    async fn should_return_atomic_wal_rows_in_increasing_seq_order() {
        let table_store = test_table_store();
        // Each file contains out-of-order rows and a sequence that appears twice.
        // The iterator must keep both rows for a sequence in one batch, while the
        // sequence range of the second batch must follow the first.
        let wal_entries = [
            vec![
                RowEntry::new_value(b"key_001", &[b'x'; 128], 2),
                RowEntry::new_value(b"key_002", &[b'x'; 128], 1),
                RowEntry::new_value(b"key_003", &[b'x'; 128], 2),
            ],
            vec![
                RowEntry::new_value(b"key_004", &[b'x'; 128], 4),
                RowEntry::new_value(b"key_005", &[b'x'; 128], 3),
                RowEntry::new_value(b"key_006", &[b'x'; 128], 4),
            ],
        ];
        let mut expected_rows = BTreeMap::new();
        let mut expected_rows_by_seq = BTreeMap::<u64, BTreeSet<Bytes>>::new();
        let wal_file_count = wal_entries.len() as u64;
        for (file_index, entries) in wal_entries.iter().enumerate() {
            let wal_id = file_index as u64 + 1;
            for row in entries {
                expected_rows.insert(row.key.clone(), (row.clone(), wal_id));
                expected_rows_by_seq
                    .entry(row.seq)
                    .or_default()
                    .insert(row.key.clone());
            }
        }
        for (index, entries) in wal_entries.into_iter().enumerate() {
            let mut builder = table_store.table_builder();
            for entry in entries {
                builder.add(entry).await.unwrap();
            }
            let encoded_sst = builder.build().await.unwrap();
            table_store
                .write_sst((index as u64 + 1).into(), &encoded_sst)
                .await
                .unwrap();
        }
        let mut wal_iter = SlateDbWalIterator::range(
            1,
            WalIteratorEndBound::Exclusive(wal_file_count + 1),
            SlateDbWalIteratorOptions::default(),
            Arc::clone(&table_store),
        )
        .unwrap();

        let mut returned_rows = BTreeMap::new();
        let mut previous_max_seq = None;
        let mut last_consumed_wal_file_id = 0;
        while let Some(batch) = wal_iter.next().await.unwrap() {
            let batch_min_seq = batch.rows.iter().map(|r| r.seq).min().unwrap();
            let batch_max_seq = batch.rows.iter().map(|r| r.seq).max().unwrap();
            if let Some(previous_max_seq) = previous_max_seq {
                assert!(
                    batch_min_seq > previous_max_seq,
                    "consecutive WAL batches have overlapping sequence ranges"
                );
            }
            previous_max_seq = Some(batch_max_seq);

            let mut batch_rows_by_seq = BTreeMap::<u64, BTreeSet<Bytes>>::new();
            for row in &batch.rows {
                assert!(
                    returned_rows.insert(row.key.clone(), row.clone()).is_none(),
                    "row was returned more than once: {:?}",
                    row.key
                );
                batch_rows_by_seq
                    .entry(row.seq)
                    .or_default()
                    .insert(row.key.clone());
            }
            for (seq, batch_rows) in batch_rows_by_seq {
                assert_eq!(
                    expected_rows_by_seq.get(&seq),
                    Some(&batch_rows),
                    "rows for seq {seq} were split across WAL batches"
                );
            }

            assert!(
                batch.last_consumed_wal_file_id >= last_consumed_wal_file_id,
                "consumed WAL file watermark moved backwards"
            );
            assert!(batch.last_consumed_wal_file_id <= wal_file_count);
            for wal_id in 1..=batch.last_consumed_wal_file_id {
                let file_fully_returned =
                    expected_rows.iter().all(|(key, (_, expected_wal_id))| {
                        *expected_wal_id != wal_id || returned_rows.contains_key(key)
                    });
                assert!(
                    file_fully_returned,
                    "WAL file {wal_id} was marked consumed before all its rows were returned"
                );
            }
            last_consumed_wal_file_id = batch.last_consumed_wal_file_id;
        }

        let expected_returned_rows = expected_rows
            .into_iter()
            .map(|(key, (row, _wal_id))| (key, row))
            .collect();
        assert_eq!(returned_rows, expected_returned_rows);
        assert_eq!(last_consumed_wal_file_id, wal_file_count);
    }

    #[tokio::test]
    async fn should_share_fetch_and_buffer_limits_across_loading_files() {
        let table_store = test_table_store();
        let mut handles = Vec::new();
        for wal_id in 1..=2 {
            let mut builder = table_store.table_builder();
            builder
                .add(RowEntry::new_value(
                    format!("key_{wal_id}").as_bytes(),
                    &[b'x'; 128],
                    wal_id,
                ))
                .await
                .unwrap();
            let encoded_sst = builder.build().await.unwrap();
            handles.push(
                table_store
                    .write_sst(wal_id.into(), &encoded_sst)
                    .await
                    .unwrap(),
            );
        }

        let mut block_fetch_sizes = Vec::new();
        for handle in &handles {
            let index = table_store.read_index(handle).await.unwrap();
            let blocks = table_store.block_range_for_target_bytes(handle, &index, 0, 1);
            block_fetch_sizes.push(table_store.block_range_size(handle, &index, blocks));
        }
        let one_file_buffer_limit = *block_fetch_sizes.iter().max().unwrap();
        let mut wal_iter = SlateDbWalIterator::range(
            1,
            WalIteratorEndBound::Exclusive(3),
            SlateDbWalIteratorOptions {
                target_bytes_to_fetch: 1,
                max_buffered_bytes: one_file_buffer_limit,
                max_fetch_tasks: 2,
            },
            table_store,
        )
        .unwrap();
        let fetch_limiter = wal_iter.sst_iter_options.fetch_limiter.clone();
        let buffer_limiter = wal_iter.sst_iter_options.buffer_limiter.clone();

        wal_iter.spawn_opens();
        assert_eq!(wal_iter.next_files.len(), 2);
        assert!(wal_iter.await_next_file().await);
        assert!(wal_iter.await_next_file().await);
        assert_eq!(wal_iter.loading_files.len(), 2);

        wal_iter.spawn_loading_file_fetches();
        assert!(
            buffer_limiter.allocate(1, false).is_none(),
            "one loading file should consume the shared byte limit"
        );
        let spare_fetch = fetch_limiter
            .allocate(1, false)
            .expect("the byte limit should stop the second loading file before the task limit");
        assert!(fetch_limiter.allocate(1, false).is_none());
        drop(spare_fetch);

        drop(wal_iter);
        assert!(fetch_limiter.allocate(2, false).is_some());
        assert!(buffer_limiter
            .allocate(one_file_buffer_limit, false)
            .is_some());
    }

    #[tokio::test]
    // tests cancellation-safety of moving completed pre-fetches to the ready files queue
    async fn should_not_pop_completed_file_until_await_returns() {
        use futures::FutureExt;

        let table_store = test_table_store();
        for wal_id in 1..=3 {
            table_store.write_wal_fence(wal_id.into()).await.unwrap();
        }
        let mut iter = SlateDbWalIterator::range(
            1,
            WalIteratorEndBound::Exclusive(4),
            SlateDbWalIteratorOptions::default(),
            table_store,
        )
        .unwrap();
        iter.spawn_opens();
        // wait for all the file waits to finish
        while !iter
            .next_files
            .iter()
            .all(tokio::task::JoinHandle::is_finished)
        {
            tokio::task::yield_now().await;
        }
        // exhaust the current tasks's budget and ensure the future is pending even though
        // all the file waits are finished
        while tokio::task::coop::has_budget_remaining() {
            tokio::task::consume_budget().await;
        }
        assert!(iter.next().now_or_never().is_none());
        // clear the budget
        tokio::task::yield_now().await;

        let first = iter.next().await.unwrap().unwrap();

        assert_eq!(first.last_consumed_wal_file_id, 1);
    }

    fn test_table_store() -> Arc<WalTableStore> {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        Arc::new(WalTableStore::new(
            object_store,
            SsTableFormat::default(),
            path,
            TableStoreKind::Main,
        ))
    }
}
