use bytes::Bytes;

use crate::db::DbInner;
use crate::error::SlateDBError;
use crate::oracle::Oracle;
use crate::prefix_extractor::{PrefixExtractor, PrefixTarget};
use crate::wal_replay::ReplayedMemtable;

/// Extract the segment prefix (RFC-0024) from `key` under `extractor`.
///
/// Returns the prefix bytes — a cheap refcount slice of `key`. An
/// absent or zero-length prefix is a hard error
/// ([`SlateDBError::EmptySegmentPrefix`]): every key in a segmented DB
/// must map to a non-empty segment. This is the single point where a
/// key's segment prefix is derived; the write path, WAL replay, and
/// checkpoint filtering all route through it so they agree on both the
/// extraction and the empty-prefix error.
pub(crate) fn extract_segment_prefix(
    extractor: &dyn PrefixExtractor,
    key: &Bytes,
) -> Result<Bytes, SlateDBError> {
    match extractor.prefix_len(&PrefixTarget::Point(key.clone())) {
        Some(0) | None => Err(SlateDBError::EmptySegmentPrefix { key: key.clone() }),
        Some(n) => Ok(key.slice(0..n)),
    }
}

impl DbInner {
    /// Freezes the active memtable when its estimated encoded size reaches
    /// [`Settings::max_unflushed_bytes`](crate::config::Settings::max_unflushed_bytes).
    ///
    /// The frozen table is stamped with `replay_after_wal_id` and announced to
    /// the memtable flusher. The caller must therefore pass the highest WAL ID
    /// fully represented by the active memtable. This method does nothing when
    /// the active memtable is below the threshold.
    ///
    /// # Arguments
    ///
    /// * `replay_after_wal_id` - Durable WAL boundary for the active memtable.
    pub(crate) fn maybe_freeze_memtable(&self, replay_after_wal_id: u64) {
        let mut guard = self.state.write();
        let metadata = guard.memtable().table().metadata();
        let estimated_bytes = self
            .table_store
            .estimate_encoded_size_compacted(metadata.entry_num, metadata.entries_size_in_bytes);

        if estimated_bytes >= self.settings.max_unflushed_bytes {
            self.freeze_current_memtable_with_state_guard(&mut guard, replay_after_wal_id);
        }
    }

    pub(crate) fn replay_memtable(
        &self,
        current_memtable_wal_id: u64,
        replayed_memtable: ReplayedMemtable,
    ) -> Result<(), SlateDBError> {
        let mut guard = self.state.write();

        // The active memtable was installed by the previous replay step, so its
        // durable WAL boundary is the WAL buffer's current boundary. Stamp the
        // frozen table with that boundary before advancing the buffer for the new
        // replayed memtable.
        self.freeze_current_memtable_with_state_guard(&mut guard, current_memtable_wal_id);

        let last_wal = replayed_memtable.last_wal_id;
        guard.modify(|modifier| modifier.state.manifest.value.core.next_wal_sst_id = last_wal + 1);

        // update seqs and clock
        // we know these won't move backwards (even though the replayed wal files might contain some
        // older rows) because the wal replay iterator ignores any entries with seq num lower than
        // l0_last_seq from the manifest
        assert!(self.oracle.last_seq() <= replayed_memtable.last_seq);
        self.oracle.advance_last_seq(replayed_memtable.last_seq);
        assert!(self.oracle.last_committed_seq() <= replayed_memtable.last_seq);
        self.oracle
            .advance_committed_seq(replayed_memtable.last_seq);
        self.mono_clock.set_last_tick(replayed_memtable.last_tick)?;

        // replace the memtable
        guard.replace_memtable(replayed_memtable.table);
        let dirty_manifest = guard.state().manifest.clone();
        drop(guard);
        self.status_manager.report_manifest(dirty_manifest.into());
        Ok(())
    }
}
