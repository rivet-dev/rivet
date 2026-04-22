# RivetKit Core State Management

This page is the short version of the actor state contract shared by `rivetkit-core`, `rivetkit-napi`, and the TypeScript runtime adapter. The important bit: runtime actor state is delta-only after boot. Do not bring back public replace/mutate APIs just because a call site wants a shortcut. That shortcut is how this shit gets weird.

## Ownership

- `rivetkit-core` owns persistence, save scheduling, KV writes, save completion tracking, and persisted connection/schedule metadata.
- The foreign runtime owns user-level state serialization. For TypeScript actors, JS keeps the live `c.state` object and returns encoded deltas through the NAPI `serializeState` callback.
- NAPI only translates between JS values and core types. It should not decide whether state is dirty, when a save is durable, or how deltas are applied.

## API Surface

- `ActorContext::set_state_initial(bytes)` installs the bootstrap snapshot before lifecycle/dispatch work starts. It is the only state-replacement path and should stay boot-only.
- `ActorContext::request_save(RequestSaveOpts { immediate, max_wait_ms })` is a save hint. It marks a save request, emits `LifecycleEvent::SaveRequested`, and lets the runtime serialize state later.
- `ActorContext::request_save_and_wait(opts)` uses the same request path, then waits until the matching save request revision completes. TypeScript uses this for immediate durable saves.
- `ActorContext::save_state(Vec<StateDelta>)` applies structured runtime output. Deltas can replace the actor-state blob, persist hibernatable connection bytes, or remove hibernation records.
- `ActorContext::persist_state(SaveStateOpts)` is internal core persistence for core-owned dirty data, shutdown cleanup, and schedule metadata. It persists the current `PersistedActor` snapshot and should not become a user-facing runtime mutation API.

## Save Flow

1. User code mutates runtime-owned state, such as the TypeScript `c.state` object.
2. The runtime calls `request_save(...)`, or core calls it after core-owned hibernatable connection state changes.
3. `ActorTask` receives `LifecycleEvent::SaveRequested` and dispatches `SerializeState { reason: Save }` through the foreign-runtime callback.
4. The runtime returns `StateDelta` values.
5. Core applies those deltas with `save_state(...)`, writes the encoded records to KV, updates in-memory snapshots, and marks the save request revision complete.

Immediate saves are the same flow with a zero debounce and a waiter. They must not bypass `serializeState`.

## Delta Contract

- `StateDelta::ActorState(bytes)` replaces the persisted actor-state blob under the single-byte KV key `[1]`.
- `StateDelta::ConnHibernation { conn, bytes }` writes hibernatable connection state under the connection KV prefix.
- `StateDelta::ConnHibernationRemoved(conn)` removes a persisted hibernatable connection record.

Core prepares the write batch while holding the save guard, then releases the guard before awaiting KV. Waiters that need durability use save-request revisions or the in-flight write counter rather than holding the save guard across I/O.

## Do Not Reintroduce

- Public `set_state` or `mutate_state` on core or NAPI actor contexts.
- Boolean `saveState(true)`-style shims. JS callers should use `requestSave({ immediate, maxWaitMs })`, `requestSaveAndWait(...)`, or structured `saveState(deltas)`.
- Direct `serializeForTick("save")` calls from TypeScript save sites. Durable saves should go through native `serializeState` dispatch so immediate and deferred behavior stays one path.
- TS-side hibernatable connection dirty flags. `ConnHandle::set_state` owns dirty tracking for hibernatable conns.
