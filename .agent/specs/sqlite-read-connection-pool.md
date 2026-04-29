# SQLite Read Connection Pool

## Goal

Allow independent read-only SQLite statements inside one Rivet Actor to run in parallel so VFS round trips can overlap. Keep writes, migrations, explicit transactions, sleep cleanup, and SQLite connection state deterministic.

This targets workloads like multiple expensive aggregates issued concurrently:

```ts
await Promise.all([
	c.db.execute("SELECT count(*) FROM events WHERE kind = ?", "click"),
	c.db.execute("SELECT avg(duration) FROM events WHERE kind = ?", "job"),
	c.db.execute("SELECT max(created_at) FROM events"),
]);
```

Today these calls serialize through the TypeScript database mutex and through the single native SQLite handle in `rivetkit-core`.

## Current Shape

- `rivetkit-typescript/packages/rivetkit/src/common/database/mod.ts` serializes public `c.db.execute(...)` calls with `AsyncMutex`.
- `rivetkit-typescript/packages/rivetkit/src/common/database/native-database.ts` serializes lower-level `exec/query/run` calls with `AsyncMutex`.
- `rivetkit-typescript/packages/rivetkit/src/db/drizzle.ts` also serializes Drizzle callback execution and raw `execute(...)` calls with `AsyncMutex`.
- `rivetkit-core/src/actor/sqlite.rs` stores one `NativeDatabaseHandle` behind `Arc<parking_lot::Mutex<Option<_>>>`.
- `rivetkit-sqlite/src/database.rs::open_database_from_envoy` registers one VFS named `envoy-sqlite-{actor_id}` and opens one `sqlite3*`.
- `rivetkit-sqlite/src/vfs.rs` owns VFS state: generation, head txid, page cache, protected cache, write buffer, recent page hints, aux files, and dead/fence state.
- `open_database(...)` configures `PRAGMA locking_mode = EXCLUSIVE`. That is compatible with a single writable connection, but not with any design that keeps readers open beside a writer.

## Design

Introduce one actor-local SQLite connection manager with two mutually exclusive modes:

```text
SqliteDb
  SqliteConnectionManager
    read mode: N read-only connections, single-statement reads
    write mode: exactly one read/write connection
    shared VFS registration/context/cache
    mode gate
```

Connection modes:

- **Read mode**: one or more read-only connections may be open. Only single prepared statements verified by SQLite as read-only run here.
- **Write mode**: exactly one writable connection is open. All mutations, migrations, explicit transactions, multi-statement `exec`, and fallback queries run here. No reader connection may be open while the writable connection is open.
- **Shared VFS**: all connections for one actor use the same registered VFS and same `VfsContext`, so page cache, read-ahead predictor, preload hints, aux files, generation fencing, and dead state stay actor-global.

The pool is implemented in Rust/core. TypeScript and NAPI call into one authoritative native routing layer instead of maintaining parallel SQL routing logic.

## Routing Rules

- `run(...)` always enters write mode.
- `exec(...)` enters write mode in v1. Multi-statement read-only routing can come later.
- single-statement `execute(...)` prepares/classifies once without stepping, then routes. Read-only statements use read mode. Non-read-only statements use write mode.
- `query(...)` and `run(...)` become compatibility wrappers around native `execute(...)` where possible. They must not be the policy boundary.
- Explicit transaction APIs, once exposed, hold write mode for the whole transaction.
- Raw transaction-control statements are write-mode only. They are never allowed on readers, even if SQLite reports them as read-only.
- Before entering write mode, the manager stops admitting new reads, waits for active readers to finish, closes all reader connections, then opens exactly one writable connection.
- While `sqlite3_get_autocommit(writer) == 0`, the manager remains in write mode. All DB operations route to the writable connection until autocommit becomes true again.
- After write-mode work completes and autocommit is true, close the writable connection before admitting read-mode work again.
- Migration mode routes all DB calls through write mode and prevents reader creation. This includes Drizzle's migration reads such as `SELECT created_at FROM __drizzle_migrations`.
- After migration mode or schema-changing writer work, the next read-mode connections must be fresh.
- Inspector database reads can use the read path. Inspector execute uses native `execute(...)` and must not bypass gates.

## Read-Only Enforcement

Do not classify SQL with string parsing.

Use SQLite enforcement in `rivetkit-sqlite`:

- prepare/classify without stepping
- reject non-whitespace tail text from `sqlite3_prepare_v2` for reader routing
- use `sqlite3_stmt_readonly(stmt)` as one check before reader routing
- use an authorizer during classification to collect transaction, attach, temp, schema, function, and write actions
- open reader connections with `SQLITE_OPEN_READONLY` against the shared VFS
- never use SQLite URI `immutable=1`
- set `PRAGMA query_only = ON` on reader connections
- install a mandatory SQLite authorizer on reader connections

The reader authorizer must deny:

- transaction control: `BEGIN`, `COMMIT`, `ROLLBACK`, `SAVEPOINT`, `RELEASE`, `ROLLBACK TO`
- `ATTACH` and `DETACH`
- schema writes and temp schema writes
- non-whitelisted `PRAGMA` statements
- non-whitelisted function calls if user-defined functions are ever exposed
- all write opcodes

For v1, if a statement cannot be confidently prepared and verified as one read-only statement, route it to the writer before stepping. Error only when the statement is explicitly disallowed, malformed, multi-statement where unsupported, denied by shutdown, or denied by fence state.

The classification path must be explicit. A viable v1 approach is:

1. Acquire a short classifier mutex.
2. Prepare on a classifier or temporary writable connection without stepping.
3. Check tail text, authorizer action log, transaction-control actions, and `sqlite3_stmt_readonly`.
4. Finalize the classifier statement.
5. Acquire the read or write permit.
6. Prepare and step on the selected execution connection.

## Scheduling

Use a writer-preferential mode gate in core:

- read-only query acquires a shared read-mode permit
- writer work requests write mode
- once a writer is waiting, new reads wait behind it so writes do not starve
- active reads are allowed to finish before the writer starts
- write-mode transition closes all readers before opening the writable connection
- the writable connection stays open only for the full writer statement, `exec`, migration phase, or explicit transaction API

The gate belongs in `rivetkit-core`, not TypeScript, so Rust, NAPI, inspector, and future runtimes share the same semantics.

TypeScript should remove or narrow the DB-level `AsyncMutex` only after native routing is authoritative. TS may still serialize conversion and close bookkeeping, but it must not serialize all read queries once the pool is active.

Manual raw transactions are a compatibility mode, not a new isolation guarantee. If user code starts `BEGIN` and then yields, the manager stays in write mode with exactly one writable connection until `COMMIT` or `ROLLBACK`. No reader connections may open during that window.

## Pool Sizing

Default policy:

- `max_readers = 4`
- `min_readers = 0`
- open readers lazily when concurrent read demand exists
- keep idle readers warm for 60 seconds
- close idle readers only, never active readers
- close all readers on actor sleep, destroy, actor lost-fence, and final database close
- close all readers before opening a writable connection
- close the writable connection before opening or reusing readers

Config lives in the central SQLite optimization flag/config path with these logical knobs:

- `sqlite_read_pool_enabled`
- `sqlite_read_pool_max_readers`
- `sqlite_read_pool_idle_ttl_ms`

## VFS Refactor

The current `NativeDatabase` owns both one `sqlite3*` and one `SqliteVfs`. That does not work for a pool because each extra connection would try to register the same VFS name or create duplicate VFS state.

Refactor to separate:

- `NativeVfsHandle`: owns VFS registration and shared `VfsContext`
- `NativeConnection`: owns one `sqlite3*`
- `NativeConnectionManager`: owns one `NativeVfsHandle`, either one writable `NativeConnection` or lazy reader `NativeConnection`s, and the mode gate

The VFS name should include the SQLite generation or another unique pool generation, not only actor id. This makes stale actor cleanup/name reuse failures fail visibly instead of colliding with the next actor generation.

Opening a connection uses the manager's registered VFS name. Dropping a connection closes only its `sqlite3*`. Dropping the manager first closes all SQLite connections, then unregisters the VFS.

The VFS must support read-only opens and role-aware file handles:

- store connection/file role on `VfsFile` and `AuxFileHandle`
- set `pOutFlags` accurately from open flags
- reject `xWrite`, `xTruncate`, `xDelete`, dirty-page sync, and atomic-write file controls from reader-owned handles
- deny reader aux-file creation in v1 unless a specific read-only SQLite path is proven safe
- deny `ATTACH`, temp tables, temp schema writes, and reader journal creation through authorizer and VFS role checks

The shared VFS state must distinguish committed state from writer-local state. Reader file handles read only committed pages for their statement. Writer dirty buffers, journal-like aux state, and atomic-write state are writer-owned until commit publishes them.

`PRAGMA locking_mode = EXCLUSIVE` may remain for write mode because the writable connection is the only open connection. Read mode should not set exclusive locking on reader connections.

Pooled mode must implement enough intra-actor SQLite lock state for multiple connections:

- concurrent SHARED reader locks are allowed
- writable RESERVED/PENDING/EXCLUSIVE locks are granted only in write mode after all readers are closed
- `xCheckReservedLock` reports real reserved-writer state
- VFS callbacks assert that write-only operations hold the write-mode permit

If a first implementation keeps SQLite locks as no-ops, the feature must remain disabled behind a test-only flag until VFS role assertions prove every SQLite entrypoint is gated correctly.

## Snapshot Semantics

Target v1 semantics:

- Parallel readers observe a stable SQLite snapshot for each statement.
- Write mode waits for active readers to finish and closes all readers before opening the writable connection.
- New readers wait behind a pending write-mode request.
- Readers are allowed to observe either the state before a waiting writer or the state after that writer completes. They must not observe a partially committed write.

Because write mode cannot start while reads run, the VFS `head_txid`, `db_size_pages`, write buffer, and page cache are stable for active readers. The writable connection updates VFS meta and cache only while holding the write-mode permit.

Schema changes are broader than migrations. Because readers are closed before every write mode, any read connections after schema-changing work are fresh.

Future optimization can allow reads to continue during writes by pinning per-reader head txids, but v1 should not attempt that.

## TypeScript And Migration Integration

The TypeScript work is required for the feature to have any effect.

- `common/database/mod.ts` raw `db().execute(...)` must stop using a per-query mutex once native routing is authoritative.
- `common/database/native-database.ts::wrapJsNativeDatabase` must stop serializing all `query(...)` calls.
- `db/drizzle.ts` must stop serializing all Drizzle callback reads and raw `execute(...)` calls.
- These wrappers should keep closed-state checks with an in-flight counter or close gate. `close()` stops admission and waits for in-flight native calls before closing.
- Drizzle and raw DB migration hooks run in native migration mode, which routes every DB call through write mode and prevents reader creation.
- TS string heuristics such as `sqlReturnsRows(...)` and `hasMultipleStatements(...)` should be reduced to compatibility fallbacks. Add a native `execute(...)` API that returns `{ columns, rows, changes, routedAs }` for single statements so TS does not decide read/write behavior by string.
- Core and TS inspector database execute endpoints should both use the native `execute(...)` path.

## Error Behavior

- If classification finds a statement is not read-only, route through write mode before stepping. Do not step it on a reader.
- If a reader authorizer or VFS role check rejects a statement that should have used write mode, treat that as a routing bug, fail closed, and increment a metric.
- If any connection sees `SqliteFenceMismatch`, mark the shared VFS dead and close all idle readers. Future operations fail closed.
- If close/sleep/destroy begins, enter manager closing state, stop admitting new work, wait for active connection jobs to finish or observe the existing shutdown cancellation path, close SQLite handles, then unregister/free the VFS.
- The VFS context must be refcounted so active `VfsFile`s keep it alive until `xClose`.
- If a reader idle close fails, log with tracing and mark the connection unusable.

## Metrics

Add core Prometheus metrics for pool internals:

- active reader count
- idle reader count
- read pool wait duration
- write-mode wait duration
- routed read-only queries
- write-mode fallback queries
- manual transaction mode count/duration
- reader open/close counts
- reader rejected mutation count
- read-to-write mode transition count/duration
- write-to-read mode transition count/duration

Existing VFS metrics should continue to aggregate at the shared VFS level.

TS `trackSql(...)` remains query-duration logging and should not duplicate pool internals.

## Implementation Plan

1. Add `sqlite3_stmt_readonly` support in `rivetkit-sqlite::query`.
2. Add single-statement prepare-tail validation and classification authorizer support.
3. Split `NativeDatabase` into VFS ownership and connection ownership.
4. Add VFS role-aware file handles and reader write rejection.
5. Replace exclusive locking with pooled-mode lock behavior.
6. Add `NativeConnectionManager` with read/write modes, lazy reader connections, idle TTL, closing state, and close ordering.
7. Replace `SqliteDb.db: Arc<Mutex<Option<NativeDatabaseHandle>>>` with the pool handle.
8. Implement writer-preferential async gate in core. Avoid holding sync locks across awaits.
9. Implement manual transaction mode based on `sqlite3_get_autocommit(writer)`.
10. Route native `execute(...)` read-only statements to reader pool. Keep `run` and `exec` write-mode only in v1 compatibility paths.
11. Update raw TS DB, Drizzle, native wrapper, and inspector execute to use native routing without serializing all reads.
12. Add metrics and config flags. Default the feature off until stress tests pass, then default on.

## Test Plan

Rust unit and integration tests:

- `sqlite3_stmt_readonly` classification for `SELECT`, read-only `PRAGMA`, mutating `PRAGMA`, `INSERT ... RETURNING`, CTE writes, `VACUUM`, `ATTACH`, and multi-statement SQL.
- prepare-tail rejection for `SELECT 1; INSERT ...` on the reader path.
- transaction-control statements never route to readers: `BEGIN`, `COMMIT`, `ROLLBACK`, `SAVEPOINT`, `RELEASE`, and `ROLLBACK TO`.
- reader authorizer denies attach, detach, temp writes, schema writes, unsafe pragmas, and unsafe functions.
- reader VFS handles reject `xWrite`, `xTruncate`, `xDelete`, dirty-page sync, and atomic-write controls.
- VFS lock transitions allow concurrent SHARED readers and protect write-mode RESERVED/EXCLUSIVE locks.
- Multiple concurrent read queries use multiple reader connections and complete faster than serialized reads with an artificial VFS delay.
- Write mode waits for active readers and new readers wait behind a pending write-mode request.
- Manual `BEGIN` puts the manager in write mode. All later operations route to the writable connection until autocommit is restored.
- Migrations run in write mode. Reader schema cache is refreshed after migration.
- DDL runs with no reader connections open. Later reads use fresh reader connections.
- Reader pool closes idle readers after TTL and never closes active readers.
- Sleep/destroy closes readers or the writable connection in deterministic order.
- Fence mismatch from any reader kills the shared VFS state.
- VFS page cache is shared across read mode and write mode.
- Active VFS files keep the context alive until close even during pool shutdown.

TypeScript driver tests:

- `Promise.all` of read-only `c.db.execute(...)` calls overlaps in wall time with an injected VFS delay.
- Drizzle parallel read callbacks overlap in wall time with an injected VFS delay.
- Concurrent read and write preserves write ordering and does not throw random busy errors.
- Explicit `BEGIN` / `COMMIT` sequences remain exclusive on the writer.
- `onMigrate` and Drizzle migrations do not open readers before migration completes.
- Inspector execute handles `SELECT`, `INSERT RETURNING`, plain `INSERT`, mutating `PRAGMA`, and rejected multi-statement SQL through native routing.
- Background timers using `c.db` after close still get the existing closed database error.

Stress tests:

- concurrent read aggregates while queued writes run
- concurrent inspector reads while user reads and writes run
- actor sleep/wake churn with active readers
- lost-fence / actor replacement during active reads

## Non-Goals

- Parallel writes.
- Parallel stepping on one SQLite connection.
- Multi-statement read-only routing in v1.
- Read/write overlap with pinned historical snapshots in v1.
- SQL string parsing as the authority for read-only classification.
- Solving cross-action interleaving inside user-managed raw transactions beyond preserving today's single-writer behavior.

## Open Questions

- Should `max_readers` default to 2 instead of 4 for actor density?
- Do reader connections need a smaller SQLite pager cache because the VFS cache is shared underneath?
- Should the classifier use a dedicated temporary connection, a selected reader, or a lightweight parser plus reader prepare with distinguishable authorizer failures?
