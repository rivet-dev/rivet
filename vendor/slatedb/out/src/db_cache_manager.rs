use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

use bytes::Bytes;
use log::{debug, warn};
use tokio::sync::OnceCell;

use crate::bytes_range::BytesRange;
use crate::db_cache::CacheTarget;
use crate::db_state::{SsTableHandle, SsTableId, SsTableView};
use crate::error::SlateDBError;
use crate::flatbuffer_types::SsTableIndexOwned;
use crate::manifest::ManifestCore;
use crate::partitioned_keyspace::partitions_covering_range;
use crate::reader::ReadTrace;
use crate::tablestore::TableStore;

fn find_sst<'a>(
    manifest: &'a ManifestCore,
    sst_id: &SsTableId,
) -> Option<(Bytes, &'a SsTableView)> {
    if let Some(view) = manifest
        .tree
        .sst_views()
        .find(|view| view.sst.id == *sst_id)
    {
        return Some((Bytes::new(), view));
    }
    manifest.segments.iter().find_map(|segment| {
        segment
            .tree
            .sst_views()
            .find(|view| view.sst.id == *sst_id)
            .map(|view| (segment.prefix.clone(), view))
    })
}

pub(crate) async fn warm_sst_impl(
    table_store: &Arc<TableStore>,
    manifest: &ManifestCore,
    sst_id: SsTableId,
    targets: &[CacheTarget],
) -> Result<(), crate::Error> {
    if targets.is_empty() {
        return Ok(());
    }
    if table_store.cache().is_none() {
        warn!("warm_sst called on a Db without a block cache configured");
        return Ok(());
    }

    // Reuse the handle embedded in the manifest view instead of calling
    // `open_sst`, which would issue an extra object_store GET for info+version.
    // RFC-0024: walk every tree (unsegmented + segments). Each SST id appears
    // in at most one view (segments have disjoint key spaces; within a tree an
    // SST is in either L0 or one SR), so `find` is enough.
    let Some((segment, view)) = find_sst(manifest, &sst_id) else {
        debug!(
            "warm_sst: SST {:?} not reachable from current manifest",
            sst_id
        );
        return Ok(());
    };
    let handle = view.sst.clone();
    let visible_ranges: Vec<BytesRange> = view
        .calculate_view_range(BytesRange::unbounded())
        .into_iter()
        .collect();
    // Shared lazy index — populated at most once, so parallel target fanout
    // can share a single object-store read.
    let index_cell: OnceCell<Result<Arc<SsTableIndexOwned>, SlateDBError>> = OnceCell::new();

    for target in targets {
        match target {
            CacheTarget::Filters => warm_filters(table_store, &handle, &segment).await?,
            CacheTarget::Index => warm_index(table_store, &handle, &index_cell, &segment).await?,
            CacheTarget::Stats => warm_stats(table_store, &handle, &segment).await?,
            CacheTarget::Data(data_range) => {
                warm_data(
                    table_store,
                    &handle,
                    &index_cell,
                    &visible_ranges,
                    data_range,
                    &sst_id,
                    &segment,
                )
                .await?
            }
        }
    }
    Ok(())
}

pub(crate) async fn evict_cached_sst_impl(
    table_store: &Arc<TableStore>,
    manifest: &ManifestCore,
    handle: &SsTableHandle,
    targets: &[CacheTarget],
) -> Result<(), crate::Error> {
    if table_store.cache().is_none() {
        warn!("evict_cached_sst called on a Db without a block cache configured");
        return Ok(());
    }
    // The segment tag is a hint for the index read that a `Data` target
    // needs. An SST that left the manifest has none.
    let reads_index = targets.iter().any(|t| matches!(t, CacheTarget::Data(_)));
    let segment = if reads_index {
        find_sst(manifest, &handle.id).map(|(segment, _)| segment)
    } else {
        None
    };
    table_store
        .evict_sst_targets_from_cache(handle, targets, segment)
        .await;
    Ok(())
}

pub(crate) async fn flush_cache_to_disk_impl(
    table_store: &Arc<TableStore>,
) -> Result<(), crate::Error> {
    let Some(cache) = table_store.cache() else {
        debug!("flush_cache_to_disk called on a Db without a block cache configured");
        return Ok(());
    };
    cache.flush_to_disk().await
}

/// Clamp a requested data range against the SST's visible views, returning the
/// sub-ranges to warm. Empty result means there is nothing to do — either the
/// request collapses to empty bounds, or it overlaps no visible view.
fn plan_warm_intersections(
    visible_ranges: &[BytesRange],
    data_range: &(Bound<Bytes>, Bound<Bytes>),
) -> Vec<BytesRange> {
    let Some(requested) = BytesRange::try_new(data_range.0.clone(), data_range.1.clone()) else {
        return Vec::new();
    };
    visible_ranges
        .iter()
        .filter_map(|v| v.intersect(&requested))
        .collect()
}

async fn warm_data(
    table_store: &Arc<TableStore>,
    handle: &SsTableHandle,
    index_cell: &OnceCell<Result<Arc<SsTableIndexOwned>, SlateDBError>>,
    visible_ranges: &[BytesRange],
    data_range: &(Bound<Bytes>, Bound<Bytes>),
    sst_id: &SsTableId,
    segment: &Bytes,
) -> Result<(), crate::Error> {
    let intersections = plan_warm_intersections(visible_ranges, data_range);
    if intersections.is_empty() {
        debug!(
            "warm_sst: SST {:?} data range {:?} has no blocks to warm",
            sst_id, data_range
        );
        return Ok(());
    }

    let index = ensure_index(table_store, handle, index_cell, segment).await?;
    for r in &intersections {
        let block_range = partitions_covering_range(
            &index.borrow(),
            r.start_bound().map(|b| b.as_ref()),
            r.end_bound().map(|b| b.as_ref()),
        );
        if block_range.is_empty() {
            continue;
        }
        table_store
            .read_blocks_using_index(
                handle,
                index.clone(),
                block_range,
                true,
                Some(segment.clone()),
            )
            .await?;
    }
    Ok(())
}

async fn warm_index(
    table_store: &Arc<TableStore>,
    handle: &SsTableHandle,
    index_cell: &OnceCell<Result<Arc<SsTableIndexOwned>, SlateDBError>>,
    segment: &Bytes,
) -> Result<(), crate::Error> {
    ensure_index(table_store, handle, index_cell, segment).await?;
    Ok(())
}

async fn warm_filters(
    table_store: &Arc<TableStore>,
    handle: &SsTableHandle,
    segment: &Bytes,
) -> Result<(), crate::Error> {
    // filter_len == 0 means "no filters"; filter_offset aliases index_offset
    // in that case and cannot be meaningfully probed or warmed.
    if handle.info.filter_len == 0 {
        return Ok(());
    }
    table_store
        .read_filters(
            handle,
            true,
            Some(segment.clone()),
            &ReadTrace::new(None),
            None,
        )
        .await?;
    Ok(())
}

async fn warm_stats(
    table_store: &Arc<TableStore>,
    handle: &SsTableHandle,
    segment: &Bytes,
) -> Result<(), crate::Error> {
    // stats_len == 0 means "no stats block"; stats_offset is 0 in that case
    // and collides with the first data block's cache key.
    if handle.info.stats_len == 0 {
        return Ok(());
    }
    table_store
        .read_stats(handle, true, Some(segment.clone()))
        .await?;
    Ok(())
}

async fn ensure_index(
    table_store: &Arc<TableStore>,
    handle: &SsTableHandle,
    index_cell: &OnceCell<Result<Arc<SsTableIndexOwned>, SlateDBError>>,
    segment: &Bytes,
) -> Result<Arc<SsTableIndexOwned>, SlateDBError> {
    let result: &Result<Arc<SsTableIndexOwned>, SlateDBError> = index_cell
        .get_or_init(|| async {
            table_store
                .read_index(
                    handle,
                    true,
                    Some(segment.clone()),
                    &ReadTrace::new(None),
                    None,
                )
                .await
        })
        .await;
    match result {
        Ok(index) => Ok(index.clone()),
        Err(e) => Err(e.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Bound::Unbounded;

    use crate::config::{FlushOptions, FlushType, PutOptions, Settings, WriteOptions};
    use crate::db::Db;
    use crate::db_cache::{CachedKey, DbCache};
    use crate::manifest::VersionedManifest;
    use crate::DbCacheManagerOps;
    use object_store::memory::InMemory;
    use object_store::ObjectStore;

    const PATH: &str = "/cache_manager_test";

    async fn open_db_single_sst(object_store: Arc<dyn ObjectStore>) -> Db {
        // No l0_sst_size_bytes cap so one flush yields a single SST whose cache
        // we can then inspect in isolation.
        Db::builder(PATH, object_store)
            .with_settings(Settings {
                flush_interval: None,
                ..Default::default()
            })
            .build()
            .await
            .expect("failed to open db")
    }

    /// For each data block in the SST (ordered), returns whether it is
    /// currently present in the block cache. Used by tests to assert cache
    /// population directly instead of going through metrics.
    async fn cached_block_mask(
        table_store: &Arc<TableStore>,
        sst_id: SsTableId,
        segment: Bytes,
    ) -> Vec<bool> {
        let handle = table_store
            .open_sst(&sst_id, Some(segment.clone()))
            .await
            .expect("open_sst");
        let index = table_store
            .read_index(&handle, false, Some(segment), &ReadTrace::new(None), None)
            .await
            .expect("read_index");
        let cache = table_store.cache().expect("cache configured").clone();
        let num_blocks = index.borrow().block_meta().len();
        let mut mask = Vec::with_capacity(num_blocks);
        for i in 0..num_blocks {
            let offset = index.borrow().block_meta().get(i).offset();
            let key: CachedKey = (sst_id, offset).into();
            mask.push(cache.get_block(&key).await.unwrap().is_some());
        }
        mask
    }

    async fn is_key_cached(table_store: &Arc<TableStore>, sst_id: SsTableId, key: &[u8]) -> bool {
        let handle = table_store
            .open_sst(&sst_id, Some(Bytes::new()))
            .await
            .expect("open_sst");
        let index = table_store
            .read_index(
                &handle,
                false,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .expect("read_index");
        let block_idx =
            partitions_covering_range(&index.borrow(), Bound::Included(key), Bound::Included(key))
                .start;
        let offset = index.borrow().block_meta().get(block_idx).offset();
        let cache = table_store.cache().expect("cache configured").clone();
        let cache_key: CachedKey = (sst_id, offset).into();
        cache.get_block(&cache_key).await.unwrap().is_some()
    }

    async fn flush_to_l0(db: &Db) {
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .expect("failed to flush memtable");
    }

    async fn write_keys(db: &Db, count: usize) {
        // Pad values so the resulting SST spans several blocks; otherwise
        // small values pack into a single block and range-scoped warming
        // cannot be distinguished from whole-SST warming.
        let padding = vec![b'x'; 256];
        for i in 0..count {
            let key = format!("key{:06}", i);
            let mut value = format!("value{:06}", i).into_bytes();
            value.extend_from_slice(&padding);
            db.put_with_options(
                key.as_bytes(),
                &value,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .expect("put failed");
        }
    }

    fn first_l0_sst(db: &Db) -> SsTableHandle {
        let manifest = db.manifest();
        manifest
            .l0()
            .iter()
            .next()
            .map(|v| v.sst.clone())
            .expect("expected at least one L0 SST")
    }

    fn data_bounds(target: &CacheTarget) -> &(Bound<Bytes>, Bound<Bytes>) {
        match target {
            CacheTarget::Data(bounds) => bounds,
            _ => panic!("expected Data variant"),
        }
    }

    #[test]
    fn should_build_data_target_from_closed_range() {
        // given
        let range = b"a".as_slice()..b"z".as_slice();

        // when
        let target = CacheTarget::data(range);

        // then
        let (start, end) = data_bounds(&target);
        assert_eq!(start, &Bound::Included(Bytes::from_static(b"a")));
        assert_eq!(end, &Bound::Excluded(Bytes::from_static(b"z")));
    }

    #[test]
    fn should_build_data_target_from_unbounded_range() {
        // given / when
        let target = CacheTarget::data::<&[u8], _>(..);

        // then
        let (start, end) = data_bounds(&target);
        assert_eq!(start, &Unbounded);
        assert_eq!(end, &Unbounded);
    }

    #[test]
    fn should_plan_no_intersections_for_collapsed_range() {
        // given: reversed bounds collapse to an empty interval
        let visible = [BytesRange::unbounded()];
        let data = (
            Bound::Included(Bytes::from_static(b"z")),
            Bound::Excluded(Bytes::from_static(b"a")),
        );

        // when
        let out = plan_warm_intersections(&visible, &data);

        // then
        assert!(out.is_empty());
    }

    #[test]
    fn should_plan_no_intersections_when_request_outside_visible_view() {
        // given: visible view starts at key000032, request ends before it
        let visible = [BytesRange::from_ref("key000032".as_bytes()..)];
        let data = (
            Bound::Included(Bytes::from_static(b"key000000")),
            Bound::Excluded(Bytes::from_static(b"key000010")),
        );

        // when
        let out = plan_warm_intersections(&visible, &data);

        // then
        assert!(out.is_empty());
    }

    #[test]
    fn should_plan_intersection_clamped_to_visible_view() {
        // given: visible view is the upper half; request is full range
        let visible = [BytesRange::from_ref("key000032".as_bytes()..)];
        let data = (Unbounded, Unbounded);

        // when
        let out = plan_warm_intersections(&visible, &data);

        // then: one range clamped to the visible view
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], BytesRange::from_ref("key000032".as_bytes()..));
    }

    #[test]
    fn should_plan_union_across_multiple_visible_views() {
        // given: two disjoint visible views over the same SST
        let visible = [
            BytesRange::from_ref("key000000".as_bytes().."key000016".as_bytes()),
            BytesRange::from_ref("key000048".as_bytes()..),
        ];
        let data = (Unbounded, Unbounded);

        // when
        let out = plan_warm_intersections(&visible, &data);

        // then: both views are returned
        assert_eq!(out.len(), 2);
        assert_eq!(
            out[0],
            BytesRange::from_ref("key000000".as_bytes().."key000016".as_bytes()),
        );
        assert_eq!(out[1], BytesRange::from_ref("key000048".as_bytes()..));
    }

    #[tokio::test]
    async fn should_cache_all_blocks_when_warming_full_range() {
        // given: a single-SST DB with its cache evicted
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = open_db_single_sst(os).await;
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        let sst_id = sst.id;
        db.evict_cached_sst(&sst, &CacheTarget::all())
            .await
            .expect("evict");

        // when
        db.warm_sst(sst.id, &[CacheTarget::data::<&[u8], _>(..)])
            .await
            .expect("warm_sst");

        // then: every data block is in cache
        let mask = cached_block_mask(&db.inner.table_store, sst_id, Bytes::new()).await;
        assert!(!mask.is_empty(), "expected SST to have data blocks");
        assert!(
            mask.iter().all(|&b| b),
            "expected all blocks cached, got {:?}",
            mask,
        );

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_cache_only_requested_sub_range() {
        // given: a single-SST DB with its cache evicted
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = open_db_single_sst(os).await;
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        let sst_id = sst.id;
        db.evict_cached_sst(&sst, &CacheTarget::all())
            .await
            .expect("evict");

        // when: we warm only the upper half of the keyspace
        db.warm_sst(sst.id, &[CacheTarget::data(b"key000032".as_slice()..)])
            .await
            .expect("warm_sst");

        // then: a key above the boundary is cached, a key below is not
        assert!(
            is_key_cached(&db.inner.table_store, sst_id, b"key000040").await,
            "block containing key000040 should be cached",
        );
        assert!(
            !is_key_cached(&db.inner.table_store, sst_id, b"key000000").await,
            "block containing key000000 should not be cached",
        );

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_return_closed_after_db_close() {
        // given: a closed DB with a known SST
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = open_db_single_sst(os).await;
        write_keys(&db, 8).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        db.close().await.expect("close");

        // when / then: both ops reject the call with Closed
        let warm_err = db
            .warm_sst(sst.id, &[CacheTarget::Index])
            .await
            .expect_err("warm_sst on closed db");
        assert_eq!(
            warm_err.kind(),
            crate::ErrorKind::Closed(crate::CloseReason::Clean),
        );
        let evict_err = db
            .evict_cached_sst(&sst, &CacheTarget::all())
            .await
            .expect_err("evict_cached_sst on closed db");
        assert_eq!(
            evict_err.kind(),
            crate::ErrorKind::Closed(crate::CloseReason::Clean),
        );
        let flush_err = db
            .flush_cache_to_disk()
            .await
            .expect_err("flush_cache_to_disk on closed db");
        assert_eq!(
            flush_err.kind(),
            crate::ErrorKind::Closed(crate::CloseReason::Clean),
        );
    }

    fn project_l0_view(manifest: &mut VersionedManifest, visible_range: BytesRange) {
        let l0 = &mut Arc::make_mut(&mut manifest.manifest.core.tree).l0;
        let view = l0.pop_front().expect("expected at least one L0 view");
        l0.push_front(view.with_visible_range(visible_range));
    }

    /// Open a DB configured with a 3-byte prefix extractor — every flush of
    /// `key…`-prefixed writes lands entirely in segment `b"key"` rather than
    /// the root tree.
    async fn open_segmented_db(object_store: Arc<dyn ObjectStore>) -> Db {
        Db::builder(PATH, object_store)
            .with_settings(Settings {
                flush_interval: None,
                ..Default::default()
            })
            .with_segment_extractor(Arc::new(crate::test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .expect("failed to open segmented db")
    }

    /// Return the id of the first L0 SST in the manifest, walking root tree
    /// then named segments. Used by tests that don't care which tree the
    /// SST lives in.
    fn first_l0_sst_any_tree(db: &Db) -> SsTableHandle {
        let manifest = db.manifest();
        let sst = manifest
            .core()
            .trees()
            .flat_map(|t| t.l0.iter())
            .next()
            .map(|v| v.sst.clone())
            .expect("expected at least one L0 SST across root + segments");
        sst
    }

    #[tokio::test]
    async fn should_warm_only_within_visible_view_range() {
        // given: a single-SST DB whose cache is empty
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = open_db_single_sst(os).await;
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        let sst_id = sst.id;
        db.evict_cached_sst(&sst, &CacheTarget::all())
            .await
            .expect("evict");

        // and: a manifest snapshot that restricts the SST view to the upper half
        let mut manifest = db.manifest();
        project_l0_view(
            &mut manifest,
            BytesRange::from_ref("key000032".as_bytes()..),
        );

        // when: we warm the full range through the projected manifest
        warm_sst_impl(
            &db.inner.table_store,
            manifest.core(),
            sst.id,
            &[CacheTarget::data::<&[u8], _>(..)],
        )
        .await
        .expect("warm_sst_impl");

        // then: a block inside the visible view is cached, a block below is not
        assert!(
            is_key_cached(&db.inner.table_store, sst_id, b"key000040").await,
            "block inside visible view should be cached",
        );
        assert!(
            !is_key_cached(&db.inner.table_store, sst_id, b"key000000").await,
            "block below visible view should not be cached",
        );

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_warm_sst_resident_in_named_segment() {
        // RFC-0024: a DB configured with a segment extractor routes flushes
        // into named segments, leaving the root tree empty. `warm_sst_impl`
        // must reach those segment-resident SSTs. Pre-fix it only walked
        // `manifest.l0()`/`compacted()` and silently no-op'd here.
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = open_segmented_db(os).await;
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;

        let manifest = db.manifest();
        assert!(
            manifest.l0().is_empty() && manifest.compacted().is_empty(),
            "extractor-configured DB should leave root tree empty"
        );
        assert!(
            !manifest.segments().is_empty(),
            "extractor-configured flush should populate at least one segment"
        );

        let sst = first_l0_sst_any_tree(&db);
        let sst_id = sst.id;
        db.evict_cached_sst(&sst, &CacheTarget::all())
            .await
            .expect("evict");

        warm_sst_impl(
            &db.inner.table_store,
            manifest.core(),
            sst.id,
            &[CacheTarget::data::<&[u8], _>(..)],
        )
        .await
        .expect("warm_sst_impl");

        let segment = find_sst(manifest.core(), &sst_id)
            .expect("segment-resident SST should be in manifest")
            .0;
        let mask = cached_block_mask(&db.inner.table_store, sst_id, segment).await;
        assert!(!mask.is_empty(), "expected SST to have data blocks");
        assert!(
            mask.iter().all(|&b| b),
            "expected all blocks cached for segment-resident SST, got {:?}",
            mask,
        );

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_return_ok_when_sst_not_in_manifest() {
        // given: a DB with one flushed SST and a fresh unknown SST id
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = open_db_single_sst(os).await;
        write_keys(&db, 8).await;
        flush_to_l0(&db).await;
        let unknown_id = SsTableId::from(ulid::Ulid::new());

        // when / then: warming an unreachable SST is a no-op that returns Ok.
        // Any attempted IO against a non-existent SST would surface as Err, so
        // the Ok return is itself proof that no work happened.
        db.warm_sst(unknown_id, &[CacheTarget::data::<&[u8], _>(..)])
            .await
            .expect("warm_sst should no-op for unreachable SST");

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_return_err_on_target_failure() {
        // given: a flushed SST whose cache has been evicted and whose
        // underlying object has then been deleted, so target reads must go to
        // object store and will fail
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = open_db_single_sst(os).await;
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        let sst_id = sst.id;
        db.evict_cached_sst(&sst, &CacheTarget::all())
            .await
            .expect("evict");
        db.inner
            .table_store
            .delete_sst(&sst_id)
            .await
            .expect("delete_sst");

        // when: we warm a target whose underlying IO will fail
        let result = db
            .warm_sst(
                sst.id,
                &[CacheTarget::Index, CacheTarget::data::<&[u8], _>(..)],
            )
            .await;

        // then: the failure is surfaced to the caller
        assert!(result.is_err(), "warm_sst should return Err");

        db.close().await.expect("close");
    }

    // Opens a DB whose tiny fixture SST still carries filters (default
    // min_filter_keys=1000 would suppress them), flushes one SST, and evicts
    // its cache entries so meta sections start empty.
    async fn open_db_with_evicted_meta_sections() -> (Db, SsTableId, SsTableHandle, Arc<dyn DbCache>)
    {
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder(PATH, os)
            .with_settings(Settings {
                flush_interval: None,
                min_filter_keys: 1,
                ..Default::default()
            })
            .build()
            .await
            .expect("failed to open db");
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        db.evict_cached_sst(&sst, &CacheTarget::all())
            .await
            .expect("evict");
        let cache = db
            .inner
            .table_store
            .cache()
            .expect("cache configured")
            .clone();
        (db, sst.id, sst, cache)
    }

    #[tokio::test]
    async fn should_populate_cache_for_filters_target() {
        // given
        let (db, sst_id, handle, cache) = open_db_with_evicted_meta_sections().await;
        let filter_key: CachedKey = (sst_id, handle.info.filter_offset).into();
        let index_key: CachedKey = (sst_id, handle.info.index_offset).into();
        let stats_key: CachedKey = (sst_id, handle.info.stats_offset).into();
        assert!(handle.info.filter_len > 0, "expected SST to carry filters");
        assert!(cache.get_filter(&filter_key).await.unwrap().is_none());

        // when
        db.warm_sst(sst_id, &[CacheTarget::Filters])
            .await
            .expect("warm_sst");

        // then: only the filter section was fetched
        assert!(cache.get_filter(&filter_key).await.unwrap().is_some());
        assert!(
            cache.get_index(&index_key).await.unwrap().is_none(),
            "filters target must not fetch the index",
        );
        assert!(
            cache.get_stats(&stats_key).await.unwrap().is_none(),
            "filters target must not fetch stats",
        );

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_populate_cache_for_index_target() {
        // given
        let (db, sst_id, handle, cache) = open_db_with_evicted_meta_sections().await;
        let filter_key: CachedKey = (sst_id, handle.info.filter_offset).into();
        let index_key: CachedKey = (sst_id, handle.info.index_offset).into();
        let stats_key: CachedKey = (sst_id, handle.info.stats_offset).into();
        assert!(cache.get_index(&index_key).await.unwrap().is_none());

        // when
        db.warm_sst(sst_id, &[CacheTarget::Index])
            .await
            .expect("warm_sst");

        // then: only the index section was fetched
        assert!(cache.get_index(&index_key).await.unwrap().is_some());
        assert!(
            cache.get_filter(&filter_key).await.unwrap().is_none(),
            "index target must not fetch filters",
        );
        assert!(
            cache.get_stats(&stats_key).await.unwrap().is_none(),
            "index target must not fetch stats",
        );

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_populate_cache_for_stats_target() {
        // given
        let (db, sst_id, handle, cache) = open_db_with_evicted_meta_sections().await;
        let filter_key: CachedKey = (sst_id, handle.info.filter_offset).into();
        let index_key: CachedKey = (sst_id, handle.info.index_offset).into();
        let stats_key: CachedKey = (sst_id, handle.info.stats_offset).into();
        assert!(handle.info.stats_len > 0, "expected SST to carry stats");
        assert!(cache.get_stats(&stats_key).await.unwrap().is_none());

        // when
        db.warm_sst(sst_id, &[CacheTarget::Stats])
            .await
            .expect("warm_sst");

        // then: only the stats section was fetched
        assert!(cache.get_stats(&stats_key).await.unwrap().is_some());
        assert!(
            cache.get_filter(&filter_key).await.unwrap().is_none(),
            "stats target must not fetch filters",
        );
        assert!(
            cache.get_index(&index_key).await.unwrap().is_none(),
            "stats target must not fetch the index",
        );

        db.close().await.expect("close");
    }

    /// An SST that a compaction retired is no longer in the manifest. Its
    /// handle still names every entry, and the index read for a `Data`
    /// target succeeds because the object still exists.
    #[tokio::test]
    async fn should_evict_an_sst_that_is_not_in_the_manifest() {
        // given: one L0 SST whose blocks, index, and filter the flush cached
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder(PATH, os)
            .with_settings(Settings {
                flush_interval: None,
                min_filter_keys: 1,
                ..Default::default()
            })
            .build()
            .await
            .expect("failed to open db");
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        let table_store = db.inner.table_store.clone();
        let cache = table_store.cache().expect("cache configured").clone();
        let index_key: CachedKey = (sst.id, sst.info.index_offset).into();
        assert!(cache.get_index(&index_key).await.unwrap().is_some());
        assert!(cached_block_mask(&table_store, sst.id, Bytes::new())
            .await
            .iter()
            .all(|&cached| cached));

        // when: evicting through a manifest that does not reference the SST
        evict_cached_sst_impl(
            &table_store,
            &ManifestCore::new(),
            &sst,
            &CacheTarget::all(),
        )
        .await
        .expect("evict");

        // then
        assert!(cache.get_index(&index_key).await.unwrap().is_none());
        assert!(cached_block_mask(&table_store, sst.id, Bytes::new())
            .await
            .iter()
            .all(|&cached| !cached));

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_evict_only_the_selected_targets() {
        // given: one L0 SST whose blocks, index, and filter the flush cached
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder(PATH, os)
            .with_settings(Settings {
                flush_interval: None,
                min_filter_keys: 1,
                ..Default::default()
            })
            .build()
            .await
            .expect("failed to open db");
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        let table_store = db.inner.table_store.clone();
        let cache = table_store.cache().expect("cache configured").clone();
        let index_key: CachedKey = (sst.id, sst.info.index_offset).into();
        let filter_key: CachedKey = (sst.id, sst.info.filter_offset).into();
        assert!(cache.get_index(&index_key).await.unwrap().is_some());
        assert!(cache.get_filter(&filter_key).await.unwrap().is_some());
        let blocks_before = cached_block_mask(&table_store, sst.id, Bytes::new()).await;
        assert!(blocks_before.iter().all(|&cached| cached));

        // when
        db.evict_cached_sst(&sst, &[CacheTarget::Index, CacheTarget::Filters])
            .await
            .expect("evict");

        // then: the metadata is gone and the data blocks are left in place
        assert!(cache.get_index(&index_key).await.unwrap().is_none());
        assert!(cache.get_filter(&filter_key).await.unwrap().is_none());
        let blocks_after = cached_block_mask(&table_store, sst.id, Bytes::new()).await;
        assert_eq!(blocks_after, blocks_before);

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_evict_all_blocks_from_cache() {
        // given: a warmed SST
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = open_db_single_sst(os).await;
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        let sst_id = sst.id;
        db.warm_sst(sst.id, &[CacheTarget::data::<&[u8], _>(..)])
            .await
            .expect("warm_sst");
        let mask_before = cached_block_mask(&db.inner.table_store, sst_id, Bytes::new()).await;
        assert!(
            mask_before.iter().all(|&b| b),
            "expected all blocks cached after warm"
        );

        // when
        db.evict_cached_sst(&sst, &CacheTarget::all())
            .await
            .expect("evict");

        // then: no blocks remain in cache
        let mask_after = cached_block_mask(&db.inner.table_store, sst_id, Bytes::new()).await;
        assert!(
            mask_after.iter().all(|&b| !b),
            "expected no blocks cached after eviction, got {:?}",
            mask_after,
        );

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_noop_flush_cache_to_disk_for_non_hybrid_cache() {
        // given: a warmed SST on the default (non-hybrid) cache
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = open_db_single_sst(os).await;
        write_keys(&db, 64).await;
        flush_to_l0(&db).await;
        let sst = first_l0_sst(&db);
        let sst_id = sst.id;
        db.warm_sst(sst.id, &[CacheTarget::data::<&[u8], _>(..)])
            .await
            .expect("warm_sst");
        let mask_before = cached_block_mask(&db.inner.table_store, sst_id, Bytes::new()).await;
        assert!(
            mask_before.iter().all(|&b| b),
            "expected all blocks cached after warm"
        );

        // when: flushing to a cache with no disk tier to spill to
        db.flush_cache_to_disk().await.expect("flush");

        // then: the default no-op `flush_scope` leaves entries exactly as
        // they were; only a hybrid cache implements the flush.
        let mask_after = cached_block_mask(&db.inner.table_store, sst_id, Bytes::new()).await;
        assert_eq!(
            mask_before, mask_after,
            "non-hybrid cache should be untouched by flush_cache_to_disk"
        );

        db.close().await.expect("close");
    }

    #[tokio::test]
    async fn should_flush_cache_to_disk_ok_with_no_cache_configured() {
        // given: a DB with the block cache disabled entirely
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder(PATH, os)
            .with_db_cache_disabled()
            .build()
            .await
            .expect("failed to open db");

        // when / then: no cache configured is a no-op, not an error
        db.flush_cache_to_disk().await.expect("flush");

        db.close().await.expect("close");
    }
}
