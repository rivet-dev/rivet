use crate::garbage_collector::stats::GcStats;
use crate::garbage_collector::{retain_allowed_by_gc_filter, GcFilter, GC_DELETE_CONCURRENCY};
use crate::wal::slatedb::store::{WalFileId, WalTableStore};
use crate::wal::{WalError, WalFileRange, WalGc};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use log::error;
use slatedb_common::clock::SystemClock;
use slatedb_common::object_metadata::IdentifiedObjectMetadata;
use std::ops::Bound;
use std::sync::Arc;
use std::time::Duration;

/// Selects which class of SlateDB WAL object is collected.
///
/// Regular WAL SSTs and zero-byte WAL fence objects share the same WAL
/// directory and `WalFileId` identifier space, but they have separate
/// retention policies. This mode keeps a single implementation while
/// allowing regular WAL GC and fence WAL GC to run on independent schedules.
#[derive(Debug, Clone, Copy)]
pub(crate) enum WalGcMode {
    /// Collect non-empty WAL SSTs that are old enough for retention and unreferenced by active
    /// manifests.
    Regular,

    /// Collect zero-byte WAL fence objects under the same safety checks as regular WAL GC.
    Fence,
}

impl WalGcMode {
    pub(crate) fn resource(self) -> &'static str {
        match self {
            WalGcMode::Regular => "WAL",
            WalGcMode::Fence => "WAL fence",
        }
    }
}

#[derive(Clone)]
pub(crate) struct SlateDbWalGc {
    wal_store: Arc<WalTableStore>,
    stats: Arc<GcStats>,
    mode: WalGcMode,
    gc_filter: Option<Arc<dyn GcFilter>>,
    system_clock: Arc<dyn SystemClock>,
}

impl std::fmt::Debug for SlateDbWalGc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlateDbWalGc")
            .field("mode", &self.mode)
            .finish()
    }
}

impl SlateDbWalGc {
    pub(crate) fn new(
        wal_store: Arc<WalTableStore>,
        stats: Arc<GcStats>,
        mode: WalGcMode,
        gc_filter: Option<Arc<dyn GcFilter>>,
        system_clock: Arc<dyn SystemClock>,
    ) -> Self {
        Self {
            wal_store,
            stats,
            mode,
            gc_filter,
            system_clock,
        }
    }

    fn is_wal_sst_eligible_for_deletion(
        utc_now: &DateTime<Utc>,
        wal_sst: &IdentifiedObjectMetadata<WalFileId>,
        min_age: &chrono::Duration,
        referenced_ranges: &[WalFileRange],
    ) -> bool {
        if utc_now.signed_duration_since(wal_sst.metadata.last_modified) <= *min_age {
            return false;
        }

        let wal_sst_id = wal_sst.id.value();
        !referenced_ranges
            .iter()
            .any(|range| Self::range_contains(range, wal_sst_id))
    }

    fn range_contains(range: &WalFileRange, wal_sst_id: u64) -> bool {
        let after_start = match &range.0 {
            Bound::Included(start) => wal_sst_id >= *start,
            Bound::Excluded(start) => wal_sst_id > *start,
            Bound::Unbounded => true,
        };
        let before_end = match &range.1 {
            Bound::Included(end) => wal_sst_id <= *end,
            Bound::Excluded(end) => wal_sst_id < *end,
            Bound::Unbounded => true,
        };
        after_start && before_end
    }

    fn wal_sst_min_age(&self, min_age: Duration) -> chrono::Duration {
        chrono::Duration::from_std(min_age).expect("invalid duration")
    }

    /// Deletes the given WAL SSTs from the table store.
    ///
    /// In case of dryrun, the actual deletion doesn't happen.
    async fn maybe_delete_wal_ssts(&self, sst_ids: Vec<WalFileId>, dry_run: bool) {
        if dry_run {
            if !sst_ids.is_empty() {
                log::info!(
                    "dry run: skipping {} deletion [count={}]",
                    self.mode.resource(),
                    sst_ids.len()
                );
                if matches!(self.mode, WalGcMode::Fence) {
                    log::info!(
                        "WAL fence GC is dry-run by default. This is a conservative setting. \
                        Set wal_fence_options.dry_run=false and use a conservative min_age to enable. \
                        Silence this log with wal_fence_options=None. See #352 for details."
                    );
                }
            }
            for id in sst_ids {
                log::debug!(
                    "dry run: would delete {} but skipped [id={:?}]",
                    self.mode.resource(),
                    id
                );
            }
            return;
        }

        futures::stream::iter(sst_ids)
            .for_each_concurrent(GC_DELETE_CONCURRENCY, |id| async move {
                if let Err(e) = self.wal_store.delete_sst(id).await {
                    error!("error deleting WAL SST [id={:?}, error={}]", id, e);
                } else {
                    match self.mode {
                        WalGcMode::Regular => self.stats.gc_wal_count.increment(1),
                        WalGcMode::Fence => self.stats.gc_wal_fence_count.increment(1),
                    }
                }
            })
            .await;
    }
}

#[async_trait]
impl WalGc for SlateDbWalGc {
    async fn collect(
        &self,
        referenced_ranges: Vec<WalFileRange>,
        min_age: Duration,
        dry_run: bool,
    ) -> Result<(), WalError> {
        let utc_now = self.system_clock.now();
        let min_age = self.wal_sst_min_age(min_age);
        let ssts_to_delete = self
            .wal_store
            .list_wal_ssts(..)
            .await?
            .into_iter()
            .filter(|wal_sst| match self.mode {
                // In regular mode, only consider WAL SSTs with size > 0 for deletion.
                WalGcMode::Regular => wal_sst.metadata.size > 0,
                // In fence mode, only consider zero-byte WAL SSTs for deletion.
                WalGcMode::Fence => wal_sst.metadata.size == 0,
            })
            .filter(|wal_sst| {
                Self::is_wal_sst_eligible_for_deletion(
                    &utc_now,
                    wal_sst,
                    &min_age,
                    &referenced_ranges,
                )
            })
            .collect::<Vec<_>>();
        let ssts_to_delete = retain_allowed_by_gc_filter(&self.gc_filter, ssts_to_delete).await;
        let sst_ids_to_delete = ssts_to_delete
            .into_iter()
            .map(|wal_sst| wal_sst.id)
            .collect::<Vec<_>>();

        self.maybe_delete_wal_ssts(sst_ids_to_delete, dry_run).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::sst::SsTableFormat;
    use crate::tablestore::TableStoreKind;
    use crate::RowEntry;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::MockSystemClock;
    use slatedb_common::metrics::MetricsRecorderHelper;
    use std::time::Duration;

    fn build_wal_store() -> Arc<WalTableStore> {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        Arc::new(WalTableStore::new(
            object_store,
            SsTableFormat::default(),
            Path::from("/"),
            TableStoreKind::GC,
        ))
    }

    fn build_collector(
        wal_store: Arc<WalTableStore>,
        clock: Arc<MockSystemClock>,
        mode: WalGcMode,
    ) -> SlateDbWalGc {
        SlateDbWalGc::new(
            wal_store,
            Arc::new(GcStats::new(&MetricsRecorderHelper::noop())),
            mode,
            None,
            clock,
        )
    }

    async fn write_regular_wal(wal_store: &WalTableStore, wal_id: u64) {
        let mut sst = wal_store.table_builder();
        sst.add(RowEntry::new_value(b"key", b"value", wal_id))
            .await
            .unwrap();
        let sst = sst.build().await.unwrap();
        wal_store.write_sst(wal_id.into(), &sst).await.unwrap();
    }

    async fn write_fence_wal(wal_store: &WalTableStore, wal_id: u64) {
        wal_store.write_wal_fence(wal_id.into()).await.unwrap();
    }

    async fn wal_ids(wal_store: &WalTableStore) -> Vec<u64> {
        wal_store
            .list_wal_ssts(..)
            .await
            .unwrap()
            .into_iter()
            .map(|wal| wal.id.value())
            .collect()
    }

    async fn make_all_wals_older_than(
        wal_store: &WalTableStore,
        clock: &MockSystemClock,
        min_age: Duration,
    ) {
        let newest_wal = wal_store
            .list_wal_ssts(..)
            .await
            .unwrap()
            .into_iter()
            .map(|wal| wal.metadata.last_modified)
            .max()
            .expect("expected at least one WAL");
        let min_age_millis =
            i64::try_from(min_age.as_millis()).expect("min_age should fit in i64 milliseconds");
        clock.set(newest_wal.timestamp_millis() + min_age_millis + 1_000);
    }

    fn protect_outer_wals() -> Vec<WalFileRange> {
        vec![
            WalFileRange(Bound::Included(1), Bound::Excluded(2)),
            WalFileRange(Bound::Included(4), Bound::Unbounded),
        ]
    }

    #[tokio::test]
    async fn regular_mode_deletes_unreferenced_range_and_keeps_referenced_wals() {
        let wal_store = build_wal_store();
        let clock = Arc::new(MockSystemClock::new());
        for wal_id in 1..=4 {
            write_regular_wal(&wal_store, wal_id).await;
        }
        make_all_wals_older_than(&wal_store, &clock, Duration::ZERO).await;
        let collector = build_collector(wal_store.clone(), clock, WalGcMode::Regular);

        collector
            .collect(protect_outer_wals(), Duration::ZERO, false)
            .await
            .unwrap();

        assert_eq!(wal_ids(&wal_store).await, vec![1, 4]);
    }

    #[tokio::test]
    async fn regular_mode_does_not_touch_fence_wals() {
        let wal_store = build_wal_store();
        let clock = Arc::new(MockSystemClock::new());
        write_regular_wal(&wal_store, 1).await;
        write_fence_wal(&wal_store, 2).await;
        make_all_wals_older_than(&wal_store, &clock, Duration::ZERO).await;
        let collector = build_collector(wal_store.clone(), clock, WalGcMode::Regular);

        collector
            .collect(vec![], Duration::ZERO, false)
            .await
            .unwrap();

        assert_eq!(wal_ids(&wal_store).await, vec![2]);
    }

    #[tokio::test]
    async fn regular_mode_respects_min_age() {
        let wal_store = build_wal_store();
        let clock = Arc::new(MockSystemClock::new());
        write_regular_wal(&wal_store, 1).await;
        let last_modified = wal_store.metadata(1.into()).await.unwrap().last_modified;
        let min_age = Duration::from_secs(60 * 60);
        let collector = build_collector(wal_store.clone(), clock.clone(), WalGcMode::Regular);

        clock.set((last_modified + chrono::Duration::minutes(30)).timestamp_millis());
        collector.collect(vec![], min_age, false).await.unwrap();
        assert_eq!(wal_ids(&wal_store).await, vec![1]);

        clock.set((last_modified + chrono::Duration::minutes(61)).timestamp_millis());
        collector.collect(vec![], min_age, false).await.unwrap();
        assert!(wal_ids(&wal_store).await.is_empty());
    }

    #[tokio::test]
    async fn fence_mode_deletes_unreferenced_range_and_keeps_referenced_wals() {
        let wal_store = build_wal_store();
        let clock = Arc::new(MockSystemClock::new());
        for wal_id in 1..=4 {
            write_fence_wal(&wal_store, wal_id).await;
        }
        make_all_wals_older_than(&wal_store, &clock, Duration::ZERO).await;
        let collector = build_collector(wal_store.clone(), clock, WalGcMode::Fence);

        collector
            .collect(protect_outer_wals(), Duration::ZERO, false)
            .await
            .unwrap();

        assert_eq!(wal_ids(&wal_store).await, vec![1, 4]);
    }

    #[tokio::test]
    async fn fence_mode_does_not_touch_regular_wals() {
        let wal_store = build_wal_store();
        let clock = Arc::new(MockSystemClock::new());
        write_fence_wal(&wal_store, 1).await;
        write_regular_wal(&wal_store, 2).await;
        make_all_wals_older_than(&wal_store, &clock, Duration::ZERO).await;
        let collector = build_collector(wal_store.clone(), clock, WalGcMode::Fence);

        collector
            .collect(vec![], Duration::ZERO, false)
            .await
            .unwrap();

        assert_eq!(wal_ids(&wal_store).await, vec![2]);
    }

    #[tokio::test]
    async fn fence_mode_respects_min_age() {
        let wal_store = build_wal_store();
        let clock = Arc::new(MockSystemClock::new());
        write_fence_wal(&wal_store, 1).await;
        let last_modified = wal_store.metadata(1.into()).await.unwrap().last_modified;
        let min_age = Duration::from_secs(60 * 60);
        let collector = build_collector(wal_store.clone(), clock.clone(), WalGcMode::Fence);

        clock.set((last_modified + chrono::Duration::minutes(30)).timestamp_millis());
        collector.collect(vec![], min_age, false).await.unwrap();
        assert_eq!(wal_ids(&wal_store).await, vec![1]);

        clock.set((last_modified + chrono::Duration::minutes(61)).timestamp_millis());
        collector.collect(vec![], min_age, false).await.unwrap();
        assert!(wal_ids(&wal_store).await.is_empty());
    }
}
