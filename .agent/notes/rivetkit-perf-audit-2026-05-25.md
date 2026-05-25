# RivetKit performance + bug audit — 2026-05-25

Five parallel sub-agent audits of `rivetkit-core`, `rivetkit-typescript`, `envoy-client`,
and `pegboard-envoy`. Focus: actor creation rate, network messaging, KV, SQLite, and
general concurrency hot-path hygiene.

This report ranks findings by impact and groups them as **Bugs** (definite correctness
issues), **Bottlenecks** (correct but slow / wasteful), and **Smells** (worth eyeballing).
Each finding has a file:line reference.

---

## 0. Top-priority fixes (impact-ranked)

These are the highest-leverage items found across all five audits. Each was flagged by
at least one agent; the ones in bold were flagged by two or more.

1. **`StdMutex<HashMap<...>>` on `SharedContext`** — `engine/sdks/rust/envoy-client/src/context.rs:27-31`.
   `actors`, `live_tunnel_requests`, `pending_hibernation_restores` all use
   `Arc<std::sync::Mutex<HashMap<...>>>`. Direct violation of the root `CLAUDE.md`
   ("Never use `Mutex<HashMap<...>>`"). Held during every actor add/remove, every
   tunnel WS open/close, every `is_request_live` check, every sleep gating call.
   Caps actor creation rate per envoy and adds latency to every WS frame. Convert
   to `scc::HashMap`.

2. **`ws_tx.lock().await` on every outbound WebSocket send** —
   `engine/sdks/rust/envoy-client/src/connection/mod.rs:125-147`,
   plus `kv.rs:39-42, 96-100`, `sqlite.rs:91,119,280,302`. Every KV / SQLite / tunnel
   write acquires a `tokio::sync::Mutex<Option<UnboundedSender>>` and many sites also
   acquire it a second time just to peek `is_some()`. The team already instruments
   `ws_tx_lock_wait_duration_seconds` and `ws_tx_lock_hold_duration_seconds` — proof
   contention is real. Replace with `arc_swap::ArcSwapOption<UnboundedSender>` (or
   a `tokio::sync::watch`) for lock-free fast path.

3. **`BufferMap` String key allocation per WS request** —
   `engine/sdks/rust/envoy-client/src/utils.rs:210-262`. Every insert/get/remove
   calls `cyrb53(buffers)` then `format!("{result:x}")` to build a `String` key from
   an 8-byte hash. Sits on **every** inbound tunnel WS frame and request dispatch.
   The natural key is `[u8; 8]` or `u64` — `engine/sdks/rust/envoy-client/src/tunnel.rs:7-12`
   already builds the right key shape and `pegboard-envoy/src/conn.rs:42` already
   uses `scc::HashMap<(GatewayId, RequestId), ()>` correctly. Easiest big win.

4. **Whole-state blob rewrite on every save** —
   `rivetkit-rust/packages/rivetkit-core/src/actor/state.rs:281-379, 714-746`. A
   single `StateDelta::ActorState(bytes)` rewrites the *entire* encoded
   `PersistedActor` (including `scheduled_events`, `input`, `has_initialized`, etc.)
   into the single `[1]` key. No diff/dirty tracking on user state. For a 100 KB
   state where 4 bytes changed: full re-serialize in user JS + full NAPI copy +
   full BARE encode + full 100 KB `kv.put` round-trip + full FDB chunked write
   (10 KB rows). Also: any actor whose serialized state exceeds 128 KiB
   (`MAX_VALUE_SIZE`) fails to save with no automatic chunking. Split user-state
   into a separate sub-key from `scheduled_events`/etc., or move heavy state to
   SQLite.

5. **No prepared-statement cache** —
   `engine/packages/depot-client/src/query.rs:23, 65, 118, 192`. Every query reaches
   `sqlite3_prepare_v2` + `sqlite3_finalize`. A Drizzle / ORM workload re-runs the
   same `SELECT … WHERE id = ?` thousands of times paying parse + plan every time.
   LRU cache on the single-threaded worker (no sync needed) is the largest steady-state
   throughput improvement available for SQLite.

6. **Per-remote-call FDB validation transaction** —
   `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs:853-998`. Every remote
   SQL request (sqlite-over-tunnel) runs `validate_remote_sqlite_actor` →
   `validate_remote_sqlite_generation` which opens a UDB tx with a range scan over
   the per-actor command subspace. Dominant per-request cost for any chatty client.
   `SQLITE_OPTIMIZATIONS.md` already tracks this as a TODO; cache the result for the
   duration of the active generation.

7. **`setInterval(() => 60_000)` bug** —
   `rivetkit-typescript/packages/rivetkit/src/client/actor-conn.ts:238`. The callback
   is `() => 60_000` and the delay argument is **missing**, so it defaults to ~0 ms.
   The interval fires ~1000×/sec with a no-op callback and **never does what the
   field name `keepNodeAliveInterval` suggests**. Should be:
   `setInterval(() => {}, 60_000)`, probably with `.unref?.()` too.

8. **JSON→JsonValue→CBOR transcode on every HTTP action** —
   `rivetkit-rust/packages/rivetkit-core/src/registry/http.rs:130-247, 846-872`.
   Both JSON and CBOR encoded clients go through
   `parse → JsonValue tree → encode_json_as_cbor → CBOR bytes` before reaching the
   actor. CBOR-encoded inputs round-trip through JsonValue for nothing. Reply path
   has the symmetric reverse transcode (`http.rs:910-940`). Per-action cost.

9. **Duplicate startup state save for fresh actors** —
   `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs:1132-1138` vs `1169-1172`.
   On `is_new`, `persist_state(immediate=true)` runs twice during startup — once
   after `set_has_initialized`, again after `spawn_run_handle`. The second write
   supersedes the first. Wasted KV round-trip on every fresh-actor cold path.

10. **`record_inbox_depths` runs every actor loop iteration** —
    `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs:447, 460, 474, 1827-1837`.
    Three Prometheus `with_label_values` calls (each takes a read-lock on the metric's
    internal RwLock) per actor per select-loop iteration. There's even a TODO note
    next to the call: "Sample inbox depths periodically instead." For 10k actors at
    1k events/sec, this alone is 30M lock acquisitions per second.

---

## 1. Bugs (correctness)

### Network / protocol

- **`convert_same_bytes` uses forbidden bytes-roundtrip pattern** —
  `engine/sdks/rust/envoy-protocol/src/versioned.rs:7-21`. Defines
  `serde_bare::from_slice(&serde_bare::to_vec(&message)?)` for cross-version
  conversions. Explicitly prohibited by `engine/CLAUDE.md` rule 6a ("No
  `serde_bare::to_vec` + `from_slice` shortcuts"). Used by dozens of versioned
  converters. Only fires on version mismatch but every wire-schema change is a
  footgun.

- **JSON-in-BARE for envoy metadata** —
  `engine/sdks/rust/envoy-client/src/connection/mod.rs:57, 67`. `metadata` ships
  as a JSON `String` field inside the BARE envelope, and
  `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs:672-691` re-parses
  with `serde_json::from_str(...).unwrap_or_default()`. Violates "CBOR/BARE at
  boundaries."

- **`processed_command_idx` clear race** —
  `engine/sdks/rust/envoy-client/src/commands.rs:120-133`. Existing TODO.

- **`KvRequestEntry` retains full payload for replay; re-application is not safe** —
  `engine/sdks/rust/envoy-client/src/kv.rs:8-14, 96-116`. On reconnect,
  `process_unsent_kv_requests` re-sends puts/deletes that may have already been
  applied by the engine before the disconnect. The 30-second
  `cleanup_old_kv_requests` tick only frees memory; it does not dedupe.
  Compare with SQLite's `fail_sent_remote_sqlite_requests_with_indeterminate_result`
  (`envoy.rs:394`) which correctly fails sent-but-unanswered requests on disconnect.
  KV has no equivalent.

- **TOCTOU on tunnel-handler actor lookup** —
  `engine/sdks/rust/envoy-client/src/tunnel.rs:77, 175`.
  `ctx.get_actor(&actor_id, None).unwrap()` after a `has_actor` check. If the
  actor is removed between the two, the tunnel handler panics.

### State / persistence

- **Mixed put+delete state save is not atomic across the wire** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/state.rs:343-357`. Existing
  `// TODO: Make this atomic`. A crash between `apply_batch(puts)` and
  `apply_batch(deletes)` leaves persisted state with stale or extra connection
  records. Protocol needs `KvApplyBatchRequest { puts, deletes }`.

- **No early payload-size enforcement client-side for KV** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/kv.rs:218-243`. Sends arbitrary
  values and lets pegboard-envoy reject after the round-trip. Add a `MAX_PUT_PAYLOAD_SIZE`
  check before sending.

### Memory / leaks

- **`std::mem::forget(old)` is not actually bounded** —
  `rivetkit-typescript/packages/rivetkit-napi/src/actor_context.rs:787-789, 802-804`.
  Comment claims "bounded to one JsObject per actor wake cycle" but it grows
  monotonically with wake count over the process lifetime. Long-lived serverless
  hosts accumulate forgotten JsObjects without bound. Route cleanup through a
  TSF call so the `Ref` can be unrefed with an `Env`.

### TS lifecycle

- **`setInterval` missing delay argument** — see top-10 item #7.

- **50 ms polling loop in actor-driver shutdown** —
  `rivetkit-typescript/packages/rivetkit/src/drivers/engine/actor-driver.ts:865-868`.
  `while (this.#actors.size > 0 && Date.now() < deadline) await setTimeout(50)`.
  Should be a Promise resolved at the matching decrement-to-zero site.

- **`#waitForIdleSleepWindow` polls 25 ms** —
  `rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts:2284-2304`.

---

## 2. Bottlenecks

### Network messaging

- **Per-message allocation count: ~15 heap allocs for one HTTP action** (smallest body).
  Headers re-collected three times (`HashableMap → HashMap → HashMap`), body cloned
  3-4 times. Trace at `engine/sdks/rust/envoy-client/src/actor.rs:541-546, 779`
  and `rivetkit-rust/packages/rivetkit-core/src/registry/http.rs:130-247`.

- **One frame per actor event; no batching** —
  `engine/sdks/rust/envoy-client/src/actor.rs:384-392`. `send_event` always
  allocates `vec![EventWrapper { ... }]` for one event. `envoy_loop` has no
  batching for `SendEvents`. Use the `recv_many` pattern that
  `pegboard-envoy/src/actor_event_demuxer.rs:127-143` already uses.

- **No TCP_NODELAY on actor-side WS socket** —
  `engine/sdks/rust/envoy-client/src/connection/native.rs:118`. With Nagle on,
  small per-message frames (every event = ~50-200 bytes) wait up to 40 ms for
  ACK. Compounds with the no-batching issue above. Engine guard side enables
  NODELAY (`engine/packages/guard-core/src/server.rs:99`); actor side does not.

- **`reqwest::Client::builder().build()` per call** —
  `rivetkit-rust/packages/rivetkit-core/src/registry/runner_config.rs:25-27`,
  `rivetkit-rust/packages/rivetkit-core/src/engine_process.rs:236-238, 319-321`.
  Fresh connection pool + TLS roots + DNS resolver per call. Share a process-wide
  `OnceCell<Arc<Client>>`.

- **All inbox channels unbounded** — CLAUDE.md mandates bounded inboxes with
  `try_reserve` + `actor.overloaded`. Currently:
  `rivetkit-rust/packages/rivetkit-core/src/registry/mod.rs:647-649` and
  `engine/sdks/rust/envoy-client/src/{actor.rs:134, envoy.rs:296,
  connection/native.rs:121}`. The `actor.overloaded` error code doesn't exist
  anywhere in the tree. Either the rule needs to be retired or every inbox needs
  bounded refit.

- **Single WS write task → head-of-line blocking** with
  `envoy_max_response_payload_size = 20 MiB`. A bulk SQLite commit response
  blocks subsequent small KV responses. Consider priority queue or chunking.

- **Demuxer spawns a task per actor with unbounded mpsc** —
  `engine/packages/pegboard-envoy/src/actor_event_demuxer.rs:59-80`. For envoys
  serving many short-lived actor IDs, churning tasks is wasteful.

- **`record_inbox_depths` per-iteration** — see top-10 item #10.

### KV

- **No per-tick get coalescing** — `await kv.get(a); await kv.get(b)` pays two
  sequential round-trips even though the engine could serve both in one
  `batch_get` tx. The TS adapter has full event-loop visibility and could implement
  microtask-batched coalescing without protocol changes. `NativeKvAdapter.get`
  at `rivetkit-typescript/packages/rivetkit/src/registry/native.ts:1321` always
  emits a single-key request.

- **No write-through cache after `kv.put(k, v)`** — a subsequent `kv.get(k)`
  re-roundtrips. Per-actor exclusivity makes this safe. `moka::Cache` with a
  bounded size in `rivetkit-core::Kv` (envoy backend) would cover most patterns.

- **`apply_state_deltas` splits puts and deletes into separate WS round-trips** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/state.rs:344-357`. Worse: the
  `chunks(128)` loop doesn't pack puts and deletes together; a 200-put + 5-delete
  save becomes 3 WS round-trips.

- **6 key copies on the KV send path** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/kv.rs:195-216` clones via
  `keys.iter().map(|k| k.to_vec())`,
  `engine/sdks/rust/envoy-client/src/handle.rs:325` clones again for `request_keys`,
  `engine/sdks/rust/envoy-client/src/kv.rs:84` does `data.clone()` on the request
  payload including all values for `KvPutRequest` (largest waste). Use
  `Arc<KvRequestData>` shared between the stored entry and the wire message.

- **`KvGetResponse` projection is O(N*M)** —
  `engine/sdks/rust/envoy-client/src/handle.rs:333-348`. 128-key batch → up to
  16384 byte-slice comparisons. Wire format should carry per-key indices or
  absent flags.

- **Each engine-side KV op opens its own UDB transaction** —
  `engine/packages/pegboard/src/actor_kv/mod.rs:86`. `db.run(|tx| ...)` overhead
  amortizes badly for tiny gets.

### SQLite

- **Single SQLite worker thread per actor** —
  `engine/packages/depot-client/src/worker.rs:99-129`,
  `vfs.rs:3200-3226`. One `*mut sqlite3` opened `READWRITE | CREATE` with
  `locking_mode=EXCLUSIVE`. All SQL — even pure reads — serializes through one
  OS thread. The CLAUDE.md-promised "read pool with multiple read-only conns,
  single writer" does **not exist in the implementation**. This is the largest
  doc-vs-code drift in the audit; the read-pool would unlock real concurrency.

- **`SqliteDb::open` is gated by an async mutex on every query, even after the
  db is set** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/sqlite.rs:158-201`. Should be
  `tokio::sync::OnceCell` for the handle, or an `AtomicBool` fast-path with the
  async mutex on the slow path.

- **`verify_batch_atomic_writes` probe transaction on every cold open** —
  `engine/packages/depot-client/src/vfs.rs:3228-3250, 1781-1825`. Runs
  `BEGIN IMMEDIATE; CREATE TABLE __rivet_batch_probe; INSERT; DELETE; DROP; COMMIT;`
  with a real `sqlite_commit` round-trip every time an actor first touches its
  DB. Result is invariant per process; cache in a `OnceLock<bool>`.

- **SQL string `String::clone`d for logging on every query** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/sqlite.rs:266, 287, 314, 340`.
  `let sql_for_log = sql.clone()` runs unconditionally even though only the
  error path uses it. `Arc<str>` for the SQL, or capture-by-move into an
  error-only closure.

- **Blob columns expanded to JSON-array-of-bytes** —
  `rivetkit-typescript/packages/rivetkit-napi/src/database.rs:218-221`,
  `rivetkit-rust/packages/rivetkit-core/src/actor/sqlite.rs:1003-1013`. A 64 KiB
  blob becomes ~64K `serde_json::Number` heap allocations per read. Pass as
  NAPI `Buffer` or base64 string instead.

- **VFS hot-page hints captured but never persisted** —
  `engine/packages/depot-client/src/vfs.rs:1261-1265, 1188-1210`. `recent_pages`
  records every page hit under `state.write()` (hot path), but the snapshot is
  never written to depot at sleep/close. Pure overhead today; the next cold open
  still only preloads page 1. The `SQLITE_OPTIMIZATIONS.md` TODO is unimplemented.

- **`remote_sqlite_executor_cell` allocates a `String` per remote SQL request,
  even on cache hit** —
  `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs:1000-1012`.
  `let key = (actor_id.to_string(), generation)` allocates before the
  `entry_async` bucket lookup, which itself takes an exclusive lock even on
  hits. Use `get_async` first, and a `&str`-keyed lookup.

### Actor startup

- **Duplicate startup state save** — see top-10 item #9.

- **`ActorContextInner` heap fragmentation** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/context.rs:55-150`. Single
  `Arc` with **65+ fields**: 9 `parking_lot::Mutex`, 9 `parking_lot::RwLock`,
  3 `tokio::sync::Mutex`, 5 `tokio::sync::Notify`, 4 `OnceLock<String>`, 1
  `scc::HashMap`, 2 `CancellationToken`, ~5 `AtomicU64`/`U32`, ~7 `AtomicBool`,
  ~3 `AtomicUsize`. Plus `SleepState` adds 4 more `parking_lot::Mutex` slots.
  For 100k actors per process the per-actor base cost is tens of KB before any
  user state. Split into hot/cold sub-allocations; replace set-once `Mutex<Option<T>>`
  with `OnceLock<T>` or `arc-swap`.

- **`broadcast::channel(32)` allocated per actor for inspector overlays** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs:385`. Allocated for
  every actor even when no inspector ever attaches. Lazy-init on first attach.

- **Unconditional alarm push on first startup** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/schedule.rs:255-289` and
  `context.rs:286`. Dirty flag is true on first startup so a no-event push fires
  on every cold start. Skip when both current and last-pushed are `None`.

- **`actor_id` `String` cloned in every `Kv` call** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/kv.rs:127-203`. `Arc<str>`
  would remove these.

- **`settle_hibernated_connections` mutex contention** —
  `engine/sdks/rust/envoy-client/src/handle.rs:267-296`. 2 std-mutex locks per
  conn during startup. Hoist into one batched call.

- **High-cardinality Prometheus labels** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/metrics.rs:14-25`. Every
  `with_label_values` per metric per actor is a labeled-vec lookup under lock.
  Drop `actor_key` from inbox-depth gauges, or pre-create per-actor wrapper
  handles in `ActorMetrics::new`.

### Concurrency / hot-path waste

- **Repeated `actor_events.read().clone()` on every action enqueue / event publish** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/context.rs:843, 1486, 1153, 1482`.
  Reads then clones an `Option<UnboundedSender>` through an `RwLock`. Use
  `ArcSwap<Option<UnboundedSender>>`.

- **`pub fn state() -> Vec<u8>` always clones** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/state.rs:99-101`. Full state
  clone per call. Add borrowing API: `read_state<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R`
  or return `Bytes`.

- **Double allocation in HTTP-CBOR encode** —
  `rivetkit-rust/packages/rivetkit-core/src/registry/http.rs:944-966, 910-920`.
  Builds `serde_json::Map`, converts to `JsonValue::Object`, then encodes that
  into CBOR. Write CBOR directly via `BTreeMap<String, ciborium::Value>`.

- **`tracing::info!` per action dispatch** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs:858, 896`. Three info
  lines per action (handling, enqueued, reply received). Demote to `debug!`.

- **AsyncCounter callback lock on every inc/dec** —
  `engine/sdks/rust/envoy-client/src/async_counter.rs:39-79`. parking_lot lock
  acquisition on every HTTP request start/finish.

---

## 3. Smells (lower priority)

- **`std::sync::RwLock<Vec<u8>>` for `ActorVars`** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/vars.rs:5`. Should be
  `parking_lot::RwLock` or `arc_swap::ArcSwap<Bytes>` for lock-free reads.

- **Queue `block_on` fallback can build a fresh tokio runtime per call** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/queue.rs:910-933`.

- **`schedule.rs:371` uses `sleep` rather than `sleep_until`** for an alarm
  arm — equivalent but `sleep_until` is more idiomatic for "fire at deadline".

- **`inject_latency` is a test hook not cfg-gated** —
  `engine/sdks/rust/envoy-client/src/utils.rs:70-76`.

- **`set_alarm_tracked` spawns a new task per push** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/schedule.rs:291-338`. Coalesce.

- **`core_dispatched_hook_reply` spawns one task per disconnect grace** —
  `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs:797-815`. Scales poorly
  for hibernation-heavy actors.

- **Reply Vec → JsonValue → JSON response on HTTP reply** mirror-image of the
  request transcode at `registry/http.rs:910-940`.

- **TOCTOU window: actor "started" but inspector not yet configured** —
  `rivetkit-rust/packages/rivetkit-core/src/registry/mod.rs:697-698`. Pass the
  inspector into `ActorTask::new`.

- **Buffered-message clone before send** —
  `engine/sdks/rust/envoy-client/src/actor.rs:1354-1368`. Clones full
  `ToRivetTunnelMessage` even on success path. Reorder.

---

## 4. Files of interest (absolute paths)

### envoy-client (highest hit rate)
- `/home/nathan/r8/engine/sdks/rust/envoy-client/src/context.rs`
- `/home/nathan/r8/engine/sdks/rust/envoy-client/src/connection/mod.rs`
- `/home/nathan/r8/engine/sdks/rust/envoy-client/src/connection/native.rs`
- `/home/nathan/r8/engine/sdks/rust/envoy-client/src/actor.rs`
- `/home/nathan/r8/engine/sdks/rust/envoy-client/src/tunnel.rs`
- `/home/nathan/r8/engine/sdks/rust/envoy-client/src/utils.rs`
- `/home/nathan/r8/engine/sdks/rust/envoy-client/src/kv.rs`
- `/home/nathan/r8/engine/sdks/rust/envoy-client/src/sqlite.rs`
- `/home/nathan/r8/engine/sdks/rust/envoy-client/src/envoy.rs`
- `/home/nathan/r8/engine/sdks/rust/envoy-protocol/src/versioned.rs`

### rivetkit-core
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/actor/context.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/actor/task.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/actor/state.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/actor/kv.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/actor/sqlite.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/actor/schedule.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/actor/vars.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/registry/http.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/registry/mod.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/registry/runner_config.rs`
- `/home/nathan/r8/rivetkit-rust/packages/rivetkit-core/src/engine_process.rs`

### depot-client / pegboard-envoy / pegboard
- `/home/nathan/r8/engine/packages/depot-client/src/vfs.rs`
- `/home/nathan/r8/engine/packages/depot-client/src/worker.rs`
- `/home/nathan/r8/engine/packages/depot-client/src/query.rs`
- `/home/nathan/r8/engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs`
- `/home/nathan/r8/engine/packages/pegboard-envoy/src/tunnel_to_ws_task.rs`
- `/home/nathan/r8/engine/packages/pegboard-envoy/src/actor_event_demuxer.rs`
- `/home/nathan/r8/engine/packages/pegboard/src/actor_kv/mod.rs`

### rivetkit-typescript
- `/home/nathan/r8/rivetkit-typescript/packages/rivetkit/src/registry/native.ts`
- `/home/nathan/r8/rivetkit-typescript/packages/rivetkit/src/client/actor-conn.ts`
- `/home/nathan/r8/rivetkit-typescript/packages/rivetkit/src/drivers/engine/actor-driver.ts`
- `/home/nathan/r8/rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts`
- `/home/nathan/r8/rivetkit-typescript/packages/rivetkit-napi/src/actor_context.rs`
- `/home/nathan/r8/rivetkit-typescript/packages/rivetkit-napi/src/database.rs`

---

## 5. Cross-cutting themes

1. **`envoy-client` is the single largest area of waste.** Three Mutex<HashMap>
   violations on its hot path (#1), the `ws_tx` lock on every send (#2), the
   `BufferMap` String allocation (#3), the `convert_same_bytes` bytes-roundtrip,
   per-event allocations, no NODELAY, and per-call payload clones. Fixing
   envoy-client gives the biggest single-area win.

2. **Doc/code drift in SQLite.** CLAUDE.md describes a read-pool design
   (multiple read-only conns, single writer, mode-switching, autocommit
   detection) that **does not exist** in the implementation. There is exactly
   one SQLite connection per actor, serialized by one worker thread. Either
   the docs need to be brought into line with reality, or the read-pool needs
   to be built.

3. **`actor.overloaded` enforcement is aspirational.** CLAUDE.md mandates
   bounded inboxes with `try_reserve` + `actor.overloaded`, but every actor
   inbox is unbounded and the `actor.overloaded` error code doesn't exist
   anywhere in the tree.

4. **State persistence model is the biggest user-visible perf cost** for
   stateful actors. Whole-blob rewrite + JS-level full re-serialize + 128 KiB
   hard cap with no chunking. Push large state into SQLite or split the blob.

5. **Hot-path metrics are themselves a meaningful cost.** `record_inbox_depths`
   on every loop iteration, AsyncCounter callbacks under parking_lot, high
   label cardinality on `actor_active`. Sampling or per-actor wrapper handles
   would help.
