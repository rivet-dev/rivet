use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use futures::future::BoxFuture;
use tokio::sync::watch;

use crate::error::SlateDBError;
use crate::manifest::VersionedManifest;
use crate::utils::{WatchableOnceCell, WatchableOnceCellReader};
use crate::CloseReason;

/// A segment (RFC-0024), identified by the key prefix it owns; the segment
/// spans the key interval `[prefix, prefix++)`.
///
/// Note: this is distinct from [`crate::manifest::Segment`], which also carries
/// per-segment LSM state. This type is prefix-only.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SegmentPrefix {
    /// The key prefix owned by the segment.
    pub prefix: Bytes,
}

/// Current status of the database, exposed via [`crate::Db::subscribe`].
///
/// Subscribers receive a [`tokio::sync::watch::Receiver<DbStatus>`] which
/// always reflects the latest state. When the database is dropped the watch
/// channel closes and [`changed()`](tokio::sync::watch::Receiver::changed)
/// returns an error.
#[derive(Clone, Debug, PartialEq)]
pub struct DbStatus {
    /// The durable sequence number. All writes with a sequence number less
    /// than or equal to this value are durably persisted to object storage
    /// and will survive process restarts.
    pub durable_seq: u64,
    /// The current in-memory manifest snapshot observed by this handle,
    /// paired with its manifest version ID.
    pub current_manifest: VersionedManifest,
    /// Segment prefixes (RFC-0024) touched by writes or WAL replay in this
    /// handle's memtables but not yet flushed to the manifest. Empty when no
    /// segment extractor is configured. Read it via
    /// [`DbStatus::list_segments`], which merges in the segments in the manifest.
    memtable_segments: BTreeSet<Bytes>,
    /// Set once the database has been closed, indicating the reason.
    pub close_reason: Option<CloseReason>,
}

impl DbStatus {
    /// List all segment prefixes (RFC-0024): those in the current manifest
    /// unioned with those touched in this handle's memtables but not yet
    /// flushed.
    ///
    /// The result is sorted ascending by prefix and deduplicated.
    pub fn list_segments(&self) -> Vec<SegmentPrefix> {
        let mut set: BTreeSet<Bytes> = self
            .current_manifest
            .core()
            .segments
            .iter()
            .map(|segment| segment.prefix().clone())
            .collect();
        set.extend(self.memtable_segments.iter().cloned());
        set.into_iter()
            .map(|prefix| SegmentPrefix { prefix })
            .collect()
    }
}

pub(crate) trait ClosedResultWriter: fmt::Debug + Send + Sync + 'static {
    fn write_result(&self, result: Result<(), SlateDBError>);
    fn result_reader(&self) -> WatchableOnceCellReader<Result<(), SlateDBError>>;
}

/// Manages database lifecycle status, including the close result and
/// status subscriptions.
#[derive(Clone)]
pub(crate) struct DbStatusManager {
    cell: WatchableOnceCell<Result<(), SlateDBError>>,
    tx: watch::Sender<DbStatus>,
    durability_waiter: DurabilityWaiter,
}

impl fmt::Debug for DbStatusManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DbStatusManager")
            .field("cell", &self.cell)
            .field("tx", &self.tx)
            .finish_non_exhaustive()
    }
}

impl DbStatusManager {
    #[cfg(test)]
    pub(crate) fn new(initial_durable_seq: u64) -> Self {
        use crate::manifest::Manifest;
        use crate::manifest::ManifestCore;
        Self::new_with_initial_values(
            initial_durable_seq,
            VersionedManifest {
                id: 1,
                manifest: Manifest::initial(ManifestCore::new()),
            },
            BTreeSet::new(),
        )
    }

    pub(crate) fn new_with_initial_values(
        initial_durable_seq: u64,
        initial_manifest: VersionedManifest,
        initial_memtable_segments: BTreeSet<Bytes>,
    ) -> Self {
        let (tx, _) = watch::channel(DbStatus {
            durable_seq: initial_durable_seq,
            current_manifest: initial_manifest,
            memtable_segments: initial_memtable_segments,
            close_reason: None,
        });
        let cell = WatchableOnceCell::new();
        let durability_waiter = new_durability_waiter(tx.subscribe(), cell.reader());

        Self {
            cell,
            tx,
            durability_waiter,
        }
    }

    pub(crate) fn report_fence_manifest(&self, durable_seq: u64, manifest: VersionedManifest) {
        self.tx.send_if_modified(|s| {
            s.durable_seq = durable_seq;
            s.current_manifest = manifest;
            true
        });
    }

    pub(crate) fn report_durable_seq(&self, seq: u64) {
        self.tx.send_if_modified(|s| {
            if seq > s.durable_seq {
                s.durable_seq = seq;
                true
            } else {
                false
            }
        });
    }

    pub(crate) fn report_manifest(&self, versioned: VersionedManifest) {
        self.tx.send_if_modified(|s| {
            if versioned.id >= s.current_manifest.id && s.current_manifest != versioned {
                s.current_manifest = versioned;
                true
            } else {
                false
            }
        });
    }

    /// Replace the published set of memtable segment prefixes (RFC-0024) with
    /// `prefixes`, the union over the handle's currently live memtables. Unlike
    /// [`Self::add_memtable_segments`], this can *shrink* the set, so it is the
    /// path used whenever a recomputed union may have dropped a prefix.
    /// Notifies subscribers only when the set actually changes.
    pub(crate) fn report_memtable_segments(&self, prefixes: BTreeSet<Bytes>) {
        self.tx.send_if_modified(|s| {
            if s.memtable_segments != prefixes {
                s.memtable_segments = prefixes;
                true
            } else {
                false
            }
        });
    }

    /// Atomically replace both the current manifest and the published memtable
    /// segment set (RFC-0024) in a single status update. Combines the guards of
    /// [`Self::report_manifest`] and [`Self::report_memtable_segments`] so
    /// subscribers never observe a torn state — e.g. the manifest already grown
    /// with the just-flushed prefixes while the memtable set has not yet shrunk
    /// to drop them (or the reverse). Notifies subscribers once if either field
    /// changes.
    pub(crate) fn report_manifest_and_memtable_segments(
        &self,
        versioned: VersionedManifest,
        prefixes: BTreeSet<Bytes>,
    ) {
        self.tx.send_if_modified(|s| {
            let mut changed = false;
            if versioned.id >= s.current_manifest.id && s.current_manifest != versioned {
                s.current_manifest = versioned;
                changed = true;
            }
            if s.memtable_segments != prefixes {
                s.memtable_segments = prefixes;
                changed = true;
            }
            changed
        });
    }

    /// Add the touched memtable segment prefixes (RFC-0024) in `prefixes` to the
    /// published set. This only ever *grows* the set, so it avoids recomputing
    /// the full union over all live memtables: it inserts just the given
    /// prefixes and notifies only when at least one was not already published.
    /// Use [`Self::report_memtable_segments`] when the set may need to shrink.
    pub(crate) fn add_memtable_segments(&self, prefixes: &BTreeSet<Bytes>) {
        if prefixes.is_empty() {
            return;
        }
        self.tx.send_if_modified(|s| {
            let mut changed = false;
            for prefix in prefixes {
                changed |= s.memtable_segments.insert(prefix.clone());
            }
            changed
        });
    }

    fn report_closed(&self, reason: CloseReason) {
        self.tx.send_if_modified(|s| {
            if s.close_reason.is_none() {
                s.close_reason = Some(reason);
                true
            } else {
                false
            }
        });
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<DbStatus> {
        self.tx.subscribe()
    }

    /// Returns the shared [`DurabilityWaiter`] for this database.
    pub(crate) fn durability_waiter(&self) -> DurabilityWaiter {
        Arc::clone(&self.durability_waiter)
    }

    pub(crate) fn status(&self) -> DbStatus {
        self.tx.borrow().clone()
    }
}

impl ClosedResultWriter for WatchableOnceCell<Result<(), SlateDBError>> {
    fn write_result(&self, result: Result<(), SlateDBError>) {
        self.write(result);
    }

    fn result_reader(&self) -> WatchableOnceCellReader<Result<(), SlateDBError>> {
        self.reader()
    }
}

impl ClosedResultWriter for DbStatusManager {
    fn write_result(&self, result: Result<(), SlateDBError>) {
        let reason = match &result {
            Ok(()) => CloseReason::Clean,
            Err(err) => CloseReason::from(crate::Error::from(err.clone()).kind()),
        };
        if self.cell.write(result) {
            self.report_closed(reason);
        }
    }

    fn result_reader(&self) -> WatchableOnceCellReader<Result<(), SlateDBError>> {
        self.cell.reader()
    }
}

/// Shared callback used by [`crate::WriteHandle`]s to wait for a sequence
/// number to become durable.
pub(crate) type DurabilityWaiter =
    Arc<dyn Fn(u64) -> BoxFuture<'static, Result<(), crate::Error>> + Send + Sync + 'static>;

/// Creates a [`DurabilityWaiter`] backed by database status and close-result
/// readers.
///
/// The waiter captures only readers, so cloning it into a write handle does not
/// keep the database's status sender alive after the database is dropped.
fn new_durability_waiter(
    status_rx: watch::Receiver<DbStatus>,
    close_result: WatchableOnceCellReader<Result<(), SlateDBError>>,
) -> DurabilityWaiter {
    Arc::new(move |seq| -> BoxFuture<'static, Result<(), crate::Error>> {
        let mut status_rx = status_rx.clone();
        let close_result = close_result.clone();

        Box::pin(async move {
            let wait_result = status_rx
                .wait_for(|status| status.durable_seq >= seq || status.close_reason.is_some())
                .await;

            match wait_result {
                Ok(status) if status.durable_seq >= seq => Ok(()),
                // The write was not durable before the database closed. Use the
                // recorded close result to preserve fencing and panic errors.
                Ok(_) | Err(_) => match close_result
                    .read()
                    .expect("database closed without recording a close result")
                {
                    Ok(()) => Err(SlateDBError::Closed.into()),
                    Err(error) => Err(error.into()),
                },
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use crate::manifest::ManifestCore;

    fn versioned_manifest(id: u64) -> VersionedManifest {
        VersionedManifest {
            id,
            manifest: Manifest::initial(ManifestCore::new()),
        }
    }

    fn manifest_with_segments(id: u64, prefixes: &[&[u8]]) -> VersionedManifest {
        use crate::manifest::{LsmTreeState, Segment};
        use std::sync::Arc;
        let mut core = ManifestCore::new();
        core.segment_extractor_name = Some("test".to_string());
        core.segments = prefixes
            .iter()
            .map(|p| Segment {
                prefix: Bytes::from(p.to_vec()),
                tree: Arc::new(LsmTreeState::default()),
            })
            .collect();
        VersionedManifest {
            id,
            manifest: Manifest::initial(core),
        }
    }

    fn segment_prefix(prefix: &[u8]) -> SegmentPrefix {
        SegmentPrefix {
            prefix: Bytes::from(prefix.to_vec()),
        }
    }

    fn segment_set(status: &DbStatus) -> BTreeSet<SegmentPrefix> {
        status
            .memtable_segments
            .iter()
            .map(|prefix| SegmentPrefix {
                prefix: prefix.clone(),
            })
            .collect()
    }

    #[test]
    fn should_initialize_with_no_segments_from_empty_manifest() {
        // given
        let mgr =
            DbStatusManager::new_with_initial_values(0, versioned_manifest(1), BTreeSet::new());

        // when
        let status = mgr.status();

        // then
        assert!(status.list_segments().is_empty());
    }

    #[test]
    fn should_initialize_with_segments_from_manifest() {
        // given
        let mgr = DbStatusManager::new_with_initial_values(
            0,
            manifest_with_segments(1, &[b"a", b"b"]),
            BTreeSet::new(),
        );

        // when
        let status = mgr.status();

        // then
        assert_eq!(
            status.list_segments(),
            vec![segment_prefix(b"a"), segment_prefix(b"b")]
        );
    }

    #[test]
    fn should_union_and_dedup_segments() {
        // given
        let mgr = DbStatusManager::new_with_initial_values(
            0,
            manifest_with_segments(1, &[b"a", b"b"]),
            BTreeSet::from([Bytes::from_static(b"b"), Bytes::from_static(b"c")]),
        );

        // when
        let segments = mgr.status().list_segments();

        // then
        assert_eq!(
            segments,
            vec![
                segment_prefix(b"a"),
                segment_prefix(b"b"),
                segment_prefix(b"c")
            ]
        );
    }

    #[test]
    fn should_return_sorted_segments() {
        // given
        let mgr = DbStatusManager::new_with_initial_values(
            0,
            manifest_with_segments(1, &[b"d", b"b"]),
            BTreeSet::from([Bytes::from_static(b"c"), Bytes::from_static(b"a")]),
        );

        // when
        let segments = mgr.status().list_segments();

        // then
        assert_eq!(
            segments,
            vec![
                segment_prefix(b"a"),
                segment_prefix(b"b"),
                segment_prefix(b"c"),
                segment_prefix(b"d")
            ]
        );
    }

    #[test]
    fn should_notify_when_reported_segments_change() {
        // given
        let mgr = DbStatusManager::new(0);
        let mut rx = mgr.subscribe();
        rx.borrow_and_update();

        // when
        mgr.report_memtable_segments(BTreeSet::from([Bytes::from_static(b"x")]));

        // then
        assert!(rx.has_changed().unwrap());
        assert_eq!(
            rx.borrow_and_update().memtable_segments,
            BTreeSet::from([Bytes::from_static(b"x")])
        );
    }

    #[test]
    fn should_notify_when_adding_new_segments() {
        // given
        let mgr = DbStatusManager::new(0);
        let mut rx = mgr.subscribe();
        rx.borrow_and_update();

        // when
        mgr.add_memtable_segments(&BTreeSet::from([Bytes::from_static(b"m")]));

        // then
        assert!(rx.has_changed().unwrap());
        assert_eq!(
            segment_set(&rx.borrow_and_update()),
            BTreeSet::from([segment_prefix(b"m")])
        );

        // when
        mgr.add_memtable_segments(&BTreeSet::from([
            Bytes::from_static(b"a"),
            Bytes::from_static(b"z"),
        ]));

        // then
        assert!(rx.has_changed().unwrap());
        assert_eq!(
            segment_set(&rx.borrow_and_update()),
            BTreeSet::from([
                segment_prefix(b"a"),
                segment_prefix(b"m"),
                segment_prefix(b"z")
            ])
        );
    }

    #[test]
    fn should_not_notify_when_adding_a_known_segment() {
        // given
        let mgr = DbStatusManager::new_with_initial_values(
            0,
            manifest_with_segments(1, &[b"a", b"b"]),
            BTreeSet::from([Bytes::from_static(b"x"), Bytes::from_static(b"y")]),
        );
        let mut rx = mgr.subscribe();
        rx.borrow_and_update();

        // when
        mgr.add_memtable_segments(&BTreeSet::from([Bytes::from_static(b"x")]));

        // then
        assert!(!rx.has_changed().unwrap());

        // when
        mgr.add_memtable_segments(&BTreeSet::from([
            Bytes::from_static(b"x"),
            Bytes::from_static(b"y"),
        ]));

        // then
        assert!(!rx.has_changed().unwrap());

        // when
        mgr.add_memtable_segments(&BTreeSet::from([
            Bytes::from_static(b"x"),
            Bytes::from_static(b"y"),
            Bytes::from_static(b"z"),
        ]));

        // then
        assert!(rx.has_changed().unwrap());
    }

    #[test]
    fn should_not_notify_when_adding_no_segments() {
        // given: a write on a non-segmented database reports an empty set.
        let mgr = DbStatusManager::new(0);
        let mut rx = mgr.subscribe();
        rx.borrow_and_update();

        // when
        mgr.add_memtable_segments(&BTreeSet::new());

        // then
        assert!(!rx.has_changed().unwrap());
        assert!(rx.borrow_and_update().list_segments().is_empty());
    }

    #[test]
    fn should_not_notify_on_same_manifest() {
        // given
        let initial = versioned_manifest(1);
        let mgr = DbStatusManager::new_with_initial_values(0, initial.clone(), BTreeSet::new());
        let mut rx = mgr.subscribe();
        rx.borrow_and_update();

        // when
        mgr.report_manifest(initial);

        // then
        assert!(!rx.has_changed().unwrap());
    }

    #[test]
    fn should_not_notify_on_older_manifest() {
        // given
        let mgr =
            DbStatusManager::new_with_initial_values(0, versioned_manifest(5), BTreeSet::new());
        let mut rx = mgr.subscribe();
        rx.borrow_and_update();

        // when
        mgr.report_manifest(versioned_manifest(3));

        // then
        assert!(!rx.has_changed().unwrap());
    }

    #[test]
    fn should_update_manifest_and_segments_atomically() {
        // given: manifest has no segments yet, and "a"/"b" are touched in the
        // memtables.
        let mgr = DbStatusManager::new_with_initial_values(
            0,
            manifest_with_segments(1, &[]),
            BTreeSet::from([Bytes::from_static(b"a"), Bytes::from_static(b"b")]),
        );
        let mut rx = mgr.subscribe();
        rx.borrow_and_update();

        // when: a flush moves "a" into the manifest and leaves only "b" in the
        // memtables, reported as a single transition.
        mgr.report_manifest_and_memtable_segments(
            manifest_with_segments(2, &[b"a"]),
            BTreeSet::from([Bytes::from_static(b"b")]),
        );

        // then: exactly one notification, and the observed state reflects both
        // halves — "a" never disappears from list_segments.
        assert!(rx.has_changed().unwrap());
        let status = rx.borrow_and_update();
        assert_eq!(status.current_manifest.id, 2);
        assert_eq!(
            status.memtable_segments,
            BTreeSet::from([Bytes::from_static(b"b")])
        );
        assert_eq!(
            status.list_segments(),
            vec![segment_prefix(b"a"), segment_prefix(b"b")]
        );
        drop(status);
        // the single send_if_modified leaves no further pending change.
        assert!(!rx.has_changed().unwrap());
    }

    #[test]
    fn should_not_notify_when_manifest_and_segments_unchanged() {
        // given
        let mgr = DbStatusManager::new_with_initial_values(
            0,
            manifest_with_segments(1, &[b"a"]),
            BTreeSet::from([Bytes::from_static(b"b")]),
        );
        let mut rx = mgr.subscribe();
        rx.borrow_and_update();

        // when: re-report the same manifest id and the same memtable segments.
        mgr.report_manifest_and_memtable_segments(
            manifest_with_segments(1, &[b"a"]),
            BTreeSet::from([Bytes::from_static(b"b")]),
        );

        // then
        assert!(!rx.has_changed().unwrap());
    }
}
