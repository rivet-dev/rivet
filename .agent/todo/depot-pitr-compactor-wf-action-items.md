# Action Items: depot PITR + compactor workflow review

Tracking items from review of `04-29-feat_sqlite_pitr_forking` and `04-30-chore_sqlite_comapctor_wf`.

Status legend: `[ ]` open · `[x]` done · `[?]` needs decision

## Bugs (confirmed by user)

- [?] **Pinned bookmark `Ready` semantics are inconsistent.** `engine/packages/depot/src/conveyer/bookmark.rs:207, 413` write `PinStatus::Ready` synchronously with `pin_object_key: None`, while legacy cold compactor code still models a `Pending -> Ready` S3 pin-layer upload. Under workflow compaction, `Ready` currently means "FDB history pin installed," not "exact cold pin materialized." Decide whether to document/test that semantic or implement two-phase cold materialization.

## Dead code to delete (confirmed by user)

- [ ] **`BranchStopState::Stopping`** — `engine/packages/depot/src/workflows/compaction.rs:462-476`. Unwritten variant; manager goes Running → DestroyRequested → Stopped directly.
- [ ] **`BranchState::Deleted`** — `engine/packages/depot/src/conveyer/types.rs:152`. Unwritten variant.
- [ ] **All v1 / legacy compatibility code.** Per user direction: "there should not be any v1 or legacy. we need to delete all legacy code." Includes `LegacyDatabaseBranchRecord` and any legacy database-scoped storage fallback paths in conveyer reads / Db pointer resolution.
- [ ] **Vbare encode/decode wrappers around gas signals/state.** `engine/packages/depot/src/workflows/compaction.rs:4951-5076`. User: "if they're not encoded as bare today, dont bother with vbare." Gas signals JSON-encode via `serde::Serialize/Deserialize`; the vbare wrappers are exercised only by tests. Drop the wrappers and the `tests/workflow_compaction_payloads.rs` tests that exist only to round-trip them.
- [ ] **`udb::scan_prefix_values` stub** — `engine/packages/depot/src/conveyer/udb.rs:30-37` is `bail!("not implemented yet")` but reachable from `DeltaPageIndex::load_from_store` (`page_index.rs:31`). Either implement or delete the call site.

## Other correctness fixes (not yet confirmed; from review)

- [ ] **Empty `bk_pin` recompute locks key at zero.** `bookmark.rs:685` writes `[0;16]` when no pins remain. Tx series: pin at `vs=100` sets `bk_pin=100`; deleting the last pin recomputes `bk_pin=0`; creating a later pin at `vs=200` runs `MutationType::ByteMin`, so `min(0, 200)=0` and the key is permanently wedged. Clear `bk_pin` when no pins remain so the next `ByteMin` initializes from a missing key.
- [x] **Fork-vs-GC OCC fence missing.** `branch.rs:525 derive_branch_at` now reads both the explicit `bk_pin` gate and `depot::gc::read_branch_gc_pin_tx` before copying source commit state. Regression: `cargo test -p depot --test conveyer_branch derive_branch_at_rejects_versionstamp_below_parent_gc_floor -- --nocapture`.
- [ ] **Truncate can retain stale above-EOF pages inside the boundary SHARD.** Original `>` vs `>=` diagnosis is not right as stated: with current `pgno / SHARD_SIZE` math, `new_db_size_pages = 64` still has live page 64 in shard 1, so deleting shard 1 outright would lose data. Real bad sequence: shard 1 contains old page 65; truncate to size 64 clears PIDX for page 65 but keeps shard 1 because its start page is 64; later grow to size 65 without writing page 65; read page 65 falls back to retained shard 1 and returns stale pre-truncate bytes instead of zero. Fix needs to scrub/rewrite the boundary shard or otherwise make above-EOF pages in retained shards invisible after later growth.
- [ ] **Truncate cleanup should use exact-value clears for PIDX/SHARD rows.** `commit.rs:272-276` snapshot-scans above-EOF PIDX/SHARD rows, then blindly clears those keys. PIDX conflict risk is weak because same-database commits are serialized and workflow paths mostly compare-clear PIDX, but SHARD rows can be written by workflow hot install/repair with plain `set`. Conservative fix: track observed values during truncate cleanup and use `COMPARE_AND_CLEAR` or regular-read fences so truncate only deletes the exact stale rows it observed. Also verify quota accounting if compare-and-clear can no-op.
- [ ] **Branch reap leaks parent refcount.** `derive_branch_at` increments the parent `branches_refcount_key` when creating a child (`branch.rs:591-595`), but `sweep_unreferenced_branch_tx` only clears the child branch keys and never decrements the parent (`gc/mod.rs:155-197`). Bad sequence: fork parent P -> child C, delete C so C refcount becomes 0, sweep C deletes C, but P keeps the extra child refcount forever. Parent GC then treats P as still externally referenced and keeps `root_pin` retention alive. Fix should decrement the observed parent refcount while reaping C, and also verify the parent fork-history pin / `desc_pin` cleanup story for the deleted child.
- [x] **Cold tier config is not coherently wired or gated.** S3/cold storage should be opt-in through Rivet config. If S3 is absent, cold compaction must be disabled and FDB SHARD data remains the durable source of truth; GC/reclaim must not treat FDB as a disposable cache. Current code is env-only (`workflow_cold_tier` reads `RIVET_SQLITE_WORKFLOW_COLD_TIER_*`), workflows can plan cold jobs against `DisabledColdTier`, and `db.rs:156-173 new_with_compaction_signaler` hardcodes `cold_tier: None` on the envoy read path (`pegboard-envoy/src/ws_to_tunnel_task.rs:730-756`). Fix by adding optional typed Rivet config, wiring the same cold-tier mode into workflow activities and `Db`, making disabled mode skip cold upload/publish/delete planning, and adding fail-first tests that prove disabled mode keeps FDB SHARD reads working after reclaim while enabled mode can read configured cold refs.
- [ ] **S3 leak when cold publish rejects after upload.** Cold upload writes S3 objects first (`workflows/compaction.rs:1990-2024`), then the manager publishes refs after `ColdJobFinished` (`:861-884`). If `PublishColdJob` rejects on lifecycle, manifest, fingerprint, or output-ref mismatch (`:2189, :2213, :2233, :2240`), the manager records completion and clears `active_cold_job` without scheduling repair cleanup. `schedule_stale_cold_output_cleanup` only runs for non-matching active jobs, so the uploaded objects have no FDB live/retired record and leak.
- [ ] **Repair reclaim activities skip lifecycle generation fence.** Repair reclaim is scheduled with `base_lifecycle_generation: 0` (`compaction.rs:1641, 1655`), and the repair branch in `reclaim_fdb_job_tx` returns before the normal `branch_record_is_live_at_generation` check (`:2331-2339`). `cleanup_repair_fdb_outputs_tx` has no branch-record check, and `DeleteOrphanColdObjectsInput` carries no lifecycle generation. Bad sequence: stale cold output schedules repair reclaim, branch destroy/recreate races the queued repair, then orphan cleanup sees no live cold ref and deletes S3 under the wrong lifecycle. It skips objects with a live ref, but it still needs the branch-generation fence.
- [?] **Forced reclaim semantics are ambiguous.** `plan_reclaim_job` takes `_force: bool` and the manager passes `input.force.reclaim`, but reclaim has no lag threshold equivalent to hot/cold. A forced reclaim with no safe reclaim inputs correctly completes as noop, so this is not a correctness bug as written. Decide whether to remove the unused force parameter/noop wording, or define force-reclaim to perform an explicit bounded scan for reclaimable work while still respecting safety gates.
- [ ] **`mix_fingerprint` is a non-cryptographic OCC combiner.** `CompactionInputFingerprint` is a `[u8; 32]` active-job fence checked by stage/upload/publish/reclaim paths, but `mix_fingerprint` (`compaction.rs:4051-4059`) uses wrapping byte arithmetic over user-controlled DB bytes. Use a real collision-resistant hash such as the already-imported SHA-256, with length/type framing for each mixed field so distinct field sequences cannot collide by concatenation.
- [ ] **Reclaim commit-prefix scan is unbounded past the reclaim txid ceiling.** `read_reclaim_input_snapshot` scans the full `branch_commit_prefix` and uses `continue` when decoded txid exceeds `max_reclaim_txid` (`compaction.rs:3549-3553`). Commit keys sort by big-endian txid, so the local loop should break; better, make the scan range bounded so FDB does not materialize the whole prefix before the loop. This is a performance issue, not a correctness bug.

## Convention violations (not yet confirmed)

- [ ] **`Db` cache fields use `parking_lot::Mutex` from async contexts.** `db.rs:107-123, 193-203`. These are called from async contexts with no forced-sync requirement, so use `tokio::sync::Mutex` where a lock is still needed. The `Mutex<scc::HashMap>` wrapping is doubly wrong; drop the outer mutex and use the concurrent map directly.
- [ ] **Workflow tracing logs lack the required `actor_id` field.** `workflows/compaction.rs:2503-2511, 3064-3074, 3084-3093` and similar logs include `database_branch_id` but not the engine-convention `actor_id` field. Add stable structured `actor_id` fields without formatting values into message strings.
- [ ] **`anyhow::anyhow!` macro usage violates depot error convention.** `commit.rs:677, 724, 738`, `quota.rs:60, 82`, `burst_mode.rs:80`, `takeover.rs:174-232`. Prefer `.context()` on fallible calls or `anyhow::Error::msg` for explicit constructed errors.
- [ ] **`S3ColdTier::get_object` string-matches NoSuchKey.** `cold_tier/mod.rs:311` uses `err.to_string().contains("NoSuchKey")`. Use the typed AWS SDK missing-key classification such as `is_no_such_key()` instead of matching rendered error text.
- [ ] **`FaultyColdTier` records metrics under hardcoded `"unknown"` node_id.** `cold_tier/mod.rs:13, 480`. Thread the real node id into the fault wrapper or remove the node label from this metric path.
- [ ] **Workflow test hook ships in debug builds.** `workflows/compaction.rs:51-54` uses `lazy_static!` plus `parking_lot::Mutex<Option<...>>` gated by `cfg(debug_assertions)`. Gate test-only cold-tier injection with `cfg(test)` so it is not compiled into normal dev/debug builds.
- [ ] **Inline `#[cfg(test)] mod tests` remains in `conveyer/types.rs`.** `conveyer/types.rs:1123` is not feature-gated. Move the test body under `tests/` and keep only a tiny source-owned path shim if private module access is needed.

## Test gaps

- [ ] **Add pinned bookmark workflow status-transition tests.** No test asserts the workflow `Pending -> Ready` flip on a pinned bookmark, and no test covers `Pending -> Failed` when cold upload fails. Given the pinned-bookmark semantic ambiguity in finding #1, lock down the intended contract explicitly.
- [ ] **Add crash-recovery coverage between cold pin upload and bookmark publish.** No test covers a crash after the cold pin layer upload succeeds but before the manager flips the bookmark status. The recovery test should prove the uploaded object is either published exactly once or safely cleaned up.
- [ ] **Add manager workflow generation-bump test during an active job.** Current coverage only checks individual companion-side rejections (`workflow_compaction_skeletons.rs:1960, 2025, 2778, 3007`). Add an end-to-end manager test where lifecycle generation changes mid-active-job and the manager records/retries/rejects consistently.
- [ ] **Add hot OCC abort test for commit during hot stage/install window.** No test covers a concurrent commit landing after hot-stage input read but before manager hot install. Add a race test proving install rejects stale staged output and does not clear PIDX or publish SHARD rows for stale inputs.

## Test quality fixes

- [ ] **`compactor_dispatch.rs:270` uses 250ms real-clock sleep.** Migrate to `start_paused = true` like adjacent test at `:280`.
- [ ] **Replace the `conveyer_commit.rs:760` 1ms wall-clock absence check.** Do not poll or sleep to prove absence unless there is a very strong reason. Replace with explicit ordering via yield, notify, channel, or another deterministic synchronization point.
- [ ] **Collapse the 17 polling helpers in `workflow_compaction_skeletons.rs:125-605`.** Each helper implements its own `loop { sleep 25ms }` while waiting for durable workflow state. These are not retry-until-success masks, but they are still real-clock polling and duplicated. Prefer direct workflow/state notifications or one shared deterministic waiter with a documented reason for any remaining polling.
- [ ] **Fix `fork_common/mod.rs:157-165 assert_storage_error` to inspect the error chain.** It uses top-level `err.downcast_ref`, while `bookmarks.rs:114` and `gc_pin_recompute_under_bookmark_delete_race.rs:16` use `err.chain().any(...)`. Switch to chain matching so context-wrapped storage errors do not silently pass/fail incorrectly.
- [ ] **Consolidate duplicated depot test helpers.** `fault_common`, `fork_common`, `bookmarks.rs:30`, and `takeover.rs:13` redefine helpers such as `test_db`, `make_db`, and `read_value`. Move shared setup/read helpers into one common test module.

## Legacy compactor deletion plan (confirmed by user, gated on migrations)

`src/compactor/` is not yet fully dead — there are real runtime callers. Delete in order:

- [ ] **Migrate `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs:340-341`** off `depot::compactor::compact_default_batch` and onto a `CompactionSignaler` wake. The VFS shouldn't synchronously drive a compaction pass; it should signal the workflow.
- [ ] **Drop `Ups` from `Db` and conveyer entirely** (don't relocate). Tests don't need UPS; bookmark/fork/list paths only need FDB writes. The two-phase pin transition runs through the `CompactionSignaler` already on `Db` (write `SQLITE_CMP_DIRTY` in the same request tx, manager workflow picks it up). Remove:
  - `Ups` parameter from `Db::new` and constructors (`conveyer/db.rs`).
  - `_ups: &Ups` parameters from `bookmark::create_pinned_bookmark` / `delete_pinned_bookmark` / `restore_to_bookmark` (`conveyer/bookmark.rs:157, 247, 265`).
  - Any `Ups` import from `conveyer/branch.rs:15`.
  - Every `test_ups()` helper and `PubSub`/`Ups` reference in `tests/bookmarks.rs`, `tests/fork_database.rs`, `tests/fork_namespace.rs`, `tests/list_databases.rs`, `tests/debug.rs`.
- [ ] **Move metric label constants** from `compactor::metrics` to `depot::metrics` or per-module owners. Remaining users: `cold_tier/mod.rs:11`.
- [ ] **Delete `src/compactor/` and all related tests:** `tests/compactor_*.rs`, `tests/cold_compactor*.rs`, `tests/dr_replay_from_s3_alone.rs`, `tests/concurrent_fork_during_eviction.rs`, `tests/eviction_during_active_read.rs`.

Once the modules are gone, the legacy-only review findings (missing OCC fence in cold Phase C, eviction lease without renewal, `concat_shard_bytes` silent fallback, per-pod pass-count quota cadence, inline `#[cfg(test)] mod tests` in three legacy worker files) all evaporate.

## More legacy / dead code to delete (second pass)

- [ ] **`legacy-inline-tests` cargo feature** — `engine/packages/depot/Cargo.toml:9` plus the three `#[cfg(all(test, feature = "legacy-inline-tests"))]` gates at `src/lib.rs:15`, `src/conveyer/ltx.rs:490`, `src/conveyer/page_index.rs:112`. Move the feature-gated tests under `tests/` and delete the feature.
- [ ] **`LegacyDatabaseBranchRecord` and the `V1` variant of `VersionedDatabaseBranchRecord`.** `src/conveyer/types.rs:471-502`. The `From<v1>` impl defaults `lifecycle_generation = 0` (was open question Q6). Per user direction "no v1, no legacy" — delete v1 entirely, keep only `V2`/`Latest`. This also resolves the original Q6 concern about workflow generation activities mistakenly accepting generation-0 records.
- [ ] **`StorageScope::Legacy` / `ReadSource::Legacy` storage-scope fallback.** Pre-PITR database-scoped storage layout. CLAUDE.md already forbids: "Namespace-branch tombstones return `DatabaseNotFound` and must not fall back to legacy database-scoped storage." Live call sites:
  - `src/conveyer/read.rs:99-100, 145, 188-208, 272, 690, 743, 750, 757, 764, 768, 771, 799` (entire `ReadSource::Legacy` arm + `StorageScope::Legacy` wiring)
  - `src/compactor/compact.rs:279, 319, 330, 430, 447, 763-880` (`StorageScope::Legacy` + `legacy_branch_id` param) — goes with the legacy compactor delete
  - `src/compactor/shard.rs:28, 127, 134, 148-149, 155 load_latest_legacy_shard_blob` — goes with the legacy compactor delete
- [ ] **`legacy_materialized_txid` fallback in commit path.** `src/conveyer/commit.rs:402, 410, 450, 455, 504, 531`. With workflow compaction owning `CMP/root` for every live branch, the fallback shouldn't exist; resolve `CMP/root` unconditionally or fail loudly.
- [ ] **`#[allow(dead_code)]` on `Db` struct.** `src/conveyer/db.rs:99`. The struct is live; the marker likely masks one or two genuinely unused fields. Remove the allow and let the compiler flag dead fields individually.

(Note: the three `#[allow(dead_code)]` markers in `src/compactor/{cold/worker.rs:83, worker.rs:80, eviction/mod.rs:108}` go away when the legacy compactor is deleted.)

(Q2 retracted — `restore_to_bookmark` incrementing `fork_depth` is by design. Per CLAUDE.md "PITR, forking, and `restore_to_bookmark` are all the same primitive: branch-at-position." Each rollback creates a new branch derived from the prior frozen branch, so depth +1 is correct ancestry. The 16-rollback bound is a deliberate consequence.)

(Q6 retracted — `load_branch_ancestry` using `0..=MAX_FORK_DEPTH` is required for a valid depth-16 chain. The limit is 16 parent edges, which means 17 branch records including the root. The loader errors only if the 17th record still has a parent.)

(Q12 retracted — the claimed "DB-pin-only staged coverage" failure does not reproduce for a pin at `head_txid`. `selected_hot_coverage_txids` always includes `head.head_txid`, staging writes shards for that head coverage, and those refs populate `latest_staged_shards` before PIDX clearing.)
