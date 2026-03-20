# Dynamic Actor SQLite Proxy Spec

## Problem

Dynamic actors run in sandboxed `secure-exec` / `isolated-vm` processes. The current SQLite path requires `@rivetkit/sqlite` WASM to load inside the isolate, which isn't set up and is the wrong direction — we plan to add a native SQLite extension on the host side. Dynamic actors need a way to use `db()` and `db()` from `rivetkit/db` and `rivetkit/db/drizzle` without running WASM in the isolate.

## Approach

Run SQLite on the **host side** and bridge a thin `execute(sql, params) → rows` RPC from isolate → host. The `ActorDriver` already has `overrideRawDatabaseClient()` and `overrideDrizzleDatabaseClient()` hooks designed for this exact purpose. The `DatabaseProvider.createClient()` already checks for overrides before falling back to KV-backed construction.

```
Isolate                              Host
──────                              ────
db.execute(sql, args)  ──bridge──►  host SQLite (per-actor)
                       ◄──────────  { rows, columns }
```

One bridge call per query instead of per KV page.

## Architecture

### Host side (manager process)

Each actor gets a dedicated SQLite database file managed by the host. For the file-system driver, this is already done for KV via `#actorKvDatabases` in `FileSystemGlobalState`. The actor's **application database** is a separate SQLite file alongside the KV database.

The host exposes two bridge callbacks to the isolate:

1. **`sqliteExec(actorId, sql, params) → string`** — Executes a SQL statement. Returns JSON-encoded `{ rows: unknown[][], columns: string[] }`. Handles both read and write queries. Params are JSON-serialized across the boundary.

2. **`sqliteBatch(actorId, statements) → string`** — Executes multiple SQL statements in a single bridge call, wrapped in a transaction. Each statement is `{ sql: string, params: unknown[] }`. Returns JSON-encoded array of `{ rows, columns }` per statement. This is critical for migrations and reduces bridge round-trips.

### Isolate side (dynamic actor process)

The isolate-side `actorDriver` (defined in `host-runtime.ts` line 1767) gains:

- `overrideRawDatabaseClient(actorId)` — Returns a `RawDatabaseClient` whose `exec()` method calls through the bridge to `sqliteExec`.
- `overrideDrizzleDatabaseClient(actorId)` — Returns a drizzle `sqlite-proxy` instance whose async callback calls through the bridge to `sqliteExec`.

Because the overrides are set, `DatabaseProvider.createClient()` in both `db/mod.ts` and `db/drizzle/mod.ts` will use them instead of trying to construct a KV-backed WASM SQLite. No `createSqliteVfs()` is needed in the dynamic actor driver.

## Detailed Changes

### 1. Bridge contract (`src/dynamic/runtime-bridge.ts`)

Add new bridge global keys:

```typescript
export const DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS = {
  // ... existing keys ...
  sqliteExec: "__rivetkitDynamicHostSqliteExec",
  sqliteBatch: "__rivetkitDynamicHostSqliteBatch",
} as const;
```

### 2. Host-side SQLite pool (`src/drivers/file-system/global-state.ts`)

Add a **per-actor application database** map alongside the existing KV database map:

```typescript
#actorAppDatabases = new Map<string, SqliteRuntimeDatabase>();
```

Add methods:

```typescript
#getOrCreateActorAppDatabase(actorId: string): SqliteRuntimeDatabase
// Opens/creates a SQLite database file at: <storagePath>/app-databases/<actorId>.db
// Separate from the KV database. Enables WAL mode for concurrency.

#closeActorAppDatabase(actorId: string): void
// Called during actor teardown, alongside #closeActorKvDatabase.

sqliteExec(actorId: string, sql: string, params: unknown[]): { rows: unknown[][], columns: string[] }
// Runs a single statement against the actor's app database.
// Uses the same SqliteRuntime (bun:sqlite / better-sqlite3) already loaded.
// Synchronous — native SQLite is sync, the bridge async wrapper handles the rest.

sqliteBatch(actorId: string, statements: { sql: string, params: unknown[] }[]): { rows: unknown[][], columns: string[] }[]
// Wraps all statements in BEGIN/COMMIT. Returns results per statement.
```

Cleanup: extend `#destroyActorData` and actor teardown to also close and delete app databases.

### 3. Host bridge wiring — `isolated-vm` path (`src/dynamic/isolate-runtime.ts`)

In `#setIsolateBridge()` (around line 880), add refs for the new bridge callbacks:

```typescript
const sqliteExecRef = makeRef(
  async (actorId: string, sql: string, paramsJson: string): Promise<{ copy(): string }> => {
    const params = JSON.parse(paramsJson);
    const result = this.#config.globalState.sqliteExec(actorId, sql, params);
    return makeExternalCopy(JSON.stringify(result));
  },
);

const sqliteBatchRef = makeRef(
  async (actorId: string, statementsJson: string): Promise<{ copy(): string }> => {
    const statements = JSON.parse(statementsJson);
    const results = this.#config.globalState.sqliteBatch(actorId, statements);
    return makeExternalCopy(JSON.stringify(results));
  },
);

await context.global.set(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.sqliteExec, sqliteExecRef);
await context.global.set(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.sqliteBatch, sqliteBatchRef);
```

### 4. Host bridge wiring — `secure-exec` path (`src/dynamic/host-runtime.ts`)

In `#setIsolateBridge()` (around line 586), add the same refs using the same base64/JSON bridge pattern already used for KV:

```typescript
const sqliteExecRef = makeRef(
  async (actorId: string, sql: string, paramsJson: string): Promise<string> => {
    const params = JSON.parse(paramsJson);
    const result = this.#config.globalState.sqliteExec(actorId, sql, params);
    return JSON.stringify(result);
  },
);
// ... same for sqliteBatch

await context.global.set("__dynamicHostSqliteExec", sqliteExecRef);
await context.global.set("__dynamicHostSqliteBatch", sqliteBatchRef);
```

And on the isolate-side `actorDriver` object (line 1767), add:

```typescript
const actorDriver = {
  // ... existing methods ...

  async overrideRawDatabaseClient(actorIdValue) {
    return {
      exec: async (query, ...args) => {
        const resultJson = await bridgeCall(
          globalThis.__dynamicHostSqliteExec,
          [actorIdValue, query, JSON.stringify(args)]
        );
        const { rows, columns } = JSON.parse(resultJson);
        return rows.map((row) => {
          const obj = {};
          for (let i = 0; i < columns.length; i++) {
            obj[columns[i]] = row[i];
          }
          return obj;
        });
      },
    };
  },

  async overrideDrizzleDatabaseClient(actorIdValue) {
    // Return undefined — let the raw override handle it.
    // Drizzle provider will fall back to using the raw override path.
    return undefined;
  },
};
```

### 5. Drizzle support

The drizzle `DatabaseProvider` in `db/drizzle/mod.ts` currently does NOT check for overrides — it always constructs a KV-backed WASM database. This needs to change.

Add an override check at the top of `createClient`:

```typescript
createClient: async (ctx) => {
  // Check for drizzle override first
  if (ctx.overrideDrizzleDatabaseClient) {
    const override = await ctx.overrideDrizzleDatabaseClient();
    if (override) {
      // Wrap with RawAccess execute/close methods and return
      return Object.assign(override, {
        execute: async (query, ...args) => { /* delegate to override */ },
        close: async () => {},
      });
    }
  }

  // Check for raw override — build drizzle sqlite-proxy on top of it
  if (ctx.overrideRawDatabaseClient) {
    const rawOverride = await ctx.overrideRawDatabaseClient();
    if (rawOverride) {
      const callback = async (sql, params, method) => {
        const rows = await rawOverride.exec(sql, ...params);
        if (method === "run") return { rows: [] };
        if (method === "get") return { rows: rows[0] ? Object.values(rows[0]) : undefined };
        return { rows: rows.map(r => Object.values(r)) };
      };
      const client = proxyDrizzle(callback, config);
      return Object.assign(client, {
        execute: async (query, ...args) => rawOverride.exec(query, ...args),
        close: async () => {},
      });
    }
  }

  // Existing KV-backed path...
}
```

This lets dynamic actors use `db()` from `rivetkit/db/drizzle` with migrations working through the bridge. The host runs the actual SQL; the isolate just sends strings.

### 6. Migrations

Drizzle inline migrations (`runInlineMigrations`) currently operate on the `@rivetkit/sqlite` `Database` WASM instance directly. For the proxy path, migrations need to run through the same `execute()` bridge.

Option A (simpler): The raw override's `exec()` already supports multi-statement SQL via the host's `db.exec()`. Migrations can use `execute()` directly. The `sqliteBatch` bridge method handles transactional migration application.

Option B: Add a dedicated `sqliteMigrate(actorId, migrationSql[])` bridge call that runs all migrations in a single transaction on the host. Cleaner but more surface area.

**Recommendation**: Option A. The `execute()` path is sufficient. The drizzle provider's `onMigrate` can call `client.execute(migrationSql)` for each pending migration, same as it does today but through the bridge.

### 7. Engine driver (`src/drivers/engine/actor-driver.ts`)

The engine driver manages dynamic actors the same way. It needs the same `sqliteExec` / `sqliteBatch` bridge wiring, backed by whatever storage the engine provides for actor application databases.

For now, this can be deferred — the engine driver can continue using the KV-backed path for static actors and throw a clear error for dynamic actors that try to use `db()` until the engine-side SQLite proxy is implemented.

## Data model

Each dynamic actor gets TWO SQLite databases on the host:

| Database | Purpose | Path | Managed by |
|----------|---------|------|------------|
| KV database | Actor KV state (`kvBatchPut`/`kvBatchGet`) | `<storage>/databases/<actorId>.db` | Existing `#actorKvDatabases` |
| App database | User-defined schema via `db()` / drizzle | `<storage>/app-databases/<actorId>.db` | New `#actorAppDatabases` |

On actor destroy, both databases are deleted. On actor sleep, both databases are closed (and reopened on wake).

## Serialization format

All data crosses the bridge as JSON strings:

- **Params**: `JSON.stringify(args)` — supports `null`, `number`, `string`, `boolean`. Binary (`Uint8Array`) params are base64-encoded.
- **Results**: `JSON.stringify({ rows: unknown[][], columns: string[] })` — column-oriented format, same as `@rivetkit/sqlite`'s `query()` return shape.
- **Batch**: Array of the above per statement.

## Error handling

- SQL errors on the host throw through the bridge. The isolate receives the error message and stack trace as a rejected promise.
- If the actor's app database doesn't exist yet, `sqliteExec` creates it on first use (same lazy-open pattern as KV databases).
- Invalid SQL, constraint violations, etc. surface as normal SQLite errors to the actor code.

## Testing

Add a driver test in `src/driver-test-suite/tests/` that:

1. Creates a dynamic actor that uses `db()` (raw) with a simple schema
2. Runs migrations, inserts rows, queries them back
3. Verifies data persists across actor sleep/wake cycles
4. Creates a dynamic actor that uses `db()` from `rivetkit/db/drizzle` with schema + migrations
5. Verifies drizzle queries work through the proxy

Add corresponding fixture actors in `fixtures/driver-test-suite/`.

## Files to modify

| File | Change |
|------|--------|
| `src/dynamic/runtime-bridge.ts` | Add `sqliteExec`, `sqliteBatch` bridge keys |
| `src/drivers/file-system/global-state.ts` | Add `#actorAppDatabases`, `sqliteExec()`, `sqliteBatch()`, cleanup |
| `src/dynamic/isolate-runtime.ts` | Wire `sqliteExec`/`sqliteBatch` refs in `#setIsolateBridge()` |
| `src/dynamic/host-runtime.ts` | Wire bridge refs + add `overrideRawDatabaseClient` to isolate-side `actorDriver` |
| `src/db/drizzle/mod.ts` | Add override check at top of `createClient` |
| `src/driver-test-suite/tests/` | New test file for dynamic SQLite proxy |
| `fixtures/driver-test-suite/` | New fixture actors using `db()` in dynamic actors |
| `docs-internal/rivetkit-typescript/DYNAMIC_ACTORS_ARCHITECTURE.md` | Document SQLite proxy bridge |

## Non-goals

- Running WASM SQLite inside the isolate.
- Implementing this for the engine driver (deferred until engine-side app database support exists).
- Shared/cross-actor databases.
- Direct filesystem access from the isolate.
