# Native SQLite via KV Channel Protocol

## Overview

Replace the WebAssembly-based SQLite implementation with a native Rust SQLite binary delivered as a napi-rs addon. The native binary statically links SQLite and implements a custom VFS that routes page-level KV operations over a new WebSocket-based "KV channel" protocol. The KV channel runs parallel to the existing runner protocol and connects directly to the engine (production) or manager (local dev), bypassing the JavaScript runtime entirely.

The KV channel is independent of the runner system. It does not require a runner key and can operate on actor data even when the actor is not currently running via the runner protocol. This makes it a general-purpose data access layer.

The WASM implementation remains as a fallback when the native addon is unavailable (e.g. postinstall didn't run, unsupported platform, or musl-based Linux). A warning is emitted via pino when falling back to WASM.

## Motivation

- **Memory**: Each actor currently gets its own WASM module instance with its own linear memory. Native SQLite shares one library with N lightweight database connections, significantly reducing per-actor memory overhead.
- **Performance**: WASM SQLite has overhead from VFS callbacks bouncing through the JS event loop, WASM async relays, and the JS KV driver. Native SQLite removes the JS runtime from the hot path for database I/O.
- **Simplicity**: The KV channel is a single WebSocket connection from Rust, avoiding the complexity of routing through the JavaScript-based runner protocol.

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Node.js Process                                │
│                                                 │
│  ┌─────────────┐  ┌─────────────┐              │
│  │  Actor A     │  │  Actor B     │   (JS)      │
│  │  c.db.query()│  │  c.db.query()│             │
│  └──────┬───────┘  └──────┬───────┘             │
│         │ N-API            │ N-API               │
│  ┌──────▼──────────────────▼───────┐            │
│  │  @rivetkit/sqlite-native        │   (Rust)   │
│  │                                 │            │
│  │  ┌───────────┐  ┌───────────┐  │            │
│  │  │ DB conn A │  │ DB conn B │  │            │
│  │  │ (VFS: A)  │  │ (VFS: B)  │  │            │
│  │  └─────┬─────┘  └─────┬─────┘  │            │
│  │        │               │        │            │
│  │  ┌─────▼───────────────▼─────┐  │            │
│  │  │  Shared WebSocket conn    │  │            │
│  │  │  → KV channel             │  │            │
│  │  └───────────────────────────┘  │            │
│  └─────────────────────────────────┘            │
└─────────────────────────────────────────────────┘
         │
         │ WebSocket (BARE serialization)
         │
         ▼
┌─────────────────────────────────┐
│  KV Channel Server              │
│  (engine or manager)            │
│                                 │
│  Validates namespace token      │
│  Enforces single-writer         │
│  Routes KV to storage           │
└─────────────────────────────────┘
```

## Multiple Databases

SQLite natively supports many open databases in one process. Each actor gets its own database connection with a VFS instance parameterized by actor ID. Every KV operation the VFS makes includes the actor ID on the wire, so the server routes to the correct actor's storage.

This is the primary win over WASM: one native SQLite library with N lightweight connections instead of N separate WASM module instances each with their own linear memory.

When an actor opens a database, the addon registers a VFS scoped to that actor. The VFS callbacks (`xRead`, `xWrite`, `xTruncate`, `xDelete`, etc.) translate to KV operations using the same 4 KiB chunk layout and key encoding as the existing WASM VFS. Data is fully compatible between native and WASM. An actor can switch between backends without data migration.

## KV Channel Protocol

The KV channel is a WebSocket connection using BARE serialization, running parallel to the runner protocol. It is independent of the runner system and does not require a runner key.

### Connection

```
ws://{endpoint}/kv/connect?token={token}&namespace={ns}&protocol_version=1
```

Authentication uses a `token` query parameter. In production, this is the engine's `admin_token` (`RIVET__AUTH__ADMIN_TOKEN`). In local dev, this is `config.token` (`RIVET_TOKEN`), which is optional in dev mode (auth skipped with warning if not set). The KV channel does not depend on the runner protocol or runner keys.

### Protocol Version Negotiation

The `protocol_version` query parameter follows the same VBARE versioning pattern as the runner protocol. The server validates the requested version on connection:

- If the server supports the requested version, the connection is accepted.
- If the version is unknown or unsupported, the server rejects the WebSocket upgrade with an HTTP error.
- Version numbers are monotonically increasing. Each version corresponds to a `.bare` schema file (e.g. `v1.bare`).
- When bumping the KV channel protocol version, add a new versioned schema file and update the server's `versioned.rs` (or `versioned.ts`) with migration logic, following the same pattern as the runner protocol (`engine/sdks/schemas/runner-protocol/`). See `engine/CLAUDE.md` for the migration process.

### Schema

```bare
# kv-channel-protocol/v1.bare

# MARK: Core

# Id is a 30-character base36 string encoding the V1 format from
# engine/packages/util-id/. Use the util-id library for parsing
# and validation. Do not hand-roll Id parsing.
type Id str

# MARK: Actor Session
#
# ActorOpen acquires a single-writer lock on an actor's KV data.
# ActorClose releases the lock. These are optimistic: the client
# does not wait for a response before sending KV requests. The
# server processes messages in WebSocket order, so the open is
# always processed before any KV requests that follow it.
#
# If the lock cannot be acquired (another connection holds it),
# the server sends an error response for the open and rejects
# subsequent KV requests for that actor with "actor_locked".

# actorId is on ToServerRequest, not on open/close. The outer
# actorId is the single source of truth for routing.
type ActorOpenRequest void

type ActorCloseRequest void

type ActorOpenResponse void

type ActorCloseResponse void

# MARK: KV
#
# These types mirror the runner protocol KV types
# (engine/sdks/schemas/runner-protocol/). Changes to KV types in
# either protocol must be mirrored in the other.
#
# Omitted from the runner protocol (not needed by the VFS):
# - KvListRequest/KvListResponse (prefix scan)
# - KvDropRequest/KvDropResponse (drop all KV data)
# - KvMetadata on responses (update timestamps)
#
# The same engine KV limits apply to both protocols. See the
# "KV Limits" section below.

type KvKey data
type KvValue data

type KvGetRequest struct {
    keys: list<KvKey>
}

type KvPutRequest struct {
    # keys and values are parallel lists. keys.len() must equal values.len().
    keys: list<KvKey>
    values: list<KvValue>
}

type KvDeleteRequest struct {
    keys: list<KvKey>
}

type KvDeleteRangeRequest struct {
    start: KvKey
    end: KvKey
}

# MARK: Request/Response

type RequestData union {
    ActorOpenRequest |
    ActorCloseRequest |
    KvGetRequest |
    KvPutRequest |
    KvDeleteRequest |
    KvDeleteRangeRequest
}

type ErrorResponse struct {
    code: str
    message: str
}

type KvGetResponse struct {
    # Only keys that exist are returned. Missing keys are omitted.
    # The client infers missing keys by comparing request keys to
    # response keys. This matches the runner protocol behavior
    # (engine/packages/pegboard/src/actor_kv/mod.rs).
    keys: list<KvKey>
    values: list<KvValue>
}

type KvPutResponse void

# KvDeleteResponse is used for both KvDeleteRequest and
# KvDeleteRangeRequest, same as the runner protocol.
type KvDeleteResponse void

type ResponseData union {
    ErrorResponse |
    ActorOpenResponse |
    ActorCloseResponse |
    KvGetResponse |
    KvPutResponse |
    KvDeleteResponse
}

# MARK: To Server

type ToServerRequest struct {
    requestId: u32
    actorId: Id
    data: RequestData
}

type ToServerPong struct {
    ts: i64
}

type ToServer union {
    ToServerRequest |
    ToServerPong
}

# MARK: To Client

type ToClientResponse struct {
    requestId: u32
    data: ResponseData
}

type ToClientPing struct {
    ts: i64
}

# Server-initiated close. Sent when the server is shutting down
# or draining connections. The client should close all actors
# and reconnect with backoff. Same pattern as the runner
# protocol's ToRunnerClose.
type ToClientClose void

type ToClient union {
    ToClientResponse |
    ToClientPing |
    ToClientClose
}
```

### Design Decisions

- **No runner key dependency.** The KV channel authenticates with a namespace-scoped token, independent of the runner system. It can access actor data even when the actor is not running.
- **Optimistic open/close.** The client does not wait for a round-trip after sending `ActorOpenRequest` or `ActorCloseRequest`. It immediately pipelines KV requests. The server processes messages in WebSocket order, so the open is always handled before subsequent KV requests.
- **Single-writer enforcement.** The server maintains a lock per actor. Only one connection can hold the lock at a time. If a second connection tries to open the same actor, it receives an error and all its KV requests for that actor are rejected with `actor_locked`.
- **`actorId` on `ToServerRequest`, not on individual request types.** A single connection may have multiple actors open (one per actor running in the process). Each `ToServerRequest` includes `actorId` so the server can route to the correct actor. `ActorOpenRequest` and `ActorCloseRequest` are void types since the outer `actorId` already identifies the actor. This avoids duplication and eliminates the ambiguity of mismatched IDs.
- **`KvDeleteResponse` for both delete and delete-range.** Both return void. The client correlates responses by `requestId`. This matches the runner protocol pattern where `KvResponseData` has no `KvDeleteRangeResponse`.
- **No KvListRequest.** The SQLite VFS only needs get/put/delete/deleteRange. It never does prefix scans. If needed later, add `KvListRequest`/`KvListResponse` to both protocols simultaneously.
- **No KvMetadata.** The VFS does not need update timestamps or version info. Raw bytes in, raw bytes out.
- **`ToClientClose` for server-initiated shutdown.** Same pattern as the runner protocol's `ToRunnerClose`. Allows graceful drain before disconnect.

### Error Codes

The server uses the following error codes in `ErrorResponse`. These mirror the error handling in the runner protocol's KV system (`engine/packages/pegboard/src/actor_kv/mod.rs`).

| Code | Meaning |
|---|---|
| `actor_locked` | Another connection holds the single-writer lock for this actor. |
| `actor_not_open` | KV request received for an actor not open on this connection. |
| `actor_not_found` | The actor ID does not exist or does not belong to the authenticated namespace. |
| `key_too_large` | A key exceeds the maximum key size (2048 bytes). |
| `value_too_large` | A single value exceeds the maximum value size (128 KiB). |
| `payload_too_large` | A `KvPutRequest` exceeds the maximum batch payload size (976 KiB total across keys + values). |
| `batch_too_large` | A request exceeds the maximum entries per batch (128 key-value pairs). |
| `storage_quota_exceeded` | The actor's total KV storage would exceed the 10 GiB limit. |
| `keys_values_length_mismatch` | `KvPutRequest` `keys` and `values` have different lengths. |
| `unauthorized` | Invalid or missing authentication token. |
| `internal_error` | Unexpected server-side error. |

### KV Limits

The same engine KV limits apply to the KV channel protocol as to the runner protocol. These are enforced server-side.

| Limit | Value |
|---|---|
| Max key size | 2048 bytes |
| Max value size | 128 KiB (per individual value) |
| Max batch payload size (`KvPutRequest`) | 976 KiB total across keys + values |
| Max entries per batch | 128 key-value pairs |
| Max total actor KV storage | 10 GiB |

These values are currently defined in two places that must stay in sync:
- **Rust:** `engine/packages/pegboard/src/actor_kv/mod.rs`
- **TypeScript:** `rivetkit-typescript/packages/rivetkit/src/drivers/file-system/kv-limits.ts`

These constants should be extracted into a shared library (e.g. `engine/packages/kv-limits/` for Rust, with the TypeScript file importing or code-generating from the same source) so they cannot drift. Until that shared library exists, changes to limits must be manually updated in both locations, both protocols, and in `website/src/content/docs/actors/limits.mdx`.

### Single-Writer Lock Semantics

The server enforces that at most one WebSocket connection holds the lock for a given actor at any time. This prevents concurrent writes from corrupting SQLite page data.

**Acquiring the lock:**
- Client sends `ToServerRequest` with `actorId` and `ActorOpenRequest`.
- Server checks if the actor is already locked by another connection.
- If unlocked: lock is acquired, server sends `ActorOpenResponse`.
- If locked by another connection: server sends `ErrorResponse { code: "actor_locked" }`.

**Releasing the lock:**
- Client sends `ToServerRequest` with `actorId` and `ActorCloseRequest`.
- Server releases the lock, sends `ActorCloseResponse`.
- If the WebSocket disconnects without closing, the server releases all locks held by that connection.

**Unconditional lock eviction on reconnect:**
- When an `ActorOpenRequest` arrives for an actor locked by a different connection on the same server, the server unconditionally transfers the lock to the new connection. The old connection's subsequent KV requests fail with `actor_not_open`. This handles reconnection scenarios where the server hasn't detected the old connection's disconnect yet.
- In multi-server deployments, locks are per-server-instance. If a client reconnects to a different server, the new server has no prior lock and the open succeeds immediately. The old server cleans up the stale lock on disconnect detection. Cross-process single-writer is enforced by actor scheduling, not the KV channel lock.

**Optimistic pipelining:**
- The client sends `ActorOpenRequest` followed immediately by KV requests without waiting for the response.
- WebSocket message ordering guarantees the server processes the open before the KV requests.
- If the open fails, the server responds with `ErrorResponse` for the open and `ErrorResponse { code: "actor_locked" }` for each subsequent KV request targeting that actor.
- The client discovers the failure when it processes responses. It can then retry or fail.

### Edge Cases

**Two connections open the same actor.** The second connection's `ActorOpenRequest` fails with `actor_locked`. Its KV requests for that actor also fail. The client discovers this on the first response it reads.

**Client crashes without closing.** The server detects the WebSocket disconnect and releases all locks held by that connection. This is the primary cleanup mechanism.

**Close from connection A, immediate open from connection B.** If B's open arrives before A's close is processed, B gets `actor_locked`. B retries with backoff. WebSocket disconnect cleanup ensures locks are never leaked even if A's close message is lost.

**KV request for an actor not opened on this connection.** Server returns `ErrorResponse { code: "actor_not_open" }`.

**Server restart.** All locks are lost. Clients reconnect and re-open their actors. Since the open is optimistic, this is fast. No data is lost because KV operations are durable.

**Server-initiated shutdown.** The server sends `ToClientClose` before closing the WebSocket. The server must keep the connection open long enough for in-flight responses to be sent (i.e. it finishes processing any requests already received before closing). The client treats `ToClientClose` the same as a disconnect: close all actors and reconnect with backoff. Same pattern as the runner protocol's `ToRunnerClose`.

**Stale lock without disconnect.** The ping/pong mechanism detects dead connections. The server sends `ToClientPing` every 3 seconds. If the server doesn't receive a `ToServerPong` within 15 seconds, it closes the WebSocket and releases locks. These values match the runner protocol defaults (`runner_update_ping_interval_ms = 3000`, `runner_ping_timeout_ms = 15000` in `engine/packages/config/src/config/pegboard.rs`).

### KV Sync Requirement

The KV types in this protocol must stay in sync with the runner protocol (`engine/sdks/schemas/runner-protocol/`). When adding, removing, or changing KV request/response types in one protocol, update the other to match. Types intentionally omitted from this protocol (`KvListRequest`, `KvDropRequest`, and `KvMetadata`) are documented in "Design Decisions" above and do not need to be added unless the VFS requires them. This is documented in CLAUDE.md.

## JavaScript Bindings

### napi-rs Addon API

The addon exposes a minimal API via napi-rs. Internally uses Rust tokio for async WebSocket I/O. SQLite operations run on tokio's blocking thread pool via `spawn_blocking`. VFS callbacks call `Handle::block_on()` from blocking threads (not tokio worker threads), which is safe. The Node.js main thread is never blocked.

```typescript
// @rivetkit/sqlite-native

// Open the shared KV channel WebSocket connection.
// In production, token is the engine's admin_token (RIVET__AUTH__ADMIN_TOKEN).
// In local dev, token is config.token (RIVET_TOKEN), optional in dev mode.
export function connect(config: {
    url: string;
    token?: string;
    namespace: string;
}): KvChannel;

// Open a database for an actor. Sends ActorOpenRequest optimistically.
export function openDatabase(channel: KvChannel, actorId: string): NativeDatabase;

// Execute a statement (INSERT, UPDATE, DELETE, CREATE, etc.).
export function execute(db: NativeDatabase, sql: string, params?: any[]): Promise<{
    changes: number;
}>;

// Run a query (SELECT, PRAGMA, etc.).
export function query(db: NativeDatabase, sql: string, params?: any[]): Promise<{
    rows: any[][];
    columns: string[];
}>;

// Run one or more SQL statements without returning results.
export function exec(db: NativeDatabase, sql: string): Promise<void>;

// Close the database connection, release the lock. Sends ActorCloseRequest.
export function closeDatabase(db: NativeDatabase): void;

// Close the KV channel WebSocket.
export function disconnect(channel: KvChannel): void;
```

### Integration with RivetKit

The database provider (`rivetkit-typescript/packages/rivetkit/src/db/mod.ts`) adds a conditional path:

```typescript
if (nativeSqliteAvailable()) {
    db = nativeSqlite.openDatabase(kvChannel, actorId);
} else {
    logger.warn(
        "native SQLite not available, falling back to WebAssembly. " +
        "run `npm rebuild` to install native bindings."
    );
    db = await sqliteVfs.open(actorId, kvStore);
}
```

The `Database` interface exposed to actors (`c.db.query()`, `c.db.execute()`) is identical regardless of backend. The actor does not know which implementation is running.

`nativeSqliteAvailable()` must attempt to actually load the `.node` addon and catch all failure modes (missing file, glibc mismatch, N-API version mismatch, corrupted binary), not just check for file existence.

## Custom VFS

The native VFS implements the same storage mapping as the WASM VFS in `rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts`.

### Key Layout

Identical to the WASM VFS (`rivetkit-typescript/packages/sqlite-vfs/src/kv.ts`). All values below are single bytes unless noted.

```
Metadata key (4 bytes):
  byte 0: SQLITE_PREFIX  = 0x08
  byte 1: SCHEMA_VERSION = 0x01
  byte 2: META_PREFIX    = 0x00
  byte 3: FILE_TAG       (0x00=main, 0x01=journal, 0x02=wal, 0x03=shm)

Chunk key (8 bytes):
  byte 0:   SQLITE_PREFIX  = 0x08
  byte 1:   SCHEMA_VERSION = 0x01
  byte 2:   CHUNK_PREFIX   = 0x01
  byte 3:   FILE_TAG       (0x00=main, 0x01=journal, 0x02=wal, 0x03=shm)
  bytes 4-7: CHUNK_INDEX    (u32 big-endian)
```

The Rust implementation must produce byte-identical keys to the TypeScript implementation in `rivetkit-typescript/packages/sqlite-vfs/src/kv.ts`. Both implementations should have cross-references in comments to keep them in sync.

### Chunk Size

Fixed 4096 bytes (4 KiB), same as the WASM VFS. SQLite page reads/writes map to KV chunk operations.

### PRAGMA Settings

Both the native and WASM VFS must apply the same PRAGMA settings for KV-backed SQLite. Keep these in sync between `rivetkit-typescript/packages/sqlite-native/src/vfs.rs` and `rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts`.

```sql
-- Keep in sync between native VFS (vfs.rs) and WASM VFS (vfs.ts).
PRAGMA busy_timeout = 5000;
PRAGMA page_size = 4096;
```

- `busy_timeout = 5000`: Wait 5 seconds if the database is locked, reducing transient failures during restarts. Matches the file-system driver (`sqlite-runtime.ts`).
- `page_size = 4096`: Matches `CHUNK_SIZE`, ensuring page-aligned writes avoid extra read-then-write round trips for partial chunks. This is SQLite's default, so omitting it is harmless, but setting it explicitly documents the dependency on `CHUNK_SIZE`.

**Migration plan:** The WASM VFS does not currently set any PRAGMAs after opening a database (there is a TODO in `vfs.ts`). As part of this work, add both PRAGMAs to the WASM VFS `open()` method after `sqlite3.open_v2()`. The native VFS sets them from day one.

WAL mode is not enabled for KV-backed SQLite. See "WAL Mode" section below.

Changes to PRAGMA settings must be updated in both the native VFS and the WASM VFS. Both files should have comments pointing to each other.

### VFS Callbacks

| SQLite VFS method | KV operation |
|---|---|
| `xRead` | `KvGetRequest` (batch of chunk keys) |
| `xWrite` | `KvGetRequest` (prefetch partial chunks) + `KvPutRequest` (batch write) |
| `xTruncate` | `KvDeleteRangeRequest` (chunks beyond truncation point) + optional partial chunk rewrite |
| `xDelete` | `KvDeleteRangeRequest` (all chunks for file tag) + `KvDeleteRequest` (metadata key) |
| `xSync` | `KvPutRequest` (flush dirty metadata) |
| `xFileSize` | `KvGetRequest` (read metadata key) |
| `xAccess` | `KvGetRequest` (check metadata key existence) |
| `xLock`/`xUnlock` | No-op (single-writer per actor, enforced by lock) |

Both the native and WASM VFS should use `KvDeleteRangeRequest` for `xTruncate` and `xDelete`. This is an O(1) server-side operation via `clear_range()`, compared to the current WASM VFS approach of enumerating individual chunk keys and calling `deleteBatch()` which is O(n) in chunk count. The chunk keys for a given file tag are lexicographically contiguous (fixed prefix + big-endian chunk index), so range deletion is always safe within a file tag.

**Migration plan:** The WASM VFS `KvVfsOptions` interface (`rivetkit-typescript/packages/sqlite-vfs/src/types.ts`) currently has no `deleteRange` method. As part of this work, add `deleteRange(start: Uint8Array, end: Uint8Array): Promise<void>` to `KvVfsOptions` and migrate the WASM VFS `xTruncate` and `#delete` to use it. The native VFS uses `KvDeleteRangeRequest` from day one.

### Caching

SQLite's built-in pager cache is used. Default is 2000 pages (8 MB at 4 KiB page size). No second cache in the VFS, same as the WASM implementation. This avoids duplicate cache invalidation logic and keeps memory predictable.

### WAL Mode

WAL mode is not enabled for KV-backed SQLite (native or WASM). The VFS file tags for WAL (0x02) and SHM (0x03) are defined for completeness and to match the WASM VFS, but neither VFS currently sets an explicit journal mode for KV-backed databases. SQLite defaults to rollback journal (`DELETE` mode). The WASM VFS has a TODO to benchmark `journal_mode=PERSIST` and `journal_size_limit` to reduce journal churn on high-latency KV. When that work is done, both VFS implementations must be updated together.

The file-system driver (`rivetkit-typescript/packages/rivetkit/src/drivers/file-system/sqlite-runtime.ts`) uses `PRAGMA journal_mode = WAL` for file-backed databases, but that code path is separate from the KV-backed VFS.

If WAL mode is enabled in the future, both the native and WASM VFS implementations must be updated together.

## Authentication

### Connection-level (once)

1. The napi-rs addon opens a WebSocket to the KV channel endpoint.
2. URL includes auth token, `namespace`, and `protocol_version` as query params.
3. **Engine (production):** Server validates `token` against `auth.admin_token` (`RIVET__AUTH__ADMIN_TOKEN`). Same token the runner protocol and public API use.
4. **Manager (local dev):** Server validates `token` against `config.token` (`RIVET_TOKEN`). If not set in dev mode, auth is skipped with a pino warning, matching existing manager KV endpoint behavior.
5. All subsequent messages on this WebSocket are authenticated by the connection.

### Per-request (actor scoping)

- Every `ToServerRequest` includes `actorId`.
- The server checks that the actor belongs to the authenticated namespace.
- The server checks that the actor's single-writer lock is held by this connection.
- If not opened: `ErrorResponse { code: "actor_not_open" }`.
- If locked by another connection: `ErrorResponse { code: "actor_locked" }`.

## Connection Lifecycle

- One WebSocket per Node.js process, not per actor.
- Opened during process initialization. Does not depend on runner protocol being connected.
- Multiple actors share the same WebSocket connection with `actorId` on each request. There is no limit on the number of concurrently open actors per connection. The number is bounded by the process-level actor slot limit configured in the runner.
- On process shutdown, the client sends `ActorCloseRequest` for each open actor, then closes the WebSocket. If the process crashes, the server detects the disconnect and releases all locks.

### Reconnect Behavior

On disconnect (network error, server restart, or `ToClientClose`), the client reconnects with exponential backoff using the same strategy as the runner protocol (`engine/sdks/typescript/runner/src/utils.ts`):

- **Initial delay:** 1000 ms
- **Max delay:** 30000 ms (30 seconds)
- **Multiplier:** 2 (exponential)
- **Jitter:** 0-25% additional random delay per attempt

KV operations block until reconnected, with a 30-second timeout per operation (`KV_EXPIRE`). If the timeout expires, the operation fails with an error. This matches the runner protocol's KV request timeout (`KV_EXPIRE = 30_000` in `engine/sdks/typescript/runner/src/mod.ts`). The operation timeout is independent of the reconnect backoff. A single backoff cycle may exceed the operation timeout, in which case blocked operations fail while reconnection continues in the background.

On successful reconnect, the client re-sends `ActorOpenRequest` for all actors that were open before the disconnect. On reconnect, the client must wait for `ActorOpenResponse` before sending KV requests for each actor. The initial open (first connection) can remain optimistic.

### In-Flight Requests on Reconnect

When a disconnect occurs, in-flight requests (sent but no response received) are failed with a connection error. They are not automatically retried. The caller (the VFS) receives an error and can retry at the SQLite level.

This is safe because:
- `KvPutRequest` is idempotent (writing the same chunks again produces the same result).
- `KvGetRequest` is read-only.
- `KvDeleteRequest` and `KvDeleteRangeRequest` are idempotent.

The `requestId` counter resets to 0 on reconnect. Since all in-flight requests were already failed, there is no risk of ID collision.

## Distribution

Prebuilt binaries for supported platforms:

- `@rivetkit/sqlite-native-linux-x64-gnu`
- `@rivetkit/sqlite-native-linux-arm64-gnu`
- `@rivetkit/sqlite-native-darwin-x64`
- `@rivetkit/sqlite-native-darwin-arm64`
- `@rivetkit/sqlite-native-win32-x64-msvc`

The main `@rivetkit/sqlite-native` package has optional dependencies on each platform package. postinstall selects the correct binary. If postinstall fails or the platform is unsupported, the WASM fallback is used with a warning logged via pino.

**musl limitation:** Linux musl targets (e.g. Alpine Linux, many Docker images) are not included in the prebuilt binaries. On musl-based systems, the native addon will not load and the WASM fallback activates automatically. This matches the runner protocol's approach to platform support via napi-rs.

### Runtime Compatibility

The napi-rs addon uses Node-API (N-API), which is supported by both Node.js and Bun. The WebSocket client is pure Rust (tokio/tungstenite), with no runtime-specific APIs beyond the N-API FFI boundary.

### SQLite Version Alignment

The native addon statically links SQLite via `libsqlite3-sys` with the `bundled` feature. The bundled SQLite version must match the version used by the WASM build (`@rivetkit/sqlite`) to avoid behavioral differences (JSON functions, FTS tokenizers, etc.). Pin the `libsqlite3-sys` version in `Cargo.toml` and document the expected SQLite version. When upgrading either build, upgrade both.

## Data Compatibility

The native VFS uses the identical KV key layout and 4 KiB chunk mapping as the WASM VFS. An actor's data can be read by either backend without migration. Switching between native and WASM is transparent.

This compatibility depends on both backends using `CHUNK_SIZE = 4096` and the same key encoding. The `SQLITE_SCHEMA_VERSION` byte (0x01) in the key layout provides a migration path if the chunk format changes in the future.

## Protocol Choice: WebSocket

WebSocket was chosen over HTTP for the KV channel transport:

- **Single authenticated connection.** No per-request auth overhead.
- **Minimal framing.** 2-6 bytes per message vs ~100-200 bytes of HTTP/2 headers. With 4 KiB payloads the difference is small, but WebSocket is strictly less overhead.
- **Simpler correlation.** Request/response matching via `requestId` field in BARE, no HTTP stream management.
- **Connection lifecycle aligns with process lifecycle.** The KV channel lives for the duration of the process.

HTTP was considered. The overhead difference is negligible for 4 KiB+ payloads, but WebSocket is a better fit for a long-lived, high-frequency request-response channel within a single process.

## Testing

### Layer 1: Rust VFS Unit Tests

Located in `rivetkit-typescript/packages/sqlite-native/src/` (standard Rust `#[cfg(test)]` modules).

Tests the VFS implementation against a mock WebSocket server that runs in-process. The mock server implements the KV channel protocol with an in-memory KV store. This validates:

- `xRead`/`xWrite` correctly map byte ranges to 4 KiB KV chunks
- Chunk boundary handling (partial reads/writes spanning chunks)
- `xTruncate` and `xDelete` produce correct KV delete ranges
- Metadata (file size) persistence via `xSync`/`xClose`
- File tags (main, journal, WAL, SHM) key encoding
- Error handling (actor not open, actor locked)
- Optimistic open pipelining (open + immediate KV requests)
- Reconnection behavior (mock server drops connection, client reconnects and re-opens actors)

A comment in the test module should note: "End-to-end tests covering the full actor lifecycle with native SQLite are in the driver test suite (`rivetkit-typescript/packages/rivetkit/src/driver-test-suite/`)."

### Layer 2: Driver Test Suite (End-to-End)

The existing driver test suite (`rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/`) already tests SQLite via the actor database providers (raw DB and Drizzle). These tests run against the file system driver and engine driver.

When native SQLite is available, the database provider automatically uses it instead of WASM. The same driver test suite validates native SQLite end-to-end without any test changes. This covers:

- Actor database operations (insert, query, update, pagination)
- Drizzle ORM integration and migrations
- Large payload handling
- Database persistence across actor restarts
- WASM fallback behavior (test with native addon absent)

No new test files are needed in the driver test suite. The existing `actor-db-raw.ts` and `actor-db-drizzle.ts` fixture actors exercise both backends transparently.

## Rust Project Structure

The native addon lives in the rivetkit-typescript workspace alongside the existing sqlite-vfs package.

```
rivetkit-typescript/packages/sqlite-native/
├── Cargo.toml                  # Rust crate: napi-rs + sqlite3 + tokio + tungstenite + serde-bare
├── package.json                # npm package: @rivetkit/sqlite-native
├── src/
│   ├── lib.rs                  # napi-rs entry point, exports connect/openDatabase/query/execute/etc.
│   ├── vfs.rs                  # Custom SQLite VFS implementation (xRead, xWrite, etc. → KV ops)
│   ├── kv.rs                   # KV key layout (mirrors sqlite-vfs/src/kv.ts)
│   ├── channel.rs              # WebSocket KV channel client (connect, reconnect, send/recv)
│   └── protocol.rs             # BARE ser/de for KV channel messages (generated or hand-written)
├── build.rs                    # napi-rs build script
└── npm/                        # Platform-specific npm packages (generated by napi-rs)
    ├── linux-x64-gnu/
    │   └── package.json        # @rivetkit/sqlite-native-linux-x64-gnu
    ├── linux-arm64-gnu/
    │   └── package.json
    ├── darwin-x64/
    │   └── package.json
    ├── darwin-arm64/
    │   └── package.json
    └── win32-x64-msvc/
        └── package.json
```

The Rust crate is part of the pnpm workspace but not the Cargo workspace (it has its own Cargo.toml with napi-rs dependencies that don't belong in the engine workspace). It statically links `libsqlite3-sys` with the `bundled` feature so no system SQLite is required.

The KV channel protocol schema lives alongside the runner protocol:

```
engine/sdks/schemas/kv-channel-protocol/
├── v1.bare
└── versioned.ts
```

Versioned schema handling follows the same VBARE migration pattern as the runner protocol (`engine/sdks/schemas/runner-protocol/`). See `engine/CLAUDE.md` for the migration process.

The server-side KV channel endpoint is added to:

- **Engine:** `engine/packages/pegboard-runner/` (or a new `engine/packages/kv-channel/` crate)
- **Manager:** `rivetkit-typescript/packages/rivetkit/src/manager/router.ts` (new WebSocket route)

## Build Pipeline

### Local Development

```bash
# Build the native addon (from rivetkit-typescript/packages/sqlite-native/)
pnpm build

# This runs napi-rs build under the hood:
# cargo build --release && napi artifacts
```

napi-rs handles the N-API binding generation. The output is a `.node` file in the package root.

### CI / Release

The native addon requires cross-compilation for each target platform. This is handled by a GitHub Actions matrix build using `napi-rs/napi-rs` official CI actions.

Add a new workflow or extend `.github/workflows/release.yaml`:

1. **Build matrix** — Build the native addon for each platform target:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-pc-windows-msvc`

2. **Upload artifacts** — Each platform build produces a `.node` binary. These are uploaded as CI artifacts.

3. **Package** — After all platform builds complete, the napi-rs `artifacts` command assembles platform-specific npm packages under `npm/`.

4. **Publish** — Platform packages are published to npm alongside the main `@rivetkit/sqlite-native` package during the `publish-sdk` release step.

### napi-rs CI Template

napi-rs provides a standard GitHub Actions template (`@napi-rs/cli`) that handles:
- Cross-compilation via `cross` or native runners
- Artifact upload/download between jobs
- Platform package generation

## Release Script Updates

The following changes are needed in `scripts/release/`:

### `update_version.ts`

Add a new entry to the `findReplace` array:

```typescript
{
    path: "rivetkit-typescript/packages/sqlite-native/package.json",
    find: /"version": ".*"/,
    replace: `"version": "${opts.version}"`,
},
{
    path: "rivetkit-typescript/packages/sqlite-native/npm/*/package.json",
    find: /"version": ".*"/,
    replace: `"version": "${opts.version}"`,
},
```

This is already partially covered by the existing `rivetkit-typescript/packages/*/package.json` glob, but the `npm/*/package.json` subdirectories need explicit handling since they are nested.

Also update `Cargo.toml` in the sqlite-native crate (this is not in the Cargo workspace, so the workspace version bump doesn't cover it):

```typescript
{
    path: "rivetkit-typescript/packages/sqlite-native/Cargo.toml",
    find: /^version = ".*"/m,
    replace: `version = "${opts.version}"`,
},
```

### `sdk.ts`

The `@rivetkit/sqlite-native` and its platform packages need to be published during the `publish-sdk` step. Since they are in `rivetkit-typescript/packages/`, they should be picked up by the existing `getRivetkitPackages()` glob. Verify the platform packages under `npm/` are also included or add them explicitly.

### `build-artifacts.ts`

Add a new function to build the native addon binaries. This may be better handled as a separate GitHub Actions workflow step (matrix build) rather than in the release script directly, since cross-compilation requires platform-specific runners.

### `.github/workflows/release.yaml`

Add a job that runs before `publish-sdk`:
1. Matrix build of sqlite-native for all target platforms
2. Upload platform `.node` binaries as artifacts
3. Download artifacts and assemble npm packages before publish
