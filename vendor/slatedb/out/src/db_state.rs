use crate::bytes_range::BytesRange;
use crate::config::CompressionCodec;
use crate::error::SlateDBError;
use crate::manifest::{Manifest, ManifestCore};
use crate::mem_table::{ImmutableMemtable, KVTable, WritableKVTable};
use crate::reader::DbStateReader;
use bytes::Bytes;
use serde::Serialize;
use slatedb_txn_obj::DirtyObject;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::ops::Bound::{Excluded, Included, Unbounded};
use std::ops::{Bound, Range, RangeBounds};
use std::sync::Arc;
use ulid::Ulid;

/// A handle to an SSTable — the physical SST on storage.
#[derive(Clone, PartialEq, Serialize)]
pub struct SsTableHandle {
    /// The unique identifier for this compacted SSTable.
    pub id: SsTableId,

    /// The format version that this SSTable was serialized with.
    pub(crate) format_version: u16,

    /// Metadata information about this SSTable.
    pub info: SsTableInfo,
}

impl Debug for SsTableHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("SsTableHandle({:?})", self.id))
    }
}

impl SsTableHandle {
    pub(crate) fn new(id: SsTableId, format_version: u16, info: SsTableInfo) -> Self {
        SsTableHandle {
            id,
            format_version,
            info,
        }
    }

    /// Returns an estimate of the SST's on-disk size in bytes.
    ///
    /// This is a rough estimate: the index is the last thing written before
    /// the info footer, so `index_offset + index_len` approximates the file size.
    pub fn estimate_size(&self) -> u64 {
        self.info.index_offset + self.info.index_len
    }
}

impl AsRef<SsTableHandle> for SsTableHandle {
    fn as_ref(&self) -> &SsTableHandle {
        self
    }
}

/// A projected view of an SSTable, combining the physical SST handle with an
/// optional visible_range projection.
#[derive(Clone, PartialEq, Serialize)]
pub struct SsTableView {
    /// Unique identifier for this view.
    pub id: Ulid,

    /// The underlying physical SSTable handle.
    pub sst: SsTableHandle,

    /// The range of keys that are visible to the user. If non-empty, this view represents a projection
    /// over the SST file.
    pub(crate) visible_range: Option<BytesRange>,

    /// The effective range of keys that are visible to the user, which is the intersection of the
    /// physical range (first_key..unbounded) and any projection range.
    effective_range: BytesRange,
}

impl Debug for SsTableView {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "SsTableView({:?}, {:?})",
            self.sst.id, self.visible_range
        ))
    }
}

impl SsTableView {
    /// Create a view using a deterministic id derived from the SST's own identity.
    /// Use this only where no `DbRand` is available and the id is not stored in
    /// the manifest.
    pub(crate) fn identity(sst: SsTableHandle) -> Self {
        Self::new(sst.id.value(), sst)
    }

    /// Create a new view with no visible_range projection.
    pub(crate) fn new(id: Ulid, sst: SsTableHandle) -> Self {
        let effective_range = sst
            .info
            .physical_range()
            .unwrap_or_else(BytesRange::new_empty);

        SsTableView {
            id,
            sst,
            visible_range: None,
            effective_range,
        }
    }

    /// Create a new projected view with an optional visible_range.
    pub(crate) fn new_projected(
        id: Ulid,
        sst: SsTableHandle,
        visible_range: Option<BytesRange>,
    ) -> Self {
        let mut effective_range = sst
            .info
            .physical_range()
            .expect("SST always has a first entry.");
        if let Some(visible_range) = &visible_range {
            assert!(
                visible_range.is_start_bound_included_or_unbounded(),
                "Start bound of the visible range must be either Included or Unbounded."
            );
            effective_range = effective_range
                .intersect(visible_range)
                .expect("An intersection of visible and physical range must be non-empty.")
        }
        SsTableView {
            id,
            sst,
            visible_range,
            effective_range,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_visible_range(&self, visible_range: BytesRange) -> Self {
        Self::new_projected(self.id, self.sst.clone(), Some(visible_range))
    }

    /// The SST's physical key range, derived from its first/last entry. This is
    /// the range that [`Self::new_projected`] intersects a visible range
    /// against.
    fn physical_range(&self) -> BytesRange {
        self.sst
            .info
            .physical_range()
            .expect("SST always has a first entry.")
    }

    /// Like [`Self::with_visible_range`], but returns `None` instead of
    /// panicking when `visible_range` does not overlap the SST's physical key
    /// range.
    ///
    /// [`Self::compacted_intersection`] bounds a sorted-run SST's logical
    /// coverage by the *next* SST's start key, which can extend past this SST's
    /// physical last key. When a projection range falls entirely into the gap
    /// between this SST's last physical key and the next SST's start key, the
    /// SST owns the range logically but holds no physical keys in it: it
    /// contributes nothing to the projection and is dropped rather than
    /// constructing a view whose physical/visible intersection is empty.
    pub(crate) fn try_with_visible_range(&self, visible_range: BytesRange) -> Option<Self> {
        self.physical_range().intersect(&visible_range)?;
        Some(Self::new_projected(
            self.id,
            self.sst.clone(),
            Some(visible_range),
        ))
    }

    /// The range of keys that are visible to the user.
    ///
    /// ## Returns
    /// - `Some(BytesRange)` if there is a projection applied to this SST.
    /// - `None` if the entire SST is visible.
    pub fn visible_range(&self) -> Option<impl RangeBounds<Bytes>> {
        self.visible_range.clone()
    }

    // Compacted (non-WAL) SSTs are never empty. They are created by compaction or
    // memtable flushes, which should never produce empty SSTs. This method returns
    // the start bound after applying projections.
    pub(crate) fn compacted_effective_start_bound(&self) -> Bound<Bytes> {
        self.effective_range.start_bound().cloned()
    }

    // Compacted (non-WAL) SSTs are never empty. They are created by compaction or
    // memtable flushes, which should never produce empty SSTs. This method returns
    // the start key after applying projections.
    pub(crate) fn compacted_effective_start_key(&self) -> &Bytes {
        match self.effective_range.start_bound() {
            Included(k) => k,
            _ => unreachable!("Invalid start bound"),
        }
    }

    pub(crate) fn compacted_effective_range(&self) -> &BytesRange {
        &self.effective_range
    }

    pub(crate) fn compacted_intersection(
        &self,
        next_view: Option<&SsTableView>,
        range: &BytesRange,
    ) -> Option<BytesRange> {
        if let Some(next_view) = next_view {
            BytesRange::new(
                self.compacted_effective_start_bound(),
                Excluded(next_view.compacted_effective_start_key().clone()),
            )
            .intersect(range)
        } else {
            self.effective_range.intersect(range)
        }
    }

    pub(crate) fn intersects_range(&self, end_bound: Bound<Bytes>, range: &BytesRange) -> bool {
        let sst_range =
            BytesRange::new(Unbounded, end_bound.clone()).intersect(&self.effective_range);
        match sst_range {
            Some(sst_range) => BytesRange::new(sst_range.start_bound().cloned(), end_bound)
                .intersect(range)
                .is_some(),
            None => false,
        }
    }

    /// Calculate the view range for the given range.
    ///
    /// This method determines the effective range that can be accessed within an SST by:
    /// 1. Intersecting the requested range with the effective range
    /// 2. Returning None if the requested range does not overlap with the effective range
    pub(crate) fn calculate_view_range(&self, range: BytesRange) -> Option<BytesRange> {
        if let Some(visible_range) = &self.visible_range {
            return range.intersect(visible_range);
        }
        if self.sst.info.last_entry.is_some() {
            return range.intersect(&self.effective_range);
        }
        Some(range)
    }

    /// Returns an estimate of the underlying SST's on-disk size in bytes.
    pub fn estimate_size(&self) -> u64 {
        self.sst.estimate_size()
    }

    pub(crate) fn estimate_visible_size(&self) -> u64 {
        const MIN_ESTIMATED_SIZE_BYTES: f64 = 1.0;

        let raw_size = self.sst.estimate_size();
        if self.visible_range.is_none() {
            return raw_size;
        }
        let fraction = self.visible_fraction();
        ((raw_size as f64) * fraction)
            .round()
            .max(MIN_ESTIMATED_SIZE_BYTES) as u64
    }

    /// This function finds the size of the visible part of a file:
    /// - Compares the start and end keys of the file and the view.
    /// - Skips the bytes that are equal and reads the bytes after them.
    /// - Uses these bytes to find the fraction of the full file size.
    fn visible_fraction(&self) -> f64 {
        const FULL_VISIBLE_FRACTION: f64 = 1.0;
        const MIN_VISIBLE_FRACTION: f64 = 1e-6;

        if self.sst.info.last_entry.is_none() {
            return FULL_VISIBLE_FRACTION;
        }

        let physical_range = self.physical_range();
        if self.effective_range == physical_range {
            return FULL_VISIBLE_FRACTION;
        }

        let skip = common_prefix_len(physical_range.start_bound(), physical_range.end_bound());

        let (phys_start, phys_end) = Self::range_span(&physical_range, skip);
        let (vis_start, vis_end) = Self::range_span(&self.effective_range, skip);

        let phys_len = (phys_end - phys_start).max(f64::EPSILON);
        let vis_len = vis_end - vis_start;
        if vis_len == 0.0 {
            return FULL_VISIBLE_FRACTION;
        }

        (vis_len / phys_len).clamp(MIN_VISIBLE_FRACTION, FULL_VISIBLE_FRACTION)
    }

    /// This function finds a position number for the start and end of a range:
    /// - Skips the bytes given by `skip`, then reads the bytes after them.
    /// - Turns these bytes into a number between 0 and 1.
    /// - Uses 0 for an open start and 1 for an open end.
    fn range_span(range: &BytesRange, skip: usize) -> (f64, f64) {
        const KEYSPACE_START: f64 = 0.0;
        const KEYSPACE_END: f64 = 1.0;

        fn key_fraction(key: &[u8], skip: usize) -> f64 {
            const BYTE_VALUE_RANGE: f64 = 256.0;
            const FRACTION_WINDOW_BYTES: usize = 8;

            let key = key.get(skip..).unwrap_or(&[]);
            let mut frac = 0.0_f64;
            let mut scale = 1.0_f64 / BYTE_VALUE_RANGE;
            // The code reads up to 8 bytes after the skip point, which gives a fine result for almost all keys.
            // Checking more bytes adds little value, but costs a little more time.
            for &byte in key.iter().take(FRACTION_WINDOW_BYTES) {
                frac += byte as f64 * scale;
                scale /= BYTE_VALUE_RANGE;
            }
            frac
        }

        let start = match range.start_bound() {
            Unbounded => KEYSPACE_START,
            Included(k) | Excluded(k) => key_fraction(k, skip),
        };
        let end = match range.end_bound() {
            Unbounded => KEYSPACE_END,
            Included(k) | Excluded(k) => key_fraction(k, skip),
        };
        (start, end)
    }
}

fn common_prefix_len(a: Bound<&Bytes>, b: Bound<&Bytes>) -> usize {
    match (a, b) {
        (Included(a) | Excluded(a), Included(b) | Excluded(b)) => {
            a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
        }
        _ => 0,
    }
}

/// Maximum number of L0 SST views whose effective ranges cover any single key.
///
/// A key `p` is covered by an L0 view when `p` lies within the view's effective
/// range. This is the per-key read-amplification for L0 lookups and is the
/// quantity bounded by `Settings::l0_max_ssts_per_key`.
///
/// Implementation: a sweep over 2·N boundary events. Each L0 view contributes a
/// "+1" event at its start bound and a "-1" event at its end bound. Events are
/// sorted by virtual position on a number line that distinguishes `just-before v`,
/// `at v`, and `just-after v`, so inclusive/exclusive bounds are ordered
/// correctly. At equal positions, `+1` events sort before `-1` so the peak
/// counts ranges that touch edge-to-edge at an inclusive boundary.
/// Complexity: O(N log N) time, O(N) space.
pub(crate) fn max_l0_overlap(l0: &VecDeque<SsTableView>) -> usize {
    if l0.is_empty() {
        return 0;
    }

    // Sign discriminates ±∞ sentinels from finite positions so derive-Ord
    // places -∞ first and +∞ last; for finite positions, we sort by (key,
    // offset) where offset is -1/0/+1 for "just before / at / just after".
    #[derive(Eq, PartialEq)]
    struct Pos {
        sign: i8,
        key: Bytes,
        offset: i8,
    }

    impl Ord for Pos {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.sign
                .cmp(&other.sign)
                .then_with(|| self.key.cmp(&other.key))
                .then_with(|| self.offset.cmp(&other.offset))
        }
    }
    impl PartialOrd for Pos {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    let start_pos = |r: &BytesRange| -> Pos {
        match r.start_bound() {
            Included(k) => Pos {
                sign: 0,
                key: k.clone(),
                offset: 0,
            },
            Excluded(k) => Pos {
                sign: 0,
                key: k.clone(),
                offset: 1,
            },
            Unbounded => Pos {
                sign: -1,
                key: Bytes::new(),
                offset: 0,
            },
        }
    };
    let end_pos = |r: &BytesRange| -> Pos {
        match r.end_bound() {
            Included(k) => Pos {
                sign: 0,
                key: k.clone(),
                offset: 0,
            },
            Excluded(k) => Pos {
                sign: 0,
                key: k.clone(),
                offset: -1,
            },
            Unbounded => Pos {
                sign: 1,
                key: Bytes::new(),
                offset: 0,
            },
        }
    };

    let mut events: Vec<(Pos, i8)> = Vec::with_capacity(l0.len() * 2);
    for view in l0 {
        let range = view.compacted_effective_range();
        events.push((start_pos(range), 1));
        events.push((end_pos(range), -1));
    }
    events.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));

    let mut active: i32 = 0;
    let mut peak: i32 = 0;
    for (_, delta) in events {
        active += delta as i32;
        if active > peak {
            peak = active;
        }
    }
    peak as usize
}

/// An identifier for a compacted SSTable.
#[derive(Clone, PartialEq, PartialOrd, Ord, Hash, Eq, Copy, Serialize)]
pub struct SsTableId(Ulid);

impl SsTableId {
    pub const fn new(value: Ulid) -> Self {
        Self(value)
    }

    pub fn value(&self) -> Ulid {
        self.0
    }
}

impl From<Ulid> for SsTableId {
    fn from(value: Ulid) -> Self {
        Self::new(value)
    }
}

impl Debug for SsTableId {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "SsTableId({})", self.0)
    }
}

/// The type of an SSTable, distinguishing between compacted and WAL SSTs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub enum SstType {
    /// A compacted SST (L0 or sorted run). This is the default for backwards compatibility.
    #[default]
    Compacted,
    /// A WAL (Write-Ahead Log) SST.
    Wal,
}

impl From<&SsTableId> for SstType {
    fn from(_id: &SsTableId) -> Self {
        SstType::Compacted
    }
}

/// Filter block format stored in SsTableInfo.
/// Default is `Composite` (the current format). `Legacy` is only set when
/// decoding old SSTs via FlatBuffers (where the field is absent and maps to 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub enum FilterFormat {
    /// Single raw bloom filter bytes (pre-composite format).
    Legacy,
    /// Block containing one or more named filters, each prefixed with its name
    ///  and length.
    #[default]
    Composite,
}

/// Metadata information about an SSTable. See [`crate::sst_builder::EncodedSsTableBuilder`] for
/// more information on the format of the SSTable and its metadata.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SsTableInfo {
    /// The first entry in the SSTable, if any.
    /// The first entry is a key in an SST for compacted data
    /// and it is a sequence number in a WAL SST.
    pub first_entry: Option<Bytes>,
    /// The last entry in the SSTable, if any.
    /// The last entry is a key in an SST for compacted data.
    /// Not set for WAL SSTs (keys are not sorted).
    pub last_entry: Option<Bytes>,
    /// The offset of the index block within the SSTable file.
    pub index_offset: u64,
    /// The length of the index block within the SSTable file.
    pub index_len: u64,
    /// The offset of the filter block within the SSTable file.
    pub filter_offset: u64,
    /// The length of the filter block within the SSTable file.
    pub filter_len: u64,
    /// The compression codec used for the SSTable, if any.
    pub compression_codec: Option<CompressionCodec>,
    /// The type of this SSTable.
    pub sst_type: SstType,
    /// The offset of the stats block within the SSTable file.
    pub stats_offset: u64,
    /// The length of the stats block within the SSTable file.
    pub stats_len: u64,
    /// Filter block format.
    pub filter_format: FilterFormat,
}

impl SsTableInfo {
    pub(crate) fn physical_range(&self) -> Option<BytesRange> {
        self.first_entry.clone().map(|first_entry| {
            let end_bound = match self.last_entry.clone() {
                Some(last_entry) => Included(last_entry),
                None => Unbounded,
            };
            BytesRange::new(Included(first_entry), end_bound)
        })
    }
}

pub(crate) trait SsTableInfoCodec: Send + Sync {
    fn encode(&self, manifest: &SsTableInfo) -> Bytes;

    fn decode(&self, bytes: &Bytes) -> Result<SsTableInfo, SlateDBError>;

    fn clone_box(&self) -> Box<dyn SsTableInfoCodec>;
}

/// Implement Clone for Box<dyn SsTableInfoCodec> by delegating to the clone_box method.
/// This is the idiomatic way to clone trait objects in Rust. It is also the only way to
/// clone trait objects without knowing the concrete type.
impl Clone for Box<dyn SsTableInfoCodec> {
    fn clone(&self) -> Self {
        self.as_ref().clone_box()
    }
}

/// A sorted run consisting of multiple compacted SSTables.
#[derive(Clone, PartialEq, Serialize, Debug)]
pub struct SortedRun {
    /// The unique identifier for this sorted run.
    pub id: u32,
    /// The list of SSTable views in this sorted run.
    ///
    /// Held behind an `Arc` so cloning a `SortedRun` (e.g. per read in the
    /// scan path) is a single refcount bump rather than a deep clone of every
    /// view's `Bytes` handles.
    sst_views: Arc<[SsTableView]>,
}

impl SortedRun {
    /// Create a sorted run from an ordered collection of SSTable views.
    pub fn new(id: u32, sst_views: impl IntoIterator<Item = SsTableView>) -> Self {
        Self {
            id,
            sst_views: sst_views.into_iter().collect(),
        }
    }

    /// Return the ordered SSTable views in this sorted run.
    pub fn sst_views(&self) -> &[SsTableView] {
        &self.sst_views
    }

    /// Estimate the total size of all SSTables in this sorted run.
    pub fn estimate_size(&self) -> u64 {
        self.sst_views.iter().map(|sst| sst.estimate_size()).sum()
    }

    pub(crate) fn estimate_visible_size(&self) -> u64 {
        self.sst_views
            .iter()
            .map(|sst| sst.estimate_visible_size())
            .sum()
    }

    /// Cheap O(1) check: does the run's overall key span overlap `range`?
    ///
    /// Lets scan init skip whole runs with no potential overlap before spawning
    /// any per-run work.
    pub(crate) fn overlaps_range(&self, range: &BytesRange) -> bool {
        let Some(first) = self.sst_views.first() else {
            return false;
        };
        let last = self
            .sst_views
            .last()
            .expect("non-empty: first exists, so last exists");
        let span = BytesRange::new(
            first.compacted_effective_range().start_bound().cloned(),
            last.compacted_effective_range().end_bound().cloned(),
        );
        span.intersect(range).is_some()
    }

    pub(crate) fn find_last_sst_with_range_covering_key(&self, key: &[u8]) -> Option<usize> {
        // returns the sst after the one whose range includes the key
        let first_sst = self
            .sst_views
            .partition_point(|sst| sst.compacted_effective_start_key() <= key);
        if first_sst > 0 {
            return Some(first_sst - 1);
        }
        // all ssts have a range greater than the key
        None
    }

    /// Returns the logical end bound for the SST view at `idx`.
    ///
    /// The bound is normally the next view's start key, but becomes inclusive
    /// when adjacent views overlap on that boundary key.
    fn table_end_bound(&self, idx: usize) -> Bound<Bytes> {
        let current_sst = &self.sst_views[idx];
        if idx + 1 < self.sst_views.len() {
            let next_sst = &self.sst_views[idx + 1];
            if current_sst
                .compacted_effective_range()
                .contains(next_sst.compacted_effective_start_key())
            {
                Included(next_sst.compacted_effective_start_key().clone())
            } else {
                Excluded(next_sst.compacted_effective_start_key().clone())
            }
        } else {
            Unbounded
        }
    }

    /// Returns the contiguous range of SST view indices that can contribute to a
    /// point read for `key`.
    ///
    /// This uses the binary-search candidate as the upper bound, then walks
    /// backward only across immediately overlapping views.
    fn point_table_idx_covering_key(&self, key: &[u8]) -> Range<usize> {
        let Some(max_idx) = self.find_last_sst_with_range_covering_key(key) else {
            return 0..0;
        };
        let point_range = BytesRange::from_slice(key..=key);
        if !self.sst_views[max_idx].intersects_range(self.table_end_bound(max_idx), &point_range) {
            return 0..0;
        }

        let mut min_idx = max_idx;
        while min_idx > 0
            && self.sst_views[min_idx - 1]
                .intersects_range(self.table_end_bound(min_idx - 1), &point_range)
        {
            min_idx -= 1;
        }

        min_idx..(max_idx + 1)
    }

    fn table_idx_covering_range(&self, range: &BytesRange) -> Range<usize> {
        let views = &self.sst_views;
        if views.is_empty() {
            return 0..0;
        }

        // Upper bound: last SST whose start key falls within the range's upper
        // bound. SST[i]'s slot begins at start_key[i]; if start_key[i] is past
        // the range's upper bound the slot cannot overlap.
        let max_idx = match range.end_bound() {
            Included(hi) => {
                let p = views.partition_point(|sst| sst.compacted_effective_start_key() <= hi);
                if p == 0 {
                    return 0..0;
                }
                p - 1
            }
            Excluded(hi) => {
                let p = views.partition_point(|sst| sst.compacted_effective_start_key() < hi);
                if p == 0 {
                    return 0..0;
                }
                p - 1
            }
            Unbounded => views.len() - 1,
        };

        if !views[max_idx].intersects_range(self.table_end_bound(max_idx), range) {
            return 0..0;
        }

        // Lower bound candidate: last SST whose start_key <= range.start. SSTs
        // before it cannot intersect except via boundary-key overlap, handled
        // by the backward walk below.
        let start_candidate = match range.start_bound() {
            Included(lo) | Excluded(lo) => views
                .partition_point(|sst| sst.compacted_effective_start_key() <= lo)
                .checked_sub(1),
            Unbounded => None,
        };

        let mut min_idx = match start_candidate {
            Some(idx) if views[idx].intersects_range(self.table_end_bound(idx), range) => idx,
            Some(idx) => idx + 1,
            None => 0,
        };

        while min_idx > 0
            && views[min_idx - 1].intersects_range(self.table_end_bound(min_idx - 1), range)
        {
            min_idx -= 1;
        }

        min_idx..(max_idx + 1)
    }

    /// Returns the SST views in this sorted run that overlap the given key range.
    pub fn tables_covering_range<R: RangeBounds<Bytes>>(&self, range: R) -> VecDeque<&SsTableView> {
        let bytes_range = BytesRange::new(range.start_bound().cloned(), range.end_bound().cloned());
        let matching_range = self.table_idx_covering_range(&bytes_range);
        self.sst_views[matching_range].iter().collect()
    }

    /// Returns the SST views that may contain entries for the point key `key`.
    pub(crate) fn tables_covering_point_key(&self, key: &[u8]) -> &[SsTableView] {
        let matching_range = self.point_table_idx_covering_key(key);
        &self.sst_views[matching_range]
    }

    pub(crate) fn into_tables_covering_range(self, range: &BytesRange) -> VecDeque<SsTableView> {
        let matching_range = self.table_idx_covering_range(range);
        // `sst_views` is shared behind an `Arc`, so we clone only the few
        // covering views rather than draining the whole run. The full slice
        // is released with a single refcount decrement when `self` drops.
        self.sst_views[matching_range].iter().cloned().collect()
    }
}

pub(crate) struct DbState {
    memtable: WritableKVTable,
    state: Arc<COWDbState>,
}

// represents the state that is mutated by creating a new copy with the mutations
#[derive(Clone)]
pub(crate) struct COWDbState {
    pub(crate) imm_memtable: VecDeque<Arc<ImmutableMemtable>>,
    pub(crate) manifest: DirtyObject<Manifest>,
}

impl COWDbState {
    pub(crate) fn core(&self) -> &ManifestCore {
        &self.manifest.value.core
    }
}

// represents a consistent view of the current db state
#[derive(Clone)]
pub(crate) struct DbStateView {
    pub(crate) memtable: Arc<KVTable>,
    pub(crate) state: Arc<COWDbState>,
}

impl DbStateReader for DbStateView {
    fn memtable(&self) -> Arc<KVTable> {
        Arc::clone(&self.memtable)
    }

    fn imm_memtable(&self) -> &VecDeque<Arc<ImmutableMemtable>> {
        &self.state.imm_memtable
    }

    fn core(&self) -> &ManifestCore {
        self.state.core()
    }
}

/// Union of the segment prefixes (RFC-0024) touched across a handle's live
/// memtables — the active memtable plus every immutable memtable.
///
/// Used to (re)establish or shrink the published memtable segment set at the
/// points where the live-memtable set changes in a way a single write cannot
/// express: open / WAL replay / checkpoint refresh, and right after a flush pops
/// a memtable and folds its prefixes into the manifest. Recomputing the union
/// here makes the set drop a prefix once the last live memtable holding it has
/// flushed (and keeps it while any other live memtable still holds it). The
/// write path, by contrast, only ever *adds* prefixes, so it publishes
/// incrementally via [`DbStatusManager::add_memtable_segments`] instead of
/// calling this.
///
/// The set is empty for a non-segmented database — one whose manifest records
/// no segment extractor — so the caller need not gate the call itself.
pub(crate) fn collect_touched_segments(
    reader: &dyn DbStateReader,
) -> std::collections::BTreeSet<Bytes> {
    if reader.core().segment_extractor_name.is_none() {
        return std::collections::BTreeSet::new();
    }
    let mut set = reader.memtable().touched_segments();
    for imm in reader.imm_memtable() {
        set.extend(imm.table().touched_segments());
    }
    set
}

impl DbState {
    pub(crate) fn new(manifest: DirtyObject<Manifest>) -> Self {
        Self {
            memtable: WritableKVTable::new(),
            state: Arc::new(COWDbState {
                imm_memtable: VecDeque::new(),
                manifest,
            }),
        }
    }

    pub(crate) fn state(&self) -> Arc<COWDbState> {
        self.state.clone()
    }

    pub(crate) fn view(&self) -> DbStateView {
        DbStateView {
            memtable: self.memtable.table().clone(),
            state: self.state.clone(),
        }
    }

    pub(crate) fn memtable(&self) -> &WritableKVTable {
        &self.memtable
    }

    pub(crate) fn freeze_memtable(&mut self, replay_after_wal_id: u64) {
        let old_memtable = std::mem::replace(&mut self.memtable, WritableKVTable::new());
        self.modify(|modifier| {
            modifier
                .state
                .imm_memtable
                .push_front(Arc::new(ImmutableMemtable::new(
                    old_memtable,
                    replay_after_wal_id,
                )))
        });
    }

    pub(crate) fn set_next_wal_id(&mut self, next_wal_id: u64) {
        self.modify(|modifier| {
            assert!(next_wal_id >= modifier.state.manifest.value.core.next_wal_sst_id);
            modifier.state.manifest.value.core.next_wal_sst_id = next_wal_id;
        })
    }

    pub(crate) fn replace_memtable(&mut self, memtable: WritableKVTable) {
        assert!(self.memtable.is_empty());
        let _ = std::mem::replace(&mut self.memtable, memtable);
    }

    pub(crate) fn merge_remote_manifest(&mut self, remote_manifest: DirtyObject<Manifest>) {
        self.modify(|modifier| modifier.merge_remote_manifest(remote_manifest));
    }

    pub(crate) fn modify<F, R>(&mut self, fun: F) -> R
    where
        F: FnOnce(&mut StateModifier<'_>) -> R,
    {
        let mut modifier = StateModifier::new(self);
        let result = fun(&mut modifier);
        modifier.finish();
        result
    }
}

pub(crate) struct StateModifier<'a> {
    db_state: &'a mut DbState,
    pub(crate) state: COWDbState,
}

impl<'a> StateModifier<'a> {
    /// Create a new state modifier
    fn new(db_state: &'a mut DbState) -> Self {
        let state = db_state.state.as_ref().clone();
        Self { db_state, state }
    }

    pub(crate) fn merge_remote_manifest(&mut self, mut remote_manifest: DirtyObject<Manifest>) {
        let my_db_state = self.state.core();
        let remote = &remote_manifest.value.core;
        let tree = my_db_state.tree.merge_from_compactor(&remote.tree);
        let segments =
            crate::manifest::merge_segments_from_compactor(&my_db_state.segments, &remote.segments);
        remote_manifest.value.core = ManifestCore {
            initialized: my_db_state.initialized,
            tree: Arc::new(tree),
            segments,
            // Segment configuration is stable; the writer is the source of truth.
            segment_extractor_name: my_db_state.segment_extractor_name.clone(),
            next_wal_sst_id: my_db_state.next_wal_sst_id,
            replay_after_wal_id: my_db_state.replay_after_wal_id,
            last_l0_clock_tick: my_db_state.last_l0_clock_tick,
            last_l0_seq: my_db_state.last_l0_seq,
            recent_snapshot_min_seq: my_db_state.recent_snapshot_min_seq,
            sequence_tracker: my_db_state.sequence_tracker.clone(),
            checkpoints: remote_manifest.value.core.checkpoints,
            wal_object_store_uri: my_db_state.wal_object_store_uri.clone(),
        };
        remote_manifest.value.prune_external_sst_ids();
        self.state.manifest = remote_manifest;
    }

    fn finish(self) {
        self.db_state.state = Arc::new(self.state);
    }
}

#[cfg(test)]
mod tests {
    use crate::bytes_range::BytesRange;
    use crate::checkpoint::Checkpoint;
    use crate::db_state::{DbState, SortedRun, SsTableHandle, SsTableId, SsTableInfo, SsTableView};
    use crate::format::sst::SST_FORMAT_VERSION_LATEST;
    use crate::manifest::store::test_utils::new_dirty_manifest;
    use crate::manifest::{LsmTreeState, Segment};
    use crate::proptest_util::arbitrary;
    use crate::seq_tracker::{FindOption, SequenceTracker, TrackedSeq};
    use crate::test_utils;
    use bytes::Bytes;
    use chrono::{TimeZone, Utc};
    use proptest::collection::vec;
    use proptest::proptest;
    use slatedb_common::clock::{DefaultSystemClock, SystemClock};
    use std::collections::BTreeSet;
    use std::collections::VecDeque;
    use std::ops::Bound::{Excluded, Included, Unbounded};
    use std::ops::RangeBounds;
    use std::sync::Arc;

    #[test]
    fn test_should_merge_db_state_with_new_checkpoints() {
        // given:
        let mut db_state = DbState::new(new_dirty_manifest());
        // mimic an externally added checkpoint
        let mut updated_state = new_dirty_manifest();
        updated_state.value.core = db_state.state.core().clone();
        let checkpoint = Checkpoint {
            id: uuid::Uuid::new_v4(),
            manifest_id: 1,
            expire_time: None,
            create_time: DefaultSystemClock::default().now(),
            name: None,
        };
        updated_state
            .value
            .core
            .checkpoints
            .push(checkpoint.clone());

        // when:
        db_state.merge_remote_manifest(updated_state);

        // then:
        assert_eq!(vec![checkpoint], db_state.state.core().checkpoints);
    }

    #[test]
    fn test_merge_remote_manifest_reestablishes_external_sst_invariant() {
        let mut db_state = DbState::new(new_dirty_manifest());
        let stale_id = SsTableId::from(ulid::Ulid::new());
        let mut remote = new_dirty_manifest();
        remote.value.external_dbs = vec![crate::manifest::ExternalDb {
            path: "/parent/db".to_string(),
            source_checkpoint_id: uuid::Uuid::new_v4(),
            final_checkpoint_id: Some(uuid::Uuid::new_v4()),
            sst_ids: vec![stale_id],
        }];

        db_state.merge_remote_manifest(remote);

        let external = &db_state.state.manifest.value.external_dbs;
        assert_eq!(external.len(), 1, "detach metadata must be retained");
        assert!(
            external[0].sst_ids.is_empty(),
            "IDs absent from the merged tree must not be resurrected"
        );
        assert!(external[0].final_checkpoint_id.is_some());
    }

    #[test]
    fn test_should_merge_db_state_with_l0s_up_to_last_compacted() {
        // given:
        let mut db_state = DbState::new(new_dirty_manifest());
        add_l0s_to_dbstate(&mut db_state, 4);
        // mimic the compactor popping off l0s
        let mut compactor_state = new_dirty_manifest();
        compactor_state.value.core = db_state.state.core().clone();
        let last_compacted = Arc::make_mut(&mut compactor_state.value.core.tree)
            .l0
            .pop_back()
            .unwrap();
        Arc::make_mut(&mut compactor_state.value.core.tree).last_compacted_l0_sst_view_id =
            Some(last_compacted.id);

        // when:
        db_state.merge_remote_manifest(compactor_state.clone());

        // then:
        let expected: Vec<SsTableId> = compactor_state
            .value
            .core
            .tree
            .l0
            .iter()
            .map(|l0| l0.sst.id)
            .collect();
        let merged: Vec<SsTableId> = db_state
            .state
            .core()
            .tree
            .l0
            .iter()
            .map(|l0| l0.sst.id)
            .collect();
        assert_eq!(expected, merged);
    }

    #[test]
    fn test_should_merge_db_state_with_all_l0s_if_none_compacted() {
        // given:
        let mut db_state = DbState::new(new_dirty_manifest());
        add_l0s_to_dbstate(&mut db_state, 4);
        let l0s = db_state.state.core().tree.l0.clone();

        // when:
        db_state.merge_remote_manifest(new_dirty_manifest());

        // then:
        let expected: Vec<SsTableId> = l0s.iter().map(|l0| l0.sst.id).collect();
        let merged: Vec<SsTableId> = db_state
            .state
            .core()
            .tree
            .l0
            .iter()
            .map(|l0| l0.sst.id)
            .collect();
        assert_eq!(expected, merged);
    }

    #[test]
    fn test_should_keep_local_segments_on_merge() {
        fn view(seq: u64) -> SsTableView {
            let ulid = ulid::Ulid::from_parts(seq, 0);
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            ))
        }
        let v1 = view(1);

        // Local writer has a segment extractor configured and a populated segment.
        let mut db_state = DbState::new(new_dirty_manifest());
        db_state.modify(|modifier| {
            let core = &mut modifier.state.manifest.value.core;
            core.segment_extractor_name = Some("hour-bucket".to_string());
            core.segments = vec![Segment {
                prefix: Bytes::from_static(b"hour=12/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![v1.clone()]),
                    compacted: vec![],
                }),
            }];
        });

        // Remote (compactor-authored) state echoes the local core but blanks
        // out segment configuration — simulating a regression where the
        // compactor failed to carry segments forward.
        let mut remote_state = new_dirty_manifest();
        remote_state.value.core = db_state.state.core().clone();
        remote_state.value.core.segments = vec![];
        remote_state.value.core.segment_extractor_name = None;

        db_state.merge_remote_manifest(remote_state);

        // The writer is the source of truth for segment config and for L0,
        // so the local segment (with its L0) must survive the merge.
        let merged = db_state.state.core();
        assert_eq!(
            merged.segment_extractor_name.as_deref(),
            Some("hour-bucket")
        );
        assert_eq!(merged.segments.len(), 1);
        assert_eq!(merged.segments[0].prefix, Bytes::from_static(b"hour=12/"));
        let l0_ids: Vec<_> = merged.segments[0].tree.l0.iter().map(|v| v.id).collect();
        assert_eq!(l0_ids, vec![v1.id]);
    }

    #[test]
    fn test_should_re_create_segment_on_late_backfill_after_drop() {
        // After the compactor has drained a segment, a writer L0 with an ID
        // above the watermark must keep the segment alive (re-creation).
        fn view(seq: u64) -> SsTableView {
            let ulid = ulid::Ulid::from_parts(seq, 0);
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            ))
        }
        let v1 = view(1);
        let v2 = view(2); // backfill, written after compactor's snapshot.

        // Local writer: v1 (already absorbed by compactor) and v2 (backfill).
        let mut db_state = DbState::new(new_dirty_manifest());
        db_state.modify(|m| {
            let core = &mut m.state.manifest.value.core;
            core.segment_extractor_name = Some("hour".into());
            core.segments = vec![Segment {
                prefix: Bytes::from_static(b"hour=12/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![v2.clone(), v1.clone()]),
                    compacted: vec![],
                }),
            }];
        });

        // Remote compactor: watermark at v1, drained.
        let mut remote_state = new_dirty_manifest();
        remote_state.value.core = db_state.state.core().clone();
        remote_state.value.core.segments = vec![Segment {
            prefix: Bytes::from_static(b"hour=12/"),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: Some(v1.id),
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: vec![],
            }),
        }];

        db_state.merge_remote_manifest(remote_state);

        let merged = db_state.state.core();
        assert_eq!(merged.segments.len(), 1);
        let l0_ids: Vec<_> = merged.segments[0].tree.l0.iter().map(|v| v.id).collect();
        assert_eq!(l0_ids, vec![v2.id]);
    }

    #[test]
    fn test_should_keep_local_sequence_tracker_on_merge() {
        let mut db_state = DbState::new(new_dirty_manifest());
        db_state.modify(|modifier| {
            let core = &mut modifier.state.manifest.value.core;
            core.last_l0_seq = 3;
            core.sequence_tracker.insert(TrackedSeq {
                seq: 1,
                ts: Utc.timestamp_opt(60, 0).single().unwrap(),
            });
            core.sequence_tracker.insert(TrackedSeq {
                seq: 2,
                ts: Utc.timestamp_opt(120, 0).single().unwrap(),
            });
            core.sequence_tracker.insert(TrackedSeq {
                seq: 3,
                ts: Utc.timestamp_opt(180, 0).single().unwrap(),
            });
        });

        // Remote has a stale sequence tracker (e.g. missing recent entries).
        let mut remote_state = new_dirty_manifest();
        remote_state.value.core = db_state.state.core().clone();
        remote_state.value.core.sequence_tracker = SequenceTracker::new();

        db_state.merge_remote_manifest(remote_state);

        // The local tracker should be preserved as-is.
        let tracker = &db_state.state.core().sequence_tracker;
        assert_eq!(
            tracker.find_ts(1, FindOption::RoundDown),
            Utc.timestamp_opt(60, 0).single()
        );
        assert_eq!(
            tracker.find_ts(2, FindOption::RoundDown),
            Utc.timestamp_opt(120, 0).single()
        );
        assert_eq!(
            tracker.find_ts(3, FindOption::RoundDown),
            Utc.timestamp_opt(180, 0).single()
        );
    }

    fn add_l0s_to_dbstate(db_state: &mut DbState, n: u32) {
        let dummy_info = create_sst_info(None);
        for i in 0..n {
            db_state.freeze_memtable(i as u64);
            let imm = db_state.state.imm_memtable.back().unwrap().clone();
            let handle = SsTableHandle::new(
                SsTableId::from(ulid::Ulid::from_parts(i as u64, 0)),
                SST_FORMAT_VERSION_LATEST,
                dummy_info.clone(),
            );
            let view: SsTableView = SsTableView::identity(handle);
            db_state.modify(|modifier| {
                Arc::make_mut(&mut modifier.state.manifest.value.core.tree)
                    .l0
                    .push_front(view);
                modifier.state.manifest.value.core.replay_after_wal_id =
                    imm.recent_flushed_wal_id();
            });
        }
    }

    #[test]
    fn test_sorted_run_collect_tables_in_range() {
        let max_bytes_len = 5;
        proptest!(|(
            table_first_keys in vec(arbitrary::nonempty_bytes(max_bytes_len), 1..10),
            range in arbitrary::nonempty_range(max_bytes_len),
        )| {
            let sorted_first_keys: BTreeSet<Bytes> = table_first_keys.into_iter().collect();
            let sorted_run = create_sorted_run(0, &sorted_first_keys);
            let covering_tables = sorted_run.tables_covering_range(range.clone());
            let borrowed_ids: Vec<_> = covering_tables.iter().map(|view| view.id).collect();
            let owned_ids: Vec<_> = sorted_run
                .clone()
                .into_tables_covering_range(&range)
                .iter()
                .map(|view| view.id)
                .collect();
            assert_eq!(owned_ids, borrowed_ids);
            let first_key = sorted_first_keys.first().unwrap().clone();

            let range_start_key = test_utils::bound_as_option(range.start_bound())
            .cloned()
            .unwrap_or_default();
            let range_end_key = test_utils::bound_as_option(range.end_bound())
            .cloned()
            .unwrap_or(vec![u8::MAX; max_bytes_len + 1].into());

            if covering_tables.is_empty() {
                assert!(range_end_key <= first_key);
            } else {
                let covering_first_key = covering_tables.front()
                .map(|t| t.compacted_effective_start_key().clone())
                .unwrap();

                if range_start_key < covering_first_key {
                    assert_eq!(covering_first_key, first_key)
                }

                let covering_last_key = covering_tables.iter().last()
                .map(|t| t.compacted_effective_start_key().clone())
                .unwrap();
                if covering_last_key == range_end_key {
                    assert_eq!(Included(range_end_key), range.end_bound().cloned());
                } else {
                    assert!(covering_last_key < range_end_key);
                }
            }
        });
    }

    #[test]
    fn test_sorted_run_collect_tables_for_point_key() {
        let sorted_run = SortedRun::new(
            0,
            [
                create_compacted_sst_view_with_bounds(b"a", Some(b"k")),
                create_compacted_sst_view_with_bounds(b"k", Some(b"k")),
                create_compacted_sst_view_with_bounds(b"k", Some(b"m")),
                create_compacted_sst_view_with_bounds(b"z", Some(b"z")),
            ],
        );

        let covering_tables = sorted_run.tables_covering_point_key(b"k");
        assert_eq!(covering_tables.len(), 3);
        assert_eq!(
            covering_tables[0].compacted_effective_start_key().as_ref(),
            b"a"
        );
        assert_eq!(
            covering_tables[1].compacted_effective_start_key().as_ref(),
            b"k"
        );
        assert_eq!(
            covering_tables[2].compacted_effective_start_key().as_ref(),
            b"k"
        );

        assert!(sorted_run.tables_covering_point_key(b"0").is_empty());
    }

    #[test]
    fn test_sorted_run_clone_shares_sst_views() {
        let sorted_run = SortedRun::new(0, [create_compacted_sst_view(Some(Bytes::from("a")))]);
        let cloned = sorted_run.clone();

        assert!(Arc::ptr_eq(&sorted_run.sst_views, &cloned.sst_views));
        assert_eq!(sorted_run.sst_views(), cloned.sst_views());
    }

    fn create_sorted_run(id: u32, first_keys: &BTreeSet<Bytes>) -> SortedRun {
        let mut ssts = Vec::new();
        for first_key in first_keys {
            ssts.push(create_compacted_sst_view(Some(first_key.clone())));
        }
        SortedRun::new(id, ssts)
    }

    fn create_compacted_sst_view(first_entry: Option<Bytes>) -> SsTableView {
        let sst_info = create_sst_info(first_entry);
        let sst_id = SsTableId::from(ulid::Ulid::from_parts(0, 0));
        let handle = SsTableHandle::new(sst_id, SST_FORMAT_VERSION_LATEST, sst_info);
        SsTableView::identity(handle)
    }

    fn create_compacted_sst_view_with_bounds(
        first_entry: &[u8],
        last_entry: Option<&[u8]>,
    ) -> SsTableView {
        let sst_info = SsTableInfo {
            first_entry: Some(Bytes::copy_from_slice(first_entry)),
            last_entry: last_entry.map(Bytes::copy_from_slice),
            ..Default::default()
        };
        let sst_id = SsTableId::from(ulid::Ulid::new());
        let handle = SsTableHandle::new(sst_id, SST_FORMAT_VERSION_LATEST, sst_info);
        SsTableView::identity(handle)
    }

    fn create_sst_info(first_entry: Option<Bytes>) -> SsTableInfo {
        SsTableInfo {
            first_entry,
            ..Default::default()
        }
    }

    fn create_compacted_sst_view_with_size(
        first_entry: &[u8],
        last_entry: &[u8],
        raw_size: u64,
    ) -> SsTableView {
        let sst_info = SsTableInfo {
            first_entry: Some(Bytes::copy_from_slice(first_entry)),
            last_entry: Some(Bytes::copy_from_slice(last_entry)),
            index_offset: raw_size,
            ..Default::default()
        };
        let sst_id = SsTableId::new(ulid::Ulid::new());
        let handle = SsTableHandle::new(sst_id, SST_FORMAT_VERSION_LATEST, sst_info);
        SsTableView::identity(handle)
    }

    #[test]
    fn estimate_size_unprojected_view_returns_raw_physical_size() {
        let view = create_compacted_sst_view_with_size(b"a", b"z", 1_000_000);
        assert_eq!(view.estimate_size(), 1_000_000);
    }

    #[test]
    fn estimate_visible_size_scales_projected_views_by_visible_fraction() {
        let raw_size = 1_000_000u64;
        let base = create_compacted_sst_view_with_size(b"a", b"z", raw_size);

        let a = Bytes::copy_from_slice(b"a");
        let m = Bytes::copy_from_slice(b"m");
        let z = Bytes::copy_from_slice(b"z");

        let first_half = base.with_visible_range(BytesRange::new(Included(a), Excluded(m.clone())));
        let second_half = base.with_visible_range(BytesRange::new(Included(m), Included(z)));

        let first_size = first_half.estimate_visible_size();
        let second_size = second_half.estimate_visible_size();

        assert!(
            first_size < raw_size && second_size < raw_size,
            "projected views must not report the full physical size: \
             first={first_size}, second={second_size}, raw={raw_size}"
        );

        let summed = first_size + second_size;
        let lower = (raw_size as f64 * 0.8) as u64;
        let upper = (raw_size as f64 * 1.2) as u64;
        assert!(
            (lower..=upper).contains(&summed),
            "sibling views over disjoint halves of one physical SST should \
             sum to ~raw_size, got {summed} (raw={raw_size})"
        );
    }

    #[test]
    fn estimate_visible_size_single_key_visible_range_falls_back_to_raw_size() {
        let raw_size = 1_000_000u64;
        let prefix = b"tenant00";
        let low = [prefix.as_slice(), &[0]].concat();
        let high = [prefix.as_slice(), &[200]].concat();
        let base = create_compacted_sst_view_with_size(&low, &high, raw_size);

        let point = base.with_visible_range(BytesRange::new(
            Included(Bytes::copy_from_slice(&low)),
            Included(Bytes::copy_from_slice(&low)),
        ));
        assert_eq!(point.estimate_visible_size(), raw_size);
    }

    #[test]
    fn estimate_visible_size_single_key_sst_fully_visible_returns_raw_size() {
        let raw_size = 1_000_000u64;
        let key = b"tenant0000000";
        let base = create_compacted_sst_view_with_size(key, key, raw_size);

        let projected = base.with_visible_range(BytesRange::new(
            Included(Bytes::copy_from_slice(key)),
            Included(Bytes::copy_from_slice(key)),
        ));
        assert_eq!(projected.estimate_visible_size(), raw_size);
    }

    #[test]
    fn estimate_visible_size_shared_prefix_does_not_collapse() {
        let raw_size = 1_000_000u64;
        let prefix = b"tenant00";
        let phys_low = [prefix.as_slice(), &[0]].concat();
        let phys_high = [prefix.as_slice(), &[200]].concat();
        let base = create_compacted_sst_view_with_size(&phys_low, &phys_high, raw_size);

        let vis_low = [prefix.as_slice(), &[0]].concat();
        let vis_high = [prefix.as_slice(), &[2]].concat();
        let slice = base.with_visible_range(BytesRange::new(
            Included(Bytes::copy_from_slice(&vis_low)),
            Excluded(Bytes::copy_from_slice(&vis_high)),
        ));

        let size = slice.estimate_visible_size();
        assert_ne!(
            size, 1,
            "shared long prefix must not collapse the estimate to the 1-byte floor"
        );
        assert!(
            (1_000..100_000).contains(&size),
            "expected roughly a 1% slice of raw_size, got {size} (raw={raw_size})"
        );
    }

    #[test]
    fn estimate_visible_size_ambiguous_deep_divergence_falls_back_to_raw_size() {
        let raw_size = 1_000_000u64;
        let phys_low = vec![0x10u8];
        let phys_high = vec![0xF0u8];
        let base = create_compacted_sst_view_with_size(&phys_low, &phys_high, raw_size);

        let vis_low = vec![0x50u8, 0, 0, 0, 0, 0, 0, 0, 0x00];
        let vis_high = vec![0x50u8, 0, 0, 0, 0, 0, 0, 0, 0xFF];
        let view = base.with_visible_range(BytesRange::new(
            Included(Bytes::copy_from_slice(&vis_low)),
            Included(Bytes::copy_from_slice(&vis_high)),
        ));

        assert_eq!(view.estimate_visible_size(), raw_size);
    }

    #[test]
    fn estimate_visible_size_missing_last_entry_falls_back_to_raw_size() {
        let raw_size = 1_000_000u64;
        let sst_info = SsTableInfo {
            first_entry: Some(Bytes::copy_from_slice(&[0x00])),
            last_entry: None,
            index_offset: raw_size,
            ..Default::default()
        };
        let handle = SsTableHandle::new(
            SsTableId::new(ulid::Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            sst_info,
        );
        let base = SsTableView::identity(handle);

        let view = base.with_visible_range(BytesRange::new(
            Included(Bytes::copy_from_slice(&[0x00])),
            Included(Bytes::copy_from_slice(&[0x01])),
        ));

        assert_eq!(view.estimate_visible_size(), raw_size);
    }

    #[test]
    fn max_l0_overlap_empty_is_zero() {
        let l0: VecDeque<SsTableView> = VecDeque::new();
        assert_eq!(super::max_l0_overlap(&l0), 0);
    }

    #[test]
    fn max_l0_overlap_disjoint_ranges_is_one() {
        // Simulates a post-union manifest where each source's L0s cover
        // disjoint key ranges — the peak per-point count stays at 1.
        let mut l0 = VecDeque::new();
        l0.push_back(create_compacted_sst_view_with_bounds(b"a", Some(b"b")));
        l0.push_back(create_compacted_sst_view_with_bounds(b"c", Some(b"d")));
        l0.push_back(create_compacted_sst_view_with_bounds(b"e", Some(b"f")));
        l0.push_back(create_compacted_sst_view_with_bounds(b"g", Some(b"h")));
        assert_eq!(super::max_l0_overlap(&l0), 1);
    }

    #[test]
    fn max_l0_overlap_full_overlap_counts_all() {
        let mut l0 = VecDeque::new();
        for _ in 0..4 {
            l0.push_back(create_compacted_sst_view_with_bounds(b"a", Some(b"z")));
        }
        assert_eq!(super::max_l0_overlap(&l0), 4);
    }

    #[test]
    fn max_l0_overlap_partial_overlap() {
        // A: [a, c], B: [b, d]. At B.start=b, both A and B contain b.
        let mut l0 = VecDeque::new();
        l0.push_back(create_compacted_sst_view_with_bounds(b"a", Some(b"c")));
        l0.push_back(create_compacted_sst_view_with_bounds(b"b", Some(b"d")));
        assert_eq!(super::max_l0_overlap(&l0), 2);
    }

    #[test]
    fn max_l0_overlap_mixed_disjoint_groups() {
        // Two disjoint groups of 3 overlapping SSTs each. Peak is 3, not 6.
        let mut l0 = VecDeque::new();
        for _ in 0..3 {
            l0.push_back(create_compacted_sst_view_with_bounds(b"a", Some(b"c")));
        }
        for _ in 0..3 {
            l0.push_back(create_compacted_sst_view_with_bounds(b"m", Some(b"p")));
        }
        assert_eq!(super::max_l0_overlap(&l0), 3);
    }

    #[test]
    fn max_l0_overlap_single_point_range_is_one() {
        // A view whose first_entry == last_entry covers exactly one key.
        let mut l0 = VecDeque::new();
        l0.push_back(create_compacted_sst_view_with_bounds(b"k", Some(b"k")));
        assert_eq!(super::max_l0_overlap(&l0), 1);
    }

    #[test]
    fn max_l0_overlap_many_point_ranges_same_key() {
        // N coincident point ranges [k, k] all cover key k → peak N.
        let mut l0 = VecDeque::new();
        for _ in 0..5 {
            l0.push_back(create_compacted_sst_view_with_bounds(b"k", Some(b"k")));
        }
        assert_eq!(super::max_l0_overlap(&l0), 5);
    }

    #[test]
    fn max_l0_overlap_mixed_point_and_longer_ranges_at_same_key() {
        // Two point ranges [k, k] and two longer ranges [k, z] all cover k.
        // Peak at k is 4; past k, only the two longer ranges remain.
        let mut l0 = VecDeque::new();
        l0.push_back(create_compacted_sst_view_with_bounds(b"k", Some(b"k")));
        l0.push_back(create_compacted_sst_view_with_bounds(b"k", Some(b"k")));
        l0.push_back(create_compacted_sst_view_with_bounds(b"k", Some(b"z")));
        l0.push_back(create_compacted_sst_view_with_bounds(b"k", Some(b"z")));
        assert_eq!(super::max_l0_overlap(&l0), 4);
    }

    #[test]
    fn max_l0_overlap_edge_touching_inclusive_counts_both() {
        // [a, b] and [b, c]: both contain b → peak 2.
        let mut l0 = VecDeque::new();
        l0.push_back(create_compacted_sst_view_with_bounds(b"a", Some(b"b")));
        l0.push_back(create_compacted_sst_view_with_bounds(b"b", Some(b"c")));
        assert_eq!(super::max_l0_overlap(&l0), 2);
    }

    #[test]
    fn max_l0_overlap_edge_touching_exclusive_end_is_disjoint() {
        // [a, b) and [b, c]: no point is in both → peak 1.
        // First view has an Excluded end at b via a visible_range projection.
        let a = Bytes::copy_from_slice(b"a");
        let b = Bytes::copy_from_slice(b"b");
        let v1 = create_compacted_sst_view_with_bounds(b"a", Some(b"b"))
            .with_visible_range(BytesRange::new(Included(a), Excluded(b.clone())));
        let v2 = create_compacted_sst_view_with_bounds(b"b", Some(b"c"));
        let mut l0 = VecDeque::new();
        l0.push_back(v1);
        l0.push_back(v2);
        assert_eq!(super::max_l0_overlap(&l0), 1);
    }

    #[test]
    fn max_l0_overlap_unbounded_end_single_view() {
        // A view with first_entry but no last_entry has effective_range
        // [first, Unbounded) — still one view, peak 1.
        let mut l0 = VecDeque::new();
        l0.push_back(create_compacted_sst_view_with_bounds(b"a", None));
        assert_eq!(super::max_l0_overlap(&l0), 1);
    }

    #[test]
    fn max_l0_overlap_unbounded_ends_share_tail() {
        // [a, ∞) and [b, ∞) both extend to +∞, so they overlap at every
        // point ≥ b. Peak is 2.
        let mut l0 = VecDeque::new();
        l0.push_back(create_compacted_sst_view_with_bounds(b"a", None));
        l0.push_back(create_compacted_sst_view_with_bounds(b"b", None));
        assert_eq!(super::max_l0_overlap(&l0), 2);
    }

    #[test]
    fn max_l0_overlap_mixed_bounded_and_unbounded_end() {
        // [a, m] ends at m; [b, ∞) starts before m and extends past it.
        // They coexist on [b, m]. Peak is 2.
        let mut l0 = VecDeque::new();
        l0.push_back(create_compacted_sst_view_with_bounds(b"a", Some(b"m")));
        l0.push_back(create_compacted_sst_view_with_bounds(b"b", None));
        assert_eq!(super::max_l0_overlap(&l0), 2);
    }

    #[test]
    fn max_l0_overlap_unbounded_end_via_visible_range() {
        // visible_range = [m, Unbounded) applied to a physical [a, z] view.
        // Effective range becomes [m, z] (physical end clamps the Unbounded).
        // Pair with [n, ∞): overlap on [n, z]. Peak is 2.
        let m = Bytes::copy_from_slice(b"m");
        let projected = create_compacted_sst_view_with_bounds(b"a", Some(b"z"))
            .with_visible_range(BytesRange::new(Included(m), Unbounded));
        let open = create_compacted_sst_view_with_bounds(b"n", None);
        let mut l0 = VecDeque::new();
        l0.push_back(projected);
        l0.push_back(open);
        assert_eq!(super::max_l0_overlap(&l0), 2);
    }

    #[test]
    fn max_l0_overlap_respects_visible_range() {
        // Two views whose physical ranges overlap but whose visible_range
        // projections are disjoint should not count as overlapping.
        let lo = Bytes::copy_from_slice(b"a");
        let mid = Bytes::copy_from_slice(b"m");
        let hi = Bytes::copy_from_slice(b"z");
        let v1 = create_compacted_sst_view_with_bounds(b"a", Some(b"z"))
            .with_visible_range(BytesRange::new(Included(lo.clone()), Excluded(mid.clone())));
        let v2 = create_compacted_sst_view_with_bounds(b"a", Some(b"z"))
            .with_visible_range(BytesRange::new(Included(mid), Included(hi)));
        let mut l0 = VecDeque::new();
        l0.push_back(v1);
        l0.push_back(v2);
        assert_eq!(super::max_l0_overlap(&l0), 1);
    }

    #[test]
    fn max_l0_overlap_proptest_matches_naive() {
        use proptest::prelude::{prop_oneof, Just, ProptestConfig, Strategy};

        // Generate views with Included starts and end bounds drawn from
        // {Included, Excluded, Unbounded}. The keyspace is intentionally
        // small (alphabet 0..3, length 1..=3) so generated views frequently
        // overlap and the brute-force reference exercises real cases.
        #[derive(Debug, Clone)]
        enum EndKind {
            Inclusive(Bytes),
            Exclusive(Bytes),
            Unbounded,
        }
        #[derive(Debug, Clone)]
        struct ViewSpec {
            start: Bytes,
            end: EndKind,
        }

        let key = vec(0u8..3u8, 1..=3).prop_map(Bytes::from);
        let end = prop_oneof![
            Just(EndKind::Unbounded),
            key.clone().prop_map(EndKind::Inclusive),
            key.clone().prop_map(EndKind::Exclusive),
        ];
        let spec =
            (key, end).prop_filter_map("non-empty effective range", |(start, end)| match &end {
                EndKind::Inclusive(k) if k < &start => None,
                EndKind::Exclusive(k) if k <= &start => None,
                _ => Some(ViewSpec { start, end }),
            });

        proptest!(ProptestConfig::with_cases(256), |(specs in vec(spec, 0..=8))| {
            let mut l0 = VecDeque::new();
            for s in &specs {
                let view = match &s.end {
                    EndKind::Inclusive(end) => {
                        create_compacted_sst_view_with_bounds(&s.start, Some(end))
                    }
                    EndKind::Unbounded => {
                        create_compacted_sst_view_with_bounds(&s.start, None)
                    }
                    EndKind::Exclusive(end) => create_compacted_sst_view_with_bounds(
                        &s.start, None,
                    )
                    .with_visible_range(BytesRange::new(
                        Included(s.start.clone()),
                        Excluded(end.clone()),
                    )),
                };
                l0.push_back(view);
            }

            // Naive reference: every view's start is Included, so the peak of
            // the sweep is always achieved at some view's start key. Counting
            // covers at each start key and taking the max is sufficient.
            let naive = specs
                .iter()
                .map(|s| {
                    l0.iter()
                        .filter(|v| v.compacted_effective_range().contains(&s.start))
                        .count()
                })
                .max()
                .unwrap_or(0);

            assert_eq!(super::max_l0_overlap(&l0), naive);
        });
    }
}
