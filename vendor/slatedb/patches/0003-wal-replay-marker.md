# 0003: WAL replay marker advanced at checkpoint

**What it changes.** When a checkpoint is written, raises `replay_after_wal_id` (and `next_wal_sst_id`) to the last flushed WAL if this writer has flushed a data WAL and every record through its last flushed sequence is in L0. At L0 publication, a later memtable's older watermark never moves the marker back.

**Why.** A projected clone refuses a source checkpoint with data WALs above the marker (`validate_no_data_wal`), and freezing a memtable records a conservative WAL watermark before that WAL's flush finishes. Tests: `checkpoint_retires_only_wal_whose_records_are_in_its_ssts`, `published_wal_watermark_never_regresses_across_frozen_memtable_batches` and `checkpoint_after_reopen_keeps_replayed_wal_data_that_is_not_in_l0` in `engine/packages/env/tests/slatedb_checkpoint.rs`.

**Invariant.** The marker covers only WALs whose records are provably in L0. Before this writer flushes a data WAL, `last_flushed_seq` is None while WALs replayed at open may still be only in memtables, so nothing is advanced; with the WAL disabled, as Depot tablets run, the advance never applies. An Oct 22 review found that treating None as covered dropped replayed, acknowledged writes from checkpoints, the next reopen and WAL GC; that is fixed here.

**Upstream.** Not fixed upstream. PR #2164 (open) advances the marker past empty WALs at open, and PR #2142 moves `next_wal_id` into the manifest writer, which will conflict at 0.18.

**Drop when.** Upstream clones from a fully flushed checkpoint without WAL replay, or Depot no longer clones sources that ran with the WAL enabled.
