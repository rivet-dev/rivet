# rivetkit-napi Receive-Loop Adapter

Status: **DRAFT — depends on `rivetkit-core-receive-loop-api.md` being accepted. Not implemented.**

Scope:

- Rewrites `rivetkit-typescript/packages/rivetkit-napi/src/actor_factory.rs` to host a Rust-side receive loop that translates `ActorEvent`s from the new `rivetkit-core` surface into TSF invocations against the callback shape used in `feat/sqlite-vfs-v2`.
- This adapter is the emulation layer for every callback the core spec does not expose. Anything lifecycle-shaped (`onCreate`, `createState`, `createVars`, `createConnState`, `onMigrate`, `onWake`, `onBeforeActorStart`, `onStateChange`, `onBeforeActionResponse`, `run`) lives here.
- Minimal edits to `rivetkit-typescript/packages/rivetkit/src/actor/instance/state-manager.ts` and `src/actor/conn/state-manager.ts`. The TS runtime's `@rivetkit/on-change` proxy stays. The throttle and KV machinery it owns today move to Rust.
- Does **not** change the public actor-authoring API in `rivetkit-typescript/packages/rivetkit/`. User actors that work at `feat/sqlite-vfs-v2` continue to work unmodified.
- Does **not** change `CoreRegistry.register` / `.serve` NAPI surface, the engine wire protocol, the KV layout, or the inspector HTTP API.

## Goals

- Preserve the feat/sqlite-vfs-v2 public API 1:1: `onCreate`, `createState`, `createVars`, `createConnState`, `onMigrate` (if kept), `onWake`, `onBeforeActorStart`, `onBeforeConnect`, `onConnect`, `onDisconnect`, `onBeforeSubscribe`, `onSleep`, `onDestroy`, `onStateChange`, `onBeforeActionResponse`, `actions`, `run`, `onRequest`, `onWebSocket`, `getWorkflowHistory`, `replayWorkflow`. `ctx.saveState({ immediate, maxWait })`, `ctx.abortSignal()`, `ctx.restartRunHandler()`, `ctx.isReady()`, `ctx.isStarted()` stay.
- One `NapiActorFactory` per actor type. TSFs built once from the definition, shared across every instance.
- Reproduce feat/sqlite-vfs-v2's shutdown ordering exactly: drain idle → `onSleep` → drain shutdown tasks → disconnect non-hibernatable conns → drain → save.
- Reproduce per-message action serialization using core's per-conn causality guarantee (no head-of-line blocking across conns; ordered within a conn).
- Keep `run` non-fatal and restartable.

## Non-goals

- No user-visible API change in `rivetkit-typescript/packages/rivetkit/src/actor/**`.
- No exposure of `ActorEvent` or `Reply<T>` to JS.
- No "bring-your-own adapter" surface: `NapiActorFactory` accepts exactly one shape, matching what `buildNativeFactory` produces today.
- No wire protocol, KV layout, inspector, or engine-startup changes.

## Motivation

See `rivetkit-core-receive-loop-api.md`. Core owns a minimal event-stream API. NAPI is where all callback-shaped emulation lives, because:

- Core shipping a callback API in addition to events defeats the core spec's simplification.
- Exposing `ActorEvent` through NAPI and duplicating dispatch logic on every future non-TS runtime (V8, WASI) is worse than each adapter choosing its own surface over the common core.
- NAPI is already pure translation code. Adding emulation for the callbacks core dropped is a translation responsibility.

## Proposal

### JS surface

`NapiActorFactory` takes a single object of TSF-callable callbacks plus `JsActorConfig`. The shape matches what `buildNativeFactory` in `native.ts:3107` produces today at `feat/sqlite-vfs-v2`, with one addition:

```ts
interface NapiActorCallbacks {
    // ---- First-create preamble (gated on ActorStart.snapshot.is_none()) ----
    createState?:     (evt: { ctx, input: Buffer | null })     => Promise<Buffer>;
    onCreate?:        (evt: { ctx, input: Buffer | null })     => Promise<void>;
    createConnState?: (evt: { ctx, conn: ConnHandle, params: Buffer, request?: Request })
                                                                => Promise<Buffer>;

    // ---- Every-start preamble ----
    createVars?:           (evt: { ctx })              => Promise<Buffer>;
    onMigrate?:            (evt: { ctx, isNew: bool }) => Promise<void>;
    onWake?:               (evt: { ctx })              => Promise<void>;
    onBeforeActorStart?:   (evt: { ctx })              => Promise<void>;

    // ---- Lifecycle termini ----
    onSleep?:              (evt: { ctx }) => Promise<void>;
    onDestroy?:            (evt: { ctx }) => Promise<void>;

    // ---- Connection lifecycle ----
    onBeforeConnect?:      (evt: { ctx, params: Buffer, request?: Request })
                                                                => Promise<void>;
    onConnect?:            (evt: { ctx, conn: ConnHandle, request?: Request })
                                                                => Promise<void>;
    onDisconnect?:         (evt: { ctx, conn: ConnHandle })     => Promise<void>;
    onBeforeSubscribe?:    (evt: { ctx, conn: ConnHandle, eventName: string })
                                                                => Promise<void>;

    // ---- Actions / HTTP / WebSocket ----
    actions:               Record<
                              string,
                              (evt: { ctx, conn: ConnHandle | null, name: string, args: Buffer })
                                  => Promise<Buffer>
                           >;
    onBeforeActionResponse?: (evt: { ctx, name: string, args: Buffer, output: Buffer })
                                                                => Promise<Buffer>;
    onRequest?:            (evt: { ctx, request: Request })
                              => Promise<{ status?: number; headers?: Record<string,string>; body?: Buffer }>;
    onWebSocket?:          (evt: { ctx, ws: WebSocket, request?: Request })
                                                                => Promise<void>;

    // ---- Long-lived entry ----
    run?:                  (evt: { ctx })                        => Promise<void>;

    // ---- Workflow integration ----
    getWorkflowHistory?:   (evt: { ctx })                        => Promise<Buffer | null>;
    replayWorkflow?:       (evt: { ctx, entryId?: string })      => Promise<Buffer | null>;

    // ---- NEW: on-demand state / conn-hibernation serialization ----
    serializeState: (reason: SerializeStateReason) => Promise<StateDeltaPayload>;
}

type SerializeStateReason = "save" | "inspector" | "sleep" | "destroy";

interface StateDeltaPayload {
    state?: Buffer;                                               // full actor state bytes
    connHibernation?: Array<{ connId: string; bytes: Buffer }>;   // dirty hibernatable conns
    connHibernationRemoved?: string[];                            // disconnected hibernatable conns
}
```

**Note:** `onStateChange` is not in this bag. It fires TS-side from the `@rivetkit/on-change` handler with the raw state object — NAPI never sees it.

### Rust-side adapter

`NapiActorFactory::constructor` builds one `Arc<CallbackBindings>` of TSFs (one per entry, keyed by name for actions). Per-instance, the factory closure returns a `BoxFuture<Result<()>>` that runs the adapter loop.

```rust
async fn run_adapter_loop(
    bindings: Arc<CallbackBindings>,
    start: ActorStart,
) -> Result<()> {
    let ActorStart { ctx, input, snapshot, hibernated, mut events } = start;

    // Synthesized AbortSignal for the NAPI ctx wrapper. Cancelled only on Destroy
    // (see "AbortSignal synthesis" below).
    let abort = CancellationToken::new();
    ctx.attach_napi_abort_token(abort.clone());

    // Dirty flag flipped by ctx.requestSave(..) on the JS side, read by maybe_serialize.
    // Registered BEFORE any TSF call so first preamble mutations aren't lost.
    let dirty = Arc::new(AtomicBool::new(false));
    ctx.on_request_save({
        let d = Arc::clone(&dirty);
        move |_immediate| d.store(true, Ordering::Release)
    });

    // JoinSet for user-spawned work (action handlers, HTTP, WebSocket, conn open, etc.).
    let mut tasks: JoinSet<()> = JoinSet::new();

    // Per-conn serialization is handled by core (per-conn event causality), so we can
    // spawn concurrently across conns without losing per-conn ordering.

    // ============ 1. Preamble ============
    // Matches feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts
    // startup order.
    let is_new = snapshot.is_none();

    if is_new {
        // First-create path.
        if let Some(cb) = &bindings.create_state {
            let bytes = with_timeout("createState", cfg.create_state_timeout, call_create_state(cb, &ctx, input.as_deref())).await?;
            ctx.set_state_initial(bytes)?;
        }
        if let Some(cb) = &bindings.on_create {
            with_timeout("onCreate", cfg.on_create_timeout, call_on_create(cb, &ctx, input.as_deref())).await?;
        }
        ctx.mark_has_initialized_and_flush().await?;      // pre-ready state save, matches reference
    } else {
        // Wake / migrate path: install snapshot, restore conns, then lifecycle callbacks.
        ctx.set_state_initial(snapshot.unwrap())?;
        for (conn, bytes) in hibernated {
            ctx.restore_hibernatable_conn(conn, bytes)?;  // re-wraps onChange proxy TS-side
        }
    }

    if let Some(cb) = &bindings.create_vars {
        let bytes = with_timeout("createVars", cfg.create_vars_timeout, call_create_vars(cb, &ctx)).await?;
        ctx.set_vars(bytes);
    }

    if let Some(cb) = &bindings.on_migrate {
        with_timeout("onMigrate", cfg.on_migrate_timeout, call_on_migrate(cb, &ctx, is_new)).await?;
    }
    if !is_new {
        if let Some(cb) = &bindings.on_wake {
            with_timeout("onWake", cfg.on_wake_timeout, call_on_wake(cb, &ctx)).await?;
        }
    }

    ctx.init_alarms().await?;                              // core-driven: resync persisted alarms
    ctx.mark_ready();                                      // flips isReady() for user code
    if let Some(cb) = &bindings.on_before_actor_start {
        with_timeout("onBeforeActorStart", cfg.on_before_actor_start_timeout,
                     call_on_before_actor_start(cb, &ctx)).await?;
    }
    ctx.mark_started();                                    // flips isStarted()

    // ============ 2. `run` handler (adapter-spawned, non-fatal) ============
    let run_handle = Arc::new(Mutex::new(
        bindings.run.as_ref().map(|cb| spawn_run_handler(cb.clone(), ctx.clone()))
    ));
    // restartRunHandler on the NAPI ctx aborts the current handle and replaces it.
    ctx.attach_run_restart({
        let rh = Arc::clone(&run_handle);
        let cb = bindings.run.clone();
        let ctx = ctx.clone();
        move || {
            let mut guard = rh.blocking_lock();
            if let Some(h) = guard.take() { h.abort(); }
            *guard = cb.as_ref().map(|cb| spawn_run_handler(cb.clone(), ctx.clone()));
        }
    });

    // Drain overdue scheduled events that accumulated during startup.
    ctx.drain_overdue_scheduled_events().await?;

    // ============ 3. Receive loop ============
    while let Some(event) = events.recv().await {
        dispatch_event(event, &bindings, &ctx, &abort, &mut tasks, &dirty).await;
        if ctx.take_end_reason().is_some() {
            break;
        }
    }

    // ============ 4. End-of-life ============
    // Cancel run handler; it's non-fatal to the actor but we drop it on shutdown.
    if let Some(h) = run_handle.lock().await.take() { h.abort(); let _ = h.await; }
    abort.cancel();
    tasks.shutdown().await;
    Ok(())
}

fn spawn_run_handler(cb: CallbackTsfn<LifecyclePayload>, ctx: ActorContext) -> JoinHandle<()> {
    // run is NON-FATAL: log Ok or Err, never save, never cancel the actor.
    // Matches feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts
    // #startRunHandler — exits are logged and the actor continues accepting events.
    tokio::spawn(async move {
        match call_run(&cb, &ctx).await {
            Ok(()) => tracing::debug!("run handler exited cleanly"),
            Err(e) => tracing::error!(error = ?e, "run handler threw"),
        }
    })
}
```

`dispatch_event` per-variant:

```rust
async fn dispatch_event(
    event: ActorEvent, bindings: &Arc<CallbackBindings>, ctx: &ActorContext,
    abort: &CancellationToken, tasks: &mut JoinSet<()>, dirty: &AtomicBool,
) {
    match event {
        // --- spawned (core guarantees per-conn ordering) ---
        ActorEvent::Action { name, args, conn, reply } => {
            let b = Arc::clone(bindings); let c = ctx.clone(); let a = abort.clone();
            tasks.spawn(async move {
                tokio::select! {
                    _ = a.cancelled() => reply.send(Err(actor_shutting_down())),
                    r = async {
                        let handler = b.actions.get(&name).cloned();
                        let raw = with_timeout(
                            "action", c.cfg().action_timeout,
                            dispatch_action(handler.as_ref(), &c, conn.as_ref(), &name, &args),
                        ).await?;
                        // onBeforeActionResponse wrapper.
                        if let Some(cb) = &b.on_before_action_response {
                            call_before_action_response(cb, &c, &name, &args, &raw).await
                        } else {
                            Ok(raw)
                        }
                    } => reply.send(r),
                }
            });
        }
        ActorEvent::HttpRequest { request, reply } => {
            spawn_reply(tasks, abort, reply, Arc::clone(bindings), ctx.clone(),
                        |b, c| dispatch_http(&b.on_request, &c, request));
        }
        ActorEvent::WebSocketOpen { ws, request, reply } => {
            spawn_reply(tasks, abort, reply, Arc::clone(bindings), ctx.clone(),
                        |b, c| dispatch_websocket(&b.on_websocket, &c, ws, request));
        }
        ActorEvent::ConnectionOpen { conn, params, request, reply } => {
            // Chain onBeforeConnect (no conn) → createConnState → onConnect → reply.
            // Matches feat/sqlite-vfs-v2 three-phase connect.
            let b = Arc::clone(bindings); let c = ctx.clone(); let a = abort.clone();
            tasks.spawn(async move {
                tokio::select! {
                    _ = a.cancelled() => reply.send(Err(actor_shutting_down())),
                    r = async {
                        if let Some(cb) = &b.on_before_connect {
                            dispatch_before_connect(cb, &c, &params, request.as_ref()).await?;
                        }
                        if let Some(cb) = &b.create_conn_state {
                            let bytes = dispatch_create_conn_state(cb, &c, &conn, &params, request.as_ref()).await?;
                            c.set_conn_state_initial(&conn, bytes)?;   // wraps the onChange proxy TS-side
                        }
                        if let Some(cb) = &b.on_connect {
                            dispatch_connect(cb, &c, &conn, request).await?;
                        }
                        Ok(())
                    } => reply.send(r),
                }
            });
        }
        ActorEvent::ConnectionClosed { conn } => {
            // No reply. onDisconnect fires whether the disconnect was client-initiated
            // or adapter-initiated via ctx.disconnect_conn / disconnect_conns.
            let b = Arc::clone(bindings); let c = ctx.clone();
            tasks.spawn(async move {
                if let Some(cb) = &b.on_disconnect {
                    let _ = dispatch_disconnect(cb, &c, conn).await;
                }
            });
        }
        ActorEvent::SubscribeRequest { conn, event_name, reply } => {
            spawn_reply(tasks, abort, reply, Arc::clone(bindings), ctx.clone(),
                        |b, c| dispatch_before_subscribe(&b.on_before_subscribe, &c, &conn, &event_name));
        }
        ActorEvent::WorkflowHistoryRequested { reply } => {
            spawn_reply(tasks, abort, reply, Arc::clone(bindings), ctx.clone(),
                        |b, c| dispatch_workflow_history(&b.get_workflow_history, &c));
        }
        ActorEvent::WorkflowReplayRequested { entry_id, reply } => {
            spawn_reply(tasks, abort, reply, Arc::clone(bindings), ctx.clone(),
                        |b, c| dispatch_workflow_replay(&b.replay_workflow, &c, entry_id));
        }

        // --- inline ---
        ActorEvent::SerializeState { reason, reply } => {
            reply.send(maybe_serialize(bindings, dirty, reason).await);
        }
        ActorEvent::Sleep { reply } => {
            // Reply is Reply<()> — state is NOT in the reply. Adapter calls
            // ctx.save_state(deltas) explicitly for any pre-termination persistence.
            drain_tasks(tasks).await;
            if let Some(cb) = &bindings.on_sleep {
                let _ = with_timeout("onSleep", ctx.cfg().on_sleep_timeout, call_on_sleep(cb, ctx)).await;
            }
            drain_tasks(tasks).await;
            for conn in ctx.conns().filter(|c| !c.is_hibernatable()) {
                if let Some(cb) = &bindings.on_disconnect {
                    let _ = dispatch_disconnect(cb, ctx, conn.clone()).await;
                }
            }
            let _ = ctx.disconnect_conns(|c| !c.is_hibernatable()).await;
            // Final persist: build payload with reason=Sleep, flush atomically.
            if dirty.load(Ordering::Acquire) || ctx.has_conn_changes() {
                let payload = call_serialize_state(&bindings.serialize_state, "sleep").await.unwrap_or_default();
                let _ = ctx.save_state(deltas_from(payload)).await;
            }
            reply.send(Ok(()));
            ctx.set_end_reason(EndReason::Sleep);
        }
        ActorEvent::Destroy { reply } => {
            abort.cancel();
            if let Some(cb) = &bindings.on_destroy {
                let _ = with_timeout("onDestroy", ctx.cfg().on_destroy_timeout, call_on_destroy(cb, ctx)).await;
            }
            drain_tasks(tasks).await;
            for conn in ctx.conns() {
                if let Some(cb) = &bindings.on_disconnect {
                    let _ = dispatch_disconnect(cb, ctx, conn.clone()).await;
                }
            }
            let _ = ctx.disconnect_conns(|_| true).await;
            if dirty.load(Ordering::Acquire) || ctx.has_conn_changes() {
                let payload = call_serialize_state(&bindings.serialize_state, "destroy").await.unwrap_or_default();
                let _ = ctx.save_state(deltas_from(payload)).await;
            }
            reply.send(Ok(()));
            ctx.set_end_reason(EndReason::Destroy);
        }
    }
}
```

`spawn_reply(tasks, abort, reply, bindings, ctx, f)`:

```rust
tasks.spawn(async move {
    tokio::select! {
        _ = abort.cancelled() => reply.send(Err(actor_shutting_down())),
        r = f(bindings, ctx) => reply.send(r),
    }
});
```

### Event → callback mapping

| `ActorEvent`                | JS callback(s) invoked                                                        | Dispatch | Reply shape              |
|-----------------------------|-------------------------------------------------------------------------------|----------|--------------------------|
| `Action`                    | `actions[name]` → (`onBeforeActionResponse` if defined)                       | spawn    | `Reply<Vec<u8>>`         |
| `HttpRequest`               | `onRequest`                                                                   | spawn    | `Reply<Response>`        |
| `WebSocketOpen`             | `onWebSocket`                                                                 | spawn    | `Reply<()>`              |
| `ConnectionOpen`            | `onBeforeConnect` → `createConnState` → `onConnect`                           | spawn    | `Reply<()>`              |
| `ConnectionClosed`          | `onDisconnect`                                                                | spawn    | (none)                   |
| `SubscribeRequest`          | `onBeforeSubscribe`                                                           | spawn    | `Reply<()>`              |
| `SerializeState { reason }` | `serializeState(reason)`                                                      | inline   | `Reply<Vec<StateDelta>>` |
| `Sleep`                     | `onSleep` + drain + disconnect + (if dirty) `ctx.save_state(...)`             | inline   | `Reply<()>`              |
| `Destroy`                   | `onDestroy` + drain + disconnect + (if dirty) `ctx.save_state(...)`           | inline   | `Reply<()>`              |
| `WorkflowHistoryRequested`  | `getWorkflowHistory`                                                          | spawn    | `Reply<Option<Vec<u8>>>` |
| `WorkflowReplayRequested`   | `replayWorkflow`                                                              | spawn    | `Reply<Option<Vec<u8>>>` |

### Concurrency model

- Adapter owns user work via a `JoinSet`; core owns zero user tasks.
- All non-terminal user dispatches (`Action`, `HttpRequest`, `WebSocketOpen`, `ConnectionOpen`, `ConnectionClosed`, `SubscribeRequest`, workflow) are `tokio::spawn`'d so slow callbacks don't stall the receive loop.
- **Per-conn ordering is guaranteed by core** (see core spec, *Per-conn event causality*). For any given `ConnHandle`, core does not enqueue event N+1 until the reply for event N arrives. This means the adapter can spawn per event without re-implementing a per-conn serialization queue, while still observing the feat/sqlite-vfs-v2 per-message ordering that user actors expect.
- Cross-conn events run in parallel. System-dispatched `Action`s (alarms, `conn: None`) are unordered with respect to each other and with respect to client-originated events.
- `run` handler: spawned once in the preamble (§1), non-fatal on exit. Panics and rejections are logged. The actor continues accepting events. `ctx.restartRunHandler()` aborts the existing JoinHandle and spawns a fresh TSF call.
- Per-callback timeouts: every TSF invocation is wrapped in `tokio::time::timeout` with the matching config (`create_state_timeout`, `on_sleep_timeout`, etc.). Timeout raises a structured error via the Reply path.
- `Sleep` drains the JoinSet twice (before and after disconnect) so any state mutations from in-flight work make it into the final delta. Bounded externally by core's `sleep_grace_period`.
- `Destroy` cancels `abort` first (so in-flight spawned work unblocks via its `select!`), then calls `onDestroy`, then drains, then disconnects, then drains again, then serializes.
- `JoinSet` is intentionally uncapped (parity with feat/sqlite-vfs-v2). A cap is a possible follow-up via `ActorConfig::max_concurrent_callbacks` without core changes.

### Dirty tracking (aligned with feat/sqlite-vfs-v2)

TS keeps its `@rivetkit/on-change` proxies. Two changes:

1. Handler calls `ctx.requestSave(false)` (or `ctx.requestSaveWithin(ms)` for `maxWait`) after flipping its dirty flag. No throttle timer TS-side.
2. `serializeForTick()` builds the payload synchronously and clears flags before returning.

Clearing flags in `serializeForTick` is safe because the `on-change` handler fires per mutation, not per "newly dirty" transition. Any mutation that lands during the Rust-side KV write re-flags `persistChanged` immediately — the next `Serialize(Save)` picks it up. Matches `feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/actor/instance/state-manager.ts:#savePersistInner`, which clears inside its write queue at the moment it builds entries (line ~421), before the KV put completes. For `reason === "inspector"`, flags are NOT cleared because no KV write happens.

Rust-side:

```rust
async fn maybe_serialize(
    bindings: &CallbackBindings, dirty: &AtomicBool, reason: SerializeStateReason,
) -> Result<Vec<StateDelta>> {
    // For reason=Inspector, we serialize even if Rust's dirty flag is clean,
    // because the flag might have been cleared by a recent Save tick but the
    // inspector subscriber still needs the current snapshot. TS side is the
    // authority on "is anything actually dirty in memory right now."
    if reason != SerializeStateReason::Inspector && !dirty.swap(false, Ordering::AcqRel) {
        return Ok(Vec::new());
    }
    if reason == SerializeStateReason::Inspector {
        dirty.store(false, Ordering::Release);   // consumed by this serialize; flags
                                                 // on the JS side are NOT cleared by
                                                 // `serializeForTick` when reason=inspector.
    }
    let payload = call_serialize_state(&bindings.serialize_state, reason).await?;
    Ok(deltas_from(payload))
}
```

One TSF per serialize. No post-write ack. The `reason` parameter flows through to `serializeState(reason)` on the JS side so the TS handler can branch on it.

### StateManager changes (TS)

In `rivetkit-typescript/packages/rivetkit/src/actor/instance/state-manager.ts`:

**Keep:**
- `initPersistProxy`, `#handleStateChange`, `#isInOnStateChange` re-entrance guard, CBOR serializability check, `stateUpdated` inspector emission, `onStateChange` user callback invocation passing `this.#persistRaw.state` (raw object, matches reference).
- `state` / `persist` / `persistRaw` getters.
- `saveState({ immediate, maxWait })` public signature.

**Delete:**
- `#persistWriteQueue: SinglePromiseQueue` and all direct KV writes.
- `savePersistThrottled`, `#pendingSaveTimeout`, `#pendingSaveScheduledTimestamp`, `#lastSaveTime`.
- `#savePersistInner`, `clearPendingSaveTimeout`, `waitForPendingWrites`.
- Direct `actorDriver.kvBatchPut` calls from within the state manager.

**Change:**
- `#handleStateChange`: after `this.#persistChanged = true`, call `this.#actor.napiCtx.requestSave(false)`.
- `conn/state-manager.ts#handleChange`: after `markConnWithPersistChanged`, call the same `ctx.requestSave(false)`.

**Add:**
- `serializeForTick(reason: SerializeStateReason): StateDeltaPayload` — builds payload from `persistRaw` + `connsWithPersistChanged` + `removedHibernatableConnIds`. For `reason === "save" | "sleep" | "destroy"`: clears all dirty flags before returning. For `reason === "inspector"`: does NOT clear flags (the next Save still needs to flush them). No cloning/snapshot needed: the TSF call is synchronous on the Node event loop, so no mutation can interleave between CBOR encode and flag clear. Mutations arriving during the Rust-side KV write re-flag via the `on-change` handler and are picked up on the next tick.
- `saveState({ immediate, maxWait })`:

```ts
async saveState(opts: SaveStateOptions): Promise<void> {
    this.#actor.assertReady();
    const dirty = this.#persistChanged ||
        this.#actor.connectionManager.connsWithPersistChanged.size > 0 ||
        this.#actor.connectionManager.hasRemovedHibernatableConnIds();
    if (!dirty) return;

    if (opts.immediate) {
        // Bypass SerializeState event; adapter persists directly.
        const payload = this.serializeForTick();
        await this.#actor.napiCtx.saveState(payload);
    } else if (opts.maxWait != null) {
        this.#actor.napiCtx.requestSaveWithin(opts.maxWait);
    } else {
        this.#actor.napiCtx.requestSave(false);
    }
}
```

### AbortSignal synthesis

Core removes `ctx.abort_signal()` / `ctx.aborted()`. The NAPI adapter synthesizes both on top of its own `CancellationToken`:

- Created at adapter start. JS `ctx.abortSignal()` returns a `#[napi] AbortSignal` wrapper that resolves when the token is cancelled.
- Cancelled **only on `Destroy`**, matching `feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts:startDestroy` (line 1046).
- Not cancelled on `Sleep` — user code in `onSleep` sees `ctx.aborted() === false`, matching reference.
- Not cancelled on `run` exit (clean or errored). Reference explicitly leaves the actor alive after run returns.
- Cancelled as part of end-of-life cleanup on adapter-future return, after the Sleep/Destroy reply has been sent and written.

### Lifecycle preamble (reference-accurate)

1. **is_new = ActorStart.snapshot.is_none()**
2. **First-create only** (`is_new`):
   1. `createState(ctx, input)` — if defined, install returned state bytes.
   2. `onCreate(ctx, input)` — if defined.
   3. Mark `hasInitialized = true`, flush state via `ctx.save_state(..)` atomically (matches reference pre-ready save).
3. **Wake path only** (`!is_new`):
   1. Install `ActorStart.snapshot` directly as state.
   2. Restore hibernated conns from `ActorStart.hibernated`. The TS `connectionManager` re-wraps each `persistedConn` in its `onChange` proxy.
4. **Every start**:
   1. `createVars(ctx)` — if defined, install returned vars bytes.
   2. `onMigrate(ctx, isNew)` — if defined (adapter-kept for TS public API compat).
   3. If `!is_new`: `onWake(ctx)`.
5. `ctx.init_alarms().await` — core-driven alarm resync.
6. Mark ready (`ctx.mark_ready()`). `isReady()` now returns true for user code.
7. `onBeforeActorStart(ctx)` — driver-level hook if defined.
8. Mark started (`ctx.mark_started()`). `isStarted()` now returns true.
9. Spawn `run` handler (detached, non-fatal).
10. Drain overdue scheduled events (`ctx.drain_overdue_scheduled_events()`).
11. Enter receive loop.

Each step is wrapped in its own timeout (see "Independent timeouts" above). Timeout or error aborts the preamble and the adapter future returns `Err(..)`.

### Shutdown path (reference-accurate)

**Sleep** (matches feat/sqlite-vfs-v2:mod.ts:onStop("sleep")):

1. `drain_tasks(tasks)` — wait for in-flight spawned work (HTTP actions, `onConnect`, user `ctx.keepAwake(fut)` tasks). Bounded by core's `sleep_grace_period`.
2. `onSleep(ctx)` if defined, wrapped in `on_sleep_timeout`.
3. `drain_tasks(tasks)` again — `onSleep` may have spawned follow-up work.
4. For each non-hibernatable conn: invoke `onDisconnect(ctx, conn)` inline.
5. `ctx.disconnect_conns(|c| !c.is_hibernatable()).await` — tears down transports.
6. If anything's dirty: `call_serialize_state("sleep")` → `ctx.save_state(deltas).await`. Durability before reply.
7. `reply.send(Ok(()))` — reply is `Reply<()>`, no state in it.
8. Break receive loop; adapter future returns.

**Destroy** (matches feat/sqlite-vfs-v2:startDestroy + onStop("destroy")):

1. `abort.cancel()` — unblocks in-flight spawned tasks via their `select!`. They reply `Err(actor_shutting_down())`.
2. `onDestroy(ctx)` if defined, wrapped in `on_destroy_timeout`.
3. `drain_tasks(tasks)`.
4. For each conn (hibernatable and ephemeral): `onDisconnect(ctx, conn)` inline.
5. `ctx.disconnect_conns(|_| true).await`.
6. If dirty: `call_serialize_state("destroy")` → `ctx.save_state(deltas).await`.
7. `reply.send(Ok(()))`.
8. Break loop; adapter future returns.

**Why inline `onDisconnect` during shutdown, not via mailbox:**

Normal (client-initiated) disconnect flow: transport dies → core fires `ConnectionClosed` into mailbox → adapter's event handler runs `onDisconnect`. This stays unchanged.

Shutdown-initiated disconnect flow: adapter is already past the receive loop's normal dispatch for these conns (it's inside the `Sleep`/`Destroy` arm). If `ctx.disconnect_conns` queued `ConnectionClosed` events, the adapter would need a second event pump to drain them before replying — adding complexity for no semantic gain. So `ctx.disconnect_conn(s)` during shutdown is transport-teardown only; the adapter fires `onDisconnect` itself. This also matches the reference's synchronous disconnect path in `#disconnectConnections`.

**`run` exit:**

- Non-fatal. Handled entirely inside `spawn_run_handler` — logged, no state save, no `abort.cancel()`, no receive-loop break. The actor continues to process events.
- User code calling `ctx.restartRunHandler()` aborts the current JoinHandle and spawns a fresh one.

### Inspector integration

Core owns the inspector overlay. The adapter's only responsibility is responding to `ActorEvent::SerializeState { reason: Inspector, .. }`:

- Adapter calls `serializeState("inspector")`.
- TS side builds the payload from `persistRaw` but does **not** clear dirty flags (the next Save still needs to write them).
- Adapter replies with the deltas.
- Core distributes bytes to attached inspector subscribers. No KV write.

When `SerializeStateReason::Save` fires, core also distributes bytes to inspector subscribers (Save is a strict superset). So in practice, an attached inspector sees every state mutation, either:
- Immediately via a SerializeState(Inspector) tick (debounced by core's `inspector_serialize_state_interval`, default ~50ms), or
- On the next Save tick (bounded by `state_save_interval`).

When no inspector is attached, core never fires SerializeState(Inspector) — zero cost.

### `ctx.keepAwake(promise)`

TS-only adapter API. Replaces reference's keep-awake region counters. Semantics:

- Registers `promise` with the adapter's JoinSet. `drain_tasks` awaits it during Sleep/Destroy.
- Blocks BOTH Sleep and Destroy — each wait until the promise settles (or the grace window expires).
- Falls INSIDE `sleep_grace_period` / `on_destroy_timeout`; does not extend them.
- On Destroy, the abort token cancels before draining, so promises that observe `ctx.abortSignal()` can short-circuit. On Sleep, the abort does not fire — the promise runs to completion or grace-window timeout.
- Implementation:

```ts
ctx.keepAwake = <T>(promise: Promise<T>): Promise<T> => {
    this.napiCtx.registerTask(promise);      // pushes onto the adapter's JoinSet via a TSF
    return promise;
};
```

The adapter-side `registerTask` is a TSF the NAPI ctx wrapper invokes synchronously to add the promise (wrapped as an async task) into the Rust-side JoinSet. No core primitive needed.

### Independent timeouts

Each callback has its own `cfg.*_timeout_ms`, already present on `JsActorConfig`. The adapter wraps every TSF with `tokio::time::timeout`. Reference config values flow through unchanged:

| Callback                | Timeout config                    |
|-------------------------|-----------------------------------|
| `createState`           | `createStateTimeoutMs`            |
| `createVars`            | `createVarsTimeoutMs`             |
| `createConnState`       | `createConnStateTimeoutMs`        |
| `onCreate`              | `onCreateTimeoutMs`               |
| `onMigrate`             | `onMigrateTimeoutMs`              |
| `onWake`                | `onWakeTimeoutMs`                 |
| `onBeforeActorStart`    | `onBeforeActorStartTimeoutMs`     |
| `onBeforeConnect`       | `onBeforeConnectTimeoutMs`        |
| `onConnect`             | `onConnectTimeoutMs`              |
| `onSleep`               | `onSleepTimeoutMs`                |
| `onDestroy`             | `onDestroyTimeoutMs`              |
| actions (per-action)    | `actionTimeoutMs`                 |
| `run` stop wait         | `runStopTimeoutMs` (on adapter shutdown) |

### Error translation

Unchanged: `BRIDGE_RIVET_ERROR_PREFIX` path in `actor_factory.rs` continues to handle structured `RivetError` round-tripping. All new dispatch helpers reuse `callback_error(name, error)`.

`Reply<T>` drop-guards inherit core's `ActorLifecycle::DroppedReply` behavior: spawned task panics fire the guard automatically. No adapter-side tracking needed.

### Panic behavior

The adapter loop does not panic. All user code runs in:
- Detached `tokio::spawn` tasks — panics surface as `JoinError` when the JoinSet drains. Logged, not rethrown.
- The detached `run_task` — panics logged in `spawn_run_handler`.
- TSF invocations whose Promise rejects — translated via `callback_error`.

A panic in NAPI-adapter Rust (not user code) aborts the actor via core's `catch_unwind` around the adapter future.

## Runtime changes (internal, not user-facing)

- `actor_factory.rs` substantially rewritten. `CallbackBindings` adds fields for the new JS callbacks (`create_state`, `create_conn_state`, `create_vars`, `on_before_actor_start`, `on_before_action_response`, `commit_serialize`) alongside the kept ones. `create_callbacks()` path deleted; replaced with `run_adapter_loop` as the `CoreActorFactory::new` entry closure body.
- Per-payload builder fns and TSF invocation helpers (`call_void`, `call_buffer`, `call_optional_buffer`, `call_request`) are **reused**. `callback_error` + `parse_bridge_rivet_error` are **reused**.
- New Rust module `napi_actor_events.rs` hosts `dispatch_event`, `dispatch_action`, `dispatch_http`, the preamble helpers, and the shutdown helpers.
- `ActorContext` NAPI wrapper gains:
  - `request_save(immediate: bool) -> void`
  - `request_save_within(ms: u32) -> void`
  - `save_state(payload: StateDeltaPayload) -> Promise<void>`
  - `disconnect_conn(id: string) -> Promise<void>`
  - `disconnect_conns(predicate: (conn) => bool) -> Promise<void>`
  - `restart_run_handler() -> void`
  - `abort_signal() -> AbortSignal` / `aborted() -> bool` (backed by adapter token)
  - Internal: `attach_napi_abort_token`, `attach_run_restart`, `mark_ready`, `mark_started`, `set_end_reason`.
- `ActorContext.set_state` / `ActorContext.set_vars` continue to exist but no longer fire `OnStateChangeRequest` in core (core removed the event). `onStateChange` firing is entirely TS-side via `@rivetkit/on-change`.

## Resolved design decisions

- **Preamble emulation**: every pre-loop callback (`createState`, `onCreate`, `createVars`, `createConnState`, `onMigrate`, `onWake`, `onBeforeActorStart`, `onBeforeActionResponse`) runs adapter-side. Core exposes only the minimal `ActorStart` bundle. Order matches `feat/sqlite-vfs-v2:mod.ts` startup exactly.
- **`run` is non-fatal and restartable**: adapter spawns once in the preamble, logs Ok/Err on exit, does not cancel or save. `ctx.restartRunHandler()` aborts and respawns.
- **`onBeforeActionResponse` kept**: adapter wraps action dispatch. User API unchanged.
- **`onStateChange` TS-side**: fires from `@rivetkit/on-change` handler with `persistRaw.state` (raw object). NAPI never sees it.
- **Action concurrency**: spawn per Action into JoinSet; rely on core's per-conn causality for ordering. Cross-conn parallel, intra-conn serialized — matches feat/sqlite-vfs-v2 observable semantics without head-of-line blocking.
- **Three-phase connect**: adapter chains `onBeforeConnect` → `createConnState` → `onConnect` inside one `ConnectionOpen` arm. Phase-specific rejection and conn-visibility invariants preserved.
- **Sleep sequence**: matches reference order. Adapter drives disconnect via `ctx.disconnect_conns` before replying.
- **Independent timeouts**: each callback wrapped individually; `sleep_grace_period` bounds overall Sleep; `on_destroy_timeout` bounds `onDestroy`.
- **`AbortSignal`**: synthesized by NAPI from its own `CancellationToken`. Cancelled only on `Destroy` and adapter end-of-life. Matches reference.
- **Dirty flag cleared inside `serializeForTick`**: safe because the `@rivetkit/on-change` handler fires per mutation (not per "newly dirty" transition), so mutations during the Rust-side KV write re-flag immediately. One TSF per save, matches `feat/sqlite-vfs-v2:state-manager.ts:#savePersistInner` flag-clear timing. For `reason === "inspector"`, flags are NOT cleared — next Save still persists them.
- **Unified `SerializeState` event with reason**: core fires one `ActorEvent::SerializeState { reason, reply }` covering Save and Inspector paths. Sleep/Destroy are separate termination events with `Reply<()>` — adapter owns its shutdown sequence and persists via explicit `ctx.save_state(deltas)` if it wants. TS `serializeState(reason)` TSF is the single serialization entrypoint.
- **Inspector overlay model**: core tracks last-written KV bytes as base, fires `SerializeState(Inspector)` on dirty flips when an inspector is attached, distributes reply bytes to subscribers without writing to KV. Zero cost when detached.
- **`ctx.keepAwake(promise)`**: TS-only adapter helper. Pushes promise into JoinSet via a TSF. Blocks Sleep AND Destroy, falls inside `sleep_grace_period` / `on_destroy_timeout`. On Destroy, promises observing `ctx.abortSignal()` can short-circuit.
- **Alarm `Action.conn`**: `null` (not a synthetic conn). User action code must handle `conn == null`. Migration note for actors that assumed conn-always-present.
- **`maxWait`**: supported via core `ctx.request_save_within(ms)`, exposed as `ctx.requestSaveWithin(ms)` on the JS ctx wrapper. Load-bearing for hibernatable WebSocket ack state.
- **Per-conn state has no user-facing callback**: matches reference. Mutations tracked via dirty flag and serialized as `StateDelta::ConnHibernation` only.
- **Per-conn dirty granularity**: `connectionManager.connsWithPersistChanged: Set<Conn>` + `removedHibernatableConnIds: Set<string>`. All flushed atomically.
- **No panics in adapter loop**: all user code in isolated tasks or TSF calls.

## What this spec does not change

- TS public actor-authoring API.
- `CoreRegistry` / `NapiActorFactory` / `ActorContext` NAPI class surfaces (except additive: `request_save`, `request_save_within`, `save_state`, `disconnect_conn`, `disconnect_conns`, `restart_run_handler` on `ActorContext`).
- Wire protocol, KV layout, inspector HTTP API.
- Engine startup / `serve()` / `startEnvoy()` plumbing.
- Workflow engine integration shape.

## Open questions

1. **`Reply<T>` drop-guard + per-conn gate release.** Core's per-conn causality requires that every conn-scoped Reply's send OR drop release the gate. Implementation: wrap `oneshot::Sender<T>` in a guard struct with a `Drop` impl that fires a hook. Both paths must be idempotent or guaranteed single-fire (e.g. `AtomicBool` flag consumed by whichever path fires first).

2. **Core-side `inspector_serialize_state_interval` default.** Pinned as ~50ms in the core spec text. Could be higher if per-mutation bursts cause CPU load on the Node thread. Measure during implementation, revise.

3. **`ctx.conns()` iterator lock-freeness.** `scc::HashMap` supports iterator-like patterns via `scan`/`scan_async`. Need to verify the adapter-side wrapper around it doesn't force a `Vec` allocation for Sleep/Destroy's linear walk.

4. **Core-side `ActorStart.input` lifetime.** Reference keeps `input` on `persistData.input` for the actor's lifetime. Core spec shows `input: Option<Vec<u8>>` on `ActorStart`. Decide: drop after preamble (forces adapter to stash it), or keep on `ctx.input()` accessor (mirrors reference, small memory cost). Lean: keep.

5. *(resolved)* Inspector subscriber fan-out uses `tokio::sync::broadcast` per actor; inspector HTTP handlers subscribe on attach via `ctx.subscribe_inspector()` and stream to their WS/SSE client. Pinned in the core spec's "Inspector integration" section.
