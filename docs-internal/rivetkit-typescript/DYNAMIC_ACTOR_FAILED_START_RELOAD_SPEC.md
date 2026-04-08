# Dynamic Actor Failed-Start Reload Spec

## Status

Draft

## Summary

Dynamic actors need a first-class failed-start path that still allows a
driver-level `reload` endpoint to recover the actor immediately.

Today, reload is implemented as "sleep the running actor so the next request
loads fresh code." That works only when there is already a live dynamic actor
runtime. It does not handle the case where dynamic startup fails before the
runtime becomes runnable.

This spec defines identical behavior for the file-system and engine drivers:

- Failed dynamic startup leaves a host-side wrapper alive in memory.
- Normal requests receive a sanitized failed-start error.
- Failed starts use exponential backoff.
- Backoff is passive and must not create a background retry loop that keeps the
  actor effectively awake forever.
- `reload` bypasses backoff, immediately attempts a fresh startup, and returns
  the result.
- `reload` resets failure backoff state before attempting fresh startup.
- Reload on an already sleeping actor is a manager-side no-op so it does not
  wake, load, and immediately sleep again.
- All non-obvious lifecycle and routing behavior must be commented in code.

## Current Behavior

### Normal Sleep

Normal actor sleep removes the live in-memory dynamic runtime and host handler.
After sleep there is no live actor process in memory.

- File-system removes the runtime and actor entry during sleep.
- Engine removes the runtime and handler during stop.

This means "sleeping" is not a long-lived in-memory actor state. It is a
lifecycle fact plus persisted actor metadata such as `sleepTs`.

### Failed Start

Dynamic startup currently fails out of `DynamicActorIsolateRuntime.start()` or
`ensureStarted()`.

- File-system disposes partial runtime state and rethrows startup failure.
- Engine disposes partial runtime state, stores a transient startup error, and
  stops the actor.

There is no durable or explicit host-side failed-start state machine today. Any
future failed-start state introduced by this spec is intentionally ephemeral,
stored only in memory on the host-side wrapper, and cleared when that wrapper
is removed during normal sleep or stop cleanup.

### Reload

Reload is currently implemented as a pre-dispatch overlay route that sleeps a
running dynamic actor.

- This works for a running dynamic actor.
- This does not recover a failed startup cleanly.
- This also risks an unnecessary double-load if reload is sent to an already
  sleeping actor and the driver wakes it before intercepting reload.

## Goals

- Make failed-start behavior identical across file-system and engine drivers.
- Preserve the normal actor sleep lifecycle for actors that started
  successfully.
- Keep failed-start state in memory only.
- Return the actor's real startup error code to clients.
- Return full error detail only in development.
- Keep full failure detail in logs in all environments.
- Reuse existing exponential backoff logic instead of inventing a new bespoke
  retry algorithm.
- Make retry behavior configurable per dynamic actor.
- Add a configurable timeout for dynamic load and startup.
- Document all of this behavior in docs-internal.

## Non-Goals

- Persisting failed-start state across process restarts. Backoff state is
  intentionally reset on process restart. This means actors retry from initial
  backoff after a restart even if they previously reached max backoff. This is
  an acceptable trade-off of keeping state in memory only.
- Changing normal static actor lifecycle behavior.
- Hiding all startup failure information from clients. Clients should still
  receive a stable failure code.

## Scope

This spec requires parity for the file-system and engine drivers.

The memory driver does not currently participate in the normal sleep lifecycle,
so it is out of scope unless explicitly added as a follow-up.

## Terminology

### Dynamic Runtime

The isolate runtime that loads and executes dynamic actor code.

### Host Wrapper

The driver-side in-memory handler or entry that exists outside the dynamic
runtime. This wrapper can outlive a failed startup even when there is no live
dynamic runtime.

### Failed Start

Any failure in the dynamic startup pipeline before the actor becomes runnable,
including:

- loader execution
- source normalization or materialization
- sandbox/bootstrap setup
- `runtime.start()`
- `runtime.ensureStarted()`

## Required State Model

Dynamic actors need an explicit host-side runtime state for reload and failure
handling.

Recommended state shape:

- `inactive`
  The actor is not currently running. This includes the normal sleeping case.
  The host wrapper may or may not exist in this state. If the wrapper was
  removed by normal sleep cleanup, the actor is still logically inactive but
  reload is handled at the manager/gateway level (see Reload While Inactive).
- `starting`
  A startup attempt is in flight.
- `running`
  Dynamic runtime is live and can serve requests.
- `failed_start`
  The last startup attempt failed before the actor became runnable.

Required metadata:

- `lastStartErrorCode` — the `ActorError` subclass code (e.g.,
  `"dynamic_startup_failed"`, `"dynamic_load_timeout"`)
- `lastStartErrorMessage` — the error message string
- `lastStartErrorDetails` — full error details including stack trace. In
  production, this field is stored but never serialized into client responses.
  Only `lastStartErrorCode` and a sanitized message are returned to clients.
- `lastFailureAt` — timestamp of the last failure
- `retryAt` — timestamp when the next passive retry is allowed
- `retryAttempt` — number of consecutive failed attempts
- `reloadCount` — number of reload calls in the current rate-limit window
- `reloadWindowStart` — timestamp when the current rate-limit window began
- `generation` — monotonic integer counter, incremented synchronously before
  each new startup attempt is dispatched. Used to reject stale async
  completions. Note: this is distinct from the existing driver-level
  `generation` field (UUID in file-system driver, number in engine driver),
  which tracks actor identity across destroy/create cycles. This generation
  tracks startup attempts within a single actor's in-memory lifetime.
- `startupPromise` — the shared promise for the current in-flight startup
  attempt. Created via `promiseWithResolvers` when transitioning to `starting`.
  All concurrent requests and reload calls join this promise instead of
  starting parallel attempts.

Rules:

- This state is host-side and in-memory only.
- It must not be written into persisted actor storage by default.
- It must be cleared or replaced on successful startup.
- It must be cleared when the host-side wrapper is removed by normal sleep or
  stop cleanup.
- It must be safe against stale async completion. When a startup attempt
  completes, the handler must compare its captured generation against the
  current generation. If they differ, the completion is discarded silently.
- Backoff must be represented as recorded metadata such as `retryAt`, not as a
  background retry loop.
- `generation` is a per-actor, process-local monotonic counter. It must be
  incremented synchronously (before any `await`) when initiating a new startup
  attempt. This ensures that concurrent requests arriving during the transition
  from `failed_start` to `starting` always join the new attempt rather than
  racing to create their own.
- Only one startup attempt may be in flight at a time. The `startupPromise`
  field enforces this. When a startup is needed (from `inactive` or expired
  `failed_start`), the implementation must:
  1. Synchronously transition to `starting`.
  2. Synchronously increment `generation`.
  3. Synchronously create a new `promiseWithResolvers` and store it as
     `startupPromise`.
  4. Begin the async startup work.
  5. Any concurrent request that arrives while in `starting` state awaits the
     existing `startupPromise` rather than creating a new one.

## Reload Authentication

Reload must be authenticated. The implementation must use both the existing
`DynamicActorAuth` hook and a new `canReload` callback.

### Auth Flow

1. The existing `auth` hook on `dynamicActor({ auth })` is called first with
   the reload request context. If it throws, the reload is rejected with `403`.
2. If `auth` passes, the `canReload` callback is called. If it returns `false`
   or throws, the reload is rejected with `403`.

### `canReload` Callback

Add a `canReload` field to `DynamicActorConfigInput`:

```typescript
export interface DynamicActorConfigInput<TInput, TConnParams> {
  load: DynamicActorLoader<TInput>;
  auth?: DynamicActorAuth<TConnParams, TInput>;
  canReload?: (context: DynamicActorReloadContext) => boolean | Promise<boolean>;
}

export interface DynamicActorReloadContext {
  actorId: string;
  name: string;
  key: unknown[];
  request: Request;
}
```

If `canReload` is not provided, reload defaults to allowed when `auth` passes
(or when no `auth` is configured, which is only valid in development).

In development mode without a configured `auth` or `canReload`, reload is
allowed with a warning log, matching the existing inspector auth behavior.

## Request Behavior

### Normal Request While Running

Dispatch normally.

### Normal Request While Inactive

Attempt startup immediately.

If startup succeeds, handle the request normally.

If startup fails, transition to `failed_start`, log the failure, record retry
metadata, and return the failed-start error.

### Normal Request While Starting

Await the existing `startupPromise` rather than starting a new attempt. When
the promise resolves, dispatch the request normally. When it rejects, return
the failed-start error.

### Normal Request While Failed Start

If backoff is still active, return the stored failed-start error immediately.

If backoff has expired, transition synchronously to `starting`, increment
`generation`, create a new `startupPromise` via `promiseWithResolvers`, and
begin one fresh startup attempt. All concurrent requests arriving during this
startup join the same `startupPromise`.

Retries must be passive. The implementation must not schedule autonomous retry
timers that keep a failed actor spinning in memory until the next attempt. The
wrapper may remain available to return failed-start responses and serve reload,
but startup retries only happen because of an incoming request or explicit
reload.

### WebSocket Upgrade While Failed Start

WebSocket upgrade requests during `failed_start` must be rejected before the
WebSocket handshake completes. The server must respond with the same HTTP error
status and body as a normal failed-start HTTP request. The WebSocket upgrade
must not be accepted and then immediately closed.

If the actor is in `starting` state when a WebSocket upgrade arrives, the
upgrade awaits the `startupPromise`. If startup fails, the upgrade is rejected
with the failed-start HTTP error. If startup succeeds, the upgrade proceeds
normally.

### WebSocket Connections During Reload

When reload triggers a sleep on a running actor, open WebSocket connections are
closed as part of the normal sleep lifecycle. The close code must be `1012`
(Service Restart) with a reason string of `"dynamic.reload"`. This tells
clients that the closure is intentional and reconnection is appropriate.

## Reload Behavior

Reload must be handled at the manager or host wrapper layer before request
dispatch into dynamic actor code.

Reload must pass authentication before any state changes occur (see Reload
Authentication above).

### Reload While Running

Use the existing sleep-based reload behavior:

1. Stop the running actor through the normal sleep lifecycle.
2. Return success when the actor is inactive.
3. The next normal request starts the actor with fresh code.

Note: this means reload does not verify that the new code loads successfully.
The reload caller receives `200` confirming the old code was stopped. Any
startup failure surfaces on the next request that wakes the actor.

### Reload While Inactive

Return `200` without waking the actor.

This is required to prevent the double-load path where reload wakes a sleeping
actor, loads code once, then immediately sleeps it again.

Note: a reload sent to a nonexistent or misspelled actor ID is rejected at the
engine gateway level with an appropriate error before it reaches the driver.
The driver-level reload handler only sees requests for actors that the gateway
has already resolved.

### Reload While Starting

Abort the current startup attempt and immediately begin a fresh one.

The implementation must pass an `AbortController` signal through the startup
pipeline. When reload is called during `starting`:

1. Abort the current startup's `AbortController`. This signals the in-flight
   `DynamicActorIsolateRuntime.start()` to cancel (e.g., abort the loader
   fetch, stop waiting for isolate bootstrap).
2. Synchronously increment `generation`.
3. Create a new `startupPromise` via `promiseWithResolvers`.
4. Create a new `AbortController` for the fresh attempt.
5. Begin the new startup attempt.
6. Any requests that were awaiting the old `startupPromise` receive a
   rejection. They then observe the new `starting` state and join the new
   `startupPromise`.

The `AbortController` signal must be threaded through:

- The user-provided `loader` callback (available as `context.signal`).
- `DynamicActorIsolateRuntime.start()` as a parameter.
- Any internal async operations within the startup pipeline that support
  cancellation (e.g., `fetch` calls, file I/O).

Operations that do not support cancellation (e.g., `isolated-vm` context
creation) will run to completion, but the stale generation check at completion
time will discard their result.

### Reload While Failed Start

Reload resets failed-start backoff state (`retryAt`, `retryAttempt`) and
immediately attempts a fresh startup following the same synchronous transition
to `starting` described above.

If the fresh startup succeeds, return `200`.

If the fresh startup fails, return the failed-start error immediately and keep
the actor in `failed_start`.

### Reload Rate Limiting

Reload bypasses backoff, but the driver must log a warning when reload is
forced more than `N` times in `Y` interval.

Rate limiting uses a simple bucket system:

- `reloadCount` tracks the number of reload calls in the current window.
- `reloadWindowStart` tracks when the current window began.
- When a reload is received, if `now - reloadWindowStart > Y`, reset the
  bucket: set `reloadCount = 1` and `reloadWindowStart = now`.
- Otherwise, increment `reloadCount`.
- If `reloadCount > N`, log a warning with the actor ID and the count.

Default values: `N = 10`, `Y = 60_000` (60 seconds).

The first implementation only needs warning-level logging, not enforcement.

## Retry Configuration

Retry behavior must be configurable by the user on dynamic actors.

### Configuration Interface

```typescript
export interface DynamicActorConfigInput<TInput, TConnParams> {
  load: DynamicActorLoader<TInput>;
  auth?: DynamicActorAuth<TConnParams, TInput>;
  canReload?: (context: DynamicActorReloadContext) => boolean | Promise<boolean>;
  options?: DynamicActorOptions;
}

export interface DynamicActorOptions extends GlobalActorOptionsInput {
  startup?: DynamicStartupOptions;
}

export interface DynamicStartupOptions {
  /** Timeout for the full startup pipeline in ms. Default: 15_000. */
  timeoutMs?: number;

  /** Initial backoff delay in ms after a failed startup. Default: 1_000. */
  retryInitialDelayMs?: number;

  /** Maximum backoff delay in ms. Default: 30_000. */
  retryMaxDelayMs?: number;

  /** Backoff multiplier. Default: 2. */
  retryMultiplier?: number;

  /** Whether to add jitter to backoff delays. Default: true. */
  retryJitter?: boolean;

  /**
   * Maximum number of consecutive failed startup attempts before the host
   * wrapper is torn down. After this limit, the actor transitions to a
   * terminal failed state and the wrapper is removed from memory. Subsequent
   * requests will trigger a fresh startup attempt with no prior backoff
   * context, as if the actor had never been loaded.
   *
   * Default: 20.
   * Set to 0 for unlimited retries (wrapper stays alive indefinitely).
   */
  maxAttempts?: number;
}
```

### Backoff Implementation

Reuse the `p-retry` exponential backoff algorithm that is already used in
`engine-client/metadata.ts` and `client/actor-conn.ts`. The
implementation does not need to use `p-retry` directly (since retries are
passive, not loop-driven), but must compute backoff delays using the same
formula: `min(maxDelay, initialDelay * multiplier^attempt)` with optional
jitter.

### Max Attempts

When `retryAttempt` exceeds `maxAttempts`, the host wrapper is torn down. The
actor transitions to `inactive` with no in-memory state. The next request for
this actor triggers a completely fresh startup attempt with `retryAttempt = 0`,
as if the actor had never been loaded. This prevents unbounded memory
accumulation from permanently broken actors while still allowing recovery.

Reload must clear the active retry delay and failure attempt count before
attempting a fresh startup.

## Error Surfacing

Failed-start errors use the existing `ActorError` subclass hierarchy. The
error code comes from the `ActorError` subclass (e.g., the `code` field), and
the error details come from the underlying cause (e.g., the secure-exec
process output, the loader exception message, the isolate bootstrap error).

The following stable error codes must be defined as `ActorError` subclasses for
dynamic startup failures:

- `DynamicStartupFailed` — general startup failure (catch-all for unclassified
  errors from the loader, sandbox, or bootstrap).
- `DynamicLoadTimeout` — the startup pipeline exceeded the configured timeout.

These codes are always returned to clients. The distinction between what is
sanitized is the error details, not the code.

Client-facing rules:

- The `ActorError` code (e.g., `"dynamic_startup_failed"`,
  `"dynamic_load_timeout"`) is always returned to clients in both production
  and development.
- In production, the message is sanitized to a generic string (e.g., "Dynamic
  actor startup failed. Check server logs for details."). The
  `lastStartErrorDetails` field is not included in the response.
- In development, the full error message and details (including stack traces
  and loader output) are included in the response, matching how internal errors
  are currently exposed.
- Full details must always be emitted to logs in all environments.

This implies the failed-start state must retain enough structured error data to
reconstruct a sanitized or full response without re-running startup.

## Load Timeout

The startup pipeline must be wrapped in a configurable timeout.

### Scope

The timeout starts when `DynamicActorIsolateRuntime.start()` is called and
ends when `ensureStarted()` resolves. This covers:

- The user-provided `loader` callback.
- Source normalization and materialization.
- Dynamic module loading (`secure-exec`, `isolated-vm`).
- Sandbox filesystem setup.
- Bootstrap script execution.
- The isolate-side `ensureStarted()` call (actor `onStart` lifecycle hook).

Note: first-cold-start overhead (loading `secure-exec` and `isolated-vm`
modules for the first time) is included in this timeout. The default of 15
seconds is chosen to accommodate cold starts. If cold-start overhead is a
concern, the user can increase the timeout via `startup.timeoutMs`.

### Implementation

The timeout is implemented via the same `AbortController` used for reload
cancellation. When the timeout fires:

1. The `AbortController` is aborted with a `DynamicLoadTimeout` error.
2. The startup pipeline observes the abort signal and cancels where possible.
3. The actor transitions to `failed_start` with `lastStartErrorCode` set to
   `"dynamic_load_timeout"`.
4. The timeout failure participates in backoff identically to any other startup
   failure.

### Configuration

The timeout is configured under `dynamicActor({ options: { startup: { timeoutMs } } })`.

Default: `15_000` (15 seconds).

## Dynamic Actor Status Endpoint

A new `GET /dynamic/status` endpoint must be added to expose the host-side
runtime state for observability and debugging.

### Response Shape

```typescript
interface DynamicActorStatusResponse {
  state: "inactive" | "starting" | "running" | "failed_start";
  generation: number;

  // Present when state is "failed_start"
  lastStartErrorCode?: string;
  lastStartErrorMessage?: string; // sanitized in production
  lastStartErrorDetails?: string; // only in development
  lastFailureAt?: number;
  retryAt?: number;
  retryAttempt?: number;
}
```

### Authentication

The status endpoint uses the same authentication as the inspector endpoints:
Bearer token via `config.inspector.token()`, with timing-safe comparison. In
development mode without a configured token, access is allowed with a warning.

### Client-Side Support

Add a `status()` method to `ActorHandleRaw`:

```typescript
class ActorHandleRaw {
  async status(): Promise<DynamicActorStatusResponse> {
    // GET /dynamic/status
  }
}
```

This method is only meaningful for dynamic actors. Calling it on a static actor
returns `{ state: "running", generation: 0 }`.

## Sleep Interaction with Failed Start

When a sleep signal arrives while an actor is in `failed_start`, the failed-
start metadata is cleared and the host wrapper is removed. The actor
transitions to `inactive` with no in-memory state. This is the same as the
`maxAttempts` exhaustion behavior: the next request triggers a completely fresh
startup attempt.

This is intentional. A sleep on a failed actor is equivalent to a full reset.
If the underlying issue has been fixed, the next request will succeed. If not,
the actor will re-enter `failed_start` with fresh backoff starting from
attempt 0.

## Documentation Requirements

The implementation must update docs-internal to describe:

- the dynamic actor startup state model
- what `failed_start` means
- how normal requests behave during `failed_start`
- how backoff works
- that backoff is passive and does not create an autonomous retry loop
- how `reload` behaves for `running`, `inactive`, `starting`, and
  `failed_start`
- that `reload` resets backoff before retrying startup
- why reload on inactive actors is a no-op
- how errors are sanitized in production and expanded in development
- the dynamic load timeout and where it is configured
- the retry configuration and where it is configured
- reload authentication via `auth` and `canReload`
- the `GET /dynamic/status` endpoint
- WebSocket close behavior during reload (`1012`, `"dynamic.reload"`)
- the `maxAttempts` limit and what happens when it is exceeded

Minimum docs change:

- expand `docs-internal/rivetkit-typescript/DYNAMIC_ACTORS_ARCHITECTURE.md`
  with a dedicated failed-start and reload lifecycle section

The implementation is not complete until the docs-internal update ships in the
same change.

## Comment Requirements

All non-obvious logic introduced by this change must be commented in code.

Examples that require comments:

- why failed-start state is kept in the host wrapper instead of persisted actor
  state
- why reload on inactive actors is intercepted as a no-op
- how generation invalidation prevents stale startup completions from winning
- why reload bypasses backoff
- why backoff is passive instead of being driven by background timers
- why production errors are sanitized while development errors include details
- why `startupPromise` is created synchronously before the async startup work
- why WebSocket upgrades are rejected before handshake during failed start

Comments should explain intent and invariants, not implementation history.

## Implementation Outline

1. Define `DynamicStartupOptions` interface and add `startup` key to the
   existing `DynamicActorOptions` type.
2. Define `DynamicStartupFailed` and `DynamicLoadTimeout` as `ActorError`
   subclasses in `actor/errors.ts`.
3. Add `canReload` to `DynamicActorConfigInput` and `DynamicActorReloadContext`
   type.
4. Introduce a host-side dynamic runtime status model shared by file-system and
   engine driver code, using the state enum and metadata fields defined above.
5. Implement startup coalescing via `promiseWithResolvers`: synchronous state
   transition to `starting`, synchronous generation increment, shared promise
   for all concurrent waiters.
6. Thread `AbortController` through `DynamicActorIsolateRuntime.start()` and
   the user-provided `loader` callback (as `context.signal`).
7. Implement load timeout using the `AbortController` signal with a
   `setTimeout` that aborts after `startup.timeoutMs`.
8. Move or extend overlay routing so reload on inactive actors can be handled
   before waking actor code.
9. Implement reload authentication: call `auth` then `canReload` before
   processing reload.
10. Implement reload-while-starting: abort current `AbortController`, increment
    generation, create new `startupPromise`, begin fresh attempt.
11. Preserve existing sleep-based reload only for actors that are already
    running.
12. Implement passive failed-start backoff metadata (using the `p-retry`
    backoff formula) without background retry timers.
13. Implement `maxAttempts` exhaustion: tear down wrapper and transition to
    `inactive` when exceeded.
14. Implement failed-start error replay with sanitized production output and
    detailed development output.
15. Add `GET /dynamic/status` endpoint with inspector-style auth.
16. Add `status()` method to `ActorHandleRaw` on the client.
17. Reject WebSocket upgrades during `failed_start` before handshake. Close
    WebSockets during reload with code `1012` and reason `"dynamic.reload"`.
18. Implement reload rate-limit bucket (`reloadCount` / `reloadWindowStart`).
19. Update docs-internal architecture docs.
20. Add comments for every non-obvious lifecycle transition and overlay routing
    rule.

## Test Requirements

All failed-start tests must be added to the shared engine-focused integration
suite so the runtime path uses one common set of cases. This enforces the
parity requirement.

Add or update tests for:

- normal request retries startup after backoff expires
- normal request during active backoff returns stored failed-start error
- no background retry loop runs while actor is in failed-start backoff
- reload bypasses backoff and immediately retries startup
- reload resets backoff metadata before retrying
- reload on failed-start actor returns success or error from the immediate
  startup attempt
- reload on inactive actor is a no-op and does not cause double-load
- concurrent requests coalesce onto one startup via shared `startupPromise`
- stale startup generation cannot overwrite newer reload-triggered generation
- production response is sanitized (no details, has code)
- development response includes full detail
- dynamic load timeout returns `"dynamic_load_timeout"` error code
- retry options change backoff behavior as configured by the user
- `maxAttempts` exhaustion tears down the wrapper
- request after `maxAttempts` exhaustion triggers fresh startup from attempt 0
- reload authentication rejects unauthenticated callers with `403`
- `canReload` returning `false` rejects reload with `403`
- WebSocket upgrade during `failed_start` is rejected before handshake
- WebSocket connections receive close code `1012` during reload
- `GET /dynamic/status` returns correct state and metadata
- `GET /dynamic/status` respects inspector auth
- reload-while-starting aborts old attempt and starts new generation
- AbortController signal is delivered to the loader callback
- docs-internal file is updated alongside the implementation
