# Review: `04-29-feat_sqlite_pitr_forking` + `04-30-chore_sqlite_comapctor_wf`

## Overview

Two stacked PRs reshaping the SQLite storage backend (renamed from `sqlite-storage` to `depot`):

- **`04-29-feat_sqlite_pitr_forking`** (~35k LoC, 50+ commits) — Per-database branches, fork/restore primitives, bookmarks (ephemeral + pinned, two-phase), namespace forks, S3 cold tier, GC pin recompute, burst-mode quota.
- **`04-30-chore_sqlite_comapctor_wf`** (~13k LoC) — Reimplements compaction as a Gasoline workflow. New per-branch DB manager workflow + hot/cold/reclaimer companions, `CMP/root` manifest, global pin/proof/dirty indexes (partitions `0x70..=0x75`), `lifecycle_generation` field, `CompactionSignaler` injected into `Db`, 5076-line `workflows/compaction.rs`. Per CLAUDE.md, this workflow is the active compaction authority.

---

## High-Priority Correctness Issues

### Conveyer

1. **Pinned bookmarks bypass the two-phase protocol entirely.** `bookmark.rs:207, 413` write `PinStatus::Ready` synchronously. The two-phase contract requires the request tx to write `Pending`, set `SQLITE_CMP_DIRTY` (or signal the manager workflow via the `CompactionSignaler` already injected into `Db`), and have the cold companion + manager flip `Pending → Ready` after the S3 pin layer is uploaded. As written, no S3 pin layer is ever built and `pin_object_key` is permanently `None`.
2. **Empty `bk_pin` recompute permanently locks the key at zero.** `bookmark.rs:685` writes `[0;16]` when no pins remain. Subsequent `MutationType::ByteMin` on the same key sees `min(0, new) = 0`, so the next pinned bookmark on this branch never advances `bk_pin`.
3. **Fork-vs-GC OCC fence missing.** `branch.rs:525 derive_branch_at` reads `bk_pin` only; CLAUDE.md mandates a regular-read of parent `META/manifest.retention_pin_txid` so concurrent GC aborts the fork.
4. **Truncate cleanup off-by-one for fully-above-EOF SHARDs.** `commit.rs:567` and `takeover.rs:142` use `>` instead of `>=`. A whole-shard truncate (e.g. `new_db_size_pages = 64`) leaves shard 1 alive.
5. **PIDX/SHARD deletes during truncate use `clear`, not `COMPARE_AND_CLEAR`.** `commit.rs:272-274`. CLAUDE.md is explicit; this clobbers a concurrent compactor write.
6. **`load_branch_ancestry` off-by-one against `MAX_FORK_DEPTH`.** Loop is `0..=MAX_FORK_DEPTH` (17 iterations); `derive_branch_at:534` caps with `>=`. End-to-end allows reading 17 levels.
7. **Branch reap leaks parent refcount.** `gc/mod.rs:155-197 sweep_unreferenced_branch_tx` clears the branch keys but never decrements `branches_refcount_key` on the parent set in `derive_branch_at:591-595`.
8. **Cold tier disabled when commit signaler is wired.** `db.rs:156-173 new_with_compaction_signaler` passes `cold_tier: None`; the only caller is the new envoy path (`pegboard-envoy/src/ws_to_tunnel_task.rs:730-756`). `read.rs:495` short-circuits when cold_tier is None — fork descendants and historical reads break under the new path.

### Compactor workflow

9. **S3 leak when cold publish rejects after upload.** `workflows/compaction.rs:868-895` clears `active_cold_job` without GC'ing the already-uploaded objects. `schedule_stale_cold_output_cleanup` (line 909) only fires for *different* active jobs, not for failed publish of the same job.
10. **Repair reclaim activities skip the lifecycle generation fence.** `compaction.rs:1652, 1666` set `base_lifecycle_generation: 0`; `cleanup_repair_fdb_outputs_tx` (line 2462) and `DeleteOrphanColdObjects` (line 3022) never call `branch_record_is_live_at_generation`. A repair queued before destroy can run during/after destroy and delete S3 objects.
11. **Force-compaction reclaim flag silently dropped.** `plan_reclaim_job` takes `_force: bool` and never reads it (`compaction.rs:3868-3873`). Forced reclaim with no actionable lag falls through to `force_noop_reasons`. Hot/cold honor force; reclaim does not.
12. **Staged-hot install no-op when only DB-pin coverage was staged.** `compaction.rs:1957-1968` requires every PIDX row to be covered by `latest_staged_shards`, but `selected_hot_coverage_txids` only covers DB pins above `hot_watermark_txid`. Pin at `head_txid` only with no intermediate pins → install rejects with "missing staged hot shard".
13. **`mix_fingerprint` is a hand-rolled non-cryptographic combiner** (`compaction.rs:4058-4066`) used as the OCC fence on `CompactionInputFingerprint`. `sha2::Digest` is already imported (line 11). Collision-resistant hashing matters here because the fingerprint guards "active job identity" through the publish path.
14. **Reclaim commit-prefix scan uses `continue` instead of `break`** (`compaction.rs:3559-3572`). Txids are ascending big-endian; the rest of the scan is wasted on every reclaim batch.

---

## Convention Violations

- **`Db` cache fields use `parking_lot::Mutex`** (`db.rs:107-123, 193-203`). All called from async contexts — no forced-sync requirement. Should be `tokio::sync::Mutex`. The `Mutex<scc::HashMap>` wrapping is doubly wrong — drop the outer mutex.
- **Tracing logs in `workflows/compaction.rs` lack the `actor_id` field** the engine convention requires (e.g. `2503-2511, 3064-3074, 3084-3093`); only `database_branch_id` is included.
- **`anyhow::anyhow!` macro** sprinkled across `commit.rs:677, 724, 738`, `quota.rs:60, 82`, `burst_mode.rs:80`, `takeover.rs:174-232`. Convention is `.context()` / `Error::msg`.
- **`S3ColdTier::get_object` matches NoSuchKey by `err.to_string().contains("NoSuchKey")`** (`cold_tier/mod.rs:311`). Use the typed `is_no_such_key()` SDK API.
- **`FaultyColdTier` records metrics under hardcoded `"unknown"` node_id** (`cold_tier/mod.rs:13, 480`).
- **Test hook `lazy_static!` + `parking_lot::Mutex<Option<…>>`** in `workflows/compaction.rs:51-54` is gated by `cfg(debug_assertions)` rather than `cfg(test)` — shipped in dev builds.
- **Inline `#[cfg(test)] mod tests` in `conveyer/types.rs:1123`** (not feature-gated). Move to `tests/`.

---

## Test Coverage

**Strengths:** Real UDB via `test_db()`, `FilesystemColdTier` for cold, fault injection via `FaultyColdTier` + custom wrapper tiers. Force-compaction tests correctly wait on durable `force_compaction_results` (`workflow_compaction_skeletons.rs:1502, 1562, 1638, 1712, 1826`). OCC race tests use clean atomic-bool-guarded retry hooks (`fork_database.rs:99`, `fork_namespace.rs:145`).

**Gaps:**
- **No test for the workflow `Pending → Ready` flip on a pinned bookmark.** Given finding #1, this contract isn't asserted anywhere — neither the success path nor the `Pending → Failed` path on cold upload failure.
- **No test for crash between cold pin upload and the manager's bookmark-flip publish.**
- **No test for manager workflow generation bump mid-active-job** — only individual companion-side rejections (`workflow_compaction_skeletons.rs:1960, 2025, 2778, 3007`).
- **No test for hot OCC abort when a concurrent commit lands between hot-stage read and manager install.**

**Quality concerns:**
- **`conveyer_commit.rs:760`** does a 1ms wall-clock absence check. Replace with explicit yield/notify.
- **17 polling helpers in `workflow_compaction_skeletons.rs:125-605`** each implement their own `loop { sleep 25ms }` waiting for durable workflow state. Not retry-til-success masks (each polls a real state condition), but real-clock and duplicative — a single deterministic helper would shave ~250 LoC and CI time.
- **`fork_common/mod.rs:157-165 assert_storage_error`** uses `err.downcast_ref` (top-level only), while `bookmarks.rs:114` and `gc_pin_recompute_under_bookmark_delete_race.rs:16` use `err.chain().any(...)`. Inconsistent — context-wrapped errors silently pass.
- **Helper sprawl**: `fault_common`, `fork_common`, `bookmarks.rs:30`, `takeover.rs:13` all redefine `test_db`, `make_db`, `read_value` instead of sharing a single helper module.

---

## Notable Strengths

- `BookmarkStr` newtype validates the 33-char wire format at construction *and* deserialization (`types.rs:69-118`). `MutationType::ByteMin` correctly applied for branch pin atomic-min everywhere it should be.
- `read.rs` correctly caps ancestor PIDX reads by per-source `versionstamp_cap`, falls through to latest SHARD ≤ cap, and disables the PIDX cache when the read plan has multiple branches (`read.rs:144-147, 174, 285-294`).
- Cold object retire → grace → `DeleteIssued` → S3 delete → cleanup-Deleted sequence in the workflow (`compaction.rs:4715-4759`) faithfully implements the "completed retired record so the key cannot be republished" invariant.
- `resolve_namespace_fork_pins` proof walk (`compaction.rs:3214-3394`) materializes `DB_PIN(NamespaceFork)` records before reclaim and treats missing/ambiguous proof as a retention blocker, matching the constraint.
