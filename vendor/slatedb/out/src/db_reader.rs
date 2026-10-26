use crate::wal::slatedb::reader::SlateDbWalReaderOptions;
use {
    crate::{
        bytes_range::{ByteRangeBounds, BytesRange},
        cached_object_store::CachedObjectStore,
        clock::MonotonicClock,
        config::{CheckpointOptions, DbReaderOptions, ReadOptions, ScanOptions},
        db_cache::CacheTarget,
        db_cache_manager,
        db_common::extract_segment_prefix,
        db_state::{collect_touched_segments, SsTableHandle, SsTableId},
        db_stats::DbStats,
        db_status::{ClosedResultWriter, DbStatus, DbStatusManager},
        dispatcher::{MessageHandler, MessageHandlerExecutor, MessageTickerDef},
        error::SlateDBError,
        manifest::{
            store::{ManifestStore, StoredManifest},
            Manifest, ManifestCore, VersionedManifest,
        },
        mem_table::{ImmutableMemtable, KVTable, WritableKVTable},
        merge_operator::MergeOperatorType,
        oracle::DbReaderOracle,
        paths::PathResolver,
        prefix_extractor::PrefixExtractor,
        reader::{DbStateReader, Reader, ScanContext},
        tablestore::TableStore,
        types::KeyValue,
        utils::IdGenerator,
        wal::slatedb::store::WalTableStore,
        wal::WalReader as WalReaderTrait,
        wal_replay::{WalReplayIterator, WalReplayOptions},
        Checkpoint, DbCacheManagerOps, DbIterator, DbMetadataOps, DbReadOps,
    },
    async_trait::async_trait,
    bytes::Bytes,
    futures::stream::BoxStream,
    log::{info, warn},
    object_store::{path::Path, ObjectStore},
    parking_lot::RwLock,
    slatedb_common::{clock::SystemClock, DbRand},
    std::{
        collections::{BTreeSet, VecDeque},
        ops::Sub,
        sync::{Arc, LazyLock},
    },
    tokio::runtime::Handle,
    uuid::Uuid,
};

pub(crate) const DB_READER_TASK_NAME: &str = "manifest_poller";

/// Determines how a [`DbReader`] chooses and refreshes the database state it reads.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DbReaderMode {
    /// Create and maintain checkpoints while following the latest database state.
    ///
    /// The reader will automatically create a checkpoint and refresh it periodically to ensure
    /// that the reader can continue to read the latest database state without being affected by
    /// garbage collection.
    #[default]
    ManagedCheckpoint,

    /// Remain pinned to the database state referenced by the supplied checkpoint.
    Checkpoint(Uuid),

    /// Follow the latest manifest without creating a checkpoint.
    ///
    /// This mode performs no object-store writes and provides no protection from garbage
    /// collection. Reads using an older manifest may fail if referenced objects are deleted.
    /// This mode is useful for read-only access to a database that is not being actively written
    /// to, for mirrored databases where manifest changes might not be allowed, or for readers
    /// that are willing to handle missing objects gracefully.
    FollowLatest,
}

/// Where a reader stops replaying the WAL when it builds its state.
///
/// This is only reached when replay is wanted at all; a reader configured with
/// [`DbReaderOptions::skip_wal_replay`] reads no WAL, which
/// [`WalReplayEnd::for_reader`] expresses as `None`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WalReplayEnd {
    /// Stop at the manifest's `next_wal_sst_id`, replaying exactly the WAL that
    /// the manifest itself records as durable.
    Manifest,

    /// Ask the configured WAL reader for the newest WAL file and replay through it,
    /// picking up writes made after the manifest was written.
    Latest,
}

impl WalReplayEnd {
    /// Returns `None` when the reader is configured to skip WAL replay, in which
    /// case it observes only the state recorded in the manifest (L0 and below).
    fn for_reader(mode: DbReaderMode, options: &DbReaderOptions) -> Option<Self> {
        if options.skip_wal_replay {
            return None;
        }
        Some(match mode {
            // A pinned checkpoint reads the state its manifest captured, so it
            // stops at that manifest's WAL boundary instead of following WAL
            // files written after the checkpoint was taken.
            DbReaderMode::Checkpoint(_) => Self::Manifest,
            DbReaderMode::ManagedCheckpoint | DbReaderMode::FollowLatest => Self::Latest,
        })
    }
}

/// Read-only interface for accessing a database from either
/// the latest persistent state or from an arbitrary checkpoint.
pub struct DbReader {
    inner: Arc<DbReaderInner>,
    task_executor: MessageHandlerExecutor,
}

struct DbReaderInner {
    manifest_store: Arc<ManifestStore>,
    table_store: Arc<TableStore>,
    wal_reader: Arc<dyn WalReaderTrait>,
    options: DbReaderOptions,
    mode: DbReaderMode,
    state: RwLock<Arc<ReaderState>>,
    system_clock: Arc<dyn SystemClock>,
    oracle: Arc<DbReaderOracle>,
    reader: Reader,
    status_manager: DbStatusManager,
    segment_extractor: Option<Arc<dyn PrefixExtractor>>,
    rand: Arc<DbRand>,
    /// Kept alive so the underlying `MetricsRecorder` is not dropped while
    /// metric handles in `DbStats` (and other stats structs) are still in use.
    /// See: https://github.com/slatedb/slatedb/issues/1469
    #[allow(dead_code)]
    recorder: slatedb_common::metrics::MetricsRecorderHelper,
}

#[derive(Debug)]
enum DbReaderMessage {
    PollManifest,
}

#[derive(Clone)]
struct ReaderState {
    manifest_id: u64,
    checkpoint: Option<Checkpoint>,
    manifest: Manifest,
    imm_memtable: VecDeque<Arc<ImmutableMemtable>>,
    last_wal_id: u64,
    last_remote_persisted_seq: u64,
}

static EMPTY_TABLE: LazyLock<Arc<KVTable>> = LazyLock::new(|| Arc::new(KVTable::new()));

impl DbStateReader for ReaderState {
    fn memtable(&self) -> Arc<KVTable> {
        Arc::clone(&EMPTY_TABLE)
    }

    fn imm_memtable(&self) -> &VecDeque<Arc<ImmutableMemtable>> {
        &self.imm_memtable
    }

    fn core(&self) -> &ManifestCore {
        &self.manifest.core
    }
}

impl From<&ReaderState> for VersionedManifest {
    fn from(state: &ReaderState) -> Self {
        Self::from_manifest(state.manifest_id, state.manifest.clone())
    }
}

impl DbReaderInner {
    async fn new(
        manifest_store: Arc<ManifestStore>,
        table_store: Arc<TableStore>,
        wal_store: Arc<WalTableStore>,
        wal_reader: Option<Arc<dyn WalReaderTrait>>,
        options: DbReaderOptions,
        mode: DbReaderMode,
        merge_operator: Option<MergeOperatorType>,
        segment_extractor: Option<Arc<dyn PrefixExtractor>>,
        system_clock: Arc<dyn SystemClock>,
        rand: Arc<DbRand>,
        recorder: slatedb_common::metrics::MetricsRecorderHelper,
        mut manifest: StoredManifest,
    ) -> Result<Self, SlateDBError> {
        let checkpoint =
            Self::get_or_create_checkpoint(&mut manifest, mode, &options, rand.clone()).await?;
        let (manifest_id, initial_manifest) = if let Some(checkpoint) = checkpoint.as_ref() {
            (
                checkpoint.manifest_id,
                manifest_store.read_manifest(checkpoint.manifest_id).await?,
            )
        } else {
            (manifest.id(), manifest.manifest().clone())
        };
        let status_manager = DbStatusManager::new_with_initial_values(
            initial_manifest.core.last_l0_seq,
            VersionedManifest::from_manifest(manifest_id, initial_manifest.clone()),
            BTreeSet::default(),
        );
        let wal_reader = wal_reader.unwrap_or_else(|| {
            Arc::new(
                crate::wal::slatedb::reader::SlateDbWalReader::new_with_status_manager(
                    wal_store,
                    &status_manager,
                    Arc::clone(&system_clock),
                    SlateDbWalReaderOptions::default(),
                ),
            )
        });

        let initial_state = Arc::new(
            Self::build_reader_state(
                checkpoint,
                manifest_id,
                initial_manifest,
                VecDeque::new(),
                WalReplayEnd::for_reader(mode, &options),
                Arc::clone(&table_store),
                wal_reader.as_ref(),
                &options,
                segment_extractor.as_ref(),
            )
            .await?,
        );

        let mono_clock = Arc::new(MonotonicClock::new(
            system_clock.clone(),
            initial_state.core().last_l0_clock_tick,
        ));

        let initial_durable_seq = initial_state
            .last_remote_persisted_seq
            .max(initial_state.core().last_l0_seq);
        status_manager.report_durable_seq(
            initial_state
                .last_remote_persisted_seq
                .max(initial_state.core().last_l0_seq),
        );
        status_manager.report_manifest_and_memtable_segments(
            VersionedManifest::from(initial_state.as_ref()),
            collect_touched_segments(initial_state.as_ref()),
        );
        let oracle = Arc::new(DbReaderOracle::new(
            initial_durable_seq,
            status_manager.clone(),
        ));

        let db_stats = DbStats::new(&recorder);

        let state = RwLock::new(initial_state);
        let reader = Reader::new(
            Arc::clone(&table_store),
            db_stats,
            Arc::clone(&mono_clock),
            oracle.clone(),
            merge_operator,
        );

        let inner = Self {
            manifest_store,
            table_store,
            wal_reader,
            options,
            mode,
            state,
            system_clock,
            oracle,
            reader,
            status_manager,
            segment_extractor,
            rand,
            recorder,
        };
        Ok(inner)
    }

    async fn get_or_create_checkpoint(
        manifest: &mut StoredManifest,
        mode: DbReaderMode,
        options: &DbReaderOptions,
        rand: Arc<DbRand>,
    ) -> Result<Option<Checkpoint>, SlateDBError> {
        match mode {
            DbReaderMode::Checkpoint(checkpoint_id) => Ok(Some(
                manifest
                    .db_state()
                    .find_checkpoint(checkpoint_id)
                    .ok_or(SlateDBError::CheckpointMissing(checkpoint_id))?
                    .clone(),
            )),
            DbReaderMode::ManagedCheckpoint => {
                let checkpoint_options = CheckpointOptions {
                    lifetime: Some(options.checkpoint_lifetime),
                    ..CheckpointOptions::default()
                };
                let checkpoint_id = rand.rng().gen_uuid();
                Ok(Some(
                    manifest
                        .write_checkpoint(checkpoint_id, &checkpoint_options)
                        .await?,
                ))
            }
            DbReaderMode::FollowLatest => Ok(None),
        }
    }

    async fn get_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<Bytes>, SlateDBError> {
        self.get_key_value_with_options(key, options)
            .await
            .map(|kv_opt| kv_opt.map(|kv| kv.value))
    }

    async fn get_key_value_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<KeyValue>, SlateDBError> {
        self.check_closed()?;
        let db_state = Arc::clone(&self.state.read());
        self.reader
            .get_key_value_with_options(key, options, db_state.as_ref(), None, None)
            .await
    }

    async fn scan_with_options(
        &self,
        range: BytesRange,
        options: &ScanOptions,
        prefix: Option<Bytes>,
    ) -> Result<DbIterator, SlateDBError> {
        self.check_closed()?;
        let db_state = Arc::clone(&self.state.read());
        self.reader
            .scan_with_options(
                range,
                options,
                ScanContext {
                    db_state: db_state.as_ref(),
                    write_batch_iter: None,
                    max_seq: None,
                    prefix,
                },
            )
            .await
    }

    fn should_reestablish_checkpoint(&self, latest: &ManifestCore) -> bool {
        let read_guard = self.state.read();
        let current_state = read_guard.core();
        latest.tree.last_compacted_l0_sst_view_id
            != current_state.tree.last_compacted_l0_sst_view_id
            || latest.last_l0_seq > current_state.last_l0_seq
            || latest.tree.compacted != current_state.tree.compacted
            // RFC-0024: segment-only progress (per-segment compactions,
            // drains, segment-set changes) is invisible to the root-tree
            // diff above. Structural equality on `Segment` covers each
            // segment's tree state, so any change retires the snapshot.
            || latest.segments != current_state.segments
    }

    async fn replace_checkpoint(
        &self,
        stored_manifest: &mut StoredManifest,
    ) -> Result<Checkpoint, SlateDBError> {
        let current_checkpoint_id = self
            .state
            .read()
            .checkpoint
            .as_ref()
            .expect("managed reader must have a checkpoint")
            .id;
        let options = CheckpointOptions {
            lifetime: Some(self.options.checkpoint_lifetime),
            ..CheckpointOptions::default()
        };
        let new_checkpoint_id = self.rand.rng().gen_uuid();
        stored_manifest
            .replace_checkpoint(current_checkpoint_id, new_checkpoint_id, &options)
            .await
    }

    async fn reestablish_checkpoint(&self, checkpoint: Checkpoint) -> Result<(), SlateDBError> {
        let new_state = self.rebuild_checkpoint_state(checkpoint).await?;
        self.install_state(new_state);
        Ok(())
    }

    fn install_state(&self, new_state: ReaderState) {
        let durable_seq = new_state.last_remote_persisted_seq;
        let versioned_manifest = VersionedManifest::from(&new_state);
        let touched_segments = collect_touched_segments(&new_state);
        self.oracle.advance_durable_seq(durable_seq);
        let mut write_guard = self.state.write();
        *write_guard = Arc::new(new_state);
        drop(write_guard);
        self.status_manager
            .report_manifest_and_memtable_segments(versioned_manifest, touched_segments);
    }

    async fn maybe_replay_new_wals(&self) -> Result<(), SlateDBError> {
        if self.options.skip_wal_replay {
            return Ok(());
        }
        let current_state = Arc::clone(&self.state.read());
        let mut imm_memtable = current_state.imm_memtable().clone();
        let (last_wal_id, last_committed_seq) = Self::replay_wal_into(
            Arc::clone(&self.table_store),
            self.wal_reader.as_ref(),
            &self.options,
            current_state.core(),
            &mut imm_memtable,
            WalReplayEnd::Latest,
            self.segment_extractor.as_ref(),
        )
        .await?;

        if last_wal_id > current_state.last_wal_id {
            self.oracle.advance_durable_seq(last_committed_seq);
            let mut write_guard = self.state.write();
            *write_guard = Arc::new(ReaderState {
                manifest_id: current_state.manifest_id,
                checkpoint: current_state.checkpoint.clone(),
                manifest: current_state.manifest.clone(),
                imm_memtable,
                last_wal_id,
                last_remote_persisted_seq: last_committed_seq,
            });
            drop(write_guard);
            self.status_manager
                .report_memtable_segments(collect_touched_segments(self.state.read().as_ref()));
        }
        Ok(())
    }

    async fn rebuild_checkpoint_state(
        &self,
        new_checkpoint: Checkpoint,
    ) -> Result<ReaderState, SlateDBError> {
        let manifest_id = new_checkpoint.manifest_id;
        let manifest = self.manifest_store.read_manifest(manifest_id).await?;
        self.rebuild_state(Some(new_checkpoint), manifest_id, manifest)
            .await
    }

    async fn rebuild_state(
        &self,
        checkpoint: Option<Checkpoint>,
        manifest_id: u64,
        manifest: Manifest,
    ) -> Result<ReaderState, SlateDBError> {
        let prior = self.state.read().clone();
        let mut imm_memtable = VecDeque::new();

        for table in prior.imm_memtable.iter() {
            let table_meta = table.table().metadata();
            if table_meta.last_seq <= manifest.core.last_l0_seq {
                // Skip since the entire table is older than L0+.
                continue;
            } else if table_meta.first_seq > manifest.core.last_l0_seq {
                // Keep the entire table since all rows are newer than L0+.
                imm_memtable.push_back(Arc::clone(table));
            } else {
                // The table has some rows that are newer than L0+ and some that are older. This
                // happens when the table spans multiple WAL files. Some of those WAL files can
                // have sequence numbers < manifest.core.last_l0_seq, while others have sequence
                // numbers > manifest.core.last_l0_seq. Retain only those that are more recent
                // than the manifest's last L0 sequence number.
                let filtered_table = table.filter_after_seq(
                    manifest.core.last_l0_seq,
                    self.segment_extractor.as_deref(),
                )?;
                // Push to the back because we are iterating prior from newest to oldest, and we
                // want the imm memtables in checkpoint state to be ordered the same way.
                imm_memtable.push_back(Arc::new(filtered_table));
            }
        }

        Self::build_reader_state(
            checkpoint,
            manifest_id,
            manifest,
            imm_memtable,
            WalReplayEnd::for_reader(self.mode, &self.options),
            Arc::clone(&self.table_store),
            self.wal_reader.as_ref(),
            &self.options,
            self.segment_extractor.as_ref(),
        )
        .await
    }

    async fn build_reader_state(
        checkpoint: Option<Checkpoint>,
        manifest_id: u64,
        manifest: Manifest,
        mut imm_memtable: VecDeque<Arc<ImmutableMemtable>>,
        replay_wals: Option<WalReplayEnd>,
        table_store: Arc<TableStore>,
        wal_reader: &dyn WalReaderTrait,
        options: &DbReaderOptions,
        segment_extractor: Option<&Arc<dyn PrefixExtractor>>,
    ) -> Result<ReaderState, SlateDBError> {
        let (last_wal_id, last_committed_seq) = match replay_wals {
            Some(replay_end) => {
                Self::replay_wal_into(
                    Arc::clone(&table_store),
                    wal_reader,
                    options,
                    &manifest.core,
                    &mut imm_memtable,
                    replay_end,
                    segment_extractor,
                )
                .await?
            }
            // Skipping replay reads no WAL at all: the reader stays at the
            // watermark it has already reached (the most recently read manifest)
            None => Self::replayed_watermark(&manifest.core, &imm_memtable),
        };

        Ok(ReaderState {
            manifest_id,
            checkpoint,
            manifest,
            imm_memtable,
            last_wal_id,
            last_remote_persisted_seq: last_committed_seq,
        })
    }

    async fn refresh_latest_manifest(&self) -> Result<(), SlateDBError> {
        let latest_manifest = self.manifest_store.read_latest_manifest().await?;
        self.apply_latest_manifest(latest_manifest).await
    }

    async fn apply_latest_manifest(
        &self,
        latest_manifest: VersionedManifest,
    ) -> Result<(), SlateDBError> {
        let manifest_id = latest_manifest.id;
        if manifest_id <= self.state.read().manifest_id {
            return self.maybe_replay_new_wals().await;
        }

        let new_state = self
            .rebuild_state(None, manifest_id, latest_manifest.manifest)
            .await?;
        self.install_state(new_state);
        info!("refreshed reader to latest manifest [manifest_id={manifest_id}]");
        Ok(())
    }

    async fn maybe_refresh_checkpoint(
        &self,
        stored_manifest: &mut StoredManifest,
    ) -> Result<(), SlateDBError> {
        let checkpoint = self
            .state
            .read()
            .checkpoint
            .clone()
            .expect("managed reader must have a checkpoint");
        let half_lifetime = self
            .options
            .checkpoint_lifetime
            .checked_div(2)
            .expect("Failed to divide checkpoint lifetime");
        let refresh_deadline = checkpoint
            .expire_time
            .expect("Expected checkpoint expiration time to be set")
            .sub(half_lifetime);
        if self.system_clock.now() > refresh_deadline {
            let refreshed_checkpoint = match stored_manifest
                .refresh_checkpoint(checkpoint.id, self.options.checkpoint_lifetime)
                .await
            {
                Ok(refreshed_checkpoint) => refreshed_checkpoint,
                Err(SlateDBError::CheckpointMissing(id)) => {
                    // Our self-established checkpoint lapsed (e.g. a stalled poll tick
                    // outlived the lease during an object-store outage) and the writer's
                    // GC reaped it. Re-establish a fresh checkpoint against the latest
                    // manifest instead of failing the reader permanently.
                    warn!("reader checkpoint missing, re-establishing [checkpoint_id={id}]");
                    let checkpoint = self.replace_checkpoint(stored_manifest).await?;
                    self.reestablish_checkpoint(checkpoint).await?;
                    return Ok(());
                }
                Err(e) => return Err(e),
            };

            // Update our local checkpoint copy so we know the latest expiration time
            // and can calculate future refresh deadlines correctly.
            {
                let mut write_guard = self.state.write();
                let current_state = write_guard.as_ref();
                // Defensively, only update checkpoint if the id and expiry still match.
                if current_state
                    .checkpoint
                    .as_ref()
                    .is_some_and(|current_checkpoint| {
                        current_checkpoint.id == checkpoint.id
                            && current_checkpoint.expire_time == checkpoint.expire_time
                    })
                {
                    let mut updated_state = current_state.clone();
                    updated_state.checkpoint = Some(refreshed_checkpoint.clone());
                    *write_guard = Arc::new(updated_state);
                }
            }

            info!(
                "refreshed checkpoint [checkpoint_id={}, expire_time={:?}]",
                checkpoint.id, refreshed_checkpoint.expire_time
            )
        }
        Ok(())
    }

    fn spawn_manifest_poller(
        self: &Arc<Self>,
        task_executor: &MessageHandlerExecutor,
    ) -> Result<(), SlateDBError> {
        let poller = ManifestPoller {
            inner: Arc::clone(self),
        };
        let (_tx, rx) = async_channel::unbounded();
        let result = task_executor.add_handler(
            DB_READER_TASK_NAME.to_string(),
            Box::new(poller),
            rx,
            &Handle::current(),
        );
        task_executor.monitor_on(&Handle::current())?;
        result
    }

    /// The `(last replayed WAL id, last committed seq)` the reader has already
    /// reached: the watermark of the most recently replayed table, or the
    /// manifest's own boundary when nothing has been replayed into `tables`.
    fn replayed_watermark(
        core: &ManifestCore,
        tables: &VecDeque<Arc<ImmutableMemtable>>,
    ) -> (u64, u64) {
        match tables.front() {
            Some(latest_replayed_table) => (
                latest_replayed_table.recent_flushed_wal_id(),
                latest_replayed_table.table().last_seq().unwrap_or(0),
            ),
            None => (core.replay_after_wal_id, core.last_l0_seq),
        }
    }

    async fn replay_wal_into(
        table_store: Arc<TableStore>,
        wal_reader: &dyn WalReaderTrait,
        reader_options: &DbReaderOptions,
        core: &ManifestCore,
        into_tables: &mut VecDeque<Arc<ImmutableMemtable>>,
        replay_end: WalReplayEnd,
        segment_extractor: Option<&Arc<dyn PrefixExtractor>>,
    ) -> Result<(u64, u64), SlateDBError> {
        let (mut replay_after_wal_id, mut last_committed_seq) =
            Self::replayed_watermark(core, into_tables);
        let wal_id_start = replay_after_wal_id
            .checked_add(1)
            .ok_or(SlateDBError::InvalidDBState)?;
        let wal_id_end = match replay_end {
            WalReplayEnd::Manifest => core.next_wal_sst_id,
            WalReplayEnd::Latest => wal_reader
                .last_wal_file_id(replay_after_wal_id)
                .await?
                .checked_add(1)
                .ok_or(SlateDBError::InvalidDBState)?,
        };
        if wal_id_start >= wal_id_end {
            return Ok((replay_after_wal_id, last_committed_seq));
        }

        let replay_options = WalReplayOptions {
            max_memtable_bytes: reader_options.max_memtable_bytes as usize,
            // Skip entries that we already have in `imm_memtable` (that might be above
            // last_l0_seq).
            min_seq: Some(last_committed_seq),
        };

        let wal_iter = wal_reader
            .iterator((wal_id_start..wal_id_end).into())
            .await?;
        let mut replay_iter = WalReplayIterator::for_wal_iterator(
            wal_iter,
            core,
            replay_options,
            Arc::clone(&table_store),
        )?;

        while let Some(replayed_table) = match replay_iter.next().await {
            Ok(Some(replayed_table)) => Some(replayed_table),
            Ok(None) => None,
            Err(SlateDBError::WalTruncated(_)) => None,
            Err(err) => return Err(err),
        } {
            // `last_wal_id` is a conservative watermark: a table that ends mid-file
            // is tagged with the last fully replayed WAL ID, which may equal the
            // watermark of the previous table.
            assert!(replayed_table.last_wal_id >= replay_after_wal_id);
            replay_after_wal_id = replayed_table.last_wal_id;
            if !replayed_table.table.is_empty() && replayed_table.last_seq > last_committed_seq {
                let first_seq = replayed_table
                    .table
                    .table()
                    .first_seq()
                    .expect("expected first_seq on non-empty table");
                // The entire table should be newer than the last committed seq, since we filtered
                // out entries <= last_committed_seq when creating the replay iterator.
                assert!(first_seq > last_committed_seq);
                last_committed_seq = replayed_table.last_seq;
                if let Some(extractor) = segment_extractor {
                    Self::record_replayed_touched_segments(
                        extractor.as_ref(),
                        &replayed_table.table,
                    )?;
                }
                let imm_memtable =
                    ImmutableMemtable::new(replayed_table.table, replayed_table.last_wal_id);
                into_tables.push_front(Arc::new(imm_memtable));
            }
        }

        Ok((replay_after_wal_id, last_committed_seq))
    }

    /// Re-derive each replayed entry's segment prefix (RFC-0024) and record the
    /// table's touched-segment set, mirroring the writer's replay path. Durable
    /// WAL entries were validated when accepted, so the antichain check is not
    /// re-run; an empty/absent prefix under the configured extractor remains a
    /// hard error.
    fn record_replayed_touched_segments(
        extractor: &dyn PrefixExtractor,
        table: &WritableKVTable,
    ) -> Result<(), SlateDBError> {
        let mut touched_segments: BTreeSet<Bytes> = BTreeSet::new();
        let mut iter = table.table().iter();
        while let Some(entry) = iter.next_sync() {
            touched_segments.insert(extract_segment_prefix(extractor, &entry.key)?);
        }
        table.record_touched_segments(touched_segments);
        Ok(())
    }

    /// Returns the latest database status.
    ///
    /// This is a snapshot of the current state and will not update automatically.
    /// Use [`subscribe`](DbReader::subscribe) to receive real-time updates.
    pub(crate) fn status(&self) -> DbStatus {
        self.status_manager.status()
    }

    /// Returns an error if the reader has been closed.
    ///
    /// ## Returns
    /// - `Ok(())` if the reader is still open.
    /// - `Err(SlateDBError::Closed)` if the reader was closed successfully (state.result_reader()
    ///   returns Ok(())).
    /// - `Err(e)` if the reader was closed with an error, where `e` is the error
    ///   (state.result_reader() returns Err(e)).
    pub(crate) fn check_closed(&self) -> Result<(), SlateDBError> {
        let closed_result_reader = self.status_manager.result_reader();
        if let Some(result) = closed_result_reader.read() {
            return match result {
                Ok(()) => Err(SlateDBError::Closed),
                Err(e) => Err(e),
            };
        }
        Ok(())
    }
}

struct ManifestPoller {
    inner: Arc<DbReaderInner>,
}

#[async_trait]
impl MessageHandler<DbReaderMessage> for ManifestPoller {
    fn tickers(&mut self) -> Vec<MessageTickerDef<DbReaderMessage>> {
        vec![MessageTickerDef::new(
            self.inner.options.manifest_poll_interval,
            Box::new(|| DbReaderMessage::PollManifest),
        )]
    }

    async fn handle(&mut self, message: DbReaderMessage) -> Result<(), SlateDBError> {
        assert!(matches!(message, DbReaderMessage::PollManifest));
        match self.inner.mode {
            DbReaderMode::ManagedCheckpoint => {
                let mut manifest = StoredManifest::load(
                    Arc::clone(&self.inner.manifest_store),
                    self.inner.system_clock.clone(),
                )
                .await?;

                let latest_manifest = manifest.manifest();
                if self
                    .inner
                    .should_reestablish_checkpoint(&latest_manifest.core)
                {
                    let checkpoint = self.inner.replace_checkpoint(&mut manifest).await?;
                    self.inner.reestablish_checkpoint(checkpoint).await?;
                } else {
                    self.inner.maybe_replay_new_wals().await?;
                }

                self.inner.maybe_refresh_checkpoint(&mut manifest).await
            }
            DbReaderMode::FollowLatest => {
                let result = self.inner.refresh_latest_manifest().await;
                if let Err(error) = result {
                    warn!("failed to refresh reader to latest manifest [error={error:?}]");
                }
                Ok(())
            }
            // No polling is needed for a pinned checkpoint, so we just return Ok(()).
            DbReaderMode::Checkpoint(_) => Ok(()),
        }
    }

    async fn cleanup(
        &mut self,
        _messages: BoxStream<'async_trait, DbReaderMessage>,
        _result: Result<(), SlateDBError>,
    ) -> Result<(), SlateDBError> {
        if self.inner.mode != DbReaderMode::ManagedCheckpoint {
            return Ok(());
        }
        let mut manifest = StoredManifest::load(
            Arc::clone(&self.inner.manifest_store),
            self.inner.system_clock.clone(),
        )
        .await?;
        let checkpoint_id = self
            .inner
            .state
            .read()
            .checkpoint
            .as_ref()
            .expect("managed reader must have a checkpoint")
            .id;
        info!(
            "deleting reader established checkpoint for shutdown [checkpoint_id={}]",
            checkpoint_id
        );
        manifest.delete_checkpoint(checkpoint_id).await?;
        Ok(())
    }
}

impl DbReader {
    fn validate_options(mode: DbReaderMode, options: &DbReaderOptions) -> Result<(), SlateDBError> {
        if mode != DbReaderMode::ManagedCheckpoint {
            return Ok(());
        }
        if options.checkpoint_lifetime.as_millis() < 1000 {
            return Err(SlateDBError::InvalidCheckpointLifetime(
                options.checkpoint_lifetime,
            ));
        }

        let double_poll_interval = options.manifest_poll_interval.checked_mul(2).ok_or(
            SlateDBError::InvalidManifestPollInterval(options.manifest_poll_interval),
        )?;
        if options.checkpoint_lifetime < double_poll_interval {
            return Err(SlateDBError::CheckpointLifetimeTooShort {
                lifetime: options.checkpoint_lifetime,
                interval: double_poll_interval,
            });
        }
        Ok(())
    }

    /// Preload the disk cache from the current manifest state.
    pub(crate) async fn preload_cache(
        &self,
        cached_obj_store: &CachedObjectStore,
        path: Path,
    ) -> Result<(), SlateDBError> {
        let state = Arc::clone(&self.inner.state.read());
        let external_ssts = state.manifest.external_ssts();
        let path_resolver = PathResolver::new_with_external_ssts(path, external_ssts);
        let cache_opts = &self.inner.options.object_store_cache_options;
        crate::utils::preload_cache_from_manifest(
            &state.manifest.core,
            cached_obj_store,
            &path_resolver,
            cache_opts.preload_disk_cache_on_startup,
            cache_opts.max_cache_size_bytes.unwrap_or(usize::MAX),
        )
        .await
    }

    /// Creates a database reader that can read the contents of a database (but cannot write any
    /// data). [`DbReaderMode`] controls whether the reader manages a checkpoint, remains pinned to
    /// a supplied checkpoint, or follows the latest manifest without garbage-collection
    /// protection.
    pub async fn open<P: Into<Path>>(
        path: P,
        object_store: Arc<dyn ObjectStore>,
        mode: DbReaderMode,
        options: DbReaderOptions,
    ) -> Result<Self, crate::Error> {
        // Use the builder API internally
        Self::builder(path, object_store)
            .with_options(options)
            .with_reader_mode(mode)
            .build()
            .await
    }

    /// Creates a new builder for a database reader at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the database.
    /// * `object_store` - The object store to use.
    ///
    /// # Returns
    ///
    /// A `DbReaderBuilder` that can be used to configure and build a `DbReader`.
    ///
    /// # Examples
    ///
    /// ```
    /// use {
    ///     slatedb::{
    ///         object_store::{memory::InMemory, ObjectStore},
    ///         Db, DbReader, Error,
    ///     },
    ///     std::sync::Arc,
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     // First create a database
    ///     let db = Db::open("test_db", Arc::clone(&object_store)).await?;
    ///     db.close().await?;
    ///     // Then open a reader
    ///     let reader = DbReader::builder("test_db", object_store).build().await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn builder<P: Into<Path>>(
        path: P,
        object_store: Arc<dyn ObjectStore>,
    ) -> crate::db::builder::DbReaderBuilder<P> {
        crate::db::builder::DbReaderBuilder::new(path, object_store)
    }

    pub(crate) async fn open_internal(
        manifest_store: Arc<ManifestStore>,
        table_store: Arc<TableStore>,
        wal_store: Arc<WalTableStore>,
        mode: DbReaderMode,
        wal_reader: Option<Arc<dyn WalReaderTrait>>,
        merge_operator: Option<MergeOperatorType>,
        segment_extractor: Option<Arc<dyn PrefixExtractor>>,
        options: DbReaderOptions,
        system_clock: Arc<dyn SystemClock>,
        rand: Arc<DbRand>,
        recorder: slatedb_common::metrics::MetricsRecorderHelper,
    ) -> Result<Self, SlateDBError> {
        Self::validate_options(mode, &options)?;

        let manifest =
            StoredManifest::load(Arc::clone(&manifest_store), system_clock.clone()).await?;
        if !manifest.db_state().initialized {
            return Err(SlateDBError::InvalidDBState);
        }

        manifest
            .db_state()
            .validate_extractor_configuration(segment_extractor.as_deref())?;

        let inner = Arc::new(
            DbReaderInner::new(
                manifest_store,
                table_store,
                wal_store,
                wal_reader,
                options,
                mode,
                merge_operator,
                segment_extractor,
                system_clock.clone(),
                rand,
                recorder,
                manifest,
            )
            .await?,
        );
        let task_executor = MessageHandlerExecutor::new(
            Arc::new(inner.status_manager.clone()),
            system_clock.clone(),
        );

        // Pinned checkpoints never advance. Managed checkpoints and unprotected readers both
        // poll for newer database state according to `DbReaderOptions`.
        if !matches!(mode, DbReaderMode::Checkpoint(_)) {
            inner.spawn_manifest_poller(&task_executor)?;
        }

        Ok(Self {
            inner,
            task_executor,
        })
    }

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
    ///
    /// ## Examples
    ///
    /// ```
    /// use {
    ///     slatedb::{
    ///         config::DbReaderOptions,
    ///         object_store::{memory::InMemory, ObjectStore},
    ///         Db, DbReader, DbReaderMode, Error,
    ///     },
    ///     std::sync::Arc,
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", Arc::clone(&object_store)).await?;
    ///     db.put(b"key", b"value").await?;
    ///     db.flush().await?;
    ///
    ///     let reader = DbReader::open(
    ///         "test_db",
    ///         Arc::clone(&object_store),
    ///         DbReaderMode::ManagedCheckpoint,
    ///         DbReaderOptions::default(),
    ///     )
    ///     .await?;
    ///     assert_eq!(reader.get(b"key").await?, Some("value".into()));
    ///     Ok(())
    /// }
    /// ```
    pub async fn get<K: AsRef<[u8]> + Send>(&self, key: K) -> Result<Option<Bytes>, crate::Error> {
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
    /// - `options`: the read options to use (Note that [`ReadOptions::read_level`] has no effect
    ///   for readers, which can only observe committed state).
    ///
    /// ## Returns
    /// - `Result<Option<Bytes>, Error>`:
    ///     - `Some(Bytes)`: the value if it exists
    ///     - `None`: if the value does not exist
    ///
    /// ## Errors
    /// - `Error`: if there was an error getting the value
    ///
    /// ## Examples
    ///
    /// ```
    /// use {
    ///     slatedb::{
    ///         config::{DbReaderOptions, ReadOptions},
    ///         object_store::{memory::InMemory, ObjectStore},
    ///         Db, DbReader, DbReaderMode, Error,
    ///     },
    ///     std::sync::Arc,
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", Arc::clone(&object_store)).await?;
    ///     db.put(b"key", b"value").await?;
    ///     db.flush().await?;
    ///
    ///     let reader = DbReader::open(
    ///         "test_db",
    ///         Arc::clone(&object_store),
    ///         DbReaderMode::ManagedCheckpoint,
    ///         DbReaderOptions::default(),
    ///     )
    ///     .await?;
    ///     assert_eq!(
    ///         db.get_with_options(b"key", &ReadOptions::default()).await?,
    ///         Some("value".into())
    ///     );
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<Bytes>, crate::Error> {
        self.inner
            .get_with_options(key, options)
            .await
            .map_err(Into::into)
    }

    /// Get a key-value pair from the reader with default read options.
    pub async fn get_key_value<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
    ) -> Result<Option<KeyValue>, crate::Error> {
        self.get_key_value_with_options(key, &ReadOptions::default())
            .await
    }

    /// Get a key-value pair from the reader with custom read options.
    pub async fn get_key_value_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<KeyValue>, crate::Error> {
        let kv = self
            .inner
            .get_key_value_with_options(key, options)
            .await
            .map_err(crate::Error::from)?;
        Ok(kv)
    }

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
    ///
    /// ## Examples
    ///
    /// ```
    /// use {
    ///     slatedb::{
    ///         config::DbReaderOptions,
    ///         object_store::{memory::InMemory, ObjectStore},
    ///         Db, DbReader, DbReaderMode, Error,
    ///     },
    ///     std::sync::Arc,
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", Arc::clone(&object_store)).await?;
    ///     db.put(b"a", b"a_value").await?;
    ///     db.put(b"b", b"b_value").await?;
    ///     db.flush().await?;
    ///
    ///     let reader = DbReader::open(
    ///         "test_db",
    ///         Arc::clone(&object_store),
    ///         DbReaderMode::ManagedCheckpoint,
    ///         DbReaderOptions::default(),
    ///     )
    ///     .await?;
    ///     let mut iter = reader.scan("a".."b").await?;
    ///     let kv = iter.next().await?.unwrap();
    ///     assert_eq!(kv.key.as_ref(), b"a");
    ///     assert_eq!(kv.value.as_ref(), b"a_value");
    ///     assert_eq!(None, iter.next().await?);
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan<T>(&self, range: T) -> Result<DbIterator, crate::Error>
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
    /// - `options`: the read options to use (Note that [`ReadOptions::read_level`] has no effect
    ///   for readers, which can only observe committed state).
    ///
    /// ## Errors
    /// - `Error`: if there was an error scanning the range of keys
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, DbReader, DbReaderMode, config::DbReaderOptions, config::ScanOptions, config::DurabilityLevel, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", Arc::clone(&object_store)).await?;
    ///     db.put(b"a", b"a_value").await?;
    ///     db.put(b"b", b"b_value").await?;
    ///     db.flush().await?;
    ///
    ///     let reader = DbReader::open(
    ///       "test_db",
    ///       Arc::clone(&object_store),
    ///       DbReaderMode::ManagedCheckpoint,
    ///       DbReaderOptions::default(),
    ///     ).await?;
    ///     let mut iter = reader.scan_with_options("a".."b", &ScanOptions {
    ///         read_ahead_bytes: 1024 * 1024,
    ///         ..ScanOptions::default()
    ///     }).await?;
    ///     let kv = iter.next().await?.unwrap();
    ///     assert_eq!(kv.key.as_ref(), b"a");
    ///     assert_eq!(kv.value.as_ref(), b"a_value");
    ///     assert_eq!(None, iter.next().await?);
    ///     Ok(())
    /// }
    pub async fn scan_with_options<T>(
        &self,
        range: T,
        options: &ScanOptions,
    ) -> Result<DbIterator, crate::Error>
    where
        T: ByteRangeBounds + Send,
    {
        let start = range.start_bound().map(Bytes::copy_from_slice);
        let end = range.end_bound().map(Bytes::copy_from_slice);
        let range = BytesRange::from((start, end));
        self.inner
            .scan_with_options(range, options, None)
            .await
            .map_err(Into::into)
    }

    /// Scan keys that share the provided prefix, restricted to `subrange`,
    /// using the default scan options.
    ///
    /// The subrange bounds are key *suffixes* interpreted relative to the
    /// prefix: a bound `s` selects the full key `prefix ++ s`. Pass `..` to
    /// scan the prefix's entire keyspace.
    ///
    /// ## Arguments
    /// - `prefix`: the key prefix to scan
    /// - `subrange`: the range of key suffixes (relative to `prefix`) to scan; `..` scans all keys
    ///   with the prefix
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    pub async fn scan_prefix<P, T>(
        &self,
        prefix: P,
        subrange: T,
    ) -> Result<DbIterator, crate::Error>
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
    /// - `subrange`: the range of key suffixes (relative to `prefix`) to scan; `..` scans all keys
    ///   with the prefix
    /// - `options`: the scan options to use
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    pub async fn scan_prefix_with_options<P, T>(
        &self,
        prefix: P,
        subrange: T,
        options: &ScanOptions,
    ) -> Result<DbIterator, crate::Error>
    where
        P: AsRef<[u8]> + Send,
        T: ByteRangeBounds + Send,
    {
        let prefix = Bytes::copy_from_slice(prefix.as_ref());
        let range = BytesRange::from_prefix_and_subrange(prefix.as_ref(), subrange);
        self.inner
            .scan_with_options(range, options, Some(prefix))
            .await
            .map_err(Into::into)
    }

    /// Close the database reader.
    ///
    /// ## Returns
    /// - `Result<(), Error>`: if there was an error closing the reader
    ///
    /// ## Examples
    ///
    /// ```
    /// use {
    ///     slatedb::{
    ///         config::DbReaderOptions,
    ///         object_store::{memory::InMemory, ObjectStore},
    ///         Db, DbReader, DbReaderMode, Error,
    ///     },
    ///     std::sync::Arc,
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store.clone()).await?;
    ///     let options = DbReaderOptions::default();
    ///     let reader = DbReader::open(
    ///         "test_db",
    ///         object_store.clone(),
    ///         DbReaderMode::ManagedCheckpoint,
    ///         options,
    ///     )
    ///     .await?;
    ///     reader.close().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn close(&self) -> Result<(), crate::Error> {
        self.task_executor
            .shutdown_task(DB_READER_TASK_NAME)
            .await
            .map_err(Into::<crate::Error>::into)?;

        if let Err(e) = self.inner.table_store.close_cache().await {
            warn!("failed to close block cache [error={:?}]", e);
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl DbReadOps for DbReader {
    async fn get_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<Bytes>, crate::Error> {
        DbReader::get_with_options(self, key, options).await
    }

    async fn get_key_value_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<KeyValue>, crate::Error> {
        DbReader::get_key_value_with_options(self, key, options).await
    }

    async fn scan_with_options<T>(
        &self,
        range: T,
        options: &ScanOptions,
    ) -> Result<DbIterator, crate::Error>
    where
        T: ByteRangeBounds + Send,
    {
        DbReader::scan_with_options(self, range, options).await
    }

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
        DbReader::scan_prefix_with_options(self, prefix, subrange, options).await
    }
}

impl DbMetadataOps for DbReader {
    fn manifest(&self) -> VersionedManifest {
        let state = Arc::clone(&self.inner.state.read());
        VersionedManifest::from(state.as_ref())
    }

    fn subscribe(&self) -> tokio::sync::watch::Receiver<DbStatus> {
        self.inner.status_manager.subscribe()
    }

    fn status(&self) -> DbStatus {
        self.inner.status()
    }
}

impl DbReader {
    /// See [`DbMetadataOps::manifest`].
    pub fn manifest(&self) -> VersionedManifest {
        <Self as DbMetadataOps>::manifest(self)
    }

    /// See [`DbMetadataOps::subscribe`].
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<DbStatus> {
        <Self as DbMetadataOps>::subscribe(self)
    }

    /// See [`DbMetadataOps::status`].
    pub fn status(&self) -> DbStatus {
        <Self as DbMetadataOps>::status(self)
    }
}

#[async_trait]
impl DbCacheManagerOps for DbReader {
    async fn warm_sst(
        &self,
        sst_id: SsTableId,
        targets: &[CacheTarget],
    ) -> Result<(), crate::Error> {
        self.inner.check_closed()?;
        let manifest = self.manifest();
        db_cache_manager::warm_sst_impl(&self.inner.table_store, manifest.core(), sst_id, targets)
            .await
    }

    async fn evict_cached_sst(
        &self,
        sst: &SsTableHandle,
        targets: &[CacheTarget],
    ) -> Result<(), crate::Error> {
        self.inner.check_closed()?;
        let manifest = self.manifest();
        db_cache_manager::evict_cached_sst_impl(
            &self.inner.table_store,
            manifest.core(),
            sst,
            targets,
        )
        .await
    }

    async fn flush_cache_to_disk(&self) -> Result<(), crate::Error> {
        self.inner.check_closed()?;
        db_cache_manager::flush_cache_to_disk_impl(&self.inner.table_store).await
    }
}

#[cfg(test)]
mod tests {
    use crate::wal::slatedb::reader::SlateDbWalReaderOptions;
    use {
        super::{DbReaderMessage, ManifestPoller, ReaderState, WalReplayEnd},
        crate::{
            block_cache_policy::BlockCachePolicy,
            clock::MonotonicClock,
            config::{
                CheckpointOptions, CheckpointScope, CloseOptions, FlushOptions, FlushType,
                MergeOptions, PutOptions, Settings, WriteOptions,
            },
            db_reader::{DbReader, DbReaderInner, DbReaderMode, DbReaderOptions},
            db_state::SstType,
            db_stats::DbStats,
            db_status::DbStatusManager,
            dispatcher::MessageHandler,
            error::SlateDBError,
            format::sst::SsTableFormat,
            iter::IterationOrder,
            manifest::{
                store::{ManifestStore, StoredManifest},
                Manifest, ManifestCore, VersionedManifest,
            },
            mem_table::{ImmutableMemtable, WritableKVTable},
            merge_operator::MergeOperatorType,
            oracle::DbReaderOracle,
            paths::PathResolver,
            proptest_util::{rng::new_test_rng, sample},
            reader::Reader,
            tablestore::{TableStore, TableStoreKind},
            test_utils,
            types::RowEntry,
            wal::{
                slatedb::store::WalTableStore, WalError, WalFileRange, WalIterator,
                WalReader as WalReaderTrait, WalRows,
            },
            CloseReason, Db,
        },
        bytes::Bytes,
        fail_parallel::FailPointRegistry,
        object_store::{memory::InMemory, path::Path, ObjectStore, ObjectStoreExt},
        rstest::rstest,
        slatedb_common::{
            clock::{DefaultSystemClock, SystemClock},
            DbRand, MockSystemClock,
        },
        std::{
            collections::{BTreeMap, BTreeSet, VecDeque},
            sync::{
                atomic::{AtomicUsize, Ordering},
                Arc,
            },
            time::Duration,
        },
        uuid::Uuid,
    };

    struct EmptyTestWalIterator;

    #[async_trait::async_trait]
    impl WalIterator for EmptyTestWalIterator {
        async fn next(&mut self) -> Result<Option<WalRows>, WalError> {
            Ok(None)
        }
    }

    #[derive(Default)]
    struct CountingWalReader {
        iterator_calls: AtomicUsize,
        last_wal_file_id_calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl WalReaderTrait for CountingWalReader {
        async fn iterator(
            &self,
            _wal_file_id_range: WalFileRange,
        ) -> Result<Box<dyn WalIterator>, WalError> {
            self.iterator_calls.fetch_add(1, Ordering::Relaxed);
            Ok(Box::new(EmptyTestWalIterator))
        }

        async fn last_wal_file_id(&self, _replay_after_wal_id: u64) -> Result<u64, WalError> {
            self.last_wal_file_id_calls.fetch_add(1, Ordering::Relaxed);
            Ok(10)
        }
    }

    #[tokio::test]
    async fn should_get_value_from_db() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let key = b"test_key";
        let value = b"test_value";

        db.put(key, value)
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap();
        db.flush().await.unwrap();

        let reader = DbReader::open(
            path.clone(),
            Arc::clone(&object_store),
            DbReaderMode::ManagedCheckpoint,
            DbReaderOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(
            reader.get(key).await.unwrap(),
            Some(Bytes::from_static(value))
        );
    }

    #[tokio::test]
    async fn db_reader_builder_should_use_custom_wal_reader() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_custom_wal_reader");
        let db = Db::open(path.clone(), Arc::clone(&object_store))
            .await
            .unwrap();
        db.close().await.unwrap();

        let wal_reader = Arc::new(CountingWalReader::default());
        let reader = DbReader::builder(path, object_store)
            .with_reader_mode(DbReaderMode::FollowLatest)
            .with_wal_reader(wal_reader.clone())
            .with_options(DbReaderOptions {
                manifest_poll_interval: Duration::from_secs(60 * 60),
                ..DbReaderOptions::default()
            })
            .build()
            .await
            .unwrap();

        assert!(wal_reader.last_wal_file_id_calls.load(Ordering::Relaxed) > 0);
        assert!(wal_reader.iterator_calls.load(Ordering::Relaxed) > 0);
        reader.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_return_current_versioned_manifest() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_reader_manifest_accessor");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        db.put(b"test_key", b"test_value").await.unwrap();
        db.flush().await.unwrap();

        let reader = DbReader::open(
            path,
            object_store,
            DbReaderMode::ManagedCheckpoint,
            DbReaderOptions::default(),
        )
        .await
        .unwrap();

        let manifest = reader.manifest();
        let expected: VersionedManifest =
            VersionedManifest::from(reader.inner.state.read().as_ref());
        assert_eq!(manifest, expected);
    }

    #[tokio::test]
    async fn should_get_latest_value_from_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let key = b"test_key";
        let value1 = b"test_value";
        let value2 = b"updated_value";

        db.put(key, value1).await.unwrap();
        db.flush().await.unwrap();
        db.put(key, value2).await.unwrap();
        let checkpoint_result = db
            .create_checkpoint(CheckpointScope::All, &CheckpointOptions::default())
            .await
            .unwrap();

        let reader = DbReader::open_internal(
            test_provider.manifest_store(),
            test_provider.table_store(),
            test_provider.wal_store(),
            DbReaderMode::Checkpoint(checkpoint_result.id),
            None,
            None,
            None,
            DbReaderOptions::default(),
            test_provider.system_clock.clone(),
            test_provider.rand.clone(),
            slatedb_common::metrics::MetricsRecorderHelper::noop(),
        )
        .await
        .unwrap();

        assert_eq!(
            reader.get(key).await.unwrap(),
            Some(Bytes::from_static(value2))
        );
    }

    #[tokio::test]
    async fn should_get_from_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let key = b"test_key";
        let checkpoint_value = b"test_value";
        let updated_value = b"updated_value";

        db.put(key, checkpoint_value).await.unwrap();
        let checkpoint_result = db
            .create_checkpoint(CheckpointScope::All, &CheckpointOptions::default())
            .await
            .unwrap();
        db.put(key, updated_value).await.unwrap();

        let reader = DbReader::open(
            path.clone(),
            Arc::clone(&object_store),
            DbReaderMode::Checkpoint(checkpoint_result.id),
            DbReaderOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(
            reader.get(key).await.unwrap(),
            Some(Bytes::from_static(checkpoint_value))
        );
    }

    #[tokio::test]
    async fn should_report_memtable_segments_in_status() {
        // given
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_reader_subscribe_reports_memtable_segments";
        let db = Db::builder(path, object_store.clone())
            .with_settings(Settings::default())
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();
        let write_opts = WriteOptions {
            ..Default::default()
        };
        db.put_with_options(b"abc-1", b"v1", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        // when
        let reader = DbReader::builder(path, object_store.clone())
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();

        // then
        let prefixes: Vec<Bytes> = reader
            .status()
            .list_segments()
            .into_iter()
            .map(|seg| seg.prefix)
            .collect();
        assert_eq!(prefixes, vec![Bytes::from_static(b"abc")]);
    }

    #[tokio::test]
    async fn should_reject_reader_with_mismatched_extractor() {
        #[derive(Debug)]
        struct OtherExtractor;
        impl crate::prefix_extractor::PrefixExtractor for OtherExtractor {
            fn name(&self) -> &str {
                "other"
            }
            fn prefix_len(&self, _target: &crate::prefix_extractor::PrefixTarget) -> Option<usize> {
                Some(3)
            }
        }

        // given
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_reader_rejects_mismatched_extractor";
        let db = Db::builder(path, object_store.clone())
            .with_settings(Settings::default())
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();
        db.close().await.unwrap();

        // when
        let err = match DbReader::builder(path, object_store.clone())
            .with_segment_extractor(Arc::new(OtherExtractor))
            .build()
            .await
        {
            Ok(_) => panic!("expected mismatched-extractor error"),
            Err(err) => err,
        };

        // then
        assert_eq!(err.kind(), crate::ErrorKind::Invalid);

        // when
        // a segmented database also rejects a reader opened without any extractor
        let err = match DbReader::builder(path, object_store).build().await {
            Ok(_) => panic!("expected missing-extractor error"),
            Err(err) => err,
        };

        // then
        assert_eq!(err.kind(), crate::ErrorKind::Invalid);
        assert!(
            err.to_string()
                .contains("segment extractor configuration mismatch"),
            "unexpected error message: {err}"
        );
    }

    #[tokio::test]
    async fn should_list_checkpoint_segments_for_checkpoint_reader() {
        // given
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_reader_lists_checkpoint_segments";
        let db = Db::builder(path, object_store.clone())
            .with_settings(Settings::default())
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();
        let write_opts = WriteOptions {
            ..Default::default()
        };
        db.put_with_options(b"abc-1", b"v1", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();
        let checkpoint = db
            .create_checkpoint(CheckpointScope::All, &CheckpointOptions::default())
            .await
            .unwrap();
        db.put_with_options(b"xyz-1", b"v2", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();
        db.close().await.unwrap();

        // when
        let reader = DbReader::builder(path, object_store)
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .with_reader_mode(DbReaderMode::Checkpoint(checkpoint.id))
            .build()
            .await
            .unwrap();

        // then
        let segments: Vec<Bytes> = reader
            .status()
            .list_segments()
            .into_iter()
            .map(|seg| seg.prefix)
            .collect();
        assert_eq!(segments, vec![Bytes::from_static(b"abc")]);
    }

    #[tokio::test]
    async fn should_fail_if_db_is_uninitialized() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));
        let manifest_store = test_provider.manifest_store();

        let parent_manifest = Manifest::initial(ManifestCore::new());
        let parent_path = "/tmp/parent_store".to_string();
        let source_checkpoint_id = Uuid::new_v4();

        let _ = StoredManifest::store_uninitialized_clone(
            Arc::clone(&manifest_store),
            Manifest::cloned(
                &parent_manifest,
                parent_path,
                source_checkpoint_id,
                Arc::new(DbRand::default()),
            ),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let err = test_provider
            .new_db_reader(DbReaderOptions::default(), None, None)
            .await;
        assert!(matches!(err, Err(SlateDBError::InvalidDBState)));
    }

    #[tokio::test]
    async fn should_scan_from_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let checkpoint_key = b"checkpoint_key";
        let value = b"value";

        db.put(checkpoint_key, value).await.unwrap();
        let checkpoint_result = db
            .create_checkpoint(CheckpointScope::All, &CheckpointOptions::default())
            .await
            .unwrap();

        let post_checkpoint_key = b"post_checkpoint_key";
        db.put(post_checkpoint_key, value).await.unwrap();

        let reader = test_provider
            .new_db_reader(DbReaderOptions::default(), Some(checkpoint_result.id), None)
            .await
            .unwrap();

        let mut db_iter = reader.scan(..).await.unwrap();
        let mut table = BTreeMap::new();
        table.insert(
            Bytes::copy_from_slice(checkpoint_key),
            Bytes::copy_from_slice(value),
        );

        test_utils::assert_ranged_db_scan(&table, .., IterationOrder::Ascending, &mut db_iter)
            .await;
    }

    #[tokio::test(start_paused = true)]
    async fn follow_latest_should_refresh_without_object_store_writes() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_follow_latest_reader");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));
        let db = test_provider.new_db(Settings::default()).await.unwrap();

        let key = b"key";
        db.put(key, b"initial").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        let recording_store = Arc::new(test_utils::RecordingObjectStore::new(Arc::clone(
            &object_store,
        )));
        let reader_store: Arc<dyn ObjectStore> = recording_store.clone();
        let reader = DbReader::open(
            path,
            reader_store,
            DbReaderMode::FollowLatest,
            DbReaderOptions {
                manifest_poll_interval: Duration::from_millis(100),
                // FollowLatest does not create a checkpoint, so checkpoint validation is
                // intentionally inapplicable to this mode.
                checkpoint_lifetime: Duration::ZERO,
                ..DbReaderOptions::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(
            reader.get(key).await.unwrap(),
            Some(Bytes::from_static(b"initial"))
        );
        let initial_manifest = reader.manifest();
        let initial_manifest_id = initial_manifest.id();

        db.put(key, b"updated").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();
        let latest_manifest = test_provider
            .manifest_store()
            .read_latest_manifest()
            .await
            .unwrap();
        assert!(latest_manifest.id > initial_manifest_id);
        assert!(latest_manifest.manifest.core.checkpoints.is_empty());

        let mut poller = ManifestPoller {
            inner: Arc::clone(&reader.inner),
        };
        poller.handle(DbReaderMessage::PollManifest).await.unwrap();

        assert!(reader.manifest().id() >= latest_manifest.id);
        assert_eq!(
            reader.get(key).await.unwrap(),
            Some(Bytes::from_static(b"updated"))
        );

        let refreshed_manifest_id = reader.manifest().id();
        reader
            .inner
            .apply_latest_manifest(initial_manifest)
            .await
            .unwrap();
        assert_eq!(reader.manifest().id(), refreshed_manifest_id);
        assert!(recording_store.write_kinds().is_empty());

        reader.close().await.unwrap();
        assert!(recording_store.write_kinds().is_empty());
    }

    #[tokio::test]
    async fn follow_latest_refresh_failure_should_keep_last_good_state() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_follow_latest_refresh_failure");
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));
        let db = test_provider.new_db(Settings::default()).await.unwrap();

        db.put(b"key", b"value").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();
        db.close().await.unwrap();

        let reader = DbReader::open_internal(
            test_provider.manifest_store(),
            test_provider.table_store(),
            test_provider.wal_store(),
            DbReaderMode::FollowLatest,
            None,
            None,
            None,
            DbReaderOptions {
                manifest_poll_interval: Duration::from_secs(60 * 60),
                ..DbReaderOptions::default()
            },
            test_provider.system_clock.clone(),
            test_provider.rand.clone(),
            slatedb_common::metrics::MetricsRecorderHelper::noop(),
        )
        .await
        .unwrap();
        let manifest_id = reader.manifest().id();

        let manifest_store = test_provider.manifest_store();
        let mut saved_manifests = Vec::new();
        for manifest in manifest_store.list_manifests(..).await.unwrap() {
            let location = manifest.metadata.location;
            let bytes = object_store
                .get(&location)
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap();
            object_store.delete(&location).await.unwrap();
            saved_manifests.push((location, bytes));
        }

        let mut poller = ManifestPoller {
            inner: Arc::clone(&reader.inner),
        };
        poller.handle(DbReaderMessage::PollManifest).await.unwrap();

        assert_eq!(reader.manifest().id(), manifest_id);
        assert_eq!(
            reader.get(b"key").await.unwrap(),
            Some(Bytes::from_static(b"value"))
        );

        for (location, bytes) in saved_manifests {
            object_store.put(&location, bytes.into()).await.unwrap();
        }
        let db = test_provider.new_db(Settings::default()).await.unwrap();
        db.put(b"key", b"updated").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();
        db.close().await.unwrap();

        poller.handle(DbReaderMessage::PollManifest).await.unwrap();
        assert!(reader.manifest().id() > manifest_id);
        assert_eq!(
            reader.get(b"key").await.unwrap(),
            Some(Bytes::from_static(b"updated"))
        );
        reader.close().await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn should_reestablish_reader_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db_options = Settings {
            l0_sst_size_bytes: 256,
            ..Settings::default()
        };
        let db = test_provider.new_db(db_options).await.unwrap();
        let reader_options = DbReaderOptions {
            manifest_poll_interval: Duration::from_millis(10),
            ..DbReaderOptions::default()
        };
        let reader = test_provider
            .new_db_reader(reader_options, None, None)
            .await
            .unwrap();
        let manifest_store = test_provider.manifest_store();
        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        let initial_checkpoint_id = manifest.manifest.core.checkpoints.first().unwrap().id;

        let mut rng = new_test_rng(None);
        let table = sample::table(&mut rng, 256, 10);
        for (key, value) in &table {
            db.put(key, value).await.unwrap();
        }
        db.flush().await.unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;
        let mut db_iter = reader.scan(..).await.unwrap();
        test_utils::assert_ranged_db_scan(&table, .., IterationOrder::Ascending, &mut db_iter)
            .await;

        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert!(!manifest.manifest.core.checkpoints.is_empty());
        assert_eq!(
            None,
            manifest
                .manifest
                .core
                .find_checkpoint(initial_checkpoint_id)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn should_refresh_reader_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let _db = test_provider.new_db(Settings::default()).await;
        let reader_options = DbReaderOptions {
            manifest_poll_interval: Duration::from_millis(500),
            checkpoint_lifetime: Duration::from_millis(1000),
            ..DbReaderOptions::default()
        };

        let manifest_store = test_provider.manifest_store();
        let reader = test_provider
            .new_db_reader(reader_options, None, None)
            .await
            .unwrap();

        let initial_manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(1, initial_manifest.manifest.core.checkpoints.len());
        let initial_reader_checkpoint = initial_manifest
            .manifest
            .core
            .checkpoints
            .first()
            .unwrap()
            .clone();

        tokio::time::sleep(Duration::from_millis(5000)).await;

        let updated_manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(1, updated_manifest.manifest.core.checkpoints.len());
        let updated_reader_checkpoint = updated_manifest
            .manifest
            .core
            .checkpoints
            .first()
            .unwrap()
            .clone();
        assert_eq!(initial_reader_checkpoint.id, updated_reader_checkpoint.id);
        assert!(
            updated_reader_checkpoint.expire_time.unwrap()
                > initial_reader_checkpoint.expire_time.unwrap()
        );

        // The checkpoint is removed on shutdown
        reader.close().await.unwrap();
        let updated_manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(0, updated_manifest.manifest.core.checkpoints.len());
    }

    // Regression test for https://github.com/slatedb/slatedb/issues/1750.
    // It constructs DbReaderInner without the background manifest poller,
    // advances a mock clock past the first checkpoint refresh deadline, then
    // verifies the next manual poll does not write another manifest before
    // half of the refreshed checkpoint lifetime has elapsed.
    #[tokio::test]
    async fn should_not_refresh_reader_checkpoint_on_every_poll() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from(format!(
            "/tmp/test_db_reader_checkpoint_refresh_cadence_{}",
            Uuid::new_v4()
        ));
        let clock = Arc::new(MockSystemClock::new());
        let mut test_provider = TestProvider::new(path, Arc::clone(&object_store));
        test_provider.system_clock = clock.clone();

        let manifest_store = test_provider.manifest_store();
        let table_store = test_provider.table_store();

        // Seed the DB with its initial manifest. Opening a reader without a
        // user-provided checkpoint will add a second manifest containing the
        // self-established reader checkpoint.
        let stored_manifest = StoredManifest::create_new_db(
            Arc::clone(&manifest_store),
            ManifestCore::new(),
            clock.clone(),
        )
        .await
        .unwrap();
        let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();

        // Build DbReaderInner directly instead of DbReader so no spawned poller
        // can race the manual calls to maybe_refresh_checkpoint below.
        let inner = DbReaderInner::new(
            Arc::clone(&manifest_store),
            table_store,
            test_provider.wal_store(),
            None,
            DbReaderOptions {
                manifest_poll_interval: Duration::from_millis(100),
                checkpoint_lifetime: Duration::from_millis(1000),
                ..DbReaderOptions::default()
            },
            DbReaderMode::ManagedCheckpoint,
            None,
            None,
            clock.clone(),
            test_provider.rand.clone(),
            recorder,
            stored_manifest,
        )
        .await
        .unwrap();

        // Manifest 1 is the initial DB manifest. Manifest 2 is written when
        // DbReaderInner creates its checkpoint.
        let initial_manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(
            2,
            initial_manifests.len(),
            "expected initial db manifest plus reader checkpoint manifest"
        );

        // Move just past the first refresh deadline: the checkpoint was created
        // at t=0 with a 1000ms lifetime, so checkpoint_lifetime / 2 is 500ms.
        // This refresh is expected and should write exactly one new manifest.
        clock.advance(Duration::from_millis(501)).await;
        let mut stored_manifest = StoredManifest::load(Arc::clone(&manifest_store), clock.clone())
            .await
            .unwrap();
        inner
            .maybe_refresh_checkpoint(&mut stored_manifest)
            .await
            .unwrap();

        let manifests_after_first_refresh = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(3, manifests_after_first_refresh.len());

        // Simulate the next manifest poll 100ms later. Because the first
        // refresh extended the checkpoint lifetime to t=1501ms, the next
        // refresh deadline should be t=1001ms. A second manifest here shows
        // the in-memory checkpoint expiry was not updated after the refresh.
        clock.advance(Duration::from_millis(100)).await;
        let mut stored_manifest = StoredManifest::load(Arc::clone(&manifest_store), clock.clone())
            .await
            .unwrap();
        inner
            .maybe_refresh_checkpoint(&mut stored_manifest)
            .await
            .unwrap();

        // Correct behavior keeps the manifest count at 3.
        let manifests_after_second_poll = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(
            manifests_after_first_refresh.len(),
            manifests_after_second_poll.len(),
            "checkpoint refresh should not write another manifest until half of the refreshed lifetime has elapsed"
        );
    }

    // Regression test for https://github.com/slatedb/slatedb/issues/1888.
    // If a poll tick stalls past the refresh deadline (e.g. during an object
    // store brownout), the reader's checkpoint can expire and be reaped by the
    // writer's GC. The next refresh must re-establish a fresh checkpoint
    // instead of failing the reader permanently with CheckpointMissing.
    #[tokio::test]
    async fn should_reestablish_reader_checkpoint_when_missing() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from(format!(
            "/tmp/test_db_reader_reestablish_missing_checkpoint_{}",
            Uuid::new_v4()
        ));
        let clock = Arc::new(MockSystemClock::new());
        let mut test_provider = TestProvider::new(path, Arc::clone(&object_store));
        test_provider.system_clock = clock.clone();

        let manifest_store = test_provider.manifest_store();
        let table_store = test_provider.table_store();

        let stored_manifest = StoredManifest::create_new_db(
            Arc::clone(&manifest_store),
            ManifestCore::new(),
            clock.clone(),
        )
        .await
        .unwrap();
        let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();

        // Build DbReaderInner directly so no spawned poller can race the
        // manual call to maybe_refresh_checkpoint below.
        let inner = DbReaderInner::new(
            Arc::clone(&manifest_store),
            table_store,
            test_provider.wal_store(),
            None,
            DbReaderOptions {
                manifest_poll_interval: Duration::from_millis(100),
                checkpoint_lifetime: Duration::from_millis(1000),
                ..DbReaderOptions::default()
            },
            DbReaderMode::ManagedCheckpoint,
            None,
            None,
            clock.clone(),
            test_provider.rand.clone(),
            recorder,
            stored_manifest,
        )
        .await
        .unwrap();
        let reader_checkpoint_id = inner.state.read().checkpoint.as_ref().unwrap().id;

        // Simulate the writer's GC reaping the expired checkpoint.
        let mut stored_manifest = StoredManifest::load(Arc::clone(&manifest_store), clock.clone())
            .await
            .unwrap();
        stored_manifest
            .delete_checkpoint(reader_checkpoint_id)
            .await
            .unwrap();

        // Move past the refresh deadline (checkpoint_lifetime / 2) and poll.
        clock.advance(Duration::from_millis(501)).await;
        let mut stored_manifest = StoredManifest::load(Arc::clone(&manifest_store), clock.clone())
            .await
            .unwrap();
        inner
            .maybe_refresh_checkpoint(&mut stored_manifest)
            .await
            .unwrap();

        // The reader should have replaced the reaped checkpoint with a new one.
        let new_checkpoint_id = inner.state.read().checkpoint.as_ref().unwrap().id;
        assert_ne!(reader_checkpoint_id, new_checkpoint_id);
        let latest_manifest = manifest_store.read_latest_manifest().await.unwrap();
        let checkpoints = &latest_manifest.manifest.core.checkpoints;
        assert_eq!(1, checkpoints.len());
        assert_eq!(new_checkpoint_id, checkpoints[0].id);
    }

    #[tokio::test(start_paused = true)]
    async fn should_replay_new_wals() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));
        let db = test_provider.new_db(Settings::default()).await.unwrap();

        let reader_options = DbReaderOptions {
            manifest_poll_interval: Duration::from_millis(500),
            checkpoint_lifetime: Duration::from_millis(1000),
            ..DbReaderOptions::default()
        };

        let reader = test_provider
            .new_db_reader(reader_options, None, None)
            .await
            .unwrap();
        let key = b"test_key";
        let value = b"test_value";
        db.put(key, value)
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap();
        db.flush().await.unwrap();

        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(
            reader.get(key).await.unwrap(),
            Some(Bytes::from_static(value))
        );
    }

    #[tokio::test]
    async fn replay_wal_into_should_use_latest_existing_table_and_keep_newest_first_order() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_db_reader_replay_order");
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));
        let table_store = test_provider.table_store();
        let wal_store = test_provider.wal_store();

        write_wal_sst(
            Arc::clone(&wal_store),
            3,
            vec![RowEntry::new_value(b"stale_key", b"stale_value", 3)],
        )
        .await
        .unwrap();
        write_wal_sst(
            Arc::clone(&wal_store),
            4,
            vec![RowEntry::new_value(b"fresh_key", b"fresh_value", 4)],
        )
        .await
        .unwrap();

        let mut into_tables = VecDeque::new();
        into_tables.push_front(immutable_memtable(
            3,
            vec![RowEntry::new_value(b"stale_key", b"stale_value", 3)],
        ));
        into_tables.push_back(immutable_memtable(
            2,
            vec![RowEntry::new_value(b"older_key", b"older_value", 2)],
        ));

        let mut core = ManifestCore::new();
        core.next_wal_sst_id = 5;
        let status_manager = status_manager_for_core(&core);

        let (last_wal_id, last_committed_seq) = DbReaderInner::replay_wal_into(
            Arc::clone(&table_store),
            &native_wal_reader(&wal_store, &status_manager),
            &DbReaderOptions::default(),
            &core,
            &mut into_tables,
            WalReplayEnd::Manifest,
            None,
        )
        .await
        .unwrap();

        assert_eq!(last_wal_id, 4);
        assert_eq!(last_committed_seq, 4);

        let newest_replayed = into_tables.front().unwrap();
        assert_eq!(newest_replayed.recent_flushed_wal_id(), 4);

        let newest_table = newest_replayed.table();
        let mut newest_iter = newest_table.iter();
        test_utils::assert_iterator(
            &mut newest_iter,
            vec![RowEntry::new_value(b"fresh_key", b"fresh_value", 4)],
        )
        .await;
    }

    #[tokio::test]
    async fn replay_wal_into_should_treat_missing_wal_sst_as_end_of_iteration() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_db_reader_missing_wal");
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));
        let table_store = test_provider.table_store();
        let wal_store = test_provider.wal_store();

        write_wal_sst(
            Arc::clone(&wal_store),
            1,
            vec![RowEntry::new_value(b"key", b"value", 1)],
        )
        .await
        .unwrap();

        let mut into_tables = VecDeque::new();
        let mut core = ManifestCore::new();
        core.next_wal_sst_id = 3;
        let status_manager = status_manager_for_core(&core);

        let (last_wal_id, last_committed_seq) = DbReaderInner::replay_wal_into(
            Arc::clone(&table_store),
            &native_wal_reader(&wal_store, &status_manager),
            &DbReaderOptions::default(),
            &core,
            &mut into_tables,
            WalReplayEnd::Manifest,
            None,
        )
        .await
        .unwrap();

        // WAL 2 is missing and ends the iteration, but the rows already replayed
        // from WAL 1 must still be returned.
        assert_eq!(last_wal_id, 1);
        assert_eq!(last_committed_seq, 1);
        assert_eq!(into_tables.len(), 1);
        assert_eq!(into_tables.front().unwrap().recent_flushed_wal_id(), 1);
    }

    #[tokio::test]
    async fn replay_wal_into_should_keep_previously_replayed_tables_before_missing_wal_sst() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_db_reader_missing_wal_after_replay");
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));
        let table_store = test_provider.table_store();
        let wal_store = test_provider.wal_store();

        let wal_1_row = RowEntry::new_value(b"a", &[b'a'; 8], 1);
        let wal_2_row_1 = RowEntry::new_value(b"b", &[b'b'; 40], 2);
        let wal_2_row_2 = RowEntry::new_value(b"c", &[b'c'; 40], 3);

        let max_memtable_bytes = table_store.estimate_encoded_size_compacted(
            2,
            wal_1_row.estimated_size() + wal_2_row_1.estimated_size(),
        ) as u64;

        write_wal_sst(Arc::clone(&wal_store), 1, vec![wal_1_row.clone()])
            .await
            .unwrap();
        write_wal_sst(
            Arc::clone(&wal_store),
            2,
            vec![wal_2_row_1.clone(), wal_2_row_2.clone()],
        )
        .await
        .unwrap();

        let mut into_tables = VecDeque::new();
        let mut core = ManifestCore::new();
        // Force the reader to attempt to read up to 4 even though 3 and 4 don't exist.
        core.next_wal_sst_id = 4;
        let reader_options = DbReaderOptions {
            max_memtable_bytes,
            ..DbReaderOptions::default()
        };
        let status_manager = status_manager_for_core(&core);

        let (last_wal_id, last_committed_seq) = DbReaderInner::replay_wal_into(
            Arc::clone(&table_store),
            &native_wal_reader(&wal_store, &status_manager),
            &reader_options,
            &core,
            &mut into_tables,
            WalReplayEnd::Manifest,
            None,
        )
        .await
        .unwrap();

        assert_eq!(last_wal_id, 2);
        assert_eq!(last_committed_seq, 3);
        assert_eq!(into_tables.len(), 1);

        let replayed = into_tables.front().unwrap();
        assert_eq!(replayed.recent_flushed_wal_id(), 2);

        let mut replayed_iter = replayed.table().iter();
        test_utils::assert_iterator(
            &mut replayed_iter,
            vec![wal_1_row, wal_2_row_1, wal_2_row_2],
        )
        .await;
    }

    #[tokio::test]
    async fn replay_wal_into_should_noop_for_fresh_db_with_no_writes() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_db_reader_fresh_db_no_writes");
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));
        let table_store = test_provider.table_store();
        let wal_store = test_provider.wal_store();

        let mut into_tables = VecDeque::new();
        let core = ManifestCore::new();
        let status_manager = status_manager_for_core(&core);

        let (last_wal_id, last_committed_seq) = DbReaderInner::replay_wal_into(
            Arc::clone(&table_store),
            &native_wal_reader(&wal_store, &status_manager),
            &DbReaderOptions::default(),
            &core,
            &mut into_tables,
            WalReplayEnd::Latest,
            None,
        )
        .await
        .unwrap();

        assert_eq!(last_wal_id, 0);
        assert_eq!(last_committed_seq, 0);
        assert!(into_tables.is_empty());
    }

    #[tokio::test]
    async fn replay_wal_into_should_replay_single_wal_for_fresh_db() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_db_reader_fresh_db_one_wal");
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));
        let table_store = test_provider.table_store();
        let wal_store = test_provider.wal_store();

        let wal_row = RowEntry::new_value(b"key", b"value", 1);
        write_wal_sst(Arc::clone(&wal_store), 1, vec![wal_row.clone()])
            .await
            .unwrap();

        let mut into_tables = VecDeque::new();
        let core = ManifestCore::new();
        let status_manager = status_manager_for_core(&core);

        let (last_wal_id, last_committed_seq) = DbReaderInner::replay_wal_into(
            Arc::clone(&table_store),
            &native_wal_reader(&wal_store, &status_manager),
            &DbReaderOptions::default(),
            &core,
            &mut into_tables,
            WalReplayEnd::Latest,
            None,
        )
        .await
        .unwrap();

        assert_eq!(last_wal_id, 1);
        assert_eq!(last_committed_seq, 1);
        assert_eq!(into_tables.len(), 1);

        let replayed = into_tables.front().unwrap();
        assert_eq!(replayed.recent_flushed_wal_id(), 1);

        let mut replayed_iter = replayed.table().iter();
        test_utils::assert_iterator(&mut replayed_iter, vec![wal_row]).await;
    }

    #[tokio::test]
    async fn replay_wal_into_should_preserve_existing_last_committed_seq_for_empty_fence_wal() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_db_reader_empty_fence_wal");
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));
        let table_store = test_provider.table_store();
        let wal_store = test_provider.wal_store();

        write_wal_sst(Arc::clone(&wal_store), 6, vec![])
            .await
            .unwrap();

        let mut into_tables = VecDeque::new();
        into_tables.push_front(immutable_memtable(
            5,
            vec![
                RowEntry::new_value(b"existing_key_1", b"existing_value_1", 9),
                RowEntry::new_value(b"existing_key_2", b"existing_value_2", 10),
            ],
        ));

        let mut core = ManifestCore::new();
        core.last_l0_seq = 8;
        core.next_wal_sst_id = 5;
        let status_manager = status_manager_for_core(&core);

        let (last_wal_id, last_committed_seq) = DbReaderInner::replay_wal_into(
            Arc::clone(&table_store),
            &native_wal_reader(&wal_store, &status_manager),
            &DbReaderOptions::default(),
            &core,
            &mut into_tables,
            WalReplayEnd::Latest,
            None,
        )
        .await
        .unwrap();

        assert_eq!(last_wal_id, 6);
        assert_eq!(last_committed_seq, 10);
    }

    #[tokio::test(start_paused = true)]
    async fn should_fail_new_reads_if_manifest_poller_crashes() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));
        let _db = test_provider.new_db(Settings::default()).await.unwrap();

        let reader_options = DbReaderOptions {
            manifest_poll_interval: Duration::from_millis(500),
            ..DbReaderOptions::default()
        };
        let reader = test_provider
            .new_db_reader(reader_options, None, None)
            .await
            .unwrap();

        fail_parallel::cfg(
            Arc::clone(&test_provider.fp_registry),
            "probe-wal-ssts",
            "return",
        )
        .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        let result = reader.get(b"key").await.unwrap_err();
        dbg!(&result);
        assert_eq!(
            result.to_string(),
            "Unavailable error: wal unavailable (io error)"
        );
    }

    #[tokio::test]
    async fn skip_wal_replay_should_not_see_wal_only_writes() {
        use crate::config::{FlushOptions, FlushType};

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        // Create a DB and write some data, then flush memtable to L0 SSTs
        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let flushed_key = b"flushed_key";
        let flushed_value = b"flushed_value";
        db.put(flushed_key, flushed_value).await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Write more data that stays in WAL (not flushed to L0)
        let wal_only_key = b"wal_only_key";
        let wal_only_value = b"wal_only_value";
        db.put(wal_only_key, wal_only_value).await.unwrap();
        // Only flush to WAL, not to L0 SSTs
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        // Open a reader with skip_wal_replay=true
        let reader_options = DbReaderOptions {
            skip_wal_replay: true,
            ..DbReaderOptions::default()
        };
        let reader = test_provider
            .new_db_reader(reader_options.clone(), None, None)
            .await
            .unwrap();

        // Should see the L0 flushed data
        assert_eq!(
            reader.get(flushed_key).await.unwrap(),
            Some(Bytes::from_static(flushed_value))
        );

        // Should NOT see the WAL-only data
        assert_eq!(reader.get(wal_only_key).await.unwrap(), None);

        // After flushing memtable to L0, a NEW reader should see the data
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Open a new reader - it should see the newly flushed data
        let reader2 = test_provider
            .new_db_reader(reader_options, None, None)
            .await
            .unwrap();
        assert_eq!(
            reader2.get(wal_only_key).await.unwrap(),
            Some(Bytes::from_static(wal_only_value))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn skip_wal_replay_should_be_respected_during_reestablish_checkpoint() {
        use crate::config::{FlushOptions, FlushType};

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();

        // Write initial data and flush to L0 so the reader opens with this state
        db.put(b"key1", b"value1").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Open reader with skip_wal_replay=true. The poller's first tick fires
        // immediately (during the next yield) and sees no manifest change.
        let reader_options = DbReaderOptions {
            manifest_poll_interval: Duration::from_millis(100),
            skip_wal_replay: true,
            ..DbReaderOptions::default()
        };
        let reader = test_provider
            .new_db_reader(reader_options, None, None)
            .await
            .unwrap();

        // Capture checkpoint ID before the flush so the wait condition is not
        // affected by a race where the poller replaces the checkpoint during
        // the flush.
        let manifest_store = test_provider.manifest_store();
        let mut stored_manifest =
            StoredManifest::load(manifest_store, test_provider.system_clock.clone())
                .await
                .unwrap();
        let initial_checkpoint_id = stored_manifest
            .manifest()
            .core
            .checkpoints
            .first()
            .unwrap()
            .id;

        // Inject a failpoint on WAL probing before flushing so it is active
        // when the poller fires. With the buggy replay_new_wals=true,
        // reestablish_checkpoint resolves the last WAL file by probing WAL SSTs
        // and hits this failpoint. With the fix (replay_new_wals=false), the
        // WAL probe is skipped entirely.
        fail_parallel::cfg(
            Arc::clone(&test_provider.fp_registry),
            "probe-wal-ssts",
            "return",
        )
        .unwrap();

        // Write more data and flush to L0, changing the manifest's L0 state.
        // This makes should_reestablish_checkpoint() return true on the next poll.
        // Note: the writer uses its own TableStore (not the test_provider's),
        // so the failpoint above does not affect the writer's flush path.
        db.put(b"key2", b"value2").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Wait for the manifest poller to see the changed L0 state and
        // reestablish the checkpoint. Without the fix, the poller crashes
        // on the WAL probing failpoint.
        let timeout = Duration::from_secs(5);
        let start = tokio::time::Instant::now();
        loop {
            assert!(
                start.elapsed() < timeout,
                "timed out waiting for checkpoint reestablishment"
            );
            let manifest = stored_manifest.refresh().await.unwrap();
            let current_checkpoint = manifest.core.checkpoints.first().unwrap();
            if current_checkpoint.id != initial_checkpoint_id {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // With the fix, the reader should still work (poller didn't crash).
        // Without the fix, the poller crashes and get() returns an error.
        let result = reader.get(b"key1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn skip_wal_replay_should_see_l0_data() {
        use crate::config::{FlushOptions, FlushType};

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        // Create a DB and write data, then flush memtable to L0 SSTs
        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let key = b"test_key";
        let value = b"test_value";
        db.put(key, value).await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Open a reader with skip_wal_replay=true
        let reader_options = DbReaderOptions {
            skip_wal_replay: true,
            ..DbReaderOptions::default()
        };
        let reader = test_provider
            .new_db_reader(reader_options, None, None)
            .await
            .unwrap();

        // Should see the L0 data
        assert_eq!(
            reader.get(key).await.unwrap(),
            Some(Bytes::from_static(value))
        );
    }

    /// A manifest records the WAL files written since the last L0 flush in
    /// `next_wal_sst_id`. Opening a reader must not read them when WAL replay
    /// is skipped: that range is exactly the expensive one, since it grows
    /// with everything written between L0 flushes.
    #[tokio::test]
    async fn skip_wal_replay_should_not_read_wals_recorded_in_manifest() {
        let recording_store = Arc::new(test_utils::RecordingObjectStore::new(Arc::new(
            InMemory::new(),
        )));
        let object_store: Arc<dyn ObjectStore> = recording_store.clone();
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let flushed_key = b"flushed_key";
        let flushed_value = b"flushed_value";
        db.put(flushed_key, flushed_value).await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Write data that stays in the WAL, then close without flushing the
        // memtable. Closing persists the manifest, so `next_wal_sst_id` covers
        // these WAL files while `replay_after_wal_id` stays at the last L0 flush.
        // The write must be awaited to durability first: closing without a
        // memtable flush does not flush the WAL, so an unawaited write would
        // race the flush interval and might never reach a WAL SST.
        let wal_only_key = b"wal_only_key";
        db.put(wal_only_key, b"wal_only_value").await.unwrap();
        db.close_with_options(CloseOptions::default().with_flush_type(Some(FlushType::Wal)))
            .await
            .unwrap();

        let core = test_provider
            .manifest_store()
            .read_latest_manifest()
            .await
            .unwrap()
            .manifest
            .core;
        assert!(
            core.replay_after_wal_id + 1 < core.next_wal_sst_id,
            "test needs a manifest that records live WAL files \
             [replay_after_wal_id={}, next_wal_sst_id={}]",
            core.replay_after_wal_id,
            core.next_wal_sst_id
        );

        recording_store.clear();
        let reader = test_provider
            .new_db_reader(
                DbReaderOptions {
                    skip_wal_replay: true,
                    ..DbReaderOptions::default()
                },
                None,
                None,
            )
            .await
            .unwrap();

        let wal_reads = recording_store
            .get_sst_types(false)
            .into_iter()
            .chain(recording_store.get_sst_types(true))
            .filter(|sst_type| *sst_type == Some(SstType::Wal))
            .count();
        assert_eq!(wal_reads, 0, "reader read WAL SSTs despite skip_wal_replay");

        assert_eq!(reader.get(wal_only_key).await.unwrap(), None);
        assert_eq!(
            reader.get(flushed_key).await.unwrap(),
            Some(Bytes::from_static(flushed_value))
        );
    }

    /// A checkpoint captures the WAL files that were durable when it was taken,
    /// so a pinned reader replays them by default. `skip_wal_replay` opts out of
    /// that read, at the cost of not seeing the checkpointed WAL writes.
    #[rstest]
    #[case(true, None)]
    #[case(false, Some(Bytes::from_static(b"wal_only_value")))]
    #[tokio::test]
    async fn skip_wal_replay_should_control_checkpoint_wal_reads(
        #[case] skip_wal_replay: bool,
        #[case] expected: Option<Bytes>,
    ) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        db.put(b"flushed_key", b"flushed_value").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // This write is only durable in the WAL, so the checkpoint references it
        // through the manifest's `next_wal_sst_id` rather than through L0. The
        // scope must be `Durable`: `All` would flush the memtable to L0 first,
        // leaving the checkpoint with no live WAL.
        let wal_only_key = b"wal_only_key";
        db.put(wal_only_key, b"wal_only_value").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();
        let checkpoint = db
            .create_checkpoint(CheckpointScope::Durable, &CheckpointOptions::default())
            .await
            .unwrap();
        db.close_with_options(CloseOptions::default().with_flush_type(Some(FlushType::Wal)))
            .await
            .unwrap();

        let core = test_provider
            .manifest_store()
            .read_manifest(checkpoint.manifest_id)
            .await
            .unwrap()
            .core;
        assert!(
            core.replay_after_wal_id + 1 < core.next_wal_sst_id,
            "test needs a checkpoint whose manifest records live WAL files \
             [replay_after_wal_id={}, next_wal_sst_id={}]",
            core.replay_after_wal_id,
            core.next_wal_sst_id
        );

        let reader = test_provider
            .new_db_reader(
                DbReaderOptions {
                    skip_wal_replay,
                    ..DbReaderOptions::default()
                },
                Some(checkpoint.id),
                None,
            )
            .await
            .unwrap();

        assert_eq!(reader.get(wal_only_key).await.unwrap(), expected);
    }

    struct TestProvider {
        object_store: Arc<dyn ObjectStore>,
        path: Path,
        fp_registry: Arc<FailPointRegistry>,
        system_clock: Arc<dyn SystemClock>,
        rand: Arc<DbRand>,
    }

    impl TestProvider {
        fn new(path: Path, object_store: Arc<dyn ObjectStore>) -> Self {
            let system_clock = Arc::new(DefaultSystemClock::new());
            let rand = Arc::new(DbRand::default());
            TestProvider {
                object_store,
                path,
                fp_registry: Arc::new(FailPointRegistry::new()),
                system_clock,
                rand,
            }
        }
    }

    impl TestProvider {
        async fn new_db(&self, options: Settings) -> Result<Db, crate::Error> {
            Db::builder(self.path.clone(), self.object_store.clone())
                .with_settings(options)
                .build()
                .await
        }

        async fn new_db_reader(
            &self,
            options: DbReaderOptions,
            checkpoint: Option<Uuid>,
            merge_operator: Option<MergeOperatorType>,
        ) -> Result<DbReader, SlateDBError> {
            let mode = checkpoint.map_or(DbReaderMode::ManagedCheckpoint, DbReaderMode::Checkpoint);
            DbReader::open_internal(
                self.manifest_store(),
                self.table_store(),
                self.wal_store(),
                mode,
                None,
                merge_operator,
                None,
                options,
                self.system_clock.clone(),
                self.rand.clone(),
                slatedb_common::metrics::MetricsRecorderHelper::noop(),
            )
            .await
        }
    }

    fn status_manager_for_core(core: &ManifestCore) -> DbStatusManager {
        DbStatusManager::new_with_initial_values(
            core.last_l0_seq,
            VersionedManifest::from_manifest(1, Manifest::initial(core.clone())),
            BTreeSet::default(),
        )
    }

    fn native_wal_reader(
        wal_store: &Arc<WalTableStore>,
        status_manager: &DbStatusManager,
    ) -> crate::wal::slatedb::reader::SlateDbWalReader {
        crate::wal::slatedb::reader::SlateDbWalReader::new_with_status_manager(
            Arc::clone(wal_store),
            status_manager,
            Arc::new(DefaultSystemClock::new()),
            SlateDbWalReaderOptions::default(),
        )
    }

    fn immutable_memtable(
        recent_flushed_wal_id: u64,
        entries: Vec<RowEntry>,
    ) -> Arc<ImmutableMemtable> {
        let table = WritableKVTable::new();
        for entry in entries {
            table.put(entry);
        }
        Arc::new(ImmutableMemtable::new(table, recent_flushed_wal_id))
    }

    async fn write_wal_sst(
        wal_store: Arc<WalTableStore>,
        wal_id: u64,
        entries: Vec<RowEntry>,
    ) -> Result<(), SlateDBError> {
        let mut builder = wal_store.table_builder();
        for entry in entries {
            builder.add(entry).await?;
        }
        let encoded_sst = builder.build().await?;
        wal_store.write_sst(wal_id.into(), &encoded_sst).await?;
        Ok(())
    }

    #[derive(Debug)]
    struct InputMemtable {
        recent_flushed_wal_id: u64,
        seqs: Vec<u64>,
    }

    impl InputMemtable {
        fn new(recent_flushed_wal_id: u64, seqs: Vec<u64>) -> Self {
            Self {
                recent_flushed_wal_id,
                seqs,
            }
        }

        fn build(&self) -> Arc<ImmutableMemtable> {
            immutable_memtable(
                self.recent_flushed_wal_id,
                self.seqs
                    .iter()
                    .map(|seq| {
                        let key = format!("key-{seq:020}");
                        let value = format!("value-{seq:020}");
                        RowEntry::new_value(key.as_bytes(), value.as_bytes(), *seq)
                    })
                    .collect(),
            )
        }
    }

    #[derive(Debug)]
    struct RebuildCheckpointCase {
        last_l0_seq: u64,
        tables: Vec<InputMemtable>,
        expected: Vec<InputMemtable>,
    }

    fn test_checkpoint(manifest_id: u64, clock: Arc<dyn SystemClock>) -> crate::Checkpoint {
        crate::Checkpoint {
            id: Uuid::new_v4(),
            manifest_id,
            expire_time: None,
            create_time: clock.now(),
            name: None,
        }
    }

    #[rstest]
    #[case::skips_table_when_last_seq_is_below_last_l0_seq(RebuildCheckpointCase {
        last_l0_seq: 10,
        tables: vec![InputMemtable::new(7, vec![7, 8, 9])],
        expected: vec![],
    })]
    #[case::skips_table_when_last_seq_equals_last_l0_seq(RebuildCheckpointCase {
        last_l0_seq: 10,
        tables: vec![InputMemtable::new(7, vec![10])],
        expected: vec![],
    })]
    #[case::keeps_entire_table_when_first_seq_is_just_after_last_l0_seq(RebuildCheckpointCase {
        last_l0_seq: 10,
        tables: vec![InputMemtable::new(7, vec![11, 12])],
        expected: vec![InputMemtable::new(7, vec![11, 12])],
    })]
    #[case::filters_table_when_first_seq_equals_last_l0_seq(RebuildCheckpointCase {
        last_l0_seq: 10,
        tables: vec![InputMemtable::new(7, vec![10, 11, 12])],
        expected: vec![InputMemtable::new(7, vec![11, 12])],
    })]
    #[case::filters_table_when_only_last_row_is_newer_than_last_l0_seq(RebuildCheckpointCase {
        last_l0_seq: 10,
        tables: vec![InputMemtable::new(7, vec![8, 9, 10, 11])],
        expected: vec![InputMemtable::new(7, vec![11])],
    })]
    #[case::preserves_order_across_keep_filter_and_skip_paths(RebuildCheckpointCase {
        last_l0_seq: 20,
        tables: vec![
            InputMemtable::new(9, vec![25, 26]),
            InputMemtable::new(8, vec![20, 21, 22]),
            InputMemtable::new(7, vec![18, 19, 20]),
            InputMemtable::new(6, vec![21, 23]),
        ],
        expected: vec![
            InputMemtable::new(9, vec![25, 26]),
            InputMemtable::new(8, vec![21, 22]),
            InputMemtable::new(6, vec![21, 23]),
        ],
    })]
    #[tokio::test]
    async fn rebuild_checkpoint_state_should_filter_existing_imm_memtables_by_last_l0_seq(
        #[case] case: RebuildCheckpointCase,
    ) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from(format!(
            "/tmp/test_db_reader_rebuild_checkpoint_state_{}",
            Uuid::new_v4()
        ));
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));
        let manifest_store = test_provider.manifest_store();
        let table_store = test_provider.table_store();
        let wal_store = test_provider.wal_store();
        let mut stored_manifest = StoredManifest::create_new_db(
            Arc::clone(&manifest_store),
            ManifestCore::new(),
            test_provider.system_clock.clone(),
        )
        .await
        .unwrap();

        // Seed the prior checkpoint state with IMMs.
        let input_tables: Vec<_> = case.tables.iter().map(InputMemtable::build).collect();
        let prior_state = ReaderState {
            manifest_id: stored_manifest.id(),
            checkpoint: Some(test_checkpoint(
                stored_manifest.id(),
                test_provider.system_clock.clone(),
            )),
            manifest: stored_manifest.manifest().clone(),
            imm_memtable: input_tables.iter().cloned().collect(),
            last_wal_id: 0,
            last_remote_persisted_seq: 0,
        };
        let next_wal_sst_id = input_tables
            .iter()
            .map(|table| table.recent_flushed_wal_id())
            .max()
            .unwrap_or(0)
            + 1;

        // Advance only the manifest fields that control the rebuild filter logic.
        // We also pin replay_after_wal_id to the last known WAL so these cases
        // exercise IMM filtering without attempting any fresh WAL replay.
        let mut dirty = stored_manifest.prepare_dirty().unwrap();
        dirty.value.core.last_l0_seq = case.last_l0_seq;
        dirty.value.core.next_wal_sst_id = next_wal_sst_id;
        dirty.value.core.replay_after_wal_id = next_wal_sst_id.saturating_sub(1);
        stored_manifest.update(dirty).await.unwrap();
        let new_manifest_id = stored_manifest.id();

        // Construct just enough DbReaderInner state to call rebuild_checkpoint_state()
        // directly. skip_wal_replay keeps the test scoped to the IMM retention logic.
        let oracle = Arc::new(DbReaderOracle::new(0, DbStatusManager::new(0)));
        let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
        let reader = Reader::new(
            Arc::clone(&table_store),
            DbStats::new(&recorder),
            Arc::new(MonotonicClock::new(
                test_provider.system_clock.clone(),
                i64::MIN,
            )),
            oracle.clone(),
            None,
        );
        let status_manager = status_manager_for_core(&stored_manifest.manifest().core);
        let wal_reader = Arc::new(native_wal_reader(&wal_store, &status_manager));
        let inner = DbReaderInner {
            manifest_store,
            table_store,
            wal_reader,
            options: DbReaderOptions {
                skip_wal_replay: true,
                ..DbReaderOptions::default()
            },
            mode: DbReaderMode::ManagedCheckpoint,
            state: parking_lot::RwLock::new(Arc::new(prior_state)),
            system_clock: test_provider.system_clock.clone(),
            oracle,
            reader,
            status_manager,
            segment_extractor: None,
            rand: test_provider.rand.clone(),
            recorder,
        };

        let rebuilt_state = inner
            .rebuild_checkpoint_state(test_checkpoint(
                new_manifest_id,
                test_provider.system_clock.clone(),
            ))
            .await
            .unwrap();

        // The rebuilt checkpoint should reflect the new manifest.
        assert_eq!(rebuilt_state.manifest.core.last_l0_seq, case.last_l0_seq);
        assert_eq!(rebuilt_state.imm_memtable.len(), case.expected.len());

        for (rebuilt_table, expected_table) in
            rebuilt_state.imm_memtable.iter().zip(case.expected.iter())
        {
            let table = rebuilt_table.table();
            let mut iter = table.iter();
            let mut seqs = Vec::new();
            while let Some(entry) = iter.next_sync() {
                seqs.push(entry.seq);
            }

            // Every retained row must be strictly newer than the manifest's last L0 seq.
            assert_eq!(seqs, expected_table.seqs);
            assert!(seqs.iter().all(|seq| *seq > case.last_l0_seq));
            assert_eq!(
                rebuilt_table.recent_flushed_wal_id(),
                expected_table.recent_flushed_wal_id
            );

            // The filtered table's metadata should agree with the rows that survived.
            let metadata = rebuilt_table.table().metadata();
            assert_eq!(metadata.first_seq, *expected_table.seqs.first().unwrap());
            assert_eq!(metadata.last_seq, *expected_table.seqs.last().unwrap());
        }
    }

    fn build_db_reader_inner(
        test_provider: &TestProvider,
        current_core: &ManifestCore,
    ) -> DbReaderInner {
        let manifest_store = test_provider.manifest_store();
        let table_store = test_provider.table_store();
        let wal_store = test_provider.wal_store();
        let status_manager = status_manager_for_core(current_core);

        let prior_state = ReaderState {
            manifest_id: 1,
            checkpoint: Some(test_checkpoint(1, test_provider.system_clock.clone())),
            manifest: Manifest::initial(current_core.clone()),
            imm_memtable: VecDeque::from([immutable_memtable(
                1,
                vec![RowEntry::new_value(b"key", b"value", 10)],
            )]),
            last_wal_id: 1,
            last_remote_persisted_seq: 10,
        };

        let oracle = Arc::new(DbReaderOracle::new(0, DbStatusManager::new(0)));
        let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
        let reader = Reader::new(
            Arc::clone(&table_store),
            DbStats::new(&recorder),
            Arc::new(MonotonicClock::new(
                test_provider.system_clock.clone(),
                i64::MIN,
            )),
            oracle.clone(),
            None,
        );
        let wal_reader = Arc::new(native_wal_reader(&wal_store, &status_manager));
        DbReaderInner {
            manifest_store,
            table_store,
            wal_reader,
            options: DbReaderOptions::default(),
            mode: DbReaderMode::ManagedCheckpoint,
            state: parking_lot::RwLock::new(Arc::new(prior_state)),
            system_clock: test_provider.system_clock.clone(),
            oracle,
            reader,
            status_manager,
            segment_extractor: None,
            rand: test_provider.rand.clone(),
            recorder,
        }
    }

    #[test]
    fn should_reestablish_checkpoint_when_latest_last_l0_seq_exceeds_last_remote_persisted_seq() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from(format!(
            "/tmp/test_db_reader_should_reestablish_checkpoint_{}",
            Uuid::new_v4()
        ));
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));

        let mut current_core = ManifestCore::new();
        current_core.last_l0_seq = 10;
        current_core.next_wal_sst_id = 2;

        let inner = build_db_reader_inner(&test_provider, &current_core);

        assert!(!inner.should_reestablish_checkpoint(&current_core));

        let mut latest = current_core.clone();
        latest.last_l0_seq = 11;

        assert!(inner.should_reestablish_checkpoint(&latest));
    }

    #[test]
    fn should_reestablish_checkpoint_when_segments_differ() {
        // RFC-0024: per-segment compactions, drains, and segment-set changes
        // are invisible to the root-tree diff. Verify the segments comparison
        // fires on each of those shapes.
        use crate::{
            db_state::{SortedRun, SsTableHandle, SsTableId, SsTableInfo, SsTableView},
            format::sst::SST_FORMAT_VERSION_LATEST,
            manifest::{LsmTreeState, Segment},
        };

        fn view(seq: u64) -> SsTableView {
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid::Ulid::from_parts(seq, 0)),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            ))
        }
        fn segment_with(prefix: &'static [u8], tree: LsmTreeState) -> Segment {
            Segment {
                prefix: Bytes::from_static(prefix),
                tree: Arc::new(tree),
            }
        }
        fn tree_l0(views: Vec<SsTableView>) -> LsmTreeState {
            LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(views),
                compacted: vec![],
            }
        }

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from(format!(
            "/tmp/test_db_reader_segments_diff_{}",
            Uuid::new_v4()
        ));
        let test_provider = TestProvider::new(path, Arc::clone(&object_store));

        let baseline_segment = segment_with(b"hour=12/", tree_l0(vec![view(1)]));
        let mut current_core = ManifestCore::new();
        current_core.segment_extractor_name = Some("hour".into());
        current_core.segments = vec![baseline_segment.clone()];

        let inner = build_db_reader_inner(&test_provider, &current_core);

        // Identical segments → no refresh.
        assert!(!inner.should_reestablish_checkpoint(&current_core));

        // New segment appears.
        let mut latest = current_core.clone();
        latest
            .segments
            .push(segment_with(b"hour=13/", tree_l0(vec![view(2)])));
        assert!(
            inner.should_reestablish_checkpoint(&latest),
            "adding a segment should retire the snapshot"
        );

        // Segment's L0 changes in place (compaction within a segment).
        let mut latest = current_core.clone();
        latest.segments = vec![segment_with(b"hour=12/", tree_l0(vec![view(1), view(3)]))];
        assert!(
            inner.should_reestablish_checkpoint(&latest),
            "segment L0 change should retire the snapshot"
        );

        // Segment's compacted list changes (sorted-run added).
        let mut latest = current_core.clone();
        latest.segments = vec![segment_with(
            b"hour=12/",
            LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![view(1)]),
                compacted: vec![SortedRun::new(0, [view(4)])],
            },
        )];
        assert!(
            inner.should_reestablish_checkpoint(&latest),
            "segment compacted change should retire the snapshot"
        );

        // Segment drained (watermark advances).
        let mut latest = current_core.clone();
        latest.segments = vec![segment_with(
            b"hour=12/",
            LsmTreeState {
                last_compacted_l0_sst_view_id: Some(ulid::Ulid::from_parts(99, 0)),
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: vec![],
            },
        )];
        assert!(
            inner.should_reestablish_checkpoint(&latest),
            "segment drain marker should retire the snapshot"
        );
    }

    #[tokio::test]
    async fn should_populate_disk_cache_on_read() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_db_reader_disk_cache");

        // Write data via Db
        let db = Db::builder(path.clone(), Arc::clone(&object_store))
            .with_settings(Settings::default())
            .build()
            .await
            .unwrap();
        db.put(b"key1", b"value1").await.unwrap();
        db.flush().await.unwrap();
        db.close().await.unwrap();

        // Open a DbReader over a user-constructed cached store
        let cache_dir = tempfile::Builder::new()
            .prefix("dbreader_cache_test_")
            .tempdir()
            .unwrap();
        let cache_path = cache_dir.keep();

        let cached_store = crate::cached_object_store::CachedObjectStore::builder(
            cache_path.clone(),
            Arc::clone(&object_store),
        )
        .with_part_size_bytes(1024)
        .build()
        .await
        .unwrap();

        let reader = DbReader::open(
            path.clone(),
            cached_store,
            DbReaderMode::ManagedCheckpoint,
            DbReaderOptions::default(),
        )
        .await
        .unwrap();

        // Read data to populate the cache
        let val = reader.get(b"key1").await.unwrap();
        assert_eq!(val, Some(Bytes::from_static(b"value1")));

        // Verify the cache directory has been populated
        let entries: Vec<_> = std::fs::read_dir(&cache_path).unwrap().collect();
        assert!(
            !entries.is_empty(),
            "Expected disk cache directory to be populated after read"
        );
    }

    #[tokio::test]
    async fn should_record_metrics_with_recorder() {
        use slatedb_common::metrics::{lookup_metric_with_labels, DefaultMetricsRecorder};

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_db_reader_metrics");

        // Write data via Db so there's an SST to read from
        let db = Db::builder(path.clone(), Arc::clone(&object_store))
            .with_settings(Settings::default())
            .build()
            .await
            .unwrap();
        db.put(b"key1", b"value1").await.unwrap();
        db.flush().await.unwrap();
        db.close().await.unwrap();

        // Open a DbReader with a metrics recorder
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let reader = DbReader::builder(path, object_store)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        // Verify that get_requests metric is incremented
        let val = reader.get(b"key1").await.unwrap();
        assert_eq!(val, Some(Bytes::from_static(b"value1")));
        assert_eq!(
            lookup_metric_with_labels(
                &metrics_recorder,
                crate::db_stats::REQUEST_COUNT,
                &[("op", "get")]
            ),
            Some(1)
        );
    }

    impl TestProvider {
        fn table_store(&self) -> Arc<TableStore> {
            Arc::new(TableStore::new_with_fp_registry(
                Arc::clone(&self.object_store),
                SsTableFormat::default(),
                PathResolver::from_root(self.path.clone()),
                Arc::clone(&self.fp_registry),
                None,
                TableStoreKind::Reader,
                BlockCachePolicy::default(),
            ))
        }

        fn wal_store(&self) -> Arc<WalTableStore> {
            Arc::new(WalTableStore::new_with_fp_registry(
                Arc::clone(&self.object_store),
                SsTableFormat::default(),
                PathResolver::from_root(self.path.clone()),
                Arc::clone(&self.fp_registry),
                TableStoreKind::Reader,
            ))
        }

        fn manifest_store(&self) -> Arc<ManifestStore> {
            Arc::new(ManifestStore::new(
                &self.path,
                Arc::clone(&self.object_store),
            ))
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn db_reader_get_returns_correct_merge_result_after_reestablish_from_l0() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_db_reader_merge_reestablish_from_l0");
        let clock = Arc::new(MockSystemClock::new());

        let mut test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));
        test_provider.system_clock = clock.clone();

        let merge_operator: MergeOperatorType = Arc::new(test_utils::StringConcatMergeOperator);

        let db = Db::builder(path.clone(), Arc::clone(&object_store))
            .with_settings(Settings {
                flush_interval: None,
                compactor_options: None,
                garbage_collector_options: None,
                ..Settings::default()
            })
            .with_system_clock(clock.clone())
            .with_merge_operator(merge_operator.clone())
            .build()
            .await
            .unwrap();

        let key = b"k";

        // Phase A: make the reader recover merge operands from WAL into replayed IMMs.
        db.merge_with_options(
            key,
            b"a",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.merge_with_options(
            key,
            b"b",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        let reader = test_provider
            .new_db_reader(
                DbReaderOptions {
                    manifest_poll_interval: Duration::from_millis(100),
                    checkpoint_lifetime: Duration::from_secs(30),
                    ..DbReaderOptions::default()
                },
                None,
                Some(merge_operator),
            )
            .await
            .unwrap();

        assert_eq!(
            reader.get(key).await.unwrap(),
            Some(Bytes::from_static(b"ab"))
        );
        assert!(
            !reader.inner.state.read().imm_memtable.is_empty(),
            "reader should have replayed WAL data into immutable memtables"
        );

        // Let the reader's immediate first ticker fire and settle before phase B.
        tokio::task::yield_now().await;

        // Phase B: write newer merge operands and flush the writer memtable to L0.
        db.merge_with_options(
            key,
            b"c",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.merge_with_options(
            key,
            b"d",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        let manifest_store = test_provider.manifest_store();
        let mut stored_manifest =
            StoredManifest::load(manifest_store, test_provider.system_clock.clone())
                .await
                .unwrap();

        let start = tokio::time::Instant::now();
        loop {
            let manifest = stored_manifest.refresh().await.unwrap();
            if manifest.core.tree.l0.len() == 1 {
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "timed out waiting for writer manifest to include the L0 flush"
            );
            tokio::task::yield_now().await;
        }

        let timeout = Duration::from_secs(30);
        let start = tokio::time::Instant::now();
        loop {
            if reader.inner.state.read().manifest.core.tree.l0.len() == 1 {
                break;
            }
            // The reader poller may observe the pre-flush manifest on one tick and
            // only see the new L0 on a later poll. Keep advancing the mock clock
            // until the reader reestablishes from the updated manifest.
            clock.advance(Duration::from_millis(100)).await;
            assert!(
                start.elapsed() < timeout,
                "timed out waiting for reader to reestablish from the new manifest"
            );
            tokio::task::yield_now().await;
        }

        // Correct behavior: the reader should return the full merged value after reestablish.
        assert_eq!(
            reader.get(key).await.unwrap(),
            Some(Bytes::from_static(b"abcd"))
        );

        reader.close().await.unwrap();
        db.close().await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn should_subscribe_to_durable_seq_updates() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider
            .new_db(Settings {
                l0_sst_size_bytes: 256,
                ..Settings::default()
            })
            .await
            .unwrap();

        // Write initial data and flush so reader can see it.
        db.put(b"k1", b"v1").await.unwrap();
        db.flush().await.unwrap();

        let reader_options = DbReaderOptions {
            manifest_poll_interval: Duration::from_millis(10),
            ..DbReaderOptions::default()
        };
        let reader = test_provider
            .new_db_reader(reader_options, None, None)
            .await
            .unwrap();

        let mut rx = reader.subscribe();
        let initial_seq = rx.borrow().durable_seq;
        assert!(initial_seq > 0);

        // Write more data and flush.
        db.put(b"k2", b"v2").await.unwrap();
        db.flush().await.unwrap();

        // Wait for the reader's manifest poll to pick up the new data.
        tokio::time::sleep(Duration::from_millis(20)).await;

        rx.changed().await.unwrap();
        let updated_seq = rx.borrow().durable_seq;
        assert!(
            updated_seq > initial_seq,
            "durable_seq should advance: {} > {}",
            updated_seq,
            initial_seq
        );

        reader.close().await.unwrap();
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_report_open_status() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let reader = test_provider
            .new_db_reader(DbReaderOptions::default(), None, None)
            .await
            .unwrap();

        assert_eq!(reader.status().close_reason, None);

        reader.close().await.unwrap();
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_report_closed_status() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let reader = test_provider
            .new_db_reader(DbReaderOptions::default(), None, None)
            .await
            .unwrap();

        reader.close().await.unwrap();
        assert_eq!(reader.status().close_reason, Some(CloseReason::Clean));

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_report_close_via_subscribe() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let test_provider = TestProvider::new(path.clone(), Arc::clone(&object_store));

        let db = test_provider.new_db(Settings::default()).await.unwrap();
        let reader = test_provider
            .new_db_reader(DbReaderOptions::default(), None, None)
            .await
            .unwrap();

        let mut rx = reader.subscribe();
        assert!(rx.borrow().close_reason.is_none());

        reader.close().await.unwrap();

        // The watch channel should report the close.
        rx.changed().await.unwrap();
        assert!(rx.borrow().close_reason.is_some());

        db.close().await.unwrap();
    }
}
