# Native SQLite KV Channel: Review Fix Spec

Fixes for validated findings from the adversarial review. Each item maps to a finding ID from `NATIVE_SQLITE_ADVERSARIAL_REVIEW.md`.

---

## C1. Use-After-Free in `close_database`

**File**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs`

**Problem**: `close_database` is synchronous and drops the `sqlite3` handle via `db.take()`. Concurrent `execute`/`query` may have already extracted `db_ptr` as `usize` and dispatched a `spawn_blocking` closure that hasn't started yet. That closure casts the stale `usize` back to `*mut sqlite3` and uses freed memory.

**Fix**: Wrap the `NativeDatabase` in `Arc<std::sync::Mutex<Option<NativeDatabase>>>`. Each `execute`/`query`/`exec` closure clones the `Arc` into `spawn_blocking` and locks it to access the database pointer. `close_database` locks the same mutex and takes the `Option`, so any subsequent or concurrent use sees `None` and returns an error. This ensures the database handle outlives all in-flight operations.

```rust
// In JsNativeDatabase:
db: Arc<std::sync::Mutex<Option<vfs::NativeDatabase>>>,

// In execute/query spawn_blocking closures:
let db_arc = db.db.clone();
spawn_blocking(move || {
    let guard = db_arc.lock().unwrap();
    let native_db = guard.as_ref().ok_or("database is closed")?;
    let db_ptr = native_db.as_ptr();
    // ... use db_ptr ...
})

// In close_database:
let db_arc = db.db.clone();
let native_db = db_arc.lock().unwrap().take();
// native_db is dropped here, closing SQLite
```

Remove `unsafe impl Sync for JsNativeDatabase` — the `Arc<Mutex<...>>` makes it naturally `Sync`.

---

## H1. Lifecycle Functions Block Node.js Event Loop

**File**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs`

**Problem**: `open_database`, `close_database`, and `disconnect` are synchronous napi exports (`pub fn`) that call `rt.block_on(...)`. Unlike `execute`/`query` which are `pub async fn` and use `spawn_blocking`, these lifecycle functions park the calling thread — which is the Node.js main thread — until the async operation completes. The tokio runtime runs on separate worker threads, but `block_on` freezes whatever thread calls it. So during the WebSocket round-trip, the entire Node.js event loop is stuck.

**Constraint**: In the `@rivetkit/sqlite-native` addon, all napi exports that perform async I/O (WebSocket round-trips, VFS callbacks) MUST be `pub async fn`, never synchronous `pub fn` with `rt.block_on(...)`. Synchronous `block_on` is only safe from within `spawn_blocking` threads (where `execute`/`query` already do it correctly via VFS callbacks).

**Fix**: Change all three to `pub async fn` napi exports. For `open_database`, the VFS registration and `sqlite3_open_v2` call should happen inside `spawn_blocking` (since they trigger synchronous VFS callbacks that do `block_on` for KV I/O — this is safe from a non-tokio thread but not from the main thread).

```rust
#[napi(js_name = "openDatabase")]
pub async fn open_database(channel: &JsKvChannel, actor_id: String) -> Result<JsNativeDatabase> {
    let ch = channel.channel.clone();
    let aid = actor_id.clone();
    ch.open_actor(&aid).await.map_err(|e| Error::from_reason(e.to_string()))?;

    let rt_handle = get_runtime().handle().clone();
    let ch2 = channel.channel.clone();
    let aid2 = actor_id.clone();
    let native_db = get_runtime()
        .spawn_blocking(move || {
            let vfs_name = format!("kv-{aid2}");
            let kv_vfs = vfs::KvVfs::register(&vfs_name, ch2, aid2.clone(), rt_handle)?;
            vfs::open_database(kv_vfs, &aid2)
        })
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?
        .map_err(Error::from_reason)?;

    // ... construct JsNativeDatabase ...
}
```

Apply the same pattern to `close_database` and `disconnect`.

Also add to `rivetkit-typescript/packages/sqlite-native/CLAUDE.md` (or the project CLAUDE.md under the Native SQLite section):

```markdown
**Native SQLite napi exports must never block the Node.js event loop.**

All napi exports that perform async I/O (WebSocket round-trips, VFS callbacks that
trigger KV operations) must be `pub async fn`, not synchronous `pub fn` with
`rt.block_on(...)`. Synchronous `block_on` parks the calling thread (the Node.js
main thread), freezing the event loop. `block_on` is only safe from `spawn_blocking`
threads, which is how `execute`/`query` VFS callbacks work.
```

---

## H2. TypeScript Dev Manager Lacks Per-Actor Request Ordering

**File**: `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts`

**Problem**: `handleToServerMessage` dispatches requests as fire-and-forget Promises (`handleRequest(msg).catch(...)`). JS is single-threaded and messages arrive in order, but `handleRequest` is `async`. Here's what happens:

1. WS message 1 arrives: `KvPut(actor=X, journal chunk 0)`
2. JS calls `handleRequest(msg1)` — runs synchronously until the first `await`
3. `handleRequest` hits `await managerDriver.kvBatchPut(...)` — yields back to the event loop
4. WS message 2 arrives: `KvPut(actor=X, journal chunk 1)`
5. JS calls `handleRequest(msg2)` — runs synchronously until the first `await`
6. `handleRequest` hits `await managerDriver.kvBatchPut(...)` — yields back to the event loop
7. Now both I/O operations are in-flight concurrently. The OS decides which completes first.
8. If msg2's write resolves before msg1's, journal chunk 1 is written before chunk 0.
9. A crash at this point means a corrupt journal.

The event loop guarantees messages are **received** in order and handlers **start** in order, but `await` points create concurrent I/O — the OS decides completion order. The Rust engine server avoids this by routing requests into per-actor `mpsc` channels processed sequentially (one request fully completes before the next starts).

**Fix**: Add a per-actor request queue using chained Promises.

```typescript
// Module-level (or on the connection object):
const actorQueues = new Map<string, Promise<void>>();

function handleToServerMessage(conn: KvChannelConnection, managerDriver: ManagerDriver, msg: ToServer): void {
    switch (msg.tag) {
        case "ToServerRequest": {
            const actorId = msg.val.actorId;
            const prev = actorQueues.get(actorId) ?? Promise.resolve();
            const next = prev.then(() =>
                handleRequest(conn, managerDriver, msg.val).catch((err) => {
                    logger().error({
                        msg: "unhandled error in kv channel request handler",
                        error: err instanceof Error ? err.message : String(err),
                    });
                })
            );
            actorQueues.set(actorId, next);
            // Clean up completed entries to avoid map growth.
            next.then(() => {
                if (actorQueues.get(actorId) === next) {
                    actorQueues.delete(actorId);
                }
            });
            break;
        }
        case "ToServerPong":
            conn.lastPongTs = Date.now();
            break;
    }
}
```

---

## H3. Native VFS Missing `amt == 0` Guard

**File**: `rivetkit-typescript/packages/sqlite-native/src/vfs.rs`

**Problem**: `(offset + amt - 1) / CHUNK_SIZE` wraps to `usize::MAX` when `amt = 0`. The WASM VFS already has this guard (`vfs.ts:702-704` for xRead, `vfs.ts:796-798` for xWrite). The native VFS does not. This is NOT an issue in the WASM VFS — only the native VFS is missing the guard.

**Fix**: Add early return at the top of `kv_io_read` and `kv_io_write`, matching the WASM VFS.

In `kv_io_read` (after getting `file` and `ctx`):
```rust
if i_amt <= 0 {
    return SQLITE_OK;
}
```

In `kv_io_write` (after getting `file` and `ctx`):
```rust
if i_amt <= 0 {
    return SQLITE_OK;
}
```

---

## M1. Add Namespace Verification to `resolve_actor`

**Files**: `engine/packages/pegboard-kv-channel/src/lib.rs`, `engine/packages/pegboard/src/ops/actor/get_for_runner.rs`

**Problem**: `resolve_actor` looks up an actor by UUID via `get_for_runner` but never verifies the actor belongs to the authenticated connection's `namespace_id`. The `namespace_id` is only passed through to `actor_kv` for quota/metrics accounting. A client authenticated for namespace A could send KV requests for an actor in namespace B if it knows the UUID. The admin_token is a global credential so this is defense-in-depth, but it should still be enforced.

**Fix**:

1. Add `namespace_id` to the `get_for_runner` output in `engine/packages/pegboard/src/ops/actor/get_for_runner.rs`. Read the actor's namespace from the workflow state (same path that already reads `name` and `runner_id`).

2. Pass `namespace_id` into `resolve_actor` and verify:

```rust
async fn resolve_actor(
    ctx: &StandaloneCtx,
    actor_id: &str,
    expected_namespace_id: Id,
) -> std::result::Result<(Id, String), protocol::ResponseData> {
    let parsed_id = Id::parse(actor_id).map_err(|err| {
        error_response("actor_not_found", &format!("invalid actor id: {err}"))
    })?;

    let actor = ctx
        .op(pegboard::ops::actor::get_for_runner::Input { actor_id: parsed_id })
        .await
        .map_err(|err| internal_error(&err))?;

    match actor {
        Some(actor) => {
            // Verify the actor belongs to the authenticated namespace.
            // Uses the same generic error message to avoid leaking actor existence
            // across namespace boundaries.
            if actor.namespace_id != expected_namespace_id {
                return Err(error_response(
                    "actor_not_found",
                    "actor does not exist or is not running",
                ));
            }
            Ok((parsed_id, actor.name))
        }
        None => Err(error_response(
            "actor_not_found",
            "actor does not exist or is not running",
        )),
    }
}
```

3. Update all call sites of `resolve_actor` to pass `namespace_id`.

4. Add the same namespace verification to the TypeScript dev manager's KV channel handler (`kv-channel.ts`). Add a comment in both implementations explaining why the check exists:

```
// Defense-in-depth: verify the actor belongs to this connection's namespace.
// The admin_token is a global credential, so this is not strictly necessary
// today, but prevents cross-namespace access if a less-privileged auth
// mechanism is introduced in the future.
```

---

## M2. Add MAX_FILE_SIZE Enforcement to Native VFS

**File**: `rivetkit-typescript/packages/sqlite-native/src/vfs.rs`, `rivetkit-typescript/packages/sqlite-native/src/kv.rs`

**Problem**: WASM VFS enforces `MAX_FILE_SIZE_BYTES = (0xFFFFFFFF + 1) * CHUNK_SIZE` (16 TiB). Native VFS does not. Chunk indices are cast to `u32` — writes beyond `u32::MAX * 4096` wrap silently.

`MAX_FILE_SIZE_BYTES` is a hardcoded structural limit derived from the fact that chunk indices are encoded as `u32` in the KV key layout (8-byte key: prefix + file_tag + chunk_index_u32_be). Not user-configurable — it's an inherent property of the key encoding.

**Fix**: Add to `kv.rs`:
```rust
/// Maximum file size in bytes. Chunk indices are u32, so a file can span at most
/// 2^32 chunks at CHUNK_SIZE bytes each.
pub const MAX_FILE_SIZE: usize = (u32::MAX as usize + 1) * CHUNK_SIZE;
```

Add checks in `kv_io_write`:
```rust
let write_end = offset + amt;
if write_end > kv::MAX_FILE_SIZE {
    return SQLITE_IOERR_WRITE;
}
```

Add checks in `kv_io_truncate`:
```rust
if new_size as usize > kv::MAX_FILE_SIZE {
    return SQLITE_IOERR_TRUNCATE;
}
```

---

## M3. Sector Size Mismatch Between Native and WASM VFS

**File**: `rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts`

**Problem**: Native VFS returns `CHUNK_SIZE` (4096) from `xSectorSize`. WASM VFS inherits the base class default of 512.

Use 4096 (matching CHUNK_SIZE) in both. This tells SQLite to align I/O to chunk boundaries. NOT a breaking change — sector size affects journal write padding for crash recovery, but the KV layer provides atomic writes at the chunk level. Existing databases are unaffected because the page size is stored in the DB header, not derived from sector size.

**Fix**: Override `xSectorSize` in the `SqliteSystem` class in `vfs.ts`:

```typescript
xSectorSize(fileId: number): number {
    return CHUNK_SIZE;
}
```

---

## M4. Key Size Validation Bug in Engine KV Channel

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs`

**Problem**: Engine checks `key.len() > MAX_KEY_SIZE` (2048) but `actor_kv` internally wraps keys with a 2-byte `KeyWrapper` tuple prefix (NESTED prefix + NIL suffix) and checks `tuple_len <= MAX_KEY_SIZE`. So a key of 2047-2048 bytes passes the KV channel check but fails at `actor_kv::put`. The TS manager's `key.byteLength + 2 > 2048` check is correct.

**Fix**: Change the check in `validate_keys`:
```rust
fn validate_keys(keys: &[protocol::KvKey]) -> std::result::Result<(), protocol::ResponseData> {
    if keys.len() > MAX_KEYS {
        return Err(error_response(
            "batch_too_large",
            &format!("a maximum of {MAX_KEYS} keys is allowed"),
        ));
    }
    for key in keys {
        // +2 accounts for the KeyWrapper tuple packing overhead (NESTED prefix + NIL suffix)
        // added by actor_kv when storing the key. This must match the check in
        // engine/packages/pegboard/src/actor_kv/utils.rs.
        if key.len() + 2 > MAX_KEY_SIZE {
            return Err(error_response(
                "key_too_large",
                &format!("key is too long (max {} bytes)", MAX_KEY_SIZE - 2),
            ));
        }
    }
    Ok(())
}
```

---

## M5. Cache `resolve_actor` Result

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs`

**Problem**: Every KV operation (get, put, delete, delete_range) calls `resolve_actor` at lines 573, 650, 689, 730. This calls `pegboard::ops::actor::get_for_runner` which does a database read (multiple UniversalDB reads in parallel: WorkflowIdKey, NameKey, RunnerIdKey, ConnectableKey). The actor name is immutable for the lifetime of an actor. A single `SELECT` touching 10 pages generates 10 redundant DB lookups for the same data. The code at `get_for_runner.rs:18` has `// TODO: Add cache`.

**How the code works**: WebSocket messages are routed into per-actor `mpsc` channels. Each actor gets a dedicated `actor_request_task` (`lib.rs:401-440`) that runs a sequential `while let Some(req) = rx.recv().await` loop. This task is spawned once per actor per connection and lives until `ActorCloseRequest`. It's the ideal caching point — long-lived, sequential, naturally invalidated on close.

**Fix**: Cache the resolved actor in a local variable inside `actor_request_task` (lines 401-440 of `lib.rs`). This task is spawned once per actor per connection and processes requests sequentially via `while let Some(req) = rx.recv().await`. It naturally invalidates when the task exits (on `ActorCloseRequest` or connection drop). No explicit invalidation needed since actor name is immutable.

Resolve on the first KV request (not on `ActorOpenRequest`, since the actor might not exist at open time). Pass cached `(Id, String)` to KV handlers instead of calling `resolve_actor`. Refactor KV handlers to accept `(Id, &str)` directly instead of a raw `actor_id: &str`:

```rust
async fn actor_request_task(
    ctx: StandaloneCtx,
    state: Arc<KvChannelState>,
    ws_handle: WebSocketHandle,
    conn_id: Uuid,
    namespace_id: Id,
    open_actors: Arc<Mutex<HashSet<String>>>,
    mut rx: mpsc::UnboundedReceiver<protocol::ToServerRequest>,
) {
    // Cached actor resolution. Populated on first KV request, reused for all
    // subsequent requests. Actor name is immutable so this never goes stale.
    let mut cached_actor: Option<(Id, String)> = None;

    while let Some(req) = rx.recv().await {
        let is_close = matches!(req.data, protocol::RequestData::ActorCloseRequest);

        let response_data = match &req.data {
            // Open/close are lifecycle ops, don't need resolved actor.
            RequestData::ActorOpenRequest | RequestData::ActorCloseRequest => {
                handle_request(&ctx, &state, conn_id, namespace_id, &open_actors, &req).await
            }
            // KV ops: resolve once, cache, reuse.
            _ => {
                // Check actor is open on this connection.
                let is_open = open_actors.lock().await.contains(&req.actor_id);
                if !is_open {
                    error_response("actor_not_open", "actor is not opened on this connection")
                } else {
                    // Lazy-resolve and cache.
                    if cached_actor.is_none() {
                        match resolve_actor(&ctx, &req.actor_id, namespace_id).await {
                            Ok(v) => { cached_actor = Some(v); }
                            Err(resp) => {
                                // Send error response, continue processing.
                                // Don't cache — next request will retry.
                                send_response(&ws_handle, req.request_id, resp).await;
                                continue;
                            }
                        }
                    }
                    let (ref parsed_id, ref actor_name) = cached_actor.as_ref().unwrap();

                    let recipient = actor_kv::Recipient {
                        actor_id: *parsed_id,
                        namespace_id,
                        name: actor_name.clone(),
                    };

                    match &req.data {
                        RequestData::KvGetRequest(body) => {
                            handle_kv_get(&ctx, &recipient, body).await
                        }
                        RequestData::KvPutRequest(body) => {
                            handle_kv_put(&ctx, &recipient, body).await
                        }
                        RequestData::KvDeleteRequest(body) => {
                            handle_kv_delete(&ctx, &recipient, body).await
                        }
                        RequestData::KvDeleteRangeRequest(body) => {
                            handle_kv_delete_range(&ctx, &recipient, body).await
                        }
                        _ => unreachable!(),
                    }
                }
            }
        };

        send_response(&ws_handle, req.request_id, response_data).await;

        if is_close {
            break;
        }
    }
}
```

Refactor `handle_kv_get`, `handle_kv_put`, `handle_kv_delete`, `handle_kv_delete_range` to accept `&Recipient` instead of raw `actor_id: &str` + `namespace_id: Id`. Remove the `resolve_actor` call from each handler.

---

## M6. Fix `kv_io_write` Error Path (Both VFS Implementations)

**Files**: `rivetkit-typescript/packages/sqlite-native/src/vfs.rs`, `rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts`

**Problem**: Both implementations update `file.size` and set `file.meta_dirty = true` BEFORE the KV put call. If the put fails, the in-memory state is wrong. This is a bug in **both** WASM and native VFS.

**Fix (native, vfs.rs)**: Save old values and rollback on failure:
```rust
let old_size = file.size;
let old_meta_dirty = file.meta_dirty;

let new_size = std::cmp::max(file.size, write_end as i64);
if new_size != file.size {
    file.size = new_size;
    file.meta_dirty = true;
}

if file.meta_dirty {
    put_keys.push(file.meta_key.to_vec());
    put_values.push(encode_file_meta(file.size));
}

if ctx.kv_put(put_keys, put_values).is_err() {
    // Rollback in-memory state on failure.
    file.size = old_size;
    file.meta_dirty = old_meta_dirty;
    return SQLITE_IOERR_WRITE;
}

if file.meta_dirty {
    file.meta_dirty = false;
}
```

**Fix (WASM, vfs.ts)**: Same pattern:
```typescript
const oldSize = file.size;
const oldMetaDirty = file.metaDirty;

// ... update size/metaDirty as before ...

try {
    await options.putBatch(entriesToWrite);
} catch {
    file.size = oldSize;
    file.metaDirty = oldMetaDirty;
    return VFS.SQLITE_IOERR_WRITE;
}

if (file.metaDirty) {
    file.metaDirty = false;
}
```

---

## M7. Blob Serialization Performance

**File**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs`, `rivetkit-typescript/packages/rivetkit/src/db/native-sqlite.ts`

**Problem**: Blob data round-trips through `JsonValue::Array` of individual `JsonValue::Number` values. A 1 MB blob becomes ~20 MB of JSON numbers. This happens because bind parameters are heterogeneous (`null | int | float | text | blob`) and `serde_json::Value` was the expedient choice. On the TS side, `Array.from(uint8Array)` turns each byte into a JS number.

**Fix**: Replace `JsonValue` with a typed napi struct using `Buffer` for blobs:

```rust
#[napi(object)]
pub struct BindParam {
    /// "null" | "int" | "float" | "text" | "blob"
    pub kind: String,
    pub int_value: Option<i64>,
    pub float_value: Option<f64>,
    pub text_value: Option<String>,
    pub blob_value: Option<Buffer>,
}
```

On the TypeScript side (`native-sqlite.ts`), pass `Uint8Array` as `Buffer.from(arg)` instead of `Array.from(arg)`.

This is a larger change — can be done as a follow-up if blob-heavy workloads are not immediate.

---

## M8. Shared Rust Crate for KV Channel Protocol

**Files**: `engine/packages/pegboard-kv-channel/src/protocol.rs`, `rivetkit-typescript/packages/sqlite-native/src/protocol.rs`, `engine/sdks/typescript/kv-channel-protocol/src/index.ts`

**Purpose of each**:
- **Engine server** (Rust #1): Decodes requests FROM clients, encodes responses TO clients.
- **Native addon client** (Rust #2): Encodes requests TO the server, decodes responses FROM the server.
- **TS dev manager server** (TS): Same role as Rust #1, for local dev.

**Existing pattern**: The runner protocol uses a shared Rust SDK crate at `engine/sdks/rust/runner-protocol/` with a `build.rs` that generates Rust types from BARE schemas via `vbare_compiler`. The crate is consumed by 8 engine packages via `rivet-runner-protocol.workspace = true`. It also auto-generates the TypeScript SDK via `@bare-ts/tools`.

**Fix**: Follow the same pattern:

1. Create `engine/sdks/rust/kv-channel-protocol/` with:
   - `Cargo.toml` (with `serde_bare`, `serde` workspace deps)
   - `build.rs` using `vbare_compiler::process_schemas_with_config()` to generate Rust types from `engine/sdks/schemas/kv-channel-protocol/v1.bare`
   - `src/lib.rs` re-exporting generated types + `PROTOCOL_VERSION` constant
   - Auto-generate the TypeScript SDK from the BARE schema (replacing the hand-written `index.ts`)

2. Add `rivet-kv-channel-protocol` as a workspace dependency.

3. Both `pegboard-kv-channel` (engine server) and `rivetkit-sqlite-native` (native addon) depend on the shared crate instead of having their own `protocol.rs`.

4. The TS dev manager imports from the auto-generated `@rivetkit/engine-kv-channel-protocol` package.

---

## M9. Clean Teardown

**Files**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs`, `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts`, `rivetkit-typescript/packages/rivetkit/src/drivers/file-system/manager.ts`

**Problem**: The global tokio runtime prevents clean process exit if `disconnect()` is not called.

**Existing shutdown order** (already correct):
1. Actor `onStop()` runs lifecycle callbacks (onSleep/onDestroy)
2. `#waitForPendingDisconnects()` — ensures onDisconnect handlers complete
3. `stateManager.saveState()` + `waitForPendingWrites()` — flushes all KV writes through the WebSocket
4. `#cleanupDatabase()` — closes VFS and per-actor DB handle (`closeDatabase`)
5. Process exit → `disconnectKvChannel()` via `process.on('beforeExit')` / `SIGTERM` / `SIGINT`

**Critical constraint**: `disconnect()` MUST happen LAST, after all actors have flushed their final DB writes. The existing ordering is correct — actors close their individual databases (step 4) before the shared KV channel is disconnected (step 5). This must be preserved and clearly documented.

**Fix (native addon)**:
1. Make `disconnect()` async (per H1).
2. Don't use global `OnceLock<Runtime>` — store the runtime in `JsKvChannel` so it's dropped when the channel is dropped. Add a comment:
```rust
// The runtime is owned by JsKvChannel, not stored globally, so it is
// dropped when the channel is dropped. This ensures clean process exit
// after disconnect(). The runtime MUST NOT be dropped before all actors
// have closed their databases, because VFS callbacks need the runtime's
// Handle to perform KV I/O during final writes.
```

**Fix (FS driver)**: Add a `shutdown()` method to the manager driver interface. In the file-system driver, clear the `actorLocks` map and cancel the stale lock sweep timer. Call `shutdown()` from the registry's `dispose()` path, AFTER all actors have stopped.

---

## L1. TS KV Channel Error Leakage

**File**: `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts:189`

**Fix**: Replace raw error message with generic string:
```typescript
return makeErrorResponse(requestId, "internal_error", "internal error");
```

---

## L2. Unbounded Per-Actor Channels

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs:367`

**Fix**: Replace `mpsc::unbounded_channel()` with `mpsc::channel(64)`. When the channel is full, send a backpressure error:

```rust
let (tx, rx) = mpsc::channel(64);
// ...
match tx.try_send(req) {
    Ok(()) => {}
    Err(mpsc::error::TrySendError::Full(_)) => {
        tracing::warn!(?actor_id, "per-actor channel full, dropping request");
        // Send error response directly via ws_tx
    }
    Err(mpsc::error::TrySendError::Closed(_)) => {
        tracing::warn!(?actor_id, "per-actor task channel closed, removing dead entry");
        actor_channels.remove(&actor_id);
    }
}
```

---

## L3. Stmt Cache Mutex Scope

**File**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs`

**Fix**: Split the cache mutex acquisition into two phases — lookup and store — so the mutex is NOT held during `sqlite3_step` (which triggers VFS I/O):

```rust
// Phase 1: Check cache for existing statement.
let cached_stmt = {
    let mut cache = cache.lock().unwrap();
    cache.pop(&sql).map(|cs| cs.0)
};

let stmt = if let Some(s) = cached_stmt {
    unsafe { sqlite3_reset(s); sqlite3_clear_bindings(s); }
    s
} else {
    // Prepare new statement (no mutex held during VFS I/O).
    prepare_stmt(db_ptr, &sql)?
};

// Execute (no mutex held, VFS I/O happens here).
let rc = unsafe { sqlite3_step(stmt) };

// Phase 2: Return to cache.
{
    let mut cache = cache.lock().unwrap();
    cache.put(sql, CachedStmt(stmt));
}
```

---

## L4. `open_actor` Adds to Set Before Confirmation

**File**: `rivetkit-typescript/packages/sqlite-native/src/channel.rs`

**Fix**: Move `open_actors` insert after successful response:

```rust
pub async fn open_actor(&self, actor_id: &str) -> Result<ResponseData, ChannelError> {
    let result = self.send_request(actor_id, RequestData::ActorOpenRequest).await?;
    // Only track after server confirms the open.
    {
        let mut open = self.inner.open_actors.lock().await;
        open.insert(actor_id.to_string());
    }
    Ok(result)
}

pub async fn close_actor(&self, actor_id: &str) -> Result<ResponseData, ChannelError> {
    let result = self.send_request(actor_id, RequestData::ActorCloseRequest).await?;
    {
        let mut open = self.inner.open_actors.lock().await;
        open.remove(actor_id);
    }
    Ok(result)
}
```

---

## L6. Dead Actor Channel Cleanup

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs:382`

**Fix**: When `tx.send()` fails, remove the dead entry and send an error response:

```rust
if tx.send(req).is_err() {
    tracing::warn!(?actor_id, "per-actor task channel closed, removing dead entry");
    actor_channels.remove(&req.actor_id);
    let resp = error_response("internal_error", "actor task exited unexpectedly");
    // ... send resp via ws_tx ...
}
```

---

## L7. Weak PRNG in VFS Randomness Callback

**File**: `rivetkit-typescript/packages/sqlite-native/src/vfs.rs:820-841`

**Fix**: Replace LCG with `getrandom`:

```rust
unsafe extern "C" fn kv_vfs_randomness(
    _p_vfs: *mut sqlite3_vfs,
    n_byte: c_int,
    z_out: *mut c_char,
) -> c_int {
    vfs_catch_unwind!(0, {
        let buf = slice::from_raw_parts_mut(z_out as *mut u8, n_byte as usize);
        if getrandom::fill(buf).is_err() {
            return 0;
        }
        n_byte
    })
}
```

Add `getrandom` to `Cargo.toml` dependencies.

---

## L8. CLAUDE.md Update for Protocol Version Sync

**File**: `CLAUDE.md`

Already applied. Added under "Keep the KV API in sync" section:

```markdown
**Keep KV channel protocol versions in sync.**

When bumping the KV channel protocol version, update all four locations together:
- `engine/packages/pegboard-kv-channel/src/protocol.rs` (`PROTOCOL_VERSION`)
- `rivetkit-typescript/packages/sqlite-native/src/channel.rs` (`PROTOCOL_VERSION`)
- `engine/sdks/typescript/kv-channel-protocol/src/index.ts` (`PROTOCOL_VERSION`)
- `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts` (imported from protocol package)
```

---

## L9. Module-Level Global Lock Table

**File**: `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts:57`

**Problem**: `actorLocks` and `staleLockSweepTimer` are module-level globals. If two manager instances exist in the same process (e.g., tests), they share the same lock table. Lock state from one instance leaks into the other.

**Fix**: Move `actorLocks` and `staleLockSweepTimer` into a factory that returns them as part of a disposable object:

```typescript
export function createKvChannelManager() {
    const actorLocks = new Map<string, KvChannelConnection>();
    let staleLockSweepTimer: ReturnType<typeof setInterval> | null = null;

    function createHandler(managerDriver: ManagerDriver) {
        // ... use local actorLocks instead of module-level ...
    }

    function shutdown() {
        if (staleLockSweepTimer) {
            clearInterval(staleLockSweepTimer);
            staleLockSweepTimer = null;
        }
        actorLocks.clear();
    }

    return { createHandler, shutdown };
}
```

Wire `shutdown()` into the FS driver's teardown path (M9).

---

## Verification: Cross-Backend VFS Tests

Tests exist at `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/cross-backend-vfs.ts`. Both directions are covered:
- WASM writes -> native reads (with `PRAGMA integrity_check`)
- Native writes -> WASM reads (with `PRAGMA integrity_check`)

Both test multi-chunk payloads (8192 bytes across chunk boundaries). Tests are skipped when the native addon is not available.

**Action**: After implementing any fix in this spec that changes VFS behavior (H3, M2, M3, M6), run the cross-backend VFS tests to verify compatibility is preserved:

```bash
cd rivetkit-typescript/packages/rivetkit && pnpm test driver-file-system -t ".*Cross-Backend.*"
```
