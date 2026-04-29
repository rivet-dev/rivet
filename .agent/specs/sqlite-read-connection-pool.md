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
- `rivetkit-core/src/actor/sqlite.rs` stores one `NativeDatabaseHandle` behind `Arc<parking_lot::Mutex<Option<_>>>`.
- `rivetkit-sqlite/src/database.rs::open_database_from_envoy` registers one VFS named `envoy-sqlite-{actor_id}` and opens one `sqlite3*`.
- `rivetkit-sqlite/src/vfs.rs` owns VFS state: generation, head txid, page cache, protected cache, write buffer, recent page hints, aux files, and dead/fence state.
- `open_database(...)` configures `PRAGMA locking_mode = EXCLUSIVE`, which is incompatible with a pooled multi-connection design unless changed or scoped out of pool mode.

## Design

Introduce one actor-local SQLite pool:

```text
SqliteDb
  SqliteConnectionPool
    writer connection: read/write, migrations, transactions, mutations
    reader connections: read-only, single-statement reads
    shared VFS registration/context/cache
    fair async read/write gate
```

Connection roles:

- **Writer connection**: read/write. All mutations, migrations, explicit transactions, multi-statement `exec`, and fallback queries run here.
- **Reader connections**: read-only. Only single prepared statements verified by SQLite as read-only run here.
- **Shared VFS**: all connections for one actor use the same registered VFS and same `VfsContext`, so page cache, read-ahead predictor, preload hints, aux files, generation fencing, and dead state stay actor-global.

## Routing Rules

- `run(...)` always uses the writer under an exclusive permit.
- `exec(...)` uses the writer under an exclusive permit in v1. Multi-statement read-only routing can come later.
- `query(...)` prepares the statement and asks SQLite whether it is read-only. Read-only statements use a reader under a shared permit. Non-read-only statements use the writer under an exclusive permit.
- Explicit transaction APIs, once exposed, always reserve the writer exclusively for the whole transaction.
- Migrations run before opening the reader pool, or close/reopen all readers after migrations complete.
- Inspector database reads can use the read path. Inspector execute uses normal routing and must not bypass gates.

## Read-Only Enforcement

Do not classify SQL with string parsing.

Use SQLite enforcement in `rivetkit-sqlite`:

- prepare the statement on the target connection
- use `sqlite3_stmt_readonly(stmt)` before stepping
- open reader connections with read-only flags once the VFS supports it
- set `PRAGMA query_only = ON` on reader connections
- add a SQLite authorizer for reader connections if needed to deny surprising side effects such as `ATTACH`, temp schema writes, or user-function writes

For v1, if a statement cannot be confidently prepared and verified as one read-only statement, route it to the writer.

## Scheduling

Use a writer-preferential async read/write gate in core:

- read-only query acquires a shared read permit
- writer work acquires an exclusive write permit
- once a writer is waiting, new reads wait behind it so writes do not starve
- active reads are allowed to finish before the writer starts

The gate belongs in `rivetkit-core`, not TypeScript, so Rust, NAPI, inspector, and future runtimes share the same semantics.

TypeScript should remove or narrow the DB-level `AsyncMutex` only after native routing is authoritative. TS may still serialize conversion and close bookkeeping, but it must not serialize all read queries once the pool is active.

## Pool Sizing

Default policy:

- `max_readers = 4`
- `min_readers = 0`
- open readers lazily when concurrent read demand exists
- keep idle readers warm for 60 seconds
- close idle readers only, never active readers
- close all readers on actor sleep, destroy, actor lost-fence, and final database close

Make the cap configurable through the central SQLite optimization/config path, not scattered environment reads.

## VFS Refactor

The current `NativeDatabase` owns both one `sqlite3*` and one `SqliteVfs`. That does not work for a pool because each extra connection would try to register the same VFS name or create duplicate VFS state.

Refactor to separate:

- `NativeVfsHandle`: owns VFS registration and shared `VfsContext`
- `NativeConnection`: owns one `sqlite3*`
- `NativeDatabasePool`: owns one `NativeVfsHandle`, one writer `NativeConnection`, lazy reader `NativeConnection`s, and the gate

Opening a connection uses the existing registered VFS name. Dropping a connection closes only its `sqlite3*`. Dropping the pool first closes all SQLite connections, then unregisters the VFS.

The VFS must support read-only opens. Read-only connections must not create or mutate aux files except for state SQLite itself requires for safe read-only operation. If SQLite attempts a write through a reader, fail closed with a structured SQLite runtime error.

`PRAGMA locking_mode = EXCLUSIVE` must be revisited. Pooled mode should either use `NORMAL` locking or apply exclusive locking only when the pool is disabled. Reader connections cannot coexist with an exclusive writer connection that permanently owns the database lock.

## Snapshot Semantics

Target v1 semantics:

- Parallel readers observe a stable SQLite snapshot for each statement.
- A writer waits for active readers before committing mutations.
- New readers wait behind a pending writer.
- Readers are allowed to observe either the state before a waiting writer or the state after that writer completes. They must not observe a partially committed write.

Because the write gate blocks writer work while reads run, the VFS `head_txid` and page cache are stable for active readers. The writer updates VFS meta and cache only while holding the exclusive permit.

Future optimization can allow reads to continue during writes by pinning per-reader head txids, but v1 should not attempt that.

## Error Behavior

- If a read-routed statement turns out not to be read-only, return a structured SQLite runtime error or reroute before stepping. Do not step it on a reader.
- If any connection sees `SqliteFenceMismatch`, mark the shared VFS dead and close all idle readers. Future operations fail closed.
- If close/sleep/destroy begins, stop admitting new reads and wait for active reads to finish or observe the existing shutdown cancellation path.
- If a reader idle close fails, log with tracing and mark the connection unusable.

## Metrics

Add actor metrics:

- active reader count
- idle reader count
- read pool wait duration
- write gate wait duration
- routed read-only queries
- writer fallback queries
- reader open/close counts
- reader rejected mutation count

Existing VFS metrics should continue to aggregate at the shared VFS level.

## Implementation Plan

1. Add `sqlite3_stmt_readonly` support in `rivetkit-sqlite::query`.
2. Split `NativeDatabase` into VFS ownership and connection ownership.
3. Add `NativeDatabasePool` with writer connection, lazy reader connections, idle TTL, and close ordering.
4. Replace `SqliteDb.db: Arc<Mutex<Option<NativeDatabaseHandle>>>` with the pool handle.
5. Implement writer-preferential async gate in core. Avoid holding sync locks across awaits.
6. Route `query` read-only statements to reader pool. Keep `run` and `exec` writer-only in v1.
7. Remove read serialization from the TS wrappers once native routing is covered.
8. Add metrics and config flags. Default the feature off until stress tests pass, then default on.

## Test Plan

Rust unit and integration tests:

- `sqlite3_stmt_readonly` classification for `SELECT`, read-only `PRAGMA`, mutating `PRAGMA`, `INSERT ... RETURNING`, CTE writes, `VACUUM`, `ATTACH`, and multi-statement SQL.
- Multiple concurrent read queries use multiple reader connections and complete faster than serialized reads with an artificial VFS delay.
- Writer waits for active readers and new readers wait behind a pending writer.
- Migrations run before readers open. Reader schema cache is refreshed after migration.
- Reader pool closes idle readers after TTL and never closes active readers.
- Sleep/destroy closes readers and writer in deterministic order.
- Fence mismatch from any reader kills the shared VFS state.
- VFS page cache is shared across writer and readers.

TypeScript driver tests:

- `Promise.all` of read-only `c.db.execute(...)` calls overlaps in wall time with an injected VFS delay.
- Concurrent read and write preserves write ordering and does not throw random busy errors.
- Explicit `BEGIN` / `COMMIT` sequences remain exclusive on the writer.
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

## Open Questions

- Should the writer connection also serve read-only queries when no reader is idle, or should all read-only work go to reader connections once the pool exists?
- Should `max_readers` default to 2 instead of 4 for actor density?
- Do reader connections need a smaller SQLite pager cache because the VFS cache is shared underneath?
- Is an SQLite authorizer required for v1, or are read-only open flags plus `query_only` plus `sqlite3_stmt_readonly` sufficient?
