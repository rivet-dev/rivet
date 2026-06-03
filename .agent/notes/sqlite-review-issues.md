# SQLite Implementation Review — Issue Tracker

Running list of issues from the adversarial reviews. User decides which ones to fix; this file is the queue.

## Format

Each issue:
- **ID** (S1, S2, ...) — short id we can refer to in conversation.
- **Title** — one-line summary.
- **Source** — which adversary surfaced it (concurrency / durability / vfs / tests) or `manual` if added by hand.
- **Severity** — BLOCKER / SERIOUS / MINOR / NIT.
- **Location** — file:line.
- **Failure mode** — concrete bad sequence in 1-3 sentences.
- **Fix sketch** — what we'd actually do.
- **Status** — `open` / `fixing` / `done` / `wontfix` / `deferred`.

## Issues

### S1 — `commit_atomic_write` poisons `last_error` on success
- **Source:** vfs · **Severity:** SERIOUS
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs:1104-1108`
- **Failure mode:** After a successful `SQLITE_FCNTL_COMMIT_ATOMIC_WRITE`, code unconditionally calls `set_last_error("post-commit atomic write succeeded: ...")`. `xGetLastError` (line ~2019) then copies that string into SQLite's error buffer, and `NativeDatabase::take_last_kv_error()` (consumed by `rivetkit-core/src/actor/sqlite.rs:263`) returns a "success" string framed as an error. Callers polling `take_last_kv_error()` after a normal commit will treat success as a transport/fence failure and trigger mark_dead / retries.
- **Fix sketch:** Replace `set_last_error(...)` with `clear_last_error()` to match the file's pattern (success paths clear at 1649, 1797; failure paths set at 899, 1952, 2004). If size diagnostics are still wanted, use `tracing::debug!` with structured fields.
- **Regression test:** `commit_atomic_write_clears_last_error_on_success` in `rivetkit-rust/packages/rivetkit-sqlite/tests/inline/vfs.rs` (added; currently fails — passes once fix lands).
- **Status:** open · **Verified:** yes — `vfs.rs:1144-1148` (line shifted from claimed 1104) unconditionally calls `set_last_error("post-commit atomic write succeeded: ...")` in the success path right after `tracing::debug!("vfs commit complete (atomic)")`; success pattern elsewhere is `clear_last_error` (e.g. lines 1649, 1797), and `set_last_error` is only called on real errors (899, 1952, 2004).

### S2 — `vfs_delete` of main DB silently no-ops with stale in-memory state
- **Source:** vfs · **Severity:** SERIOUS
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs:1924-1932`
- **Failure mode:** `xDelete` on the main DB path returns `SQLITE_OK` without clearing `db_size_pages`, `page_cache`, or the dirty buffer. Subsequent `xRead` returns zero-filled pages from cache and a stale `db_size_pages`, so SQLite sees a non-empty DB where it expects "freshly deleted." Rare in production today (DELETE journal mode rarely deletes main file), but a contract violation.
- **Fix sketch:** Either return `SQLITE_IOERR_DELETE` for main-DB deletes (loud failure), or actually reset state under a write lock and zero `db_size_pages`.
- **Regression test:** `vfs_delete_main_db_resets_in_memory_state` in `rivetkit-rust/packages/rivetkit-sqlite/tests/inline/vfs.rs` (added; currently fails — passes once either fix lands; the assertion accepts non-OK return codes OR `(db_size_pages, dirty) == (0, 0)`).
- **Status:** open · **Verified:** yes — `vfs.rs:1986-2011` (shifted): `vfs_delete` falls through to `SQLITE_OK` when `path == ctx.actor_id` (the main DB path) without touching `db_size_pages`, `page_cache`, or `write_buffer.dirty`; only the non-main aux branch calls `delete_aux_file`.

### S3 — `resolve_pages` holds `state.write()` across cache-fill loop
- **Source:** vfs · **Severity:** MINOR (perf/contention)
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs:957-975`
- **Failure mode:** Write guard bound but only used to call `state.page_cache.insert(...)` (moka, internally synced). Serializes all readers/writers for the duration of the cache-fill loop with no actual mutation needing the write lock.
- **Fix sketch:** Use `state.read()` or drop the guard entirely; let moka's internal sync handle it. Also clean up indentation that suggests a refactor leftover.
- **Status:** fixed · **Verified:** yes — `resolve_pages` now clones the internally synchronized `moka::Cache` handle under a short read and fills fetched pages through that handle, so the post-fetch loop no longer holds a VFS state write guard. The shifted indentation in that block was also cleaned up.

### S5 — `xSync` durability is transport-acked, not FDB-committed (undocumented)
- **Source:** vfs · **Severity:** NIT (design fenced, but undocumented)
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs:1741-1755`
- **Failure mode:** `io_sync` returns once the engine acks `sqlite_commit`. The contract that "ack ⇒ FDB-durable" lives in depot, not in this file. If pegboard-envoy ever batches/buffers commit acks, `xSync` silently becomes a lie. No comment in the VFS warns future maintainers.
- **Fix sketch:** Add a doc comment on `io_sync` stating the durability contract is delegated to depot's `sqlite_commit` reply.
- **Status:** done · **Verified:** yes — `io_sync` now documents that xSync returns once `ctx.flush_dirty_pages()` resolves, byte durability is delegated to depot's `sqlite_commit` reply, and pre-acking before the FDB tx commit would break the contract.

### S6 — Stale WASM TS VFS references across docs and notes
- **Source:** vfs · **Severity:** MINOR (docs)
- **Locations cleaned up:**
  - `docs-internal/engine/sqlite-vfs.md` — deleted the "Native VFS ↔ WASM VFS parity" section; renamed page to "SQLite VFS"; folded native-only chunk/PRAGMA facts into Package boundaries.
  - `docs-internal/engine/sqlite/vfs-brief.md` — replaced "Parity Links" section with "Reference Links"; dropped the dead `packages/sqlite-wasm/src/` path.
  - `CLAUDE.md` — Reference Docs entry rewritten to "native-only VFS rules".
  - `.agent/notes/rivetkit-core-walkthrough.md` lines 260, 396 — replaced parity-invariant paragraphs with native-only statements.
- **Intentionally kept:** `scripts/publish/src/lib/packages.ts` and `scripts/publish/src/local/cut-release.ts` keep `@rivetkit/sqlite-wasm` in their EXCLUDED set with explanatory comments — the npm package name is deliberately retained on the registry as deprecated; not a runtime claim.
- **Out of scope:** archive directories (`docs-internal/rivetkit-typescript/sqlite-ltx/archive/`, `scripts/ralph/archive/`) intentionally preserve historical state.
- **Status:** done

### S7 — `xFileSize` reads `state.page_size` that's initialized once and never re-read
- **Source:** vfs · **Severity:** NIT
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs:1758`
- **Failure mode:** `page_size` is set at `VfsState::new` and never refreshed from the DB header. Today open pins `page_size=4096` first, so harmless. Future `PRAGMA page_size=...` on an empty DB would diverge VFS state from on-disk reality.
- **Fix sketch:** Debug assert `page_size == DEFAULT_PAGE_SIZE`, or re-read header bytes 16-17 on first non-empty fetch.
- **Status:** wontfix · **Verified:** yes — `rg -n "seed_main_page|fetch_initial_main_page"` shows `VfsContext::new` calls `fetch_initial_main_page`, and the fetched page flows into `VfsState::seed_main_page`. `cargo check -p rivetkit-sqlite` passes and does not report either helper as dead. The claim "never refreshed from the DB header" is wrong for the production open path; the residual runtime-only concern is not this issue.

### S8 — `query_text_nul.rs` only covers TEXT round-trip, not SQL-string NUL rejection
- **Source:** vfs · **Severity:** NIT (test gap)
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/tests/query_text_nul.rs` + `src/query.rs:48`
- **Failure mode:** `CString::new(sql)` rejects SQL with embedded NULs as a generic anyhow error, but no test pins this rejection. Future regression could swallow it.
- **Fix sketch:** Add a test asserting `query_statement(db, "SELECT 1\0; --", None)` returns the CString error chain.
- **Status:** fixed · **Verified:** yes — `tests/query_text_nul.rs` now includes `query_sql_with_embedded_nul_is_rejected`, which calls `query_statement(db, "SELECT 1\0; --", None)` and asserts the returned error chain contains the `CString::new(sql)` NUL-byte rejection.

### S9 — `tests/inline/vfs.rs` lacks crash-recovery / concurrent-reader / PITR-restore coverage
- **Source:** vfs · **Severity:** MINOR (test gap)
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/tests/inline/vfs.rs`
- **Failure mode:** 80+ tests cover transport mark-dead, batch-atomic probe, multi-actor isolation, multithread churn, close-reopen, CompactionSignaler wake, and power-loss between dirty buffer and commit. Missing: concurrent reader during writer commit-atomic, PITR-restore-then-write, fork-and-immediately-reopen.
- **Fix sketch:** Add the remaining missing scenarios as new tests in `tests/inline/vfs.rs`.
- **Status:** fixed · **Verified:** yes — `tests/inline/vfs.rs` now includes `direct_engine_crash_with_dirty_buffer_recovers_last_commit`, which injects an unacked dirty page, fails the close-time commit, and verifies reopen sees the last successful commit. It also includes `concurrent_reader_during_commit_atomic_observes_consistent_snapshot`, which pauses direct commit transport while a reader issues `xRead` and asserts the reader sees either the full pre-commit or full post-commit pages, never a mixed snapshot. Depot integration coverage now includes `write_after_pitr_restore_lands_on_restored_branch`, which restores from PITR, writes on the restored branch, and verifies parent commits after the restore cap stay hidden, plus `fork_database_immediate_reopen_isolated_from_parent_later_writes`, which forks and immediately reopens a fresh `Db` handle before proving later parent writes stay hidden from another fresh fork handle.

### S11 — `wait_idle_for_test` has classic arm-after-check race (lost wakeup)
- **Source:** concurrency · **Severity:** SERIOUS
- **Location:** `engine/packages/depot/src/conveyer/read/cache_fill.rs:133-140`
- **Failure mode:** Loop checks `outstanding.load() == 0`, then awaits `idle_notify.notified()`. Between the load returning false and `.notified()` arming, every worker can finish and call `notify_waiters()` (line ~227) — `notify_waiters` does NOT store a permit, so the wakeup is lost and the waiter parks forever. Violates CLAUDE.md "waiters must arm the notification before re-checking the counter." Test-only (`#[cfg(debug_assertions)]`) but `pub` and used by integration tests; can hang CI when work drains fast.
- **Fix sketch:** Pin `let n = idle_notify.notified();` (or `Notified::enable`) **before** the load, then re-check, then `.await`.
- **Status:** fixed · **Verified:** yes — `shard_cache_fill_wait_idle_prearms_before_rechecking_outstanding` forces `outstanding` to drain and call `notify_waiters()` after `wait_idle_for_test` observes a nonzero count but before it awaits; `wait_idle_for_test` now creates, pins, and enables `idle_notify.notified()` before loading `outstanding`.

### S12 — Multi-step branch-cache invalidation in `get_pages` is non-atomic
- **Source:** concurrency · **Severity:** SERIOUS
- **Location:** `engine/packages/depot/src/conveyer/read.rs:353-359`
- **Failure mode:** Three sequential `write().await`s on three separate `tokio::sync::RwLock`s (`branch_id`, `ancestors`, `last_access_bucket`) plus a preceding `cache.clear()`. A concurrent `get_pages` can land between any two writes and observe `branch_id = new` with `ancestors = old`. CLAUDE.md says "Db branch and PIDX caches must be invalidated when DBPTR moves … cached branch id, quota, and PIDX rows stale" — per-RwLock writes don't compose. Pegboard exclusivity makes the actor a single writer, but multiple read-side `get_pages` share the `Db`. Worst case is a stale-cache miss that resolves on next tx; no data corruption.
- **Fix sketch:** Bundle `(branch_id, ancestors, last_access_bucket)` into one `RwLock<Option<CacheSnapshot>>` and update atomically — same lock as the cache-clear if possible.
- **Status:** fixed · **Verified:** yes — `branch_cache_snapshot_is_atomic_across_dbptr_move` warms the old branch cache, rolls DBPTR back to a new branch, then runs concurrent `get_pages` calls with an observer asserting the cache never exposes the new branch id with the old ancestry root. `Db` now stores branch id, ancestry, access bucket, and PIDX index in one `CacheSnapshot` published under a single `cache_snapshot` write lock.

### S13 — `NativeDatabase::Drop` `block_on(transport.commit)` has no timeout
- **Source:** concurrency · **Severity:** MINOR
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs:2160-2188`
- **Failure mode:** Drop calls `flush_dirty_pages()` → `runtime.block_on(transport.commit(...))`. If the envoy WS is gone (actor sleep/destroy already ran), the future may never resolve and the Drop thread wedges.
- **Fix sketch:** Wrap inner future in `tokio::time::timeout(short_deadline, ...)`; surface failure via `tracing::error!` instead of hanging.
- **Status:** fixed · **Verified:** yes — Added `native_database_drop_times_out_pending_commit`, which flips `MockProtocol` commits to a never-resolving future after open and drops the `NativeDatabase` from a `std::thread`; pre-fix it timed out after 2s. Drop-time buffered commits now use `tokio::time::timeout` with a short bound and return after logging `tracing::error!` on timeout.

### S15 — `takeover::reconcile_blocking` blocks the calling tokio worker
- **Source:** concurrency · **Severity:** MINOR
- **Location:** `engine/packages/depot/src/conveyer/db.rs:217` → `engine/packages/depot/src/takeover.rs:39-57`
- **Failure mode:** `Db::new_inner` (sync ctor) calls `reconcile_blocking`, which spawns a thread, builds a fresh single-thread tokio runtime, runs an FDB scan, and `.join()`s. The caller thread (likely a tokio worker on the hot path) blocks for the full FDB scan. `#[cfg(debug_assertions)]` only — release unaffected.
- **Fix sketch:** Make the takeover reconcile fully async, or move it to a one-shot background task signalled via `Notify` so `Db::new` doesn't block the caller.
- **Status:** fixed · **Verified:** yes — Added `db_new_does_not_wait_for_takeover_reconcile`, which pauses the debug takeover reconcile with a `Notify` gate, proves a concurrent runtime task still makes progress, and asserts `Db::new` returns before the reconcile gate is released. `Db::new_inner` now calls `takeover::reconcile_nonblocking`, which schedules reconcile on the current Tokio runtime or a detached background thread instead of joining the caller.

### S17 — Cold-tier read race: paused reader past delete-grace returns silent zero-page
- **Source:** concurrency · **Severity:** MINOR (defense-in-depth)
- **Location:** `engine/packages/depot/src/conveyer/read/cold.rs:144-184` ↔ reclaimer cold-object cleanup
- **Failure mode:** `tx_load_latest_compaction_cold_ref` reads cold-shard refs under `Snapshot` (no read-conflict-range); `cold_tier.get_object` then runs outside any tx. Reclaimer "retires FDB refs before S3 deletes + grace window" is correct only if the grace window outlives every in-flight read. A reader paused longer than `delete_after_ms` (cgroup throttle, runtime starvation) returns a missing blob, which propagates to `bytes = vec![0; PAGE_SIZE]` with `Miss` outcome at `read.rs:332` — silent zero-page for a real page.
- **Fix sketch:** Post-fetch re-validate the cold ref under `Serializable` before returning bytes; if the ref is gone, error rather than zero-fill. Or attach a fence epoch to the ref and require it match at read time.
- **Status:** fixed · **Verified:** yes — Added `cold_ref_retired_during_cold_object_fetch_errors_instead_of_zero_fill`, which pauses a cold-tier object read, retires the compaction cold-shard ref, deletes the cold object, and asserts the read returns `ShardCoverageMissing` instead of a zero-filled page. `load_cold_object_for_page` now distinguishes object-missing from page-missing, and compaction cold-shard hits re-read the live ref under `Serializable` before returning bytes.

### S18 — `Mutex<mpsc::Receiver>` shared across cache-fill workers serializes dispatch
- **Source:** concurrency · **Severity:** MINOR (perf)
- **Location:** `engine/packages/depot/src/conveyer/read/cache_fill.rs:86, 199-204`
- **Failure mode:** N workers all `lock().await` the same `mpsc::Receiver`. Effectively a queue of waiters around one receiver. Not a correctness bug (lock drops before fill work), but defeats worker concurrency for the recv stage.
- **Fix sketch:** Switch to `async-channel` (multi-consumer) or hand each worker its own receiver via fan-out at the producer.
- **Status:** fixed · **Verified:** yes — `cache_fill.rs` now uses `async-channel` with cloned worker receivers. Each worker calls `receiver.recv().await` directly, with no shared receiver mutex.

### S19 — `rivetkit-sqlite/src/vfs.rs` violates the "tiny shim" rule for tests
- **Source:** tests · **Severity:** SERIOUS
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs` — 45 `#[cfg(test)]` blocks containing `DirectStorage` (~150 lines starting line ~190), `DirectActorPages`, `DirectTransportHooks`, `MockProtocol` (line ~1277), helper methods `compaction_signals`, `take_commit_error`. Shim at line ~2243 is correct, but the test fixtures themselves live inline.
- **Failure mode:** CLAUDE.md requires only a tiny `#[cfg(test)] #[path = "..."] mod tests;` shim in `src/`; bodies must live under `tests/`. Inline fixtures widen the private-API test surface and risk the "stale shared shim" rot CLAUDE.md warns about.
- **Fix sketch:** Move `DirectStorage`/`MockProtocol`/etc. to `tests/inline/vfs_support.rs` (or `tests/common/`) reachable via the same shim; keep only the `mod tests;` line in `src/vfs.rs`.
- **Status:** open · **Verified:** yes — `src/vfs.rs` has 44 `#[cfg(test)]` blocks inline. Inline fixtures: `DirectStorage` (line 219-220), `DirectActorPages` (line 229-231), `DirectTransportHooks` (line 379-381), `MockProtocol` (line 442, not the claimed ~1277), helpers `compaction_signals` (line 374-376), `take_commit_error` (line 391-394). The shim at line 2320-2322 (`#[cfg(test)] #[path = "../tests/inline/vfs.rs"] mod tests;`) is correct, but the fixtures live inline.

### S20 — `actor-db.test.ts` polls actor actions while waiting for sleep
- **Source:** tests · **Severity:** SERIOUS
- **Location:** `rivetkit-typescript/packages/rivetkit/tests/driver/actor-db.test.ts:233-269` (and `:309-322`)
- **Failure mode:** "persists across sleep and wake cycles" calls `actor.getCount()` in a poll loop after `triggerSleep()` + `waitFor(SLEEP_WAIT_MS)`. Each `getCount()` is an action that resets the sleep deadline — CLAUDE.md flags this exactly. Test purports to verify "count after wake" but actually verifies "count after we ourselves woke it." Same pattern in the hard-crash recovery section.
- **Fix sketch:** Read state via a non-action observer (inspector state read or lifecycle event), or assert once after a single deterministic wake.
- **Status:** open · **Verified:** yes — confirmed `actor-db.test.ts:233-269` polls `actor.getCount()` after `triggerSleep()` and same pattern at `:308-322` post `hardCrashActor`; action dispatch goes through the engine HTTP request counter whose change callback calls `reset_sleep_timer()` (`rivetkit-core/src/actor/sleep.rs:531-540`), and `DispatchCommand::Action` in `actor/task.rs:923-989` wraps the reply in `wait_until`, keeping the actor awake until the action completes; no inspector / non-action observer is used in these tests.

### S21 — `actor-db-stress.test.ts` `vi.waitFor` masks an actor-handle startup/stale-handle race
- **Source:** tests · **Severity:** SERIOUS
- **Location:** `rivetkit-typescript/packages/rivetkit/tests/driver/actor-db-stress.test.ts:79-97` (also `actor-db.test.ts:144-152`, `actor-db-pragma-migration.test.ts:9-15`, `actor-db-raw.test.ts:74-82`)
- **Failure mode:** `insertBatch(10)` and `getCount()` wrapped in `vi.waitFor` because "the actor can still be starting" / "older direct targets pointing at a stopping actor instance." This is retry-until-success masking a real ordering bug — CLAUDE.md prohibits flake masking. The 10s timeout absorbs the bug. The pattern is being copy-pasted across the suite.
- **Fix sketch:** Root-cause the actor-handle invalidation on stopping; expose a "ready" lifecycle promise from `connect()` so callers can `await` it deterministically. Then delete every "wait for startup" `waitFor`.
- **Status:** open · **Verified:** yes — confirmed all four cited sites: `actor-db-stress.test.ts:79-84` (insertBatch retry "the actor can still be starting") and `:89-97` (getCount retry "direct target from the insert can already be moving through sleep teardown"); `actor-db.test.ts:144-152` (`reset()` retry "until the actor finishes startup"); `actor-db-pragma-migration.test.ts:9-15` (`waitForPragmaAction` "the pragma actor can still be booting"); `actor-db-raw.test.ts:74-82` (cross-actor `getCount` retry "fast sleep can leave older direct targets pointing at a stopping actor instance"). All wrap actions whose only documented failure mode is the startup/stale-handle race.

### S22 — `actor-sleep-db.test.ts` "active db writes interrupted by sleep" makes a timing-dependent magnitude assertion
- **Source:** tests · **Severity:** SERIOUS
- **Location:** `rivetkit-typescript/packages/rivetkit/tests/driver/actor-sleep-db.test.ts:1018-1054` + fixture `sleep-db.ts:872-873`
- **Failure mode:** Asserts `writeEntries.length > 0 && < ACTIVE_DB_WRITE_COUNT (500)` with `ACTIVE_DB_WRITE_DELAY_MS=5` + `ACTIVE_DB_GRACE_PERIOD=50` — encodes "shutdown happens somewhere in the middle of a non-deterministic loop." Fast hardware → ≥500 (false fail). Slow CI → 0 (false fail).
- **Fix sketch:** Use a Promise gate the actor awaits between writes; deterministically advance N writes, then trigger sleep, then assert exactly the expected count.
- **Status:** fixed · **Verified:** yes — replaced the `ACTIVE_DB_WRITE_DELAY_MS` loop with a WebSocket-driven Promise gate. The actor awaits `continue-write` permits and acks each completed write; `actor-sleep-db.test.ts` now advances exactly 3 writes before triggering sleep and asserts `writeEntries.length === 3`. Verified with `pnpm vitest run tests/driver/actor-sleep-db.test.ts -t active-db-writes` and a 10-run loop.

### S23 — `workflow_compaction_skeletons.rs` `wait_until` polls real time
- **Source:** tests · **Severity:** SERIOUS
- **Location:** `engine/packages/depot/tests/workflow_compaction_skeletons.rs:184-206`
- **Failure mode:** Polls every 25ms up to 5s for workflow rows. Comment: "Gasoline debug rows and UDB test-observation rows do not expose a change notification API." US-065/066/067 already replaced three similar real-clock sleeps; this is the same anti-pattern. Cumulative cost across ~50 callers in this file is significant.
- **Fix sketch:** Add a Gasoline test hook `wait_for_workflow_row` that subscribes to the same `Notify` Gasoline already uses internally for signal dispatch.
- **Status:** open · **Verified:** partial — confirmed `wait_until` at `workflow_compaction_skeletons.rs:184-206` is real-clock: it loops `check().await` and `tokio::time::sleep(Duration::from_millis(25))` against `started_at.elapsed() > Duration::from_secs(5)` with the cited comment. The Gasoline core blocker is fixed: `BumpSubSubject::WorkflowCreated { tag }` now publishes after workflow creation commits. Remaining work is to add the Depot `wait_for_workflow_row(tag)` helper and replace the polling callers.

### S24 — `actor-sleep-db.test.ts:593` `waitFor` reveals a missing `connect()` ready event
- **Source:** tests · **Severity:** MINOR (root-cause hint)
- **Location:** `rivetkit-typescript/packages/rivetkit/tests/driver/actor-sleep-db.test.ts:593-596` (also `:23, :39, :202`)
- **Failure mode:** Justification "connect() has no ready promise" is a real API gap, not a per-test problem. The right fix is in core/TS, not in every test that copies the workaround.
- **Fix sketch:** Add a `ready` promise / event to `connect()`. Replace the `waitFor`s once the API exists.
- **Status:** fixed · **Verified:** yes — `ActorConnRaw.ready` now exposes a Promise that resolves from the centralized `#setConnStatus("connected")` transition when `isConnected` first flips true. `tests/driver/actor-conn.test.ts` covers the contract without `vi.waitFor`, asserting the Promise is pending before the first connected state, resolves after the connection opens, and remains the same Promise on later reads.

### S25 — `tests/inline/vfs.rs` leaks process-global VFS registrations between tests
- **Source:** tests · **Severity:** MINOR
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/tests/inline/vfs.rs:1428-1437` (~60 sites)
- **Failure mode:** Each test registers a fresh-named VFS via `next_test_name("sqlite-direct-vfs")`; SQLite VFS registration is process-global. Tempdir cleanup is OK via `Drop`, but the global VFS registry accumulates entries; a panic in one test leaves its VFS registered.
- **Fix sketch:** Provide a teardown helper that unregisters the VFS, ideally in a guard struct with `Drop`.
- **Status:** fixed · **Verified:** yes — Added `vfs_registration_is_removed_after_registration_panic`, which registers `panic-leak-vfs-*`, panics inside `std::panic::catch_unwind`, and asserts `sqlite3_vfs_find(...)` returns null after unwinding. `SqliteVfsRegistration` now owns `sqlite3_vfs_register`/`sqlite3_vfs_unregister` as a Drop guard while `SqliteVfs` keeps the VFS name and context alive until after unregister.

### S26 — `tests/inline/vfs.rs` RocksDB driver-build retry loop with `std::thread::sleep`
- **Source:** tests · **Severity:** MINOR
- **Location:** `rivetkit-rust/packages/rivetkit-sqlite/tests/inline/vfs.rs:55-60`
- **Failure mode:** Up to 50 × 10ms retries to construct a RocksDB driver. CLAUDE.md prohibits sleep-loop polling; the underlying race (parallel tests racing on tempdir / rocksdb lock) should be fixed, not papered over. 500ms budget per test multiplies fast across the suite.
- **Fix sketch:** Each test already has its own `tempdir()` — confirm there's no real shared resource. If there is, fix the sharing; if not, drop the retry.
- **Status:** fixed · **Verified:** yes — `direct_engine_open_engine_is_concurrency_safe` reproduced the race before the fix with RocksDB `LOCK` errors. Root cause was the direct VFS harness's non-atomic `std::sync::OnceLock` get-then-set lazy init: concurrent callers opened the same harness tempdir at the same time, and RocksDB allows only one open handle per path. The harness now uses `tokio::sync::OnceCell::get_or_init` and a single `RocksDbDatabaseDriver::new(...).await` initializer with no retry loop.

### S29 — Restore-point create races with reclaim deletion of resolved COMMITS row
- **Source:** durability · **Severity:** SERIOUS
- **Location:** `engine/packages/depot/src/conveyer/restore_point/pinned.rs:26-43, 129-198` (resolve in tx A; pin write in tx B)
- **Failure mode:**
  1. tx A resolves target `txid=N` from a `PITR_INTERVAL` row whose `expires_at_ms` is at the edge.
  2. The interval expires.
  3. Reclaim sees no `DB_PIN(kind=RestorePoint)` for `txid=N` (tx B hasn't run) and `compare_and_clear`s `COMMITS/{N}` and `VTX/{vs}`.
  4. tx B writes the pin and `RestorePointRecord` referencing versionstamp `vs`.
  5. `resolve_restore_point` later returns `RestoreTargetExpired` (`resolve.rs:387-399`).
  User holds a `Ready` restore point that cannot be resolved.
- **Fix sketch:** Inside tx B, re-validate `branch_commit_key(target.txid)` (Serializable read) and abort with `RestoreTargetExpired` if missing. Or fold A and B into a single UDB tx.
- **Status:** fixed · **Verified:** yes — Added `create_restore_point_revalidates_target_commit_after_resolve_race`, which pauses after tx A resolves the timestamp target, clears the resolved `COMMITS`, `VTX`, and `PITR_INTERVAL` rows to model the reclaim gap, then resumes tx B and asserts it aborts with `RestoreTargetExpired` without writing a Ready restore point. `create_restore_point_for_resolved` now re-reads `branch_commit_key(target.txid)` under `Serializable` inside tx B before writing the restore-point record or DB pin.

### S30 — Add explicit VFS/depot test for SQLite sparse-page zero-fill semantics
- **Source:** manual · **Severity:** MINOR (coverage)
- **Location:** `engine/packages/depot/tests/conveyer_read.rs`, `rivetkit-rust/packages/rivetkit-sqlite/tests/inline/vfs.rs`
- **Failure mode:** SQLite's VFS contract requires short reads to zero-fill unread bytes. Depot/VFS sparse-page behavior should be pinned explicitly so a missing in-range page with no materialized source returns a zero page, while missing pages that have a real broken source still error. Without this test, a future refactor can regress either direction: returning `ShardCoverageMissing` for legitimate sparse pages, or incorrectly zero-filling pages whose delta/cold source is missing.
- **Fix sketch:** Add an explicit depot regression for `db_size_pages > requested_pgno`, no PIDX/delta/shard/cold source, asserting `get_pages([requested_pgno]) == page(0)`. Add a companion negative case where a PIDX/delta or compaction cold-shard candidate exists but the source is missing, asserting an error rather than zero-fill. If practical, add a VFS-level read test that opens SQLite over this storage and proves the sparse page is observed as a zero-filled page through `xRead`.
- **Status:** fixed · **Verified:** yes — Added `get_pages_zero_fills_sparse_page_without_any_source` and `get_pages_errors_for_corrupted_delta_source` in `engine/packages/depot/tests/conveyer_read.rs`; legitimate sparse in-range reads return a zero page, while a corrupted delta blob returns a decode error instead of zero-filling. Verified with `cargo test -p depot get_pages_ --test conveyer_read`.

### S30 — `restore_database` is non-atomic across rollback + undo restore-point creation
- **Source:** durability · **Severity:** SERIOUS
- **Location:** `engine/packages/depot/src/conveyer/restore_point/restore.rs:14-29`
- **Failure mode:** Four separate transactions: resolve target → capture undo → rollback DBPTR → write undo restore point. A crash between steps 3 and 4 leaves DBPTR pointing at the rolled branch with no recoverable undo restore point — user-visible state has changed before the recovery handle is durable.
- **Fix sketch:** Fold steps 3 and 4 (DBPTR swap + `create_restore_point_for_resolved` for the undo point) into one UDB transaction. `derive_branch_at` can also be inside the same tx.
- **Status:** fixed · **Verified:** yes — Added `restore_database_rollback_and_undo_pin_are_atomic`, which injects a failure after rollback work but before undo pinning and asserts the DBPTR stays on the old branch with no undo restore point. `restore_database` now folds `rollback_database_to_target_tx` and `create_restore_point_for_resolved_tx` into one UDB transaction, so rollback plus undo pinning commit or abort together.

### S31 — `admit_deltas_available` wake is best-effort; signaler failure drops both wake and timestamp update
- **Source:** durability · **Severity:** MINOR (informational; bounded by manager planning deadlines)
- **Location:** `engine/packages/depot/src/conveyer/commit/dirty.rs:46-60`, `commit/apply.rs:368-383`
- **Failure mode:** If signaler invocation fails (transport drop) after FDB tx commit, the wake is lost AND `last_deltas_available_at_ms` is not updated. Manager planning deadlines (500ms-10s) eventually catch up.
- **Fix sketch:** None required short-term; flagged for visibility. If you want bounded latency, advance the cached timestamp eagerly inside the tx and emit signaler from a retry-with-bounded-attempts wrapper.
- **Status:** open · **Verified:** yes — `apply.rs:368-383` `publish_deltas_available_if_needed` invokes the signaler after the FDB tx commit (line 362); on signaler error it `tracing::warn!`s and `return`s at line 379 without updating `last_deltas_available_at_ms` (the assignment at line 382 is gated past the early return). Both the wake and the cached timestamp are dropped together.

### S32 — Shard-cache eviction retention is exact-txid match (correctness-via-cold-coverage; missing regression test)
- **Source:** durability · **Severity:** MINOR (no bug found; coverage gap)
- **Location:** `engine/packages/depot/src/workflows/compaction/shared.rs:916-925`
- **Failure mode:** A `SHARD/{id}/{as_of_txid}` is retained only if a pin has `at_txid == as_of_txid`; pins at `at_txid > newest_shard.as_of_txid` rely on cold (`CMP/cold_shard`) being present. Correctness holds because cold coverage is required by `read_shard_cache_eviction_candidates:851-854`. If cold coverage is somehow lost (S3 deletion gone wrong despite `DeleteIssued` block), eviction has already released the cache row and read fails fast with `ShardCoverageMissing` — intentional. Risk: no regression test that pin-at-future-txid keeps reads working when only an older shard exists.
- **Fix sketch:** Add a regression test: pin at `t2 > newest_shard.as_of_txid (t1)`, read at `t2`, verify result via cold path.
- **Status:** fixed · **Verified:** yes — Added `reclaimer_eviction_preserves_future_pin_reads_via_cold_ref`, which creates a restore-point pin at txid 2, seeds a matching cold-backed SHARD at txid 1, forces reclaim to evict the FDB SHARD row, clears hot PIDX for the page, and verifies `get_pages` still returns the nonzero page via `CMP/cold_shard`. Verified with `cargo test -p depot reclaimer_eviction_preserves_future_pin_reads_via_cold_ref -- --nocapture`, `cargo check -p depot`, and `cargo test -p depot`.

### S33 — Legacy `cold_drained_txid` key consulted on every commit
- **Source:** durability · **Severity:** NIT
- **Location:** `engine/packages/depot/src/burst_mode.rs:54-65`, `conveyer/commit/dirty.rs:126-137`
- **Failure mode:** `branch_manifest_cold_drained_txid_key` is read as a fallback for `compaction_root.cold_watermark_txid`. Workflow compactor doesn't write it anymore; the fallback is dead but still adds an FDB get per commit.
- **Fix sketch:** Delete the read after confirming no production deployment still has the legacy key set; or backfill into `cold_watermark_txid` once at startup and drop the per-commit read.
- **Status:** done · **Verified:** yes — `branch_manifest_cold_drained_txid_key` production reads were removed from `burst_mode.rs` and `commit/dirty.rs`; burst lag now uses workflow `CMP/root.cold_watermark_txid`. Post-fix grep for `branch_manifest_cold_drained_txid_key|set.*cold_drained|atomic.*cold_drained` under `engine/packages/depot` shows only the key helper and tests.

### S34 — `delete_expired_pitr_interval_coverage` is dead code with unsafe semantics
- **Source:** durability · **Severity:** NIT
- **Location:** `engine/packages/depot/src/conveyer/pitr_interval.rs:81-99`
- **Failure mode:** Unused outside tests. Uses raw `clear`; the live reclaimer at `reclaimer.rs:195-199` uses `compare_and_clear`. If a future caller picks up this helper, it would race.
- **Fix sketch:** Delete the helper. Update its tests to call the reclaimer path.
- **Status:** fixed · **Verified:** yes — the dead helper was deleted; `rg -n "delete_expired_pitr_interval_coverage" engine/packages/depot/src engine/packages/depot/tests --glob '!target/**'` now returns no matches, and PITR interval expiry is covered by the live reclaimer tests using `udb::compare_and_clear` at `reclaimer.rs:195-199`.

### S35 — `SQLITE_RESTORE_POINT_COUNT_PER_NAMESPACE` ident still says NAMESPACE post-US-080 rename
- **Source:** durability · **Severity:** NIT
- **Location:** `engine/packages/depot/src/conveyer/metrics.rs:170`
- **Failure mode:** Prometheus name is `sqlite_restore_point_count_per_bucket`; Rust identifier is stale.
- **Fix sketch:** Rename to `SQLITE_RESTORE_POINT_COUNT_PER_BUCKET`.
- **Status:** open · **Verified:** yes — `metrics.rs:170` declares `pub static ref SQLITE_RESTORE_POINT_COUNT_PER_NAMESPACE` while line 171 sets the Prometheus name to `sqlite_restore_point_count_per_bucket` and line 172 documents "per bucket"; identifier is stale.

### S36 — Legacy database-scoped key fns still compiled and read by debug `takeover.rs`
- **Source:** durability · **Severity:** NIT
- **Location:** `engine/packages/depot/src/conveyer/keys.rs:833-965`, `engine/packages/depot/src/takeover.rs`
- **Failure mode:** Pre-PITR helpers (`meta_head_key`, `delta_prefix`, `pidx_delta_prefix`, `shard_prefix`) compiled and consumed by `takeover.rs` (debug-only). Modern paths never write to these keys, so the takeover invariant scan never finds violations — effectively a no-op.
- **Fix sketch:** Either rewire takeover to scan branch-scoped keys, or delete the legacy keyfns + the now-empty takeover scan.
- **Status:** fixed · **Verified:** yes — `rg -n "depot_keys::(meta_head_key|delta_prefix|pidx_delta_prefix|shard_prefix)|\\b(meta_head_key|delta_prefix|pidx_delta_prefix|shard_prefix)\\(" engine/packages --glob '!target/**'` shows production references only in `pegboard/src/actor_sqlite.rs:33,44-46,170`; the legacy takeover scan was removed, and the key helpers remain documented v1 pegboard compatibility.

### S10 — Depot `workflows/` directory layout doesn't follow one-module-per-workflow convention
- **Source:** manual · **Severity:** SERIOUS (structural)
- **Location:** `engine/packages/depot/src/workflows/`
- **Failure mode:** Current layout has a generic `compaction.rs` + `compaction/` submodule containing multiple distinct workflows (`cold`, `companion`, `hot`, `manager`, `reclaimer`) plus shared/util code (`shared.rs`, `types.rs`, `test_hooks.rs`). Convention should be: one file per workflow, named after the workflow (e.g. `workflows/db_manager.rs`, `workflows/db_cold_compactor.rs`, ...). When a workflow needs multiple files, make it a module with the workflow in `mod.rs`. **Shared non-workflow code (utils/types/test helpers) must NOT live under `workflows/`** — move it out.
- **Fix sketch:**
  - Identify the actual workflow names from each of `cold.rs`, `companion.rs`, `hot.rs`, `manager.rs`, `reclaimer.rs` and rename the files to match (e.g. `db_cold_compactor.rs`).
  - Promote each from `workflows/compaction/{name}.rs` to either `workflows/{wf_name}.rs` (single-file) or `workflows/{wf_name}/mod.rs` (multi-file).
  - Move `shared.rs`, `types.rs`, `test_hooks.rs` out of `workflows/` to a sibling location (e.g. `engine/packages/depot/src/compaction/{shared,types,test_hooks}.rs` or split between depot crate root and the depot tests crate).
  - Delete the now-empty intermediate `compaction.rs` / `compaction/` if no workflow shares the umbrella.
- **Status:** open


## Test Plan

For each issue that needs a regression test, the workflow is: (1) write the test
asserting the desired behavior — it must fail today; (2) implement the fix;
(3) re-run and confirm the test passes. Issues marked NO-TEST are doc/rename/
layout changes or perf-only smells with no observable behavior to pin.

### Done

- **S1** — `commit_atomic_write_clears_last_error_on_success` in `rivetkit-rust/packages/rivetkit-sqlite/tests/inline/vfs.rs`. Asserts `take_last_kv_error()` is `None` after a successful `commit_atomic_write` on a dirty page. Currently fails — passes once `set_last_error("post-commit ...")` is replaced with `clear_last_error()`.
- **S2** — `vfs_delete_main_db_resets_in_memory_state` in `rivetkit-rust/packages/rivetkit-sqlite/tests/inline/vfs.rs`. Calls `xDelete` on the main-DB path via `sqlite3_vfs_find` after writing a row; asserts `rc != SQLITE_OK || (db_size_pages, dirty) == (0, 0)`. Currently fails — passes once `vfs_delete` either errors loudly or resets state.

### Tier 1 — easy unit tests (no concurrency primitives)

- **S8** — Add a test in `tests/inline/query.rs` (or `tests/query_text_nul.rs`): call `query_statement(db, "SELECT 1\0; --", None)` and assert the returned error chain mentions the embedded NUL. Today this surfaces a generic anyhow error from `CString::new`; the test pins that contract.
- **S32** — Add a depot integration test in `engine/packages/depot/tests/`: pin a restore point at `t2 > newest_shard.as_of_txid (t1)`, evict shards at `t1`, read at `t2`, assert the read succeeds via the cold path. Pins the "cold-coverage gate keeps eviction safe" invariant.
- **S7** — **Re-verify first.** `cargo check` reports `seed_main_page` and `fetch_initial_main_page` as dead. Confirm whether `VfsContext::new` actually calls them. If dead, S7 is a real bug and the test is: open a DB with `PRAGMA page_size = 8192`, write pages, close, reopen, assert `xFileSize` returns `db_size_pages * 8192`. If the helpers are live, drop S7 to NO-TEST.

### Tier 2 — moderate (Drop / panic / startup races)

- **S13** — Add a test in `tests/inline/vfs.rs`: build a `NativeDatabase` with a `MockProtocol` that never resolves; spawn a `std::thread` that drops the DB, join with a `Duration::from_secs(2)` timeout, assert the join completes (i.e. Drop returned). Today the Drop blocks indefinitely. Fix: wrap the inner `block_on` in `tokio::time::timeout`.
- **S25** — Fixed by `vfs_registration_is_removed_after_registration_panic`; `SqliteVfsRegistration` unregisters during panic unwind.
- **S26** — Diagnose the RocksDB construct flake first; the retry loop at `tests/inline/vfs.rs:47-61` papers over a real race. The "test" is: delete the loop, run the suite — it must pass without retries. Likely root cause: shared global state in `RocksDbDatabaseDriver::new` despite per-test tempdir.

### Tier 3 — TS API gap (S24 unlocks S20 and S21)

- **S24** — Add `connection.ready: Promise<void>` to the rivetkit TS client. Test: assert `await connection.ready` resolves exactly once `isConnected` flips true; assert calls awaiting `ready` succeed without retry. This is the upstream root-cause behind S20 and S21.
- **S21** — After S24, replace startup `vi.waitFor(...)` calls in `actor-db-stress.test.ts:79-97`, `actor-db.test.ts:144-152`, `actor-db-pragma-migration.test.ts:9-15`, `actor-db-raw.test.ts:74-82` with `await connection.ready`. The test asserts `insertBatch(10)` and `getCount()` succeed on the first call after `ready`.
- **S20** — After S24, rewrite `actor-db.test.ts:233-269` and `:309-322` to use a non-action lifecycle observer (inspector state read or `connection.lastSync` event) instead of polling `getCount()`. Test asserts the sleep deadline is not reset by the observer.
- **S22** — Replace `ACTIVE_DB_WRITE_DELAY_MS` race with a Promise-gate fixture: the actor `await`s a gate between writes; the test resolves the gate exactly N times, then triggers sleep, then asserts `writeEntries.length === N`. Pure deterministic — no real-clock tolerance window.

### Tier 4 — deterministic concurrency repros

- **S11** — Fixed with `shard_cache_fill_wait_idle_prearms_before_rechecking_outstanding`. The regression uses a one-shot hook after the nonzero `outstanding` load to drain the counter and call `notify_waiters()` before the waiter awaits; `wait_idle_for_test` now pre-arms and enables `idle_notify.notified()` before re-checking the counter.
- **S12** — Repro half-updated branch cache. Test: drive two concurrent `get_pages` against a `Db` while a DBPTR-move runs between them; gate the move so the test sees `branch_id == new` between the three sequential `write().await` calls; assert no caller observes `(branch_id == new) && (ancestors == old)`. Fix: collapse the triple into a single `RwLock<Option<CacheSnapshot>>`.
- **S15** — Fixed with `db_new_does_not_wait_for_takeover_reconcile`. The regression pauses the debug reconcile via a `Notify` gate instead of real-clock sleep, then asserts a concurrent runtime task makes progress and `Db::new` returns before the reconcile is released.
- **S17** — Repro paused-reader-past-grace silent zero-fill. Test: inject a sleep around `cold_tier.get_object` longer than `delete_after_ms`; concurrently run reclaimer; assert the read returns `ShardCoverageMissing` (or other explicit error), never a zero-filled page. Fix: post-fetch re-validate the cold ref under `Serializable`.
- **S30** — Test: inject a fault hook between `branch::rollback_database_to_target` and `create_restore_point_for_resolved` in `restore_database`; simulate a process crash there; on restart, assert either DBPTR is unchanged OR an undo restore point is durably present. Never both rolled-and-no-undo. Fix: fold rollback + undo-rp into one UDB tx.
- **S29** — Test: open a restore-point at `txid=N` whose covering interval is at the edge of `expires_at_ms`; advance time past expiry between `resolve_restore_target` and `create_restore_point_for_resolved`; trigger reclaim in the gap; assert tx B aborts with `RestoreTargetExpired` or the resulting Ready handle resolves. Fix: re-validate `branch_commit_key(target.txid)` inside tx B.
- **S23** — **Core change done.** `BumpSubSubject::WorkflowCreated { tag }` now exists. Next: add `wait_for_workflow_row(tag)` and rewrite `engine/packages/depot/tests/workflow_compaction_skeletons.rs:184-206` to use it instead of real-clock 25ms polling.
