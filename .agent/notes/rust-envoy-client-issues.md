# Rust Envoy Client: Known Issues

Audit of `engine/sdks/rust/envoy-client/src/` performed 2026-04-07.

---

## Behavioral Bugs

### B1: `destroy_actor` bypasses engine protocol -- FIXED

**File:** `handle.rs:60-65`, `envoy.rs:239-241`, `actor.rs:192-195`

The TS version sends `ActorIntentStop` and waits for the engine to issue a `CommandStopActor` with `StopActorReason::Destroy`. The Rust version sent `ToActor::Destroy` directly to the actor, force-killing it locally without engine confirmation.

**Fix applied:** `destroy_actor` now sends `ActorIntentStop` event (matching TS behavior). Removed `DestroyActor` message variant and `ToActor::Destroy`.

### B2: Graceful shutdown force-kills after 1 second -- FIXED

**File:** `envoy.rs:409-416`

`handle_shutdown` spawned a task that slept 1 second then sent `Stop`, which dropped all actor channels. Actors got no `on_actor_stop` callback.

**Fix applied:** Now polls actor handle closure with a deadline from `serverlessDrainGracePeriod` in `ProtocolMetadata` (falls back to 30s). All actors get a chance to stop cleanly.

---

## Performance: Fixed

### P1: `ws_send` clones entire outbound message -- FIXED

**File:** `connection.rs:213-223`

Took `&protocol::ToRivet`, then called `message.clone()` because `wrap_latest` needed ownership.

**Fix applied:** Changed signature to take `protocol::ToRivet` by value. All callers construct fresh values inline (except `send_actor_message` which clones for potential buffering).

### P2: Stringify allocates even when debug logging is off -- FIXED

**File:** `connection.rs:160, 214`, `actor.rs:897`

`tracing::debug!(data = stringify_to_envoy(&decoded), ...)` eagerly evaluated the stringify function regardless of log level.

**Fix applied:** Gated behind `tracing::enabled!(tracing::Level::DEBUG)`.

### P3: `handle_commands` clones config, hibernating_requests, preloaded_kv -- FIXED

**File:** `commands.rs:18-23`

`val.config.clone()`, `val.hibernating_requests.clone()`, `val.preloaded_kv.clone()` when the fields could be moved.

**Fix applied:** Only clone `val.config.name` for the `ActorEntry`, then move the rest into `create_actor`.

### P4: `_config` field in `ActorContext` is unused, forces a clone -- FIXED

**File:** `actor.rs:81, 129`

`_config: config.clone()` stored a full `ActorConfig` that was never read.

**Fix applied:** Removed the `_config` field. `config` is passed directly to `on_actor_start` without cloning.

### P5: `kv_put` clones keys and values -- FIXED

**File:** `handle.rs:202-203`

`entries.iter().map(|(k, _)| k.clone()).collect()` when `entries` is owned and could be consumed.

**Fix applied:** `let (keys, values): (Vec<_>, Vec<_>) = entries.into_iter().unzip();`

### P6: `parse_list_response` clones values -- FIXED

**File:** `handle.rs:358-366`

Keys were consumed via `into_iter()` but values were indexed and cloned.

**Fix applied:** `resp.keys.into_iter().zip(resp.values).collect()`

---

## Performance: Not Worth the Effort

### P7: BufferMap uses `HashMap<String, T>` with string key allocation

**File:** `utils.rs:122-172`

`cyrb53()` returns a hex `String` on every lookup. Used on hot paths (tunnel message dispatch). However, inputs are only 8 bytes total, producing ~13-char strings. Tiny allocations served from thread-local caches.

### P8: Redundant inner `Arc` on `ws_tx` and `protocol_metadata`

**File:** `context.rs:15-16`

`SharedContext` is already behind `Arc<SharedContext>`. Inner `Arc` wrappers add one extra indirection and refcount. Negligible impact.

### P9: `tokio::sync::Mutex` vs `std::sync::Mutex`

**File:** `context.rs:15-16`

Neither lock is held across `.await`. `protocol_metadata` is a clear-cut candidate for `std::sync::Mutex`. `ws_tx` holds the lock during `serde_bare` encode, making `tokio::sync::Mutex` defensible.

### P10: O(n*m) key matching in `kv_get`

**File:** `handle.rs:107-119`

Nested loop over requested keys and response keys. Real quadratic complexity, but n is typically small for KV gets.

### P11: Double actor lookups in tunnel.rs

**File:** `tunnel.rs:45-59, 112-148`

`get_actor` called twice (once to check existence, once to use). Borrow checker prevents naive fix since `get_actor(&self)` borrows all of `ctx`.

### P12: Headers cloned from HashableMap instead of moved

**File:** `actor.rs:309, tunnel.rs:140`

`req.headers.iter().map(|(k, v)| (k.clone(), v.clone())).collect()` when `req` is owned. Could use `into_iter()`.

### P13: `handle_ack_events` iterates checkpoints twice

**File:** `events.rs:37-67`

First pass retains events, second pass checks for actor removal. Could collect removals in first pass.
