# Native SQLite Implementation: Adversarial Review Findings

Adversarial review of `NATIVE_SQLITE_DATA_CHANNEL.md` focused on the concurrency model, data integrity, and threading behavior. Findings were then counter-reviewed by a second round of adversarial agents to eliminate false positives.

## Dismissed Findings

These were flagged by reviewers but are non-issues:

- **SQLite pager cache inconsistency after reconnect.** VFS IO errors are transient filesystem faults from SQLite's perspective. SQLite handles them the same way it handles NFS failures or power loss: abort the statement, attempt rollback, and if rollback also fails, the next operation retries. No special close/reopen logic needed. SQLite's journal recovery handles partial writes on re-open if the handle is ever recreated.
- **Non-atomic write perception (write succeeds server-side, client gets disconnect).** Same as above. SQLite treats the `SQLITE_IOERR_WRITE` as a failed write and rolls back. The "orphaned" server-side write is harmless because FDB transactions are atomic per batch, and SQLite's journal recovery is designed for exactly this crash scenario.
- **Shared VFS state race.** Each actor gets its own VFS instance with a unique name. No shared mutable state between actors. Non-issue by design.
- **Client-side mutex convoy on send_request.** Three sequential mutex acquisitions in `send_request` (`next_request_id`, `in_flight`, `outgoing_tx`) are each held for sub-microsecond durations (atomic increment, HashMap insert, mpsc send). With ~50 concurrent actors, total serialization overhead is ~0.1ms vs 1-10ms KV round-trips. Not measurable. Premature optimization.
- **requestId u32 overflow.** Original claim of "50 minutes to overflow" was wrong by 30x. At realistic throughput (~50k req/s with 50 actors), overflow takes ~24 hours. Collision probability at overflow is ~0.0000012% (50 in-flight entries / 4 billion). Counter also resets on every reconnect. Not worth worrying about.
- **xTruncate non-atomicity.** Self-contradictory finding: acknowledged that SQLite's journal recovery handles partial failure, then rated itself MEDIUM. The protocol has no multi-operation transaction type, so the suggested "batch" optimization is not achievable atomically. xTruncate is rarely called in normal workloads (only during VACUUM or auto_vacuum). Same pattern already ships in WASM VFS without issues.
- **SQLite SERIALIZED mode overhead.** "Global mutex" is a mischaracterization. In SERIALIZED mode (`SQLITE_THREADSAFE=1`), the relevant mutex is per-connection, not global. With separate connections per actor, it is never contended. Cost is ~25ns on a 1-10ms VFS operation (0.003%). The proposed fix (`SQLITE_OPEN_NOMUTEX`) removes a safety net for zero measurable gain and introduces a concurrency hazard: with `spawn_blocking`, two concurrent `Promise.all` queries on the same actor could race on the raw `sqlite3*` handle without the serialized-mode mutex protecting them.
- **WebSocket head-of-line blocking.** All actors share one WebSocket (one TCP connection). A large `KvPutRequest` (512 KiB) must be fully written before the next message begins. At datacenter bandwidth (10+ Gbps), this takes ~0.04ms. Negligible. Not worth fixing.

## Finding 1: napi exports must use `spawn_blocking` to avoid blocking the event loop

**Severity: MEDIUM**

The spec's "JavaScript Bindings" section (lines 330-361) shows TypeScript signatures that return synchronous values. If implemented as synchronous napi exports, they block the Node.js main thread during every KV round-trip, freezing the event loop and preventing parallelism between actors.

SQLite's C API is fundamentally synchronous. `xRead` must return bytes, not a future. There is no way to make VFS callbacks async in native code (the WASM VFS avoids this via Asyncify, which is a WASM-specific capability). Something must block. The idiomatic tokio approach is `spawn_blocking`, which runs the blocking SQLite work on a dedicated thread pool separate from the async worker pool:

```rust
#[napi]
pub async fn query(db_ref: ..., sql: String, params: Option<Vec<JsonValue>>) -> Result<QueryResult> {
    get_runtime().spawn_blocking(move || {
        // sqlite3_prepare_v2, bind, step loop
        // VFS callbacks call Handle::block_on() from this blocking thread (safe)
    }).await.map_err(|e| Error::from_reason(e.to_string()))?
}
```

This is one blocking thread per concurrent actor (not per request). SQLite is single-connection-per-actor, so 50 actors = 50 blocking threads from a pool of 512. The Node.js main thread returns a Promise and is free immediately. The tokio async worker pool (which runs the WebSocket read/write tasks) is never starved because `spawn_blocking` uses a separate pool.

### Design note: threading approaches considered

SQLite's C API is synchronous. `xRead` must return bytes, not a future. There is no way to make VFS callbacks async in native code. Three approaches were evaluated:

| Approach | How it works | Pros | Cons |
|---|---|---|---|
| `spawn_blocking` | napi `async fn` dispatches to tokio's blocking thread pool. VFS callbacks call `Handle::block_on()` from blocking threads. | Simplest (3 lines). Tokio manages pool. Idiomatic. | Thread may change between queries (slightly worse cache locality). |
| Dedicated thread per actor | One `std::thread` per actor, receives SQL via `mpsc` channel, sends results via oneshot. | `sqlite3*` stays on one thread (no `Send` needed). Best cache locality. | Manual lifecycle management. One idle thread per open actor. |
| Channel + block in place | Sync napi function, VFS callbacks send requests via `std::sync::mpsc` and block on `recv()`. | No tokio dependency for blocking. | Still blocks the Node.js main thread (same problem as today). Does NOT solve the core issue. |

`spawn_blocking` is the recommended approach. The dedicated thread model is viable but more implementation work for negligible performance difference vs 1-10ms KV round-trips. The channel approach does not solve the main-thread blocking problem.

### Spec update needed

Change the TypeScript signatures to return `Promise<...>` and add a note: "SQLite operations run on tokio's blocking thread pool via `spawn_blocking`. VFS callbacks call `Handle::block_on()` from blocking threads (not tokio worker threads), which is safe. The Node.js main thread is never blocked."

## Finding 2: Engine server should process requests for different actors concurrently

**Severity: MEDIUM**

The engine's KV channel server (`pegboard-kv-channel/src/lib.rs`, message_loop lines 246-300) processes each request fully before reading the next WebSocket message. If Actor A's `KvPutRequest` takes 50ms, Actor B's unrelated `KvGetRequest` waits behind it. The TypeScript manager server (`manager/kv-channel.ts`) already handles this correctly with fire-and-forget request dispatch (JS single-threading preserves intra-actor ordering naturally). The runner protocol (`ws_to_tunnel_task.rs:68-77`) has the same sequential pattern.

**Important:** The naive fix of `tokio::spawn` per request is WRONG. It breaks the spec's optimistic pipelining guarantee (lines 298-301): "WebSocket message ordering guarantees the server processes the open before the KV requests." It also breaks journal write ordering for the same actor.

### Correct approach

Per-actor channel routing. `HashMap<ActorId, mpsc::Sender<Request>>` maps each actor to a channel. Each actor has a spawned tokio task that drains its channel sequentially. When a request arrives, it is routed to the actor's channel (created on first request). Cross-actor requests execute concurrently. Intra-actor ordering is preserved by the channel FIFO. Tokio tasks are lightweight (a few hundred bytes each), so one per active actor is negligible.

A single actor never has multiple concurrent KV requests. VFS callbacks are sequential within a SQLite operation, and SQLite connections are single-threaded. The client physically cannot send a second KV request for the same actor until the first completes. Per-actor sequential processing matches the client's behavior exactly.

This only matters after Finding 1 is implemented (with sync napi exports, only one actor does VFS operations at a time anyway).

### TODO

The runner protocol's KV handling (`ws_to_tunnel_task.rs:68-77`, `pegboard-runner/src/ws_to_tunnel_task.rs`) has the same sequential message processing pattern. Apply the per-actor channel approach there too for cross-actor parallelism.

### Spec update needed

No spec change needed. The spec correctly states the ordering guarantee the client depends on. This is implementation guidance for the engine server.

## Finding 3: tokio thread pool exhaustion

**Severity: HIGH (resolved by Finding 1)**

VFS callbacks use `Handle::block_on()` which parks the calling thread. If more threads are simultaneously blocked than there are tokio worker threads, the WebSocket read loop (which dispatches responses) cannot get a thread, and all blocked callers wait until the 30s timeout.

### Resolution

Resolved by Finding 1. `spawn_blocking` uses a separate, auto-growing thread pool (default cap 512) that does not consume tokio worker threads. The WebSocket read/write tasks run on the tokio async worker pool and are never starved.

## Finding 4: Engine should unconditionally evict old locks on ActorOpenRequest

**Severity: HIGH**

On reconnect, the client re-sends `ActorOpenRequest` for previously open actors. If the old connection hasn't been cleaned up yet (server hasn't detected the disconnect within the 15s ping timeout), the re-open fails with `actor_locked`. Every subsequent KV request for that actor fails.

### Design: Immediate unconditional eviction (engine-side)

When an `ActorOpenRequest` arrives and the actor is locked by a different connection, the engine unconditionally transfers the lock to the new connection. No signaling, no waiting, no negotiation.

**Changes to `pegboard-kv-channel/src/lib.rs`:**

1. In `handle_actor_open` (line 405): if the actor is locked by a different `conn_id`, evict the old connection and grant the lock to the new one.

2. Store a reference to each connection's `open_actors` set in the lock map so eviction can invalidate the old connection's fast-path check. Change `actor_locks` from `HashMap<String, u64>` to `HashMap<String, (u64, Arc<Mutex<HashSet<String>>>)>`.

3. On eviction, remove the actor from the old connection's `open_actors` set via the stored `Arc` reference. This makes the eviction take effect immediately on the old connection's next KV request without needing to check the global lock map on every operation.

**How the eviction works step by step:**

Currently the KV operation auth check (line 368) is a fast local check:
```rust
let is_open = open_actors.lock().await.contains(&req.actor_id);
```
This checks only the per-connection `HashSet`, no global lock needed. After eviction, the old connection's `open_actors` no longer contains the actor (we removed it in step 3), so this check fails immediately. The old connection then hits the slow path which checks `actor_locks` and returns `actor_not_open`.

The new connection's `open_actors` has the actor (inserted during `handle_actor_open`), so its KV requests pass the fast-path check with zero overhead.

The global `actor_locks` mutex is only acquired during `handle_actor_open` and `handle_actor_close`, never on the KV hot path.

**Why this is safe (same server):**
- Locks are in-memory only. No FDB state involved. No distributed coordination needed.
- The old connection is either dead (network issue) or stale (same process reconnecting). In both cases the new connection should win.
- After eviction, the old connection's in-flight FDB transactions complete normally (FDB is stateless with respect to locks). Its next KV request fails with `actor_not_open`, surfacing as `SQLITE_IOERR` to SQLite, which handles it gracefully.
- Single-writer safety is maintained: `actor_locks` is protected by a `Mutex`, so the transfer is atomic. The old connection's `open_actors` removal happens under the same lock scope, so there is no window where both connections pass the auth check.

**Multi-server (horizontally distributed) case:**

In production, multiple engine instances (2-10 pods) run behind a Kubernetes Service with no sticky sessions. On reconnect, the client may be routed to a different engine pod.

- **Reconnect hits different pod:** New pod has no lock for the actor. `ActorOpenRequest` succeeds immediately. Old pod still holds a stale lock. This is safe because:
  - The old connection is dead from the client's perspective. The client failed all in-flight requests and is no longer sending on the old WebSocket.
  - The only risk is an in-flight FDB transaction on the old pod that was sent before disconnect and hasn't completed. FDB transactions complete in sub-10ms, and the new connection waits for `ActorOpenResponse` before sending KV requests, so there is no overlap.
  - The old pod detects the disconnect within 15s (ping timeout) and cleans up the stale lock. No new requests arrive on the old connection during this window.
  - Single-writer at the actor scheduling level (one process per actor) is the primary mechanism. The KV channel lock is defense-in-depth, not the sole protection.

- **Two different processes opening the same actor on different pods:** Should not happen. Actor scheduling ensures one process per actor. If it does happen (bug), the KV channel lock on each pod is independent, so both would succeed. This is a scheduling bug, not a KV channel bug. The KV channel lock protects against duplicate connections from the same process, not cross-process conflicts.

**No distributed lock needed.** The in-memory per-pod lock is sufficient because the reconnect window is harmless (dead client, no new requests) and cross-process conflicts are prevented at the scheduling layer.

**Client-side change:** On reconnect, wait for `ActorOpenResponse` before pipelining KV requests. The initial open (first connection) can remain optimistic.

### Spec update needed

Add to the "Single-Writer Lock Semantics" section:

> When an `ActorOpenRequest` arrives for an actor locked by a different connection on the same server, the server unconditionally transfers the lock to the new connection. The old connection's subsequent KV requests fail with `actor_not_open`. This handles reconnection scenarios where the server hasn't detected the old connection's disconnect yet.
>
> In multi-server deployments, locks are per-server-instance. If a client reconnects to a different server, the new server has no prior lock and the open succeeds immediately. The old server cleans up the stale lock on disconnect detection. Cross-process single-writer is enforced by actor scheduling, not the KV channel lock.
>
> On reconnect, the client must wait for `ActorOpenResponse` before sending KV requests for each actor.

## Summary

| # | Finding | Severity | Spec update needed? |
|---|---------|----------|-------------------|
| 1 | napi exports must use `spawn_blocking` | MEDIUM | Yes |
| 2 | Engine per-actor concurrent processing | MEDIUM | No (implementation guidance) |
| 3 | tokio thread pool exhaustion | HIGH | Resolved by #1 |
| 4 | Unconditional lock eviction on open | HIGH | Yes |
