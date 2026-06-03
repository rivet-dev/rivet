# SQLite Parallel Read Workers

## Status

Deferred. The active design is `.agent/specs/sqlite-channel-worker-executor.md`, which uses one worker-owned readwrite SQLite connection and no parallel readers.

## Why Deferred

Parallel reader workers looked attractive, but adversarial review found enough unresolved correctness hazards that they should not be part of the first worker-executor refactor.

## Design Notes To Preserve

1. **Reader handles likely need recycling after every writer commit.**
   - SQLite readonly connections have their own pager/schema caches.
   - Sharing the VFS `moka` page cache does not prove reused reader handles observe committed writer changes.

2. **Idle readwrite connection semantics need a real decision.**
   - Current code uses `PRAGMA locking_mode = EXCLUSIVE`.
   - Current VFS lock callbacks are no-ops for a single-connection world.
   - A long-lived idle writer plus readonly readers may violate those assumptions.

3. **Reader VFS reads must not see writer dirty pages.**
   - Current VFS resolves `write_buffer.dirty` before committed cache pages.
   - Reader-owned handles would need role-aware committed-only read resolution.

4. **VFS role tagging must be concrete.**
   - `VfsFile` would need a role stored at `xOpen`.
   - Role should be derived from open flags plus a coordinator-issued capability or epoch.
   - Every mutating callback and relevant read path must check the role.

5. **Aux/temp files are a shared mutable surface.**
   - Readers should probably use `PRAGMA temp_store = MEMORY`.
   - Reader aux writes should fail closed, or aux files should become per-connection/private.

6. **Reader open must be proven network-free.**
   - Fresh reader opens should reuse the shared VFS and cache.
   - Add a debug/test-only VFS no-network guard around readonly reader open/setup before revisiting this.

7. **Parser admission is tricky.**
   - `sqlparser` with `SQLiteDialect` is acceptable only as a strict opt-in scheduler filter.
   - `SELECT` is not enough because connection-affine reads exist.
   - Examples that should not go to reader workers without special handling: `last_insert_rowid()`, `changes()`, `total_changes()`, temp schema reads, and session-state queries.

8. **Classifier mismatch should not reroute.**
   - If a strict parser admits a reader candidate and SQLite classification proves it is write-required, that is an internal classifier bug.
   - SQLite prepare errors are user/query errors, not classifier mismatches.

9. **Manual transactions pin all work to writer.**
   - `BEGIN; INSERT; SELECT; COMMIT` must not block itself behind a read lane.

10. **Queued read behavior under writer pressure must be exact.**
    - Once a writer is pending, no queued read should dispatch to readers until the writer completes.

11. **Shutdown cannot unregister VFS under active workers.**
    - Close timeout may report an error, but VFS must remain alive while any worker may still be inside SQLite/VFS.

12. **Backpressure and cancellation need exact accounting.**
    - Sends should use non-awaiting bounded queue operations.
    - Cancelling queued writers must decrement writer pressure and wake scheduling.

## Revisit Criteria

Only revisit parallel readers after the single-worker executor is stable and measured.

Required before implementation:

- Benchmark single-worker throughput and identify read parallelism as a real bottleneck.
- Prove fresh reader open/setup is network-free with a VFS no-network assertion.
- Define exact VFS role model.
- Define exact parser allowlist and connection-affinity denylist.
- Add tests for reader recycling after writer commits.
- Add tests for temp files, truncate, schema changes, close timeout, and writer pressure.
