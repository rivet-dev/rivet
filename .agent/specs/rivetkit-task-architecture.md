# rivetkit-core Actor Lifecycle + Concurrency Architecture

Status: **DRAFT — design direction accepted, implementation not started**.

This supersedes the earlier "one actor task owns everything" draft. The direction is now:

- **Actor task owns lifecycle coordination**: startup, ready/started state, sleep, destroy, restart, run-handler supervision, and child-task draining.
- **Mutable user-layer state stays concurrent**: state, connections, queue, KV, SQLite, broadcasts, and WebSocket callbacks use concurrency-safe primitives where that is the natural ownership model.
- **Subsystems notify lifecycle by events**: state mutations, connection activity, queue waits, WebSocket callback activity, and save scheduling emit bounded lifecycle events instead of forcing every operation through the actor task.
- **Implementation scope is `rivetkit-core` plus minimal `envoy-client` user-layer glue**: do not touch `rivetkit-napi` or the TypeScript `rivetkit` package for this work.

Implementation source scope:

- `rivetkit-rust/packages/rivetkit-core/src/actor/context.rs`
- `rivetkit-rust/packages/rivetkit-core/src/actor/lifecycle.rs`
- `rivetkit-rust/packages/rivetkit-core/src/actor/action.rs`
- `rivetkit-rust/packages/rivetkit-core/src/actor/sleep.rs`
- `rivetkit-rust/packages/rivetkit-core/src/registry.rs`
- `engine/sdks/rust/envoy-client/` only where user-layer lifecycle integration requires it

## Goals

- Preserve the TypeScript actor lifecycle semantics while moving load-bearing coordination into Rust.
- Reduce scattered lifecycle atomics and untracked `tokio::spawn` calls.
- Keep changes to `envoy-client` minimal and focused on user-layer integration points.
- Avoid routing KV, SQLite, queue, and other inherently concurrent subsystems through a lifecycle mailbox.
- Use bounded queues with explicit overload errors instead of unbounded memory growth.

## Non-Goals

- No wire protocol changes.
- No KV, queue, SQLite, or persisted snapshot layout changes.
- No public TypeScript API redesign unless required to expose an overload/error condition correctly.
- No large `envoy-client` refactor.
- No edits to `rivetkit-typescript/packages/rivetkit-napi/`.
- No edits to `rivetkit-typescript/packages/rivetkit/`.
- No new lifecycle semantics for TypeScript-only `vars`; existing compatibility APIs can remain untouched until a separate bridge/package migration.
- No work on the suspended high-level Rust `rivetkit` wrapper except where core API shape needs to be recorded.

## Invariants

- **No next-instance waiting**: work arriving while an actor instance is `Sleeping`, `Destroying`, or `Terminated` fails fast. It does not wait for the next actor instance.
- **No implicit queueing behind lifecycle shutdown**: new actions, HTTP requests, WebSocket opens/messages/closes, inspector actions, and scheduled events are rejected once shutdown begins.
- **Engine-side Ready gate**: the engine gateway holds inbound client requests until the actor publishes its engine-side Ready signal (see "Startup Flow"). Core rejection of work before `Started` is defense-in-depth and should not normally be observable to callers.
- **Sleep and destroy both drain tracked work**: work accepted before shutdown is tracked and allowed to finish until the effective sleep grace period expires. This matches the TypeScript runtime (both paths call `#waitShutdownTasks` with the same grace).
- **Grace period is the shutdown cap**: after the effective sleep grace period expires, remaining tracked work is aborted via the cancellation token and the actor instance proceeds with shutdown.
- **Destroy differs from sleep in the callback and hibernatable handling**: destroy runs `on_destroy` (with its own `on_destroy_timeout`) instead of `on_sleep`, preserves hibernatable connections the same way sleep does, and skips the idle-sleep-window wait.
- **No silent lifecycle event loss**: if a required lifecycle event cannot be reserved/sent, the originating operation fails with `actor/overloaded`.
- **State mutations are serializable**: `mutate_state(callback)` holds a synchronous mutation lock, performs no I/O, and commits one mutation at a time.
- **State mutation does not wait for hooks**: `mutate_state(callback)` returns after the state write and lifecycle event enqueue; `on_state_change` runs later.
- **Re-entrant state mutation errors**: mutating state from inside an `on_state_change` callback fails with `actor/state_mutation_reentrant` rather than deadlocking or silently livelocking.
- **Direct KV/SQLite may fail during shutdown**: direct subsystem calls are not routed through the actor task and may fail when the actor is shutting down; this must produce an explicit warning.

## Option C Proposal: Lifecycle Task + Concurrent Runtime Primitives

Option C is the hybrid model:

- **One actor lifecycle task per actor instance** manages state transitions and task supervision.
- **One user task per in-flight user operation** runs actions, HTTP callbacks, `run`, and WebSocket callbacks.
- **One lifetime task per WebSocket** is acceptable and preferred over moving WebSocket transport deeper into `envoy-client`.
- **Direct subsystem access remains direct** for KV, SQLite, queue, events, broadcasts, and similar APIs.
- **Lifecycle events bridge concurrent work back to the actor task** without making the lifecycle loop a global lock.

This keeps the actor task small: it coordinates lifecycle, but it does not become a giant serialized executor for all actor behavior.

## Core Types

```rust
struct ActorTask {
	actor_id: String,
	generation: u32,
	lifecycle_inbox: mpsc::Receiver<LifecycleCommand>,
	dispatch_inbox: mpsc::Receiver<DispatchCommand>,
	lifecycle_events: mpsc::Receiver<LifecycleEvent>,
	children: JoinSet<ActorChildOutcome>,

	lifecycle: LifecycleState,
	callbacks: Arc<ActorInstanceCallbacks>,
	ctx: ActorContext,

	run_handle_active: bool,
	sleep_timer_active: bool,
	destroy_requested: bool,
	stop_requested: Option<StopReason>,
}

enum LifecycleState {
	Loading,
	Migrating,
	Waking,
	Ready,
	Started,
	Sleeping,
	Destroying,
	Terminated,
}

enum LifecycleCommand {
	Start { reply: oneshot::Sender<Result<()>> },
	Stop { reason: StopReason, reply: oneshot::Sender<Result<()>> },
	FireAlarm { reply: oneshot::Sender<Result<()>> },
}

enum DispatchCommand {
	Action { request: ActionRequest, reply: oneshot::Sender<ActionResult> },
	Http { request: OnRequestRequest, reply: oneshot::Sender<HttpResult> },
	OpenWebSocket { request: OnWebSocketRequest, reply: oneshot::Sender<Result<()>> },
}

enum LifecycleEvent {
	StateMutated { reason: StateMutationReason },
	ActivityDirty,
	UserTaskFinished { kind: UserTaskKind },
	SaveRequested { immediate: bool },
	SleepTick,
}
```

The exact enum names can change during implementation. The important contract is that command senders get a request/response path, while lifecycle events are bounded notifications used to re-evaluate lifecycle.

`ActivityDirty` is a coalesced notification used by connection, queue, WebSocket callback, and other high-churn activity paths. Each subsystem owns a dirty flag plus a "notification pending" atomic. A subsystem mutation sets the dirty flag unconditionally; if the pending flag CAS'es from 0 to 1 the subsystem sends a single `ActivityDirty` event. The actor task clears the pending flag before re-reading all activity counters, so further mutations during processing produce exactly one follow-up event. Result: unbounded connection/queue churn produces at most one in-flight `ActivityDirty` event per actor at a time, regardless of mutation rate.

`DestroyRequested` is intentionally not a lifecycle event. Destroy is requested through `LifecycleCommand::Stop { reason: StopReason::Destroy, .. }` so it shares the command path with other lifecycle transitions and waits for its reply.

### Command Channel Split (Resolution of #7)

Lifecycle commands and dispatch commands use separate bounded senders so a burst of actions cannot starve a `Stop` or `FireAlarm`.

- `lifecycle_inbox`, default capacity `64`. Rare, bursty-in-small-increments, must always be reachable. Used for `Start`, `Stop`, `FireAlarm`. Overload is a bug — if 64 lifecycle commands back up, something else is already broken.
- `dispatch_inbox`, default capacity `1024`. High-throughput. Used for `Action`, `Http`, `OpenWebSocket`. Overload returns `actor/overloaded { channel: "dispatch_inbox", .. }` to the caller.

The actor task selects over both inboxes and the lifecycle event receiver. Priority within `tokio::select!` is biased toward `lifecycle_inbox` (use `tokio::select! { biased; .. }`) so a saturated dispatch inbox cannot delay lifecycle transitions.

Neither channel carries queue operations. Queue `send`, `enqueue_and_wait`, `next`, `wait_for_names`, and `complete` remain direct on the `Queue` handle (see "Queue").

### Supporting Enums

```rust
enum StopReason {
	Sleep,
	Destroy,
}

enum UserTaskKind {
	Action,
	Http,
	WebSocketLifetime,
	WebSocketCallback,
	QueueWait,
	RunHandler,
	OnStateChange,
	ScheduledAction,
	DisconnectCallback,
	WaitUntil,
}

enum StateMutationReason {
	UserSetState,
	UserMutateState,
	InternalReplace,
	ScheduledEventsUpdate,
	InputSet,
	HasInitialized,
}

enum ActorChildOutcome {
	UserTaskFinished { kind: UserTaskKind, result: Result<()> },
	RunHandlerFinished { result: Result<()> },
	UserTaskPanicked { kind: UserTaskKind, payload: Box<dyn std::any::Any + Send> },
}
```

- `StopReason` is carried on `LifecycleCommand::Stop` and determines which shutdown flow runs.
- `UserTaskKind` labels tracked child tasks for metrics and drain reporting.
- `StateMutationReason` labels `LifecycleEvent::StateMutated` for metrics.
- `ActorChildOutcome` is what each spawned child task returns via the `JoinSet`. The actor task converts it into the appropriate reply and a corresponding `LifecycleEvent::UserTaskFinished` if needed. There are two bookkeeping paths (JoinSet outcome plus lifecycle event); they are the same logical signal and must not be double-counted.

### Main Loop Sketch

```rust
impl ActorTask {
	async fn run(mut self) -> Result<()> {
		loop {
			tokio::select! {
				biased;
				Some(cmd) = self.lifecycle_inbox.recv() => self.handle_lifecycle(cmd).await,
				Some(event) = self.lifecycle_events.recv() => self.handle_event(event).await,
				Some(outcome) = self.children.join_next() => self.handle_child_outcome(outcome),
				Some(cmd) = self.dispatch_inbox.recv(), if self.accepting_dispatch() => self.handle_dispatch(cmd),
				_ = self.sleep_tick(), if self.sleep_timer_active => self.on_sleep_tick().await,
				else => break,
			}

			if self.should_terminate() {
				break;
			}
		}
		Ok(())
	}
}
```

`biased;` gives lifecycle commands and lifecycle events priority over dispatch, so a saturated `dispatch_inbox` cannot delay a `Stop`. `accepting_dispatch()` returns false once lifecycle is `Sleeping`/`Destroying`/`Terminated`. Dispatch commands received while `accepting_dispatch()` is false sit in the inbox and are rejected as soon as the select picks them up — by that point `handle_dispatch` returns the appropriate `Stopping`/`Destroying` error.

### Stop During Startup

`Start` and `Stop` both flow through `lifecycle_inbox`. While the actor task is processing `Start` (awaiting step 1-14 of startup), a `Stop` sits in `lifecycle_inbox` and is picked up as soon as startup returns. The registry awaits the `Start` reply before considering the actor live, so the effective sequence is `Start -> reply -> Stop -> reply`. If startup fails, the pending `Stop` still drains cleanly because the task returns to the main loop to consume it.

If the registry needs to pre-empt an in-progress startup (e.g. namespace deleted mid-start), it sends `Stop` anyway; startup observes `self.cancellation_requested()` at well-defined yield points (between major steps) and returns early with an error. The pending `Stop` then runs the destroy flow.

## Startup Flow

Ordering matches the TypeScript runtime in `rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts:.start()` on `feat/sqlite-vfs-v2`.

1. Load persisted actor from KV.
2. Build runtime-backed `ActorContext`.
3. Create callbacks through the `ActorFactory`.
4. Initialize core-owned state and mark `has_initialized`.
5. Run `on_migrate`.
6. Restore hibernatable connections. TS restores during `#loadState` so `on_wake` can observe them.
7. Run `on_wake`.
8. Initialize alarms (re-sync scheduled events against the driver alarm).
9. Set internal lifecycle `Ready`.
10. Run any driver hook (`on_before_actor_start`).
11. Set internal lifecycle `Started`.
12. Reset sleep timer.
13. Spawn `run` as a tracked child task.
14. Drain overdue scheduled events (the `on_alarm` pump). This is the final step and may spawn tracked work that continues after `.start()` returns.

Engine-side Ready publication is handled by the existing driver layer (envoy-client): the driver's `on_before_actor_start` (step 10) or a subsequent driver event is what signals the engine that the actor can accept tunneled traffic. This is not a new lifecycle step owned by core; the spec does not introduce a separate "publish engine Ready" action.

Client-visible safety: the engine gateway already holds inbound client requests waiting for the actor's `Ready` signal, up to `ACTOR_READY_TIMEOUT` (10s, with retry across `Stopped` events), so callers do not observe the actor's internal pre-`Started` states.

Within core, actions, HTTP callbacks, WebSocket callbacks, and scheduled events are rejected until internal lifecycle reaches `Started`. This should be unreachable through the gateway path and acts as defense-in-depth for in-process callers (inspector, reconciler). They also fail fast after the actor enters `Sleeping`, `Destroying`, or `Terminated`. Queue operations do not go through this gate; see "Queue".

## User Work

Actions and callbacks run outside the actor task:

```rust
fn handle_dispatch(&mut self, command: DispatchCommand) {
	let reply = command.reply_sender();
	match self.lifecycle {
		LifecycleState::Sleeping => {
			let _ = reply.send(Err(ActorLifecycle::Stopping.build().into()));
			return;
		}
		LifecycleState::Destroying | LifecycleState::Terminated => {
			let _ = reply.send(Err(ActorLifecycle::Destroying.build().into()));
			return;
		}
		LifecycleState::Started => {}
		_ => {
			let _ = reply.send(Err(ActorLifecycle::NotReady.build().into()));
			return;
		}
	}

	let callbacks = self.callbacks.clone();
	let ctx = self.ctx.clone();
	let kind = command.user_task_kind();
	self.children.spawn(async move {
		let guard = ctx.begin_user_task(kind);
		let result = command.invoke(&callbacks, &ctx).await;
		drop(guard);
		ActorChildOutcome::UserTaskFinished { kind, result }
	});
}
```

Rules:

- **Do not hold the actor task while user code runs**.
- **Do track every user task** so sleep/destroy can drain correctly.
- **Destroy waits for in-flight user work** before dropping the actor instance.
- **New work after stop/destroy starts fails fast** with a structured lifecycle error. It does not attach to the dying instance and does not wait for a replacement instance.

## State Mutation

Do not model state mutation as `SetState` over the actor command channel.

Use a concurrency-safe state primitive with a mutation callback:

```rust
impl ActorStateHandle {
	pub fn mutate_state<F>(&self, reason: StateMutationReason, mutate: F) -> Result<()>
	where
		F: FnOnce(&mut Vec<u8>) -> Result<()>,
	{
		let permit = self
			.lifecycle_tx
			.try_reserve()
			.map_err(|_| actor_overloaded("state mutation lifecycle event queue is full"))?;

		{
			let mut state = self.state.write();
			mutate(&mut state)?;
			self.snapshot.store(state.clone());
			self.mark_dirty();
		}

		permit.send(LifecycleEvent::StateMutated { reason });
		Ok(())
	}
}
```

Important details:

- **Reserve lifecycle-event capacity before mutating** so overload cannot leave state changed without a lifecycle notification.
- **Mutations are serializable**: the callback runs while holding the state mutation lock and must not do I/O, await, or call back into actor APIs.
- **Mutation returns after the state write**, not after the actor loop processes the lifecycle event.
- **`set_state(bytes)` becomes a replace wrapper** around `mutate_state`.
- **`on_state_change`, save scheduling, sleep reset, and inspector updates are triggered from the lifecycle event path**.
- **State reads stay concurrent** through the current snapshot.

This gives user code fast mutation while keeping lifecycle coordination explicit.

### `on_state_change` Dispatch

Keep the existing coalescing + single-runner behavior already implemented in `state.rs:394-460`:

- Each successful state mutation increments a revision and bumps `pending`.
- `running` is set when the runner starts; the runner loop drains `pending` one callback at a time, reading the latest state on each iteration, and exits when `pending == 0`.
- This matches the TS runtime's one-callback-per-mutation model (TS `state-manager.ts` is synchronous inline; the Rust runtime keeps the spawned-runner optimization because it must not block the mutating caller).
- If lifecycle event capacity cannot be reserved before a mutation, the mutation fails with `actor/overloaded` before changing state.

### Re-Entrant `mutate_state`

Re-entrant mutation (calling `mutate_state` from inside an `on_state_change` callback) must fail explicitly rather than deadlocking or livelocking. This is a deliberate divergence from the TS runtime (which silently no-ops via `!this.#isInOnStateChange`): an explicit error makes user-code bugs visible.

- The `in_callback` flag lives on `ActorContext` (shared with subsystems), not in a `tokio::task_local!`, so it is visible to nested spawned tasks that share the same `ActorContext`. A mutation from *any* task while the callback is running fails.
- `mutate_state` checks the flag first and returns `actor/state_mutation_reentrant` before reserving a lifecycle permit or taking the write lock.
- Driver-suite regression: "calling `set_state` from `on_state_change` returns a structured error."

### `on_state_change` Execution Site

The callback continues to run in a detached runtime task (current `state.rs:439` behavior), but the runner task is registered with the actor's `JoinSet` as a tracked user task so sleep/destroy drain can wait for a trailing callback.

## TypeScript Vars

Remove `vars` from the new core lifecycle model.

`vars` is a TypeScript-package convenience for ephemeral object references. Other user runtimes can pass their own captured runtime state through callbacks, actor structs, closures, or language-native context objects. Core does not need to own a generic `vars: Vec<u8>` blob for lifecycle correctness.

Current source still has `ActorVars` because the existing TypeScript/NAPI bridge exposes `ctx.vars` and `ctx.setVars`. For this scoped task:

- Do not expand `ActorVars`.
- Do not add new `ActorVars` call sites.
- Do not add `LifecycleEvent::VarsMutated`.
- Do not make sleep, state-save, or shutdown behavior depend on vars mutation.
- Leave existing bridge compatibility untouched unless the task scope explicitly expands to `rivetkit-napi` and the TypeScript `rivetkit` package.

The explicit end state is to remove `ActorVars`, `ActorContext::vars`, `ActorContext::set_vars`, `create_vars_timeout`, and vars-related startup metrics from `rivetkit-core` once TypeScript vars ownership is moved fully out of core. TypeScript should own its vars cache/objects at the TypeScript layer.

## Public Actor Context Surface

These TS APIs must be mirrored on `ActorContext` and are load-bearing for existing driver tests.

- `ctx.set_prevent_sleep(enabled: bool)` — toggle the `prevent_sleep` flag observed by `#canSleep` and by the `#waitShutdownTasks` drain loop. While set, the drain loop keeps looping up to the grace deadline even if every tracked counter is zero.
- `ctx.keep_awake<F>(future: F) -> impl Future` — enter an external keep-awake region for the duration of `future`. Increments `active_async_regions.keep_awake` on entry and decrements on exit via a guard. User-facing.
- `ctx.internal_keep_awake<F>(future: F) -> impl Future` — same pattern but increments `active_async_regions.internal_keep_awake`. Subsystems (queue, websocket) use the thunk form to enter the region before user callback starts, avoiding a race where the sleep timer fires underneath newly scheduled work.
- `ctx.cancelled() -> impl Future<Output = ()>` and `ctx.is_cancelled() -> bool` — alias the existing `abort_signal()` and `is_cancelled()` surface in `context.rs` (`context.rs:142, 324, 362-367`). Do not remove the existing names; add the new names as aliases to avoid churning callers.
- `ctx.restart_run_handler()` — force-restart the `run` handler. Drops the current tracked `run` task and respawns. Matches TS `restartRunHandler` (`instance/mod.ts:923-935`). Used by drivers.

Each keep-awake region decrement must fire the coalesced `ActivityDirty` lifecycle event so sleep readiness re-evaluates when a region closes.

## Direct Subsystems

These are not owned by the actor task:

- **KV**: direct `Arc<Kv>` access.
- **SQLite**: direct `Arc<SqliteDb>` access.
- **Queue**: direct queue handle, with its own concurrency/backpressure behavior.
- **Events/broadcast**: direct broadcaster.
- **Connections**: concurrent manager with lifecycle activity events.
- **Inspector reads**: direct snapshots/concurrent handles.

This avoids deadlocks where user code inside an action needs a subsystem while the actor task is supervising that action.

Direct subsystem rules:

- KV and SQLite operations are caller-owned direct operations.
- KV and SQLite operations do not keep the actor alive by themselves.
- If KV or SQLite is called while the actor is shutting down and the operation fails, log `tracing::warn!` with actor id, subsystem, operation, and lifecycle state.
- Queue and connection mutations that affect sleep readiness must reserve lifecycle notification capacity before mutating. If reservation fails, fail the mutation with `actor/overloaded`.
- For finish/drop paths that cannot fail after work already ran, use guards that update counters synchronously on drop instead of relying on a fallible finish event.

## Queue

Queue is entirely actor-local KV-backed storage driven by user code running on the same actor. There is no engine-dispatched queue handler; the engine never invokes a user callback on receipt of a queue message. Consumption is pull-model: user code in an action, `run`, or similar calls `queue.next`, `queue.next_batch`, `queue.wait_for_names`, etc.

All queue operations stay direct on the `Queue` handle:

- `send(name, body)` and `enqueue_and_wait(name, body, opts)` (producer side).
- `next(opts)`, `next_batch(opts)`, `wait_for_names(...)`, `wait_for_names_available(...)`, `try_next(opts)`, `try_next_batch(opts)` (consumer side).
- `QueueMessage::complete(response)` and `CompletableQueueMessage::complete(response)` (completion side).
- `enqueue_and_wait` uses an in-process `oneshot::Sender` stored in `completion_waiters: HashMap<u64, oneshot::Sender>`, consumed by `complete` on the same actor.

No new `ActorCommand` variants exist for queue. Lifecycle gating is implicit: the caller is already running as tracked user work under the actor task (action, `run`, HTTP callback, WebSocket callback), which has already been lifecycle-checked at dispatch time.

Lifecycle contract:

- `ActiveQueueWaitGuard` increments/decrements `active_queue_wait_count` and fires the existing `wait_activity_callback`. Rewire that callback to update the coalesced activity dirty bit and emit a single `ActivityDirty` lifecycle event, so sleep readiness sees queue waits without one event per wait.
- Queue size updates fire the existing `inspector_update_callback` directly; this path does not need a lifecycle event.
- Queue overload (`queue/full`, `queue/message_too_large`) uses existing explicit `queue/*` errors, independent of actor lifecycle overload.
- Abort: `Queue::new` already accepts an `abort_signal: Option<CancellationToken>`; wire this to the actor's cancellation token so `wait_for_message` and `wait_for_names` unwind promptly during sleep/destroy.
- `wait_for_completion_response` (the `enqueue_and_wait` completion wait) intentionally does NOT observe the cancellation token — TS behavior at `queue-manager.ts:219-245` also does not abort completion waits. The caller's surrounding user-task guard is aborted at the shutdown grace deadline, which then drops the receiver.

## WebSockets

Use **one lifetime task per WebSocket** at the user layer.

Intent:

- Keep `envoy-client` changes minimal.
- Let core own user callback supervision.
- Track WebSocket callback activity for sleep readiness.
- Close/log explicitly on callback errors.
- Do not spawn a new task per WebSocket message unless implementation proves it is required for existing behavior.

Sketch:

```rust
self.children.spawn(async move {
	let ws_guard = ctx.begin_websocket_lifetime_task(conn_id);
	let result = run_websocket_lifetime(ctx, callbacks, ws).await;
	ActorChildOutcome::WebSocketFinished { conn_id, result }
});
```

The lifetime task owns the socket loop and invokes open/message/close callbacks. Multiple WebSockets can run concurrently. Within one WebSocket, callbacks should run in that socket's lifetime task unless existing compatibility requires per-callback concurrency.

## Sleep

Sleep readiness stays centralized in core. It reads concurrent counters/snapshots matching the TS `#canSleep()` check (`instance/mod.ts:2497-2528` on `feat/sqlite-vfs-v2`):

- not started / not ready
- `prevent_sleep` flag
- no-sleep config
- active HTTP requests
- user tasks in flight (`active_async_regions.user_task`)
- internal keep-awake regions (`active_async_regions.internal_keep_awake`)
- external keep-awake regions (`active_async_regions.keep_awake`)
- active queue waits
- active connections
- pending disconnect callbacks
- active WebSocket callbacks (`active_async_regions.websocket_callbacks`)

Asymmetric interaction that must be preserved: the `run` handler being active blocks sleep UNLESS the handler is currently blocked inside a queue wait (`active_queue_wait_count > 0`). This lets actors whose `run()` loops on `queue.next()` go to sleep — without it, such actors can never sleep. Mirror `mod.ts:2509`.

### Sleep Grace Period

Match the TypeScript runtime (`rivetkit-typescript/packages/rivetkit/src/actor/config.ts` on `feat/sqlite-vfs-v2`):

- Default `sleepGracePeriod = 15_000` ms.
- If the user overrides `onSleepTimeout` or `waitUntilTimeout` without setting `sleepGracePeriod`, the effective grace period is `onSleepTimeout + waitUntilTimeout`.
- Default `onSleepTimeout = 5_000` ms (the `on_sleep` callback deadline).
- Default `waitUntilTimeout = 15_000` ms (the `wait_until` registered task deadline).
- Default `runStopTimeout = 15_000` ms (the `run` handler abort-grace deadline).

These defaults are shared with the existing TS runtime and should live in `ActorConfig`.

### Sleep Flow

Mirrors `instance/mod.ts:.onStop("sleep")` at `:942-1022` on `feat/sqlite-vfs-v2`.

1. Cancel the sleep timer.
2. Wait for the idle-sleep window (TS `#waitForIdleSleepWindow`).
3. Move lifecycle to `Sleeping`.
4. Raise the actor's cancellation token (Rust analogue of `AbortController.abort()`, observable by user code via `ctx.cancelled()`).
5. Stop accepting new dispatch commands. `lifecycle_inbox` still accepts `Stop`/`Start` for the registry transition.
6. Wait for the `run` handler to finish with `run_stop_timeout` (default 15s). Done first so `on_sleep` observes `run` already stopped.
7. Compute the shutdown task deadline = `now + effective_sleep_grace_period`.
8. Run `on_sleep` with `on_sleep_timeout`.
9. Drain tracked work until the shutdown task deadline: `preventSleep` flag must be clear AND every tracked counter must hit zero. Any newly-entered `preventSleep` region keeps the drain loop running until deadline.
10. Persist hibernatable connections.
11. Disconnect non-hibernatable connections. Hibernatable connections stay attached so they can be re-delivered on wake.
12. Drain tracked work again (this lets WS close callbacks finish).
13. Flush state immediately.
14. Wait for pending state writes and pending alarm writes.
15. Cleanup SQLite.
16. Cancel any driver-level alarm timer.
17. Abort any still-running tracked tasks. Their replies receive `actor/shutdown_timeout`.
18. Terminate the actor task.

## Destroy

Mirrors `instance/mod.ts:.onStop("destroy")`. Shares most steps with sleep:

1. Move lifecycle to `Destroying`.
2. Raise the cancellation token immediately (TS calls `#abortController.abort()` in `startDestroy` before the state change).
3. Stop accepting new dispatch commands.
4. Wait for the `run` handler to finish with `run_stop_timeout`.
5. Compute the shutdown task deadline = `now + effective_sleep_grace_period`.
6. Run `on_destroy` with `on_destroy_timeout` (default 5s, independent of the sleep grace period).
7. Drain tracked work until the shutdown task deadline. Same rules as sleep.
8. Disconnect non-hibernatable connections (destroy still preserves hibernatable the same way sleep does; TS `#disconnectConnections` honors the hibernatable flag).
9. Drain tracked work again (for WS close callbacks).
10. Flush state immediately.
11. Wait for pending state writes and pending alarm writes.
12. Cleanup SQLite.
13. Cancel any driver-level alarm timer.
14. Abort any still-running tracked tasks. Their replies receive `actor/shutdown_timeout`.
15. Mark destroy completion.
16. Terminate the actor task.

Differences from sleep:
- No idle-sleep-window wait.
- `on_destroy` instead of `on_sleep`, with its own timeout.
- No rearm of alarms, no preserved sleep-timer state.

New incoming work during sleep or destroy fails fast. It does not wait for another actor instance.

## Cancellation

User-observable cancellation is a `tokio_util::sync::CancellationToken` per actor instance, surfaced through `ActorContext::cancelled()` and `ActorContext::is_cancelled()`. This is the Rust analogue of the TS `AbortController`/`AbortSignal` exposed on the actor context.

- Sleep raises the token before running `on_sleep`.
- Destroy raises the token immediately at entry.
- User-layer long-running work (`run`, `wait_until`, queue waits, WebSocket lifetime tasks) is expected to observe cancellation cooperatively.
- Work that ignores cancellation is force-aborted via `JoinSet::abort_all` when its grace window expires.

## Tracked Work

Tracked work blocks sleep and destroy until it completes or the effective sleep grace period expires:

- Actions
- HTTP callbacks
- WebSocket lifetime tasks and in-flight callbacks
- Active queue waits (via the existing `ActiveQueueWaitGuard`)
- Scheduled action executions
- `run`
- `wait_until` registrations
- `ctx.keep_awake(...)` regions
- `ctx.internal_keep_awake(...)` regions
- `prevent_sleep` flag (holds the drain loop open even if all counters are zero)
- `on_state_change` runner task
- State saves
- SQLite cleanup
- Connection disconnect callbacks

Reply behavior:

- Work accepted before sleep/destroy should receive its normal result if it completes before the grace period expires.
- Work rejected after sleep or destroy starts receives a structured lifecycle error (`actor/stopping` during sleep, `actor/destroying` during destroy).
- If the grace period expires before accepted work completes, core should best-effort send `actor/shutdown_timeout` to pending replies before aborting remaining work.
- Panicked child tasks surface as `ActorChildOutcome::UserTaskPanicked`; the waiting `oneshot` receives an `actor/shutdown_timeout`-shaped error annotated with the panic payload via `tracing::error!` (no custom panic error variant).
- No accepted request should hang because a `oneshot` reply was dropped without a result or structured error.

## Backpressure

All actor-owned channels are bounded.

Defaults to start with:

- `lifecycle_inbox`: bounded, default `64`. Carries `Start`, `Stop`, `FireAlarm`. Overload here implies a bug elsewhere; failure is surfaced but not expected in normal operation.
- `dispatch_inbox`: bounded, default `1024`. Carries `Action`, `Http`, `OpenWebSocket`. Overload returns `actor/overloaded { channel: "dispatch_inbox", .. }` to the caller.
- `lifecycle_event_inbox`: bounded, default `4096`. Because connection/queue/WebSocket activity uses the coalesced `ActivityDirty` notification, inbox pressure is dominated by `StateMutated` and `UserTaskFinished`, both of which have one-to-one relationships with user operations rather than churn.
- Per-WebSocket inbound user-layer queue: not introduced. The lifetime-per-WebSocket model reads messages inline from the transport and invokes the user callback in the same task. The transport's own framing buffer provides backpressure.

On overload:

- Return a structured Rivet error: `actor/overloaded`.
- Include metadata: actor id, queue name, capacity, operation.
- Log a `tracing::warn!` once per rate window so overload is visible without log spam.
- Expose inspector/metrics counters for dropped or rejected lifecycle notifications and command sends.

No silent no-ops. No unbounded channel as the default escape hatch.

All lifecycle events are required. There is no best-effort lifecycle-event drop path. If an operation cannot reserve event capacity before making a lifecycle-relevant mutation, fail the operation before mutation. Release/drop paths that must not fail should use guards with synchronous counter updates.

## Error Taxonomy

All lifecycle errors use `rivet_error::RivetError` in the `actor` group and follow the `#[error(code, short, formatted?)]` shape used in `engine/packages/pegboard/src/errors.rs`. Definitions live in `rivetkit-rust/packages/rivetkit-core/src/error.rs` (or alongside existing core error modules) and are re-exported where the bridge needs them.

```rust
use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("actor")]
pub enum ActorLifecycle {
	#[error("not_ready", "Actor is not ready to accept work.")]
	NotReady,

	#[error("stopping", "Actor is sleeping and cannot accept new work.")]
	Stopping,

	#[error("destroying", "Actor is being destroyed and cannot accept new work.")]
	Destroying,

	#[error(
		"shutdown_timeout",
		"Actor shutdown grace period expired before the work completed."
	)]
	ShutdownTimeout,

	#[error(
		"overloaded",
		"Actor backpressure exceeded.",
		"Actor backpressure exceeded on {channel} (capacity {capacity}, operation {operation})."
	)]
	Overloaded {
		channel: String,
		capacity: usize,
		operation: String,
	},

	#[error(
		"state_mutation_reentrant",
		"Cannot mutate actor state from inside on_state_change."
	)]
	StateMutationReentrant,
}
```

Producer rules:

- `NotReady`: returned by the actor task when lifecycle is anything before `Started` and the command is not a lifecycle command (`Start`, `Stop`).
- `Stopping`: returned by the actor task when lifecycle is `Sleeping`.
- `Destroying`: returned by the actor task when lifecycle is `Destroying` or `Terminated`.
- `ShutdownTimeout`: returned to `oneshot` reply senders whose accepted work was aborted at the grace boundary.
- `Overloaded`: returned by any `try_reserve` / `try_send` failure on an actor-owned bounded channel; `channel` is one of `lifecycle_inbox`, `dispatch_inbox`, `lifecycle_event_inbox`.
- `StateMutationReentrant`: returned by `mutate_state` when invoked from within an `on_state_change` callback.

No custom non-`RivetError` types cross the runtime boundary for these failures.

## Warnings and Diagnostics

Add warnings for sharp edges:

- **Self-call / re-entrant dispatch risk**: warn when an actor tries to dispatch work to the same actor while current lifecycle state would park it behind the current instance.
- **Work sent to a stopping instance**: warn and fail fast with a structured lifecycle error.
- **Lifecycle event overload**: warn with actor id, event type, and channel capacity.
- **Long drain on sleep/destroy**: warn if user tasks exceed a configured diagnostic threshold before shutdown completes.

Warnings should be structured tracing fields, not formatted strings.

Warning rate limiting means suppressing repeated identical warnings after the first few logs in a short window, so one broken actor or overloaded queue does not spam logs forever. Use both:

- **Per-actor rate limit**: prevents a single actor from flooding logs.
- **Global rate limit**: protects the process if many actors hit the same warning at once.
- Suppressed warning counts should be emitted when the window resets.

## Envoy-Client Boundary

Allowed `envoy-client` changes:

- Minimal adapter/glue needed to hand user-layer HTTP/WebSocket lifecycle work to `rivetkit-core`.
- Small error propagation additions if required to return structured fail-fast lifecycle errors.

Forbidden unless absolutely necessary:

- Protocol changes.
- Reconnect behavior changes.
- Event batching or ack behavior changes.
- Transport task rewrites.
- Broad tunnel routing refactors.

If implementation appears to require a forbidden change, stop and split that into a separate design item before touching it.

## Metrics

Add these exact actor-scoped metrics unless implementation discovers a naming conflict:

- `lifecycle_inbox_depth` gauge
- `lifecycle_inbox_overload_total{command}` counter
- `dispatch_inbox_depth` gauge
- `dispatch_inbox_overload_total{command}` counter
- `lifecycle_event_inbox_depth` gauge
- `lifecycle_event_overload_total{event}` counter
- `user_tasks_active{kind}` gauge
- `user_task_duration_seconds{kind}` histogram
- `shutdown_wait_seconds{reason}` histogram
- `shutdown_timeout_total{reason}` counter
- `state_mutation_total{reason}` counter
- `state_mutation_overload_total{reason}` counter
- `on_state_change_total` counter
- `on_state_change_coalesced_total` counter
- `direct_subsystem_shutdown_warning_total{subsystem,operation}` counter

## Config

Add actor/core config for mailbox sizing and shutdown timing:

- `lifecycle_command_inbox_capacity`, default `64`
- `dispatch_command_inbox_capacity`, default `1024`
- `lifecycle_event_inbox_capacity`, default `4096`
- `sleep_grace_period`, default `15_000` ms (TS parity)
- `on_sleep_timeout`, default `5_000` ms (TS parity)
- `wait_until_timeout`, default `15_000` ms (TS parity)
- `run_stop_timeout`, default `15_000` ms (TS parity)
- `on_destroy_timeout`, default `5_000` ms (TS parity)

Defaults should start higher than expected production needs to avoid unnecessary false-positive overload while the architecture settles. The bounded behavior is still required; configurability is for tuning, not for switching to unbounded queues.

## Registry Binding

`RegistryDispatcher` should hold actor task handles instead of active callback/context structs:

```rust
struct ActorTaskHandle {
	actor_id: String,
	generation: u32,
	lifecycle: mpsc::Sender<LifecycleCommand>,
	dispatch: mpsc::Sender<DispatchCommand>,
	join: JoinHandle<Result<()>>,
}
```

The registry does not hold the `lifecycle_events` sender. Lifecycle events are produced by subsystems owned *inside* the actor task's `ActorContext` (state, queue, connection manager, websocket lifetime tasks); the sender lives on the context, and the receiver is owned by the actor task.

Starting an actor:

1. Build context and subsystem handles.
2. Spawn actor task.
3. Send `LifecycleCommand::Start`.
4. Insert handle only after successful startup.
5. If a stop arrives during startup, record it and deliver it after start resolves.

Stopping an actor:

1. Mark instance as stopping.
2. Send `LifecycleCommand::Stop`.
3. Await stop reply.
4. Await task join.
5. Remove from active/stopping maps.
6. Complete the stop handle and fail any not-yet-accepted work with a structured lifecycle error.

## Migration Plan

Use incremental option **A**. Each step must leave the crate building and the TypeScript driver suite green before moving on.

1. Introduce an `ActorTask` shell around the current callbacks with behavior unchanged. The task owns the command inbox and lifecycle event receiver but delegates all work to the existing callback structs.
2. Move lifecycle state transitions into the task (single writer for `LifecycleState`).
3. Move tracked child-task supervision into the task (`JoinSet<ActorChildOutcome>` owned by the task).
4. Move sleep and destroy coordination into the task so it consumes lifecycle events.
5. Move action, HTTP, and WebSocket dispatch spawning behind task commands so the task gates them on lifecycle state and tracks them as children.
6. Replace `set_state` internals with `mutate_state` + lifecycle events, now that the task is in place to consume `StateMutated`.
7. Introduce the coalesced `ActivityDirty` event for connection, queue, and WebSocket callback activity.
8. Add overload errors, structured lifecycle errors, and diagnostics.
9. Remove obsolete locks/atomics after behavior parity is proven.

Steps 1-3 establish the task so that steps 4-6 have a place to land events. Do not reorder these; emitting lifecycle events before the task exists would drop them or require a temporary sink.

Do not big-bang this. Keep the TypeScript driver suite as the oracle at each step, but do not modify the TypeScript `rivetkit` package or `rivetkit-napi` as part of this task.

## Test Plan

- Run targeted Rust tests for state mutation, lifecycle events, overload errors, sleep, destroy, and child-task draining.
- Run RivetKit TypeScript driver tests from `rivetkit-typescript/packages/rivetkit` for public behavior parity without changing those package sources.
- Add regression coverage for:
  - concurrent state mutation during actions
  - sleep waiting for tracked user work
  - destroy waiting for in-flight user work
  - bounded command overload
  - lifecycle event overload
  - WebSocket task activity blocking sleep
  - work arriving while an instance is stopping

Per repo rules, pipe test output to `/tmp/` and grep logs in follow-up implementation work.

## Accepted Decisions From Discussion

- **Q1**: Work arriving during sleep/destroy fails fast and does not wait for the next actor instance.
- **Q2**: There is no next-instance wait timeout because work does not wait.
- **Q3**: Concurrent `mutate_state(callback)` calls are serializable through the state write lock (the existing `RwLock::write()` on `ActorStateInner::current_state`); there is no separate "mutation lock."
- **Q4**: `mutate_state(callback)` returns before `on_state_change` runs.
- **Q5**: `on_state_change` uses the existing `OnStateChangeControl` pending/running counters (one callback per mutation, drained by a single runner task) rather than latest-state coalescing. Spec wording earlier in the document about "one trailing callback" is superseded by this.
- **Q6**: State mutation fails before changing state if lifecycle-event capacity cannot be reserved.
- **Q7**: Lifecycle-event overload always fails the originating operation; there is no best-effort lifecycle event drop path.
- **Q8**: Tracked work includes actions, HTTP callbacks, WebSocket lifetime tasks and callbacks, active queue waits, scheduled actions, `run`, `wait_until`, `keep_awake`/`internal_keep_awake` regions, `on_state_change` runner, state saves, SQLite cleanup, and connection disconnect callbacks. There is no engine-dispatched "queue handler."
- **Q9**: Destroy drains tracked work the same way sleep does, up to the effective sleep grace period, then aborts remaining work. This matches TS `#waitShutdownTasks` behavior.
- **Q10**: Sleep and destroy share the drain loop. They differ only in the idle-sleep-window wait (sleep only), the callback run (`on_sleep` vs `on_destroy`), and the `on_destroy_timeout` vs `on_sleep_timeout` budget for that callback.
- **Q11**: Pending replies should not hang; accepted work replies normally before grace expiry or gets a best-effort structured timeout error if aborted.
- **Q12**: Queue/connection mutations that affect sleep readiness reserve lifecycle notification capacity before mutation; release paths use synchronous guards.
- **Q13**: Direct KV/SQLite operations may fail during shutdown and must warn explicitly.
- **Q14**: WebSockets use one lifetime task per WebSocket; no per-message task unless required for compatibility.
- **Q15**: `envoy-client` changes are minimal adapter/glue only unless absolutely necessary.
- **Q16**: Remove `ActorVars` from core as a later compatibility phase after TypeScript vars ownership leaves core.
- **Q17**: Exact metrics are listed in this spec.
- **Q18**: Warning rate limiting uses both per-actor and global limits, with suppressed counts emitted when the window resets.
- **I1**: Use `mutate_state(callback)` instead of a `SetState` actor command.
- **I2**: Wait for tracked concurrency to finish before dropping an actor instance.
- **I3**: Define lifecycle/concurrency coordination around mutation events and tracked child tasks.
- **I4**: New work after stop/destroy fails fast.
- **I5**: Use bounded channels with overload errors.
- **I6**: Use channels where they make sense; do not channel-route KV/SQLite/queue hot paths.

## Open Follow-Up

- Decide final tuned channel capacities after implementation profiling.
- Bridge overload surfacing is later scoped work because this task does not touch `rivetkit-napi` or the TypeScript `rivetkit` package.
