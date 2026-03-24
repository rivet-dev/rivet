# Adversarial Review: Native SQLite KV Channel PR

Findings from 5 independent adversarial review agents (Security, Concurrency, Protocol Compatibility, Reliability, Architecture), deduplicated, then validated by adversarial counter-agents that read the actual source code to disprove each claim.

**48 raw findings -> 22 unique issues -> 19 validated (3 disproved)**

---

## CRITICAL

### C1. `close_database` Races with In-Flight Operations -> Use-After-Free

**File**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs:448-463`

`close_database` is synchronous and drops the `sqlite3` handle immediately via `db.take()`. But `execute`/`query` extract `db_ptr` as a plain `usize` (breaking Rust's borrow safety) before dispatching to `spawn_blocking`. If `close_database` runs between extraction and the blocking task starting, the task uses a dangling pointer. No borrow-checker protection since the pointer is laundered through `usize`.

**Fix**: Hold `NativeDatabase` in an `Arc` and clone into each `spawn_blocking` closure, or gate close behind a ref-counted guard that waits for in-flight ops.

---

## HIGH

### H1. `open_database`/`close_database`/`disconnect` Block Node.js Event Loop

**File**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs:174-205, 448-463, 467-471`

These are synchronous napi functions that call `rt.block_on(...)`, blocking the Node.js main thread for the duration of WebSocket round-trips (1-10ms normally, up to 30s on timeout). `execute`/`query` correctly use `spawn_blocking`, but lifecycle functions do not. These are one-time lifecycle operations, not per-query hot paths, which moderates the severity.

**Fix**: Make `open_database`, `close_database`, and `disconnect` async napi exports.

---

### H2. TypeScript KV Channel Server Lacks Per-Actor Request Ordering

**File**: `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts:517-529`

Requests are dispatched as fire-and-forget Promises. After the first `await` inside each handler, multiple requests for the same actor can interleave, breaking journal write ordering required for SQLite crash recovery. The Rust engine server correctly uses per-actor sequential channels. This only affects the TypeScript local dev manager, not the production engine.

**Fix**: Implement per-actor request queuing (e.g., `Map<string, Promise<void>>` with chained Promises).

---

### H3. Native VFS Missing `amt == 0` Guard -> Integer Underflow

**File**: `rivetkit-typescript/packages/sqlite-native/src/vfs.rs:266, 338`

`(offset + amt - 1) / CHUNK_SIZE` wraps to `usize::MAX` when `amt = 0`, causing a loop over trillions of iterations -> OOM/hang. The WASM VFS guards against this; the native VFS does not. SQLite's documentation says xRead/xWrite are always called with positive amounts, so this is a defensive programming issue, but the WASM VFS has the guard and the native VFS should match.

**Fix**: Add `if i_amt <= 0 { return SQLITE_OK; }` at the top of `kv_io_read` and `kv_io_write`.

---

## MEDIUM

### M1. Cross-Namespace Actor Access (Defense-in-Depth Gap)

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs:754-780`

`resolve_actor` looks up actor by ID via `get_for_runner` but never verifies the actor belongs to the authenticated connection's `namespace_id`. The namespace is only used for metrics. However, the KV channel authenticates via the engine's `admin_token` (a global credential), so any authenticated client already has full engine access. Actor IDs are UUIDs that cannot be enumerated. This is a defense-in-depth gap, not an exploitable vulnerability today, but becomes real if a less-privileged auth mechanism is introduced.

**Fix**: After resolving the actor, verify `actor.namespace_id == connection.namespace_id`. The `get_for_runner` op should return namespace_id for this check.

---

### M2. Missing Maximum File Size Enforcement in Native VFS

**File**: `rivetkit-typescript/packages/sqlite-native/src/vfs.rs` (kv_io_write)

WASM VFS enforces `MAX_FILE_SIZE_BYTES`; native VFS does not. Chunk indices are cast to `u32` -- writes beyond `u32::MAX * 4096` (16 TiB) wrap silently, producing colliding chunk keys. However, the per-actor storage quota is 10 GiB, making the 16 TiB threshold practically impossible to reach. Trivial defensive fix.

**Fix**: Add `MAX_FILE_SIZE` constant matching WASM and validate before writing.

---

### M3. Sector Size Mismatch: Native Returns 4096, WASM Inherits 512

**File**: `rivetkit-typescript/packages/sqlite-native/src/vfs.rs:616-618` vs WASM VFS

Native VFS `xSectorSize` returns 4096. WASM VFS inherits the default of 512 from wa-sqlite base class. This violates the documented "must match 1:1" requirement. Practical impact is limited since both use the same journal mode and PRAGMA settings.

**Fix**: Add explicit `xSectorSize` returning 4096 to the WASM VFS to match native.

---

### M4. Key Size Validation Bug in Engine KV Channel

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs` vs `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts`

Engine KV channel checks `key.len() > 2048`. TypeScript manager checks `key.byteLength + 2 > 2048`. The TS manager's `+ 2` accounts for `KeyWrapper` tuple packing overhead and is correct (matches `actor_kv/utils.rs`). The engine KV channel server has the bug: a key of 2047-2048 bytes passes the KV channel check but fails when it reaches `actor_kv::put`.

**Fix**: Add `+ 2` to the engine KV channel server's check (NOT remove it from the TS manager).

---

### M5. `resolve_actor` Does a DB Lookup on Every KV Op (No Caching)

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs:573-730`

Every KV operation calls `resolve_actor`, which hits the database for the same immutable actor name every time. A single `SELECT` touching 10 pages generates 10 DB lookups for the same data. A `// TODO: Add cache` comment exists in `get_for_runner.rs:18`.

**Fix**: Cache the `(actor_id, name)` mapping after the first `ActorOpenRequest`.

---

### M6. `kv_io_write` Updates `file.size` Before `kv_put` Succeeds

**File**: `rivetkit-typescript/packages/sqlite-native/src/vfs.rs:42-55, 425-437`

The original finding focused on `catch_unwind` + `AssertUnwindSafe` not restoring invariants after a panic, but panics in VFS callbacks are extremely unlikely. The bigger issue (discovered during validation) is the normal error path: `file.size` is updated at line 425 before `kv_put` at line 437. If `kv_put` fails, the in-memory size is wrong and metadata is marked dirty with incorrect data.

**Fix**: Update `file.size` only after successful `kv_put`. Consider also marking the file as "poisoned" after a caught panic.

---

### M7. Blob Data Round-Trips Through JSON Array (1 Number Per Byte)

**File**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs:561-585`, `rivetkit-typescript/packages/rivetkit/src/db/native-sqlite.ts:209-218`

Blob data is serialized as `JsonValue::Array` of `JsonValue::Number` -- one JSON number per byte. A 1 MB blob becomes ~20 MB of JSON representation. Real performance issue.

**Fix**: Use napi's `Buffer` type for blob parameters/return values instead of JSON arrays.

---

### M8. Three Independent Protocol Implementations with No Shared Code

**Files**: `engine/packages/pegboard-kv-channel/src/protocol.rs`, `rivetkit-typescript/packages/sqlite-native/src/protocol.rs`, `engine/sdks/typescript/kv-channel-protocol/src/index.ts`

Three fully independent BARE encode/decode implementations. Adding a new KV operation requires synchronized changes in three places with no compiler-enforced consistency. Cross-language byte tests exist for the native client but not for the engine server. This is a maintainability concern, not a security issue.

**Fix**: Extract a shared Rust crate for the two Rust implementations. Generate the TypeScript codec from the BARE schema (as done for runner protocol), or add CI byte-compatibility tests.

---

### M9. Global Tokio Runtime Prevents Clean Node.js Process Exit

**File**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs:78-93`

`OnceLock<Runtime>` spawns worker threads that never shut down. `KvChannel::connect` spawns an indefinite `connection_loop`. The process hangs after all JS work completes IF `disconnect()` is not called. The `disconnect()` function does properly shut down the connection loop; the issue is that forgetting to call it is easy.

**Fix**: Mark runtime threads as daemon, or add a shutdown hook. At minimum, document that `disconnect()` must be called.

---

## LOW

### L1. TS KV Channel Leaks Internal Error Messages to Clients

**File**: `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts:187-200`

The `handleRequest` catch block sends the raw `err.message` back to the client. The engine-side deliberately redacts internal errors. However, this is the local dev manager, not a production server.

**Fix**: Log the full error server-side and return a generic message, matching the engine's pattern.

---

### L2. Unbounded Per-Actor Channels -- No Backpressure

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs:367`

Per-actor request channels use `mpsc::unbounded_channel()`. However, WebSocket/TCP transport provides implicit backpressure, and the per-connection actor limit (1000) plus authentication requirement limit the attack surface.

**Fix**: Use a bounded channel (e.g., `mpsc::channel(64)`) and reject overflow.

---

### L3. Stmt Cache Mutex Held Across Blocking VFS I/O

**File**: `rivetkit-typescript/packages/sqlite-native/src/lib.rs:245, 299`

`std::sync::Mutex` on `stmt_cache` is held for the entire `sqlite3_step`, which triggers VFS callbacks doing `block_on(WebSocket I/O)`. However, the mutex is per-database (not global), and concurrent operations on the same actor database are serialized by SQLite's single-writer model anyway.

**Fix**: Hold the mutex only for cache lookup and final `cache.put`, releasing it before `sqlite3_step`.

---

### L4. `open_actor` Adds to Set Before Server Confirms

**File**: `rivetkit-typescript/packages/sqlite-native/src/channel.rs:243-250`

Actor is added to `open_actors` before `send_request` completes. If the server rejects the open, the actor is still tracked and re-opened on reconnect. Self-heals on next reconnect (lines 437-441 remove failed actors). Worst case is one wasted re-open request.

**Fix**: Only add to `open_actors` after successful response.

---

### L5. Global Actor Lock Table Has No Capacity Bound

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs:48`

`actor_locks` HashMap grows without bound across all connections. However, actor locks are evicted/overwritten when a new connection opens the same actor. Each entry is tiny (string + UUID + Arc pointer). Per-connection limit exists and auth is required. Memory impact is minimal.

**Fix**: Consider adding a global capacity limit and/or validating actor existence before acquiring a lock.

---

### L6. Dead Actor Channel Entries Not Cleaned Up

**File**: `engine/packages/pegboard-kv-channel/src/lib.rs:382`

If a per-actor task exits, its channel entry remains dead. But the same WebSocket failure that killed the task kills the connection loop, which clears all entries. The "30s timeout" scenario is nearly impossible in practice.

**Fix**: On `tx.send().is_err()`, remove the dead entry and return an error.

---

### L7. Weak PRNG (LCG) for VFS Randomness Callback

**File**: `rivetkit-typescript/packages/sqlite-native/src/vfs.rs:820-841`

LCG seeded from `SystemTime::now().as_nanos()`. Fully predictable. However, SQLite's xRandomness is not security-sensitive, and the WASM VFS doesn't even implement it.

**Fix**: Use `getrandom::getrandom()` instead of hand-rolled LCG.

---

### L8. `PROTOCOL_VERSION` Duplicated in 4 Places

**Files**: `pegboard-kv-channel/src/protocol.rs`, `sqlite-native/src/channel.rs`, `kv-channel-protocol/src/index.ts`, `kv-channel.ts`

This is a wire protocol negotiation constant, not a release version. It should NOT be added to `scripts/release/update_version.ts` (that would incorrectly auto-bump it). The runner protocol version is handled the same manual way.

**Fix**: Share the constant from a common crate between the two Rust implementations. Leave the release script alone.

---

### L9. Module-Level Mutable Global Lock Table (TS)

**File**: `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts:57-58`

`actorLocks` is a module-level global, which could break test isolation. However, no existing tests exercise this code path. Dev-only code.

**Fix**: Move `actorLocks` into the connection factory or expose a `resetLockState()` for tests.

---

## INFO

### I1. Auth is Optional Based on Config

**File**: `engine/packages/guard/src/routing/kv_channel.rs:54`

Intentional design for local dev environments. Identical pattern used by the runner protocol.

---

### I2. Token Transmitted in URL Query String

**File**: `rivetkit-typescript/packages/sqlite-native/src/channel.rs:525-536`

The `admin_token` is passed as `?token=...` in the WebSocket URL. Consistent with existing runner protocol pattern.

---

### I3. `get_chunk_key_range_end` Will Panic if `file_tag` Reaches 0xFF

**File**: `rivetkit-typescript/packages/sqlite-native/src/kv.rs:71`

`file_tag + 1` overflows if `file_tag == 0xFF`. Unreachable today (max tag is 0x03, constrained by `resolve_file_tag()`). Latent bug.

---

## Priority Recommendation

**Before merge** (confirmed bugs):
- C1: Use-after-free in `close_database`
- H1: Lifecycle functions block event loop
- H2: TS dev server lacks request ordering
- H3: Integer underflow on amt=0
- M3: Sector size mismatch violating 1:1 requirement
- M4: Key size check bug in engine KV channel (fix is opposite of initial suggestion)

**Fast follows**:
- M1: Add namespace check for defense-in-depth
- M2: Add MAX_FILE_SIZE guard (trivial fix)
- M5: Cache `resolve_actor` result (performance)
- M6: Fix normal error path in `kv_io_write`
- M7: Fix blob serialization perf
- M9: Document or automate `disconnect()` requirement

**Backlog**:
- M8, L1-L9, I1-I3

---

## Disproved Findings

The following findings from the initial review were disproved during validation and removed:

1. **"No WebSocket message size limit"**: Tungstenite's default `max_message_size` of 64 MiB applies. There IS a limit.
2. **"Reconnect request ID collision"**: `fail_all_in_flight` IS called before request ID reset. The race window is vanishingly narrow and self-cleans.
3. **"Add PROTOCOL_VERSION to release script"**: This is a wire protocol constant, not a release version. Auto-bumping it would be incorrect.
