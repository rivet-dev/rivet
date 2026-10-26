//! RFC-0029 GC cutoff invariants for the `.manifest` object.
//!
//! Each invariant is a plain [`Invariant<Manifest>`] predicate attached once to
//! the [`StoredManifest`](super::store::StoredManifest)'s transactional object, so
//! every manifest `update` path validates the dirty value against the current
//! committed value before the CAS write.

use std::collections::HashSet;
use std::error::Error;
use std::sync::Arc;

use slatedb_txn_obj::Invariant;
use ulid::Ulid;

use crate::error::SlateDBError;
use crate::manifest::Manifest;

/// Invariant 1 — L0 ULID cutoff. Every L0 SST newly added by `dirty` must carry a
/// physical SST ULID timestamp **not below** the current manifest's L0 watermark
/// (`current.core.max_l0_ulid_timestamp_across_trees()`) — i.e. it must be `>=`
/// the watermark; only a strictly-below timestamp is rejected.
///
/// The comparison deliberately matches the GC's deletion filter, which deletes
/// SSTs *strictly older* than the cutoff (`sst.datetime() < cutoff_dt`, at
/// millisecond granularity) — so an SST whose timestamp equals the watermark
/// survives the GC and is safe to publish. Allowing equality also means two L0s
/// minted in the same millisecond both pass, which keeps the writer's retry path
/// from spinning on same-millisecond ULID collisions; only a genuine backwards
/// clock step (timestamp strictly below the watermark) trips the invariant.
///
/// The check uses the physical SST ULID — `view.sst.id.value()` —
/// **not** the separately-allocated `view.id`. The GC keys its cutoff and its
/// deletion filter off the SST ULID (see `garbage_collector::compacted_gc`), and
/// in the normal V2 path the SST ULID is minted at upload while the `view.id` is
/// minted later when the view is added to the manifest. Validating `view.id`
/// would miss a backwards-skewed *upload*: an SST minted below the GC cutoff could
/// receive a fresh `view.id` above the watermark, pass this check, and still be
/// eligible for deletion. So both the watermark and this comparison are based on
/// the SST ULID.
///
/// This guarantees a newly published L0 is not behind any L0 the GC may have
/// already fenced, so a writer on a backwards-skewed wall clock cannot publish an
/// SST whose ULID falls below the GC cutoff — which could otherwise make a live
/// SST eligible for deletion and corrupt the database state.
///
/// "Newly added" is the set of L0 SST ULIDs present in `dirty` but absent from
/// `current`, so re-publishing existing L0s (an idempotent update) never fails
/// the check. On violation, returns [`SlateDBError::InvalidClockTick`] with the
/// watermark timestamp as `last_tick` and the offending ULID timestamp as
/// `next_tick`. The txn-obj layer wraps the boxed error in `CallbackError`, which
/// maps back to `SlateDBError::InvalidClockTick` for the caller.
pub(crate) fn l0_ulid_cutoff(
    dirty: &Manifest,
    current: &Manifest,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(watermark) = current.core.max_l0_ulid_timestamp_across_trees() else {
        // Fresh manifest with no L0 history — nothing to compare against.
        return Ok(());
    };

    // Physical SST ULIDs already committed in `current`; anything in `dirty` not
    // in this set is a newly added L0 that must not be behind the watermark.
    let existing: HashSet<Ulid> = current
        .core
        .trees()
        .flat_map(|tree| tree.l0.iter().map(|view| view.sst.id.value()))
        .collect();

    for view in dirty.core.trees().flat_map(|tree| tree.l0.iter()) {
        let sst_id = view.sst.id.value();
        if existing.contains(&sst_id) {
            continue;
        }
        // Reject only a strictly-below timestamp, mirroring the GC's `<` deletion
        // filter; an equal timestamp (e.g. a same-millisecond mint) is safe.
        if sst_id.timestamp_ms() < watermark.timestamp_ms() {
            return Err(Box::new(SlateDBError::InvalidClockTick {
                last_tick: watermark.timestamp_ms() as i64,
                next_tick: sst_id.timestamp_ms() as i64,
            }));
        }
    }
    Ok(())
}

/// The RFC-0029 invariants that run before each `.manifest` update.
pub(crate) fn manifest_invariants() -> Vec<Invariant<Manifest>> {
    vec![Arc::new(l0_ulid_cutoff)]
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;

    use ulid::Ulid;

    use super::*;
    use crate::db_state::{SsTableHandle, SsTableId, SsTableInfo, SsTableView};
    use crate::format::sst::SST_FORMAT_VERSION_LATEST;
    use crate::manifest::{LsmTreeState, ManifestCore, Segment};

    /// An L0 view whose view ULID carries timestamp `ts_ms` and random component
    /// `rand` — two views at the same `ts_ms` with different `rand` are distinct
    /// ULIDs sharing a timestamp, which is exactly the same-millisecond collision
    /// the invariant must reject.
    fn l0_view(ts_ms: u64, rand: u128) -> SsTableView {
        let view_id = Ulid::from_parts(ts_ms, rand);
        let handle = SsTableHandle::new(
            SsTableId::from(view_id),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo::default(),
        );
        SsTableView::new(view_id, handle)
    }

    /// An L0 view whose *view* ULID carries timestamp `view_ts_ms` but whose
    /// underlying physical *SST* ULID carries a different timestamp `sst_ts_ms`.
    /// This is the V2 case the invariant must handle: the SST is uploaded (and its
    /// ULID minted) separately from the later-allocated view ID, so the check must
    /// validate the SST ULID, not the view ULID.
    fn l0_view_split(view_ts_ms: u64, sst_ts_ms: u64) -> SsTableView {
        let view_id = Ulid::from_parts(view_ts_ms, 0);
        let handle = SsTableHandle::new(
            SsTableId::from(Ulid::from_parts(sst_ts_ms, 0)),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo::default(),
        );
        SsTableView::new(view_id, handle)
    }

    /// A manifest with the given L0 view timestamps on the root tree. Each view's
    /// random component is its position, so equal timestamps still yield distinct
    /// ULIDs.
    fn manifest_with_root_l0(ts: &[u64]) -> Manifest {
        let mut core = ManifestCore::new();
        Arc::make_mut(&mut core.tree).l0 = ts
            .iter()
            .enumerate()
            .map(|(i, &t)| l0_view(t, i as u128))
            .collect();
        Manifest::initial(core)
    }

    #[test]
    fn fresh_current_accepts_any_dirty_l0() {
        // No L0 history in `current` → no watermark → anything is allowed.
        let current = manifest_with_root_l0(&[]);
        let dirty = manifest_with_root_l0(&[1]);
        l0_ulid_cutoff(&dirty, &current).unwrap();
    }

    #[test]
    fn dirty_adds_l0_above_watermark_ok() {
        let current = manifest_with_root_l0(&[100]);
        let dirty = manifest_with_root_l0(&[100, 101]);
        l0_ulid_cutoff(&dirty, &current).unwrap();
    }

    #[test]
    fn dirty_adds_l0_below_watermark_errors() {
        let current = manifest_with_root_l0(&[100]);
        let dirty = manifest_with_root_l0(&[100, 50]);
        let err = l0_ulid_cutoff(&dirty, &current).unwrap_err();
        let err = err.downcast::<SlateDBError>().unwrap();
        assert!(matches!(
            *err,
            SlateDBError::InvalidClockTick {
                last_tick: 100,
                next_tick: 50
            }
        ));
    }

    #[test]
    fn dirty_adds_l0_equal_to_watermark_ok() {
        // Equality is allowed (`>=`), mirroring the GC's strict-`<` deletion
        // filter: an SST minted in the same millisecond as the watermark survives
        // the GC, so it is safe to publish and must not trip the invariant. This
        // is what keeps same-millisecond ULID collisions off the retry path.
        let current = manifest_with_root_l0(&[100]);
        let dirty = manifest_with_root_l0(&[100, 100]);
        l0_ulid_cutoff(&dirty, &current).unwrap();
    }

    #[test]
    fn reincluding_existing_l0_is_idempotent() {
        // `dirty` re-publishes the same L0 set with no additions — even though
        // those ULIDs equal the watermark, they are not "newly added" so the
        // check passes.
        let current = manifest_with_root_l0(&[100, 200]);
        let dirty = manifest_with_root_l0(&[100, 200]);
        l0_ulid_cutoff(&dirty, &current).unwrap();
    }

    #[test]
    fn watermark_uses_last_compacted_marker() {
        // The watermark must include `last_compacted_l0_sst_view_id`, not just
        // live L0 views. Here the live L0 is empty but a compacted marker at 300
        // exists, so a dirty L0 at 250 must be rejected.
        let mut core = ManifestCore::new();
        Arc::make_mut(&mut core.tree).last_compacted_l0_sst_view_id =
            Some(Ulid::from_parts(300, 0));
        let current = Manifest::initial(core);
        let dirty = manifest_with_root_l0(&[250]);
        let err = l0_ulid_cutoff(&dirty, &current).unwrap_err();
        let err = err.downcast::<SlateDBError>().unwrap();
        assert!(matches!(
            *err,
            SlateDBError::InvalidClockTick {
                last_tick: 300,
                next_tick: 250
            }
        ));
    }

    #[test]
    fn watermark_is_max_across_all_trees() {
        // A segment tree carries the highest watermark (400); a dirty L0 added to
        // the root at 350 is below the cross-tree max and must be rejected.
        let mut current_core = ManifestCore::new();
        current_core.segments.push(Segment {
            prefix: bytes::Bytes::from_static(b"seg"),
            tree: Arc::new(LsmTreeState {
                l0: VecDeque::from(vec![l0_view(400, 0)]),
                ..LsmTreeState::default()
            }),
        });
        let current = Manifest::initial(current_core);

        let mut dirty_core = ManifestCore::new();
        Arc::make_mut(&mut dirty_core.tree).l0 = VecDeque::from(vec![l0_view(350, 0)]);
        // The segment's L0 is unchanged (same ULID), so it is not a new addition.
        dirty_core.segments.push(Segment {
            prefix: bytes::Bytes::from_static(b"seg"),
            tree: Arc::new(LsmTreeState {
                l0: VecDeque::from(vec![l0_view(400, 0)]),
                ..LsmTreeState::default()
            }),
        });
        let dirty = Manifest::initial(dirty_core);

        let err = l0_ulid_cutoff(&dirty, &current).unwrap_err();
        let err = err.downcast::<SlateDBError>().unwrap();
        assert!(matches!(
            *err,
            SlateDBError::InvalidClockTick {
                last_tick: 400,
                next_tick: 350
            }
        ));
    }

    #[test]
    fn check_uses_physical_sst_ulid_not_view_ulid() {
        // Regression guard: the dirty L0's *SST* ULID (50) is below the watermark
        // (100), but its *view* ULID (999) is above it. The check must key off the
        // SST ULID — the ID the GC uses — and reject. Validating the view ULID
        // would wrongly pass and leave a sub-cutoff SST eligible for deletion.
        let current = manifest_with_root_l0(&[100]);

        let mut dirty_core = ManifestCore::new();
        Arc::make_mut(&mut dirty_core.tree).l0 = VecDeque::from(vec![l0_view_split(999, 50)]);
        let dirty = Manifest::initial(dirty_core);

        let err = l0_ulid_cutoff(&dirty, &current).unwrap_err();
        let err = err.downcast::<SlateDBError>().unwrap();
        assert!(matches!(
            *err,
            SlateDBError::InvalidClockTick {
                last_tick: 100,
                next_tick: 50
            }
        ));
    }
}
