# Spec: Move SQLite Runtime Into rivetkit-rust

## Goal

Move `sqlite-native` from `rivetkit-typescript/packages/` to `rivetkit-rust/packages/rivetkit-sqlite/`, rename to `rivetkit-sqlite`, and absorb SQLite query execution into it so the Rust `rivetkit` crate can run actors with SQLite without depending on NAPI. `rivetkit-sqlite` is the actor-side counterpart to engine-side `sqlite-storage`.

## Current State

```
rivetkit-typescript/packages/
├── sqlite-native/            ← Pure Rust crate, no NAPI deps. VFS + KV trait. (currently named rivetkit-sqlite-native)
├── rivetkit-napi/
│   ├── database.rs         ← ~300 lines pure FFI (exec/query/run) + ~250 lines NAPI wrappers
│   ├── sqlite_db.rs        ← Thin NAPI cache wrapper
│   └── envoy_handle.rs     ← Holds sqlite startup data
└── rivetkit/
    └── src/registry/native.ts  ← TS database wiring, drizzle, migrations
```

**Problem:** sqlite-native is pure Rust with zero NAPI dependencies but lives under the TypeScript package tree. The query execution layer (bind params, exec, query, run) is pure C FFI in rivetkit-napi that any Rust runtime could use. rivetkit-core has stub `db_exec`/`db_query`/`db_run` methods that unconditionally error.

## Target State

```
rivetkit-rust/packages/
├── rivetkit-sqlite/          ← Moved here. Same crate, new home.
├── rivetkit-core/
│   └── src/sqlite.rs       ← Owns: VFS lifecycle, query execution, open/close
├── rivetkit/               ← Typed Rust API for actors with SQLite
└── (no new crates needed)

rivetkit-typescript/packages/
├── rivetkit-napi/
│   └── database.rs         ← Thin NAPI wrapper delegating to rivetkit-core
└── rivetkit/
    └── src/registry/native.ts  ← Drizzle, migrations, user-facing config (unchanged)
```

## What Moves

### 1. sqlite-native → `rivetkit-rust/packages/rivetkit-sqlite/`

Physical move + rename from `rivetkit-sqlite-native` to `rivetkit-sqlite`. Update:
- `Cargo.toml` workspace path (`workspace = "../../../"` → appropriate relative path)
- Root `Cargo.toml` workspace members list
- rivetkit-napi `Cargo.toml` dependency path
- CLAUDE.md references

**Current structure (unchanged):**
- `kv.rs` — KV key layout constants (CHUNK_SIZE, key construction)
- `sqlite_kv.rs` — `SqliteKv` async trait (batch_get/put/delete, delete_range, on_open/close/error)
- `vfs.rs` — V1 VFS (KvVfs, NativeDatabase, open_database)
- `v2/vfs.rs` — V2 VFS (SqliteVfsV2, NativeDatabaseV2, commit buffering, prefetch)

**Dependencies:** `libsqlite3-sys` (bundled), `rivet-envoy-client`, `rivet-envoy-protocol`, `sqlite-storage`, `tokio`, `moka`, `parking_lot`, `async-trait`, `tracing`. All pure Rust.

### 2. Query execution FFI → `rivetkit-core` or `rivetkit-sqlite`

Move these pure Rust functions out of rivetkit-napi `database.rs`:
- `bind_params()` — Bind typed params to sqlite3_stmt
- `collect_columns()` — Extract column names from result set
- `column_value()` — Read typed column values (NULL/INTEGER/FLOAT/TEXT/BLOB)
- `execute_statement()` — Prepare, bind, step, finalize (INSERT/UPDATE/DELETE)
- `query_statement()` — Prepare, bind, step, collect rows
- `exec_statements()` — Multi-statement execution
- `sqlite_error()` — Error message extraction

~300 lines. Zero NAPI deps. Define a `BindParam` enum and `QueryResult` struct in the Rust crate.

### 3. Database lifecycle → rivetkit-core `sqlite.rs`

Expand `SqliteDb` to own the actual database handle:
- `open()` — Dispatch on schema_version (v1 KvVfs vs v2 SqliteVfsV2), open database
- `exec()` / `query()` / `run()` — Execute SQL via the FFI functions above
- `close()` — Close database handle
- `take_last_kv_error()` — Surface VFS errors
- `metrics()` — V2 VFS metrics

Wire `ActorContext::db_exec`/`db_query`/`db_run` to delegate to `SqliteDb` instead of erroring.

### 4. EnvoyKv adapter → `rivetkit-sqlite` or rivetkit-core

The `EnvoyKv` impl (routes SqliteKv trait methods to EnvoyHandle) is pure Rust. Move alongside the VFS.

### 5. NativeDatabaseHandle enum → `rivetkit-sqlite`

Wraps V1/V2 VFS handles. Pure Rust dispatch. Move alongside the VFS.

## What Stays

### rivetkit-napi (NAPI-only wrappers)

- `JsNativeDatabase` — `#[napi]` class wrapping rivetkit-core's database handle
- `JsBindParam` / `ExecuteResult` / `QueryResult` — `#[napi(object)]` for JS marshaling
- `spawn_blocking` wrappers — Offload FFI to tokio thread pool
- `open_database_from_envoy()` — `#[napi]` entry point
- `BridgeCallbacks` — ThreadsafeFunction dispatch for startup data

### TypeScript (user-facing)

- Drizzle ORM integration (type narrowing, schema introspection)
- `DatabaseProvider` abstraction (user-defined providers)
- Zod validation for database config
- Parameter binding transformation (named/positional normalization)
- AsyncMutex query serialization
- Migration execution (user code callbacks)
- Lazy `c.db` proxy

## Shared Types

Define in rivetkit-core (or rivetkit-sqlite, re-exported):

```rust
pub enum BindParam {
    Null,
    Integer(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}

pub struct ExecResult {
    pub changes: i64,
}

pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<ColumnValue>>,
}

pub enum ColumnValue {
    Null,
    Integer(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}
```

## Migration Steps

1. **Move + rename sqlite-native → rivetkit-sqlite** — Physical move to `rivetkit-rust/packages/rivetkit-sqlite/`, rename crate, update Cargo paths, verify `cargo check`
2. **Extract FFI functions** — Move bind/exec/query from rivetkit-napi to rivetkit-sqlite, add shared types
3. **Expand SqliteDb in rivetkit-core** — Add database handle, open/close lifecycle, wire exec/query/run
4. **Wire ActorContext stubs** — Replace error stubs with SqliteDb delegation
5. **Slim rivetkit-napi** — Replace inline FFI with calls to rivetkit-core/rivetkit-sqlite
6. **Update rivetkit Rust crate** — Expose typed database access on `Ctx<A>`
7. **Verify TS path unchanged** — Run driver test suite, confirm native.ts still works through NAPI

## Risks

- **libsqlite3-sys bundled** — rivetkit-core gains a C dependency. Gate behind a `sqlite` cargo feature so consumers without SQLite don't pay the compile cost.
- **Thread safety** — `*mut sqlite3` is not Send. The current NAPI layer uses `spawn_blocking`. rivetkit-core must do the same or use a dedicated thread.
- **V1/V2 dispatch** — rivetkit-core needs to know about both VFS versions. Keep the dispatch in rivetkit-sqlite and expose a unified `Database` handle.
- **CLAUDE.md constraint** — "Keep SQLite runtime code on the native `@rivetkit/rivetkit-napi` path." This spec proposes changing that constraint since the code is moving to Rust-proper.

## CLAUDE.md Updates

Remove:
- "Keep SQLite runtime code on the native `@rivetkit/rivetkit-napi` path."

Add:
- "SQLite VFS and query execution live in `rivetkit-rust/packages/rivetkit-sqlite/`. rivetkit-core owns the database lifecycle. NAPI provides only JS type marshaling."
- "Gate `libsqlite3-sys` behind a `sqlite` feature flag in rivetkit-core."
