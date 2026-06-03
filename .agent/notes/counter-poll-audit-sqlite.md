# rivetkit-sqlite counter-poll audit

Date: 2026-04-22
Story: US-028

## Scope

Searched `rivetkit-rust/packages/rivetkit-sqlite/src/` for:

- `loop { ... sleep(Duration::from_millis(_)) ... }`
- `while ... { ... sleep(Duration::from_millis(_)) ... }`
- `tokio::time::sleep`, `std::thread::sleep`
- `AtomicUsize`, `AtomicU32`, `AtomicU64`, and `AtomicBool` fields with waiters
- `Mutex<usize>`, `Mutex<bool>`, and similar scalar locks

## Converted polling sites

- None in this sweep.
  - The US-007 `MockProtocol` counter/gate fix is still present: `awaited_stage_responses` uses `AtomicUsize` plus `stage_response_awaited: Notify`, and `mirror_commit_meta` uses `AtomicBool`.
  - No remaining `Mutex<usize>` / `Mutex<bool>` scalar wait gates were found in `src/`.

## Event-driven sites

- `vfs.rs::MockProtocol::wait_for_stage_responses`
  - Classification: event-driven test waiter.
  - Uses `awaited_stage_responses: AtomicUsize` paired with `stage_response_awaited: Notify`.
  - Wait is bounded by `tokio::time::timeout(Duration::from_secs(1), ...)`.

- `vfs.rs::MockProtocol::commit_finalize`
  - Classification: event-driven test gate.
  - Uses `finalize_started: Notify` and `release_finalize: Notify`; no polling loop.

## Monotonic sequence / diagnostic atomics

- `vfs.rs::NEXT_STAGE_ID`, `NEXT_TEMP_AUX_ID`, and test `TEST_ID`
  - Classification: monotonic ID generators.
  - No waiter.

- `vfs.rs::commit_atomic_count`
  - Classification: diagnostic/test observation counter.
  - Tests read it after operations complete; no async waiter or sleep-loop polls it.

- `vfs.rs` performance counters (`resolve_pages_total`, `resolve_pages_cache_hits`, `resolve_pages_fetches`, `pages_fetched_total`, `prefetch_pages_total`, `commit_total`, timing totals)
  - Classification: metrics/snapshot counters.
  - Tests read snapshots after controlled operations; no wait-for-zero or wait-for-threshold loop.

- `vfs.rs` test `keep_reading: AtomicBool`
  - Classification: cross-thread control flag.
  - The reader thread intentionally runs SQLite reads until compaction completes; it is not waiting for the flag to become true and has no sleep-based polling interval.

## Non-counter sleep loops

- `vfs.rs::vfs_sleep`
  - Classification: SQLite VFS implementation callback.
  - Implements SQLite's `xSleep` contract by sleeping for the requested microseconds.

- `vfs.rs::DirectEngineHarness::open_engine`
  - Classification: external resource retry backoff.
  - Retries `RocksDbDatabaseDriver::new(...)` with a 10 ms sleep because the temporary DB directory can be briefly busy between test runs. It does not poll an in-process counter/flag/map size.

- `query.rs` statement loops and `vfs.rs::sqlite_step_statement`
  - Classification: SQLite stepping loops.
  - These loop over `sqlite3_step(...)` until `SQLITE_DONE` or an error; they do not sleep or poll shared state.
