# SQLite Worker Executor Spec Review

Source spec: `.agent/specs/sqlite-worker-executor-read-pool.md`

## Critical

1. **Reader handles must be recycled after writer commits.**
   - Current spec only calls out recycling after schema-changing writes.
   - Risk: SQLite readonly connections have their own pager/schema caches. Sharing the VFS `moka` page cache does not prove reused reader handles observe committed writer changes.
   - Needed decision: require closing/reopening all reader SQLite handles after every writer commit unless a tested SQLite lock/change-counter design proves reuse is safe.

2. **Idle readwrite connection semantics are unsafe/unclear.**
   - Current spec implies a long-lived writer worker owns a readwrite `sqlite3*`.
   - Risk: current code uses `PRAGMA locking_mode = EXCLUSIVE` and VFS lock callbacks are no-ops. An idle open writer connection plus reader connections may violate SQLite/VFS assumptions.
   - Needed decision: close writer handle between jobs when read pooling is enabled, make idle writer block readers, or implement a real lock ladder.

3. **Reader VFS reads must not see the dirty write buffer.**
   - Current spec says readers must not mutate the dirty buffer, but does not explicitly forbid reading it.
   - Risk: current VFS resolves `write_buffer.dirty` before committed cache pages. A reader overlapping a dirty writer path could observe uncommitted pages.
   - Needed decision: role-aware read resolution. Reader handles read committed state only; writer handles may read dirty buffer.

4. **VFS unregister after close timeout can be unsafe.**
   - Current spec says active jobs finish or hit a bounded close timeout, then workers close and VFS unregisters.
   - Risk: without `sqlite3_interrupt`, timed-out workers may still be inside SQLite/VFS. Dropping or unregistering the VFS would risk use-after-unregister.
   - Needed decision: close timeout may return an error, but the VFS must remain alive until every worker thread has actually exited, or the design must intentionally leak/retain the VFS and mark it dead.

5. **`SELECT` is not enough for reader routing.**
   - Current spec treats parser-proven `SELECT` as reader-candidate.
   - Risk: read-only SQL can be connection-affine. Examples: `last_insert_rowid()`, `changes()`, `total_changes()`, temp schema reads, or other session-state queries.
   - Needed decision: reject connection-affine functions and temp schema reads from the reader allowlist, or route all connection-affine/session-state SQL to writer.

## High

6. **VFS role tagging needs a concrete mechanism.**
   - Current spec says reader-owned and writer-owned file handles, but not how callbacks know the role.
   - Risk: role checks are aspirational unless `VfsFile` stores role and callbacks enforce it.
   - Needed decision: store role on `VfsFile` at `xOpen`, derived from open flags plus coordinator-issued capability/epoch, and check it in read/write/file-control/delete/truncate/sync paths.

7. **Aux/temp file behavior is underspecified.**
   - Current spec shares the aux-file registry and says reader aux creation should fail if it implies writes.
   - Risk: readonly SELECTs can still require temp storage. Shared aux files are mutable VFS state.
   - Needed decision: require `PRAGMA temp_store = MEMORY` on readers and fail all reader aux writes, or make aux files per-connection/private.

8. **Manual transaction routing wording can deadlock.**
   - Current spec says reads under writer pressure/manual transaction may “wait or route writer,” then later says manual transaction routes all work to writer.
   - Risk: `BEGIN; INSERT; SELECT; COMMIT` can block itself if the SELECT waits behind the transaction.
   - Needed decision: manual transaction pins all work to writer until autocommit returns. No waiting/read-lane routing inside the manual transaction.

9. **Queued-read behavior under writer pressure is too loose.**
   - Current spec says pending writer stops new reader admission.
   - Risk: reads already in `pending_reads` could still drain ahead of a writer.
   - Needed decision: once `pending_writer_count > 0`, no queued read dispatches to reader workers until the writer completes.

10. **Backpressure needs exact bounded-queue semantics.**
   - Current spec says bounded command queue and queue-full returns `actor.overloaded`.
   - Risk: awaiting channel capacity creates hidden backpressure instead of explicit overload.
   - Needed decision: public sends use `try_reserve`/`try_send`; internal pending queues and worker queues are bounded; full public or worker queues return `actor.overloaded`.

11. **Cancellation must update coordinator state.**
   - Current spec says queued requests with dropped receivers can be dropped before dispatch.
   - Risk: cancelling a queued writer may leave `pending_writer_count` elevated, freezing readers.
   - Needed decision: cancellation decrements writer/read counters, frees queue capacity, wakes coordinator, and recomputes writer pressure.

12. **Prepare errors must not become classifier mismatches.**
   - Current spec could classify all reader-worker classification disagreement as internal mismatch.
   - Risk: `sqlparser` can parse SQL that SQLite later rejects. That is a user SQL error, not an internal parser bug.
   - Needed decision: mismatch only when SQLite prepare succeeds but tail/readonly/authorizer says non-reader-eligible. Prepare errors remain normal SQLite/user errors.

13. **Writer-pressure routing needs exact state rules.**
   - Current spec says “wait or route writer according to current semantics.”
   - Risk: ambiguous ordering and hidden behavior changes.
   - Needed decision: specify behavior for pending writer, active writer, idle writer, and manual transaction separately.

14. **Migration from `NativeConnectionManager` needs a compatibility checklist.**
   - Current spec says keep old manager as fallback but not how `NativeDatabaseHandle` switches.
   - Risk: clone semantics, idempotent close, `take_last_kv_error`, preload hints, metrics, initialization, and test-only snapshots can regress.
   - Needed decision: require a backend enum/trait plus compatibility tests for old manager versus worker executor.

## Medium

15. **The `sqlparser` allowlist needs exact AST rules.**
   - Current spec says `Statement::Query`/SELECT in prose.
   - Risk: accepting broad query AST nodes admits unsupported or connection-affine SQL.
   - Needed decision: define exact accepted `sqlparser` AST variants, recursive checks for CTEs/subqueries/set ops/table factors/expressions, and route unknown AST nodes to writer.

16. **Mismatch/error metadata is too thin.**
   - Current spec metadata has only parser route, readonly, tail, and authorizer write flag.
   - Risk: hard to distinguish parser drift, user SQL error, VFS role bug, readonly open failure, query-only failure, or authorizer denial.
   - Needed decision: include prepare status, SQLite code/message class, denied authorizer action, failure phase, and route decision metadata.

17. **Rollout flag matrix is underspecified.**
   - Current spec adds `RIVETKIT_SQLITE_OPT_WORKER_EXECUTOR`, plus existing read-pool flags.
   - Risk: `worker_executor=1` with `read_pool_enabled=false` or `max_readers=0` could accidentally use legacy path or error.
   - Needed decision: define precedence. Likely: worker executor enabled with read pool disabled means writer-only worker executor.

18. **Default-on gate is too broad.**
   - Current spec says native driver tests, depot-client VFS tests, wasm/remote tests, parity and stress tests.
   - Risk: important failure modes are not named as release blockers.
   - Needed decision: add explicit gates for fault/chaos tests, close during active read/write, close with queued work, close timeout behavior, schema change plus reader reuse/recycle, worker panic/channel close, metrics parity, and wasm dependency checks.

19. **Worker thread implementation choice is still open.**
   - Current spec leaves OS threads versus stable `spawn_blocking` loops open.
   - Risk: cancellation, joining, panic handling, and runtime shutdown behavior differ.
   - Needed decision: pick one for v1 or define acceptance criteria for either.

20. **Read-only PRAGMAs need a later allowlist decision.**
   - Current spec routes PRAGMAs to writer in v1.
   - Risk: conservative but may leave performance on table.
   - Needed decision: keep writer-only in v1; add a follow-up only after parser/SQLite/connection-affinity behavior is tested.

21. **Cache invalidation on truncate needs explicit tests.**
   - Current spec asks whether `moka` invalidation on truncate is sufficient.
   - Risk: stale pages after truncate or file-size changes.
   - Needed decision: add truncate tests covering reader recycle and page-cache invalidation after writer work.

22. **Reader open must be proven network-free.**
   - Current design assumes fresh reader opens are cheap because they reuse the shared VFS and cache.
   - Risk: SQLite open/setup could accidentally trigger depot/envoy transport through page fetch, schema preload, or VFS bootstrap behavior.
   - Needed decision: add a debug/test-only VFS no-network guard around readonly reader open and setup. Any `get_pages` or commit transport during that region should panic in tests or return an internal assertion error in debug builds.
