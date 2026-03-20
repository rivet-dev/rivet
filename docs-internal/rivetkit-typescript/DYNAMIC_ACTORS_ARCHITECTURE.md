# Dynamic Actors Architecture

## Overview

Dynamic actors let a registry entry resolve actor source code at actor start time.

Dynamic actors are represented by `dynamicActor({ load, auth?, options? })`
and still participate in normal registry routing and actor lifecycle.

Driver parity is verified by running the same driver test suites against two
fixture registries:

- `fixtures/driver-test-suite/registry-static.ts`
- `fixtures/driver-test-suite/registry-dynamic.ts`

Both registries are built from `fixtures/driver-test-suite/actors/` to keep
actor behavior consistent between static and dynamic execution.

## Main Components

- Host runtime manager:
  `rivetkit-typescript/packages/rivetkit/src/dynamic/isolate-runtime.ts`
  Creates and owns one `NodeProcess` isolate per dynamic actor instance.
- Isolate bootstrap runtime:
  `rivetkit-typescript/packages/rivetkit/dynamic-isolate-runtime/src/index.cts`
  Runs inside the isolate, parses registry config via
  `RegistryConfigSchema.parse`, and exports envelope handlers.
- Runtime bridge:
  `rivetkit-typescript/packages/rivetkit/src/dynamic/runtime-bridge.ts`
  Shared envelope and callback payload types for host and isolate.
- Driver integration:
  `drivers/file-system/global-state.ts` and `drivers/engine/actor-driver.ts`
  Branch on definition type, construct dynamic runtime, and proxy fetch and websocket traffic.

## Lifecycle

1. Driver resolves actor definition from registry.
2. If definition is dynamic, driver creates `DynamicActorIsolateRuntime`.
3. Runtime calls loader and gets `{ source, sourceFormat?, nodeProcess? }`.
4. Runtime writes source into actor runtime dir:
   - `sourceFormat: "esm-js"` -> `dynamic-source.mjs` (written unchanged)
   - `sourceFormat: "commonjs-js"` -> `dynamic-source.cjs` (written unchanged)
   - default `sourceFormat: "typescript"` -> transpiled to `dynamic-source.cjs`
5. Runtime writes isolate bootstrap entry into actor runtime dir.
6. Runtime builds a locked down sandbox driver and creates `NodeProcess`.
7. Runtime injects host bridge refs and bootstrap config into isolate globals.
8. Runtime loads bootstrap module and captures exported envelope refs.

Before HTTP and WebSocket traffic is forwarded into the isolate, the host
runtime may run an optional dynamic auth hook. The auth hook receives dynamic
actor metadata, the incoming `Request`, and decoded connection params. Throwing
from auth rejects the request before actor dispatch. HTTP requests return
standard RivetKit error responses and WebSockets close with the derived
`group.code` reason.

Dynamic actors also expose an internal `PUT /dynamic/reload` control endpoint
and a `GET /dynamic/status` observability endpoint. See the
**Failed-Start and Reload Lifecycle** section below for full details.

Note: isolate bootstrap does not construct `Registry` at runtime. Constructing
`Registry` would auto-start runtime preparation on next tick in non-test mode
and pull default drivers that are not needed for dynamic actor execution.

## Failed-Start and Reload Lifecycle

### Startup State Model

Dynamic actors track a host-side runtime state with four possible values:

| State          | Description |
|----------------|-------------|
| `inactive`     | No isolate exists. The next request triggers a fresh startup. |
| `starting`     | A startup attempt is in flight. Concurrent requests join the existing `startupPromise`. |
| `running`      | The isolate is live and serving traffic. |
| `failed_start` | The most recent startup attempt failed. Error metadata and backoff timing are recorded. |

State is tracked per-actor in a `DynamicRuntimeStatus` object
(`src/dynamic/runtime-status.ts`). This state is in-memory only and is never
persisted. It is cleared when the actor wrapper is removed during sleep or
stop.

### What `failed_start` Means

When a dynamic actor's loader or isolate initialization throws (or times
out), the actor transitions to `failed_start`. In this state:

- Error metadata is recorded: `lastStartErrorCode`, `lastStartErrorMessage`,
  `lastStartErrorDetails`, and `lastFailureAt`.
- A `retryAt` timestamp is computed using exponential backoff.
- Incoming requests during the active backoff window receive the stored
  error immediately without attempting a new startup.
- Incoming requests after the backoff expires trigger a fresh startup
  attempt.

### Passive Backoff

Backoff is passive. No background timers or retry loops are scheduled.
Retries happen only when an incoming request or explicit reload arrives.
This prevents failed actors from spinning in memory indefinitely and keeps
resource usage proportional to actual demand.

The backoff delay is computed as:

```
delay = min(retryMaxDelayMs, retryInitialDelayMs * retryMultiplier ^ attempt)
```

When `retryJitter` is enabled, a uniform jitter in `[delay*0.5, delay)` is
applied.

### Startup Coalescing and Generation Tracking

`coalesceDynamicStartup` (`src/dynamic/startup-coalescing.ts`) orchestrates
all startup attempts. When a startup is needed, the function synchronously
transitions to `starting`, increments the `generation` counter, and creates
a `startupPromise` via `promiseWithResolvers`. This synchronous transition
ensures that any concurrent request arriving between the transition and the
first `await` always observes the `starting` state and joins the correct
promise rather than launching a duplicate attempt.

Each startup attempt captures the generation at the time of the synchronous
transition. If a reload or another retry increments the generation while the
original attempt is still in flight, the original attempt's completion
handler detects the mismatch and discards its result instead of overwriting
the newer attempt's state.

### Load Timeout

An `AbortController` is created for each startup attempt. A timeout
(configured via `startup.timeoutMs`, default 15 seconds) aborts the
controller if startup does not complete in time. The resulting
`DynamicLoadTimeout` error participates in the normal backoff flow. The
`AbortSignal` is passed through to the user-provided loader callback as
`context.signal`, allowing cooperative cancellation.

### `maxAttempts` Exhaustion

When `retryAttempt` exceeds `maxAttempts` (default 20), the actor
transitions to `inactive`, tearing down the host wrapper. The next request
triggers a fresh startup from attempt 0. Setting `maxAttempts` to 0 disables
the limit (unlimited retries).

### Retry Configuration

All retry parameters are configured via `DynamicStartupOptions` on the
dynamic actor definition (`options.startup`):

| Option               | Default | Description |
|----------------------|---------|-------------|
| `timeoutMs`          | 15000   | Maximum time to wait for startup before aborting |
| `retryInitialDelayMs`| 1000    | Initial backoff delay |
| `retryMaxDelayMs`    | 30000   | Maximum backoff delay |
| `retryMultiplier`    | 2       | Exponential backoff multiplier |
| `retryJitter`        | true    | Apply uniform jitter to backoff delays |
| `maxAttempts`        | 20      | Maximum startup attempts before teardown (0 = unlimited) |

Defaults are defined in `DYNAMIC_STARTUP_DEFAULTS`
(`src/dynamic/internal.ts`).

### Reload Behavior by State

The `PUT /dynamic/reload` endpoint behavior depends on the current state:

| State          | Reload Behavior |
|----------------|-----------------|
| `running`      | The actor is stopped through the normal sleep lifecycle. The next request wakes it and calls the loader again. Reload does NOT verify that the new code loads successfully. Startup failures surface on the next request. |
| `inactive`     | No-op. Returns 200 without waking the actor. This prevents a reload from accidentally triggering a double-load before the next natural request. |
| `starting`     | Aborts the current `AbortController`, increments generation, rejects the old `startupPromise`, cleans up the partial runtime, and transitions to `inactive`. Requests awaiting the old promise receive a rejection and then observe the new state on their next check. |
| `failed_start` | Resets backoff state (`retryAt`, `retryAttempt`) via `transitionToInactive`, so the next request starts a fresh attempt immediately instead of waiting for the backoff timer. |

### Reload Authentication

Reload calls are authenticated before any state changes:

1. The existing `auth` callback is called first. If it throws, the request
   is rejected with 403.
2. If auth passes, the `canReload` callback is called. If it returns `false`
   or throws, the request is rejected with 403.
3. If `canReload` is not provided, reload defaults to allowed when auth
   passes.
4. In development mode without auth or `canReload` configured, reload is
   allowed with a warning log. This matches the existing inspector auth
   behavior in dev mode.

The `canReload` callback receives a `DynamicActorReloadContext` with the
incoming `Request` object.

### Reload Rate Limiting

Reload calls are tracked with a rate-limit bucket per actor:

- `reloadCount` tracks calls in the current 60-second window
- `reloadWindowStart` tracks the window start timestamp
- When `reloadCount` exceeds 10 in a window, a warning is logged with the
  actor ID and count

Rate limiting is warning-only, not enforcement. The counters are reset when
the actor transitions to `inactive`.

### Error Sanitization

Error responses differ between production and development environments:

| Field                  | Production | Development |
|------------------------|------------|-------------|
| Error code             | Always included (`dynamic_startup_failed` or `dynamic_load_timeout`) | Always included |
| Error message          | Sanitized: "Dynamic actor startup failed. Check server logs for details." | Full original message |
| `lastStartErrorDetails`| Not included | Included (stack traces, loader output) |
| Server logs            | Full details always emitted | Full details always emitted |

Production sanitization prevents leaking internal details (file paths, stack
traces, loader output) to clients while preserving the error code for
programmatic handling. `isDev()` from `utils/env-vars.ts` determines the
environment (returns `true` when `NODE_ENV !== "production"`).

### GET /dynamic/status Endpoint

The `GET /dynamic/status` endpoint returns a `DynamicActorStatusResponse`
for debugging:

```json
{
  "state": "failed_start",
  "generation": 3,
  "lastStartErrorCode": "dynamic_startup_failed",
  "lastStartErrorMessage": "Module not found: ./actor.mjs",
  "lastStartErrorDetails": "Error: ...",
  "lastFailureAt": 1710892800000,
  "retryAt": 1710892802000,
  "retryAttempt": 2
}
```

Authentication uses the inspector token pattern: a Bearer token configured
via `config.inspector.token()` with timing-safe comparison. In development
mode without a configured token, access is allowed with a warning.

For static actors, the endpoint returns `{ state: "running", generation: 0 }`
since static actors have no dynamic lifecycle.

The `lastStartErrorDetails` field is only included in development mode.
Failure metadata fields (`lastStartErrorCode`, `lastStartErrorMessage`,
`lastFailureAt`, `retryAt`, `retryAttempt`) are only present when state is
`failed_start`.

The client-side `ActorHandleRaw.status()` method calls this endpoint and
returns the parsed `DynamicActorStatusResponse`.

### WebSocket Behavior

During `failed_start`:
- WebSocket upgrades are rejected before the handshake completes with the
  same HTTP error status and body as normal failed-start requests. The
  upgrade must not be accepted and then immediately closed, because that
  would cause clients to interpret it as a successful connection.

During `starting`:
- WebSocket upgrades await the `startupPromise`. If startup fails, the
  upgrade is rejected with the failed-start error. If startup succeeds,
  the upgrade proceeds normally.

During reload of a `running` actor:
- Open WebSocket connections are closed with code 1012 (Service Restart) and
  reason `"dynamic.reload"` before the actor is put to sleep. Code 1012
  tells clients the closure is intentional and reconnection is appropriate.

Force-close is implemented via per-actor callback registration
(`registerDynamicReloadCloseCallback` / `unregisterDynamicReloadCloseCallback`
on `FileSystemGlobalState`). Callbacks are cleaned up in both `sleepActor`
and `destroyActor` paths.

## Bridge Contract

Host to isolate calls:

- `dynamicFetchEnvelope`
- `dynamicOpenWebSocketEnvelope`
- `dynamicWebSocketSendEnvelope`
- `dynamicWebSocketCloseEnvelope`
- `dynamicDispatchAlarmEnvelope`
- `dynamicStopEnvelope`
- `dynamicGetHibernatingWebSocketsEnvelope`
- `dynamicDisposeEnvelope`

Isolate to host callbacks:

- KV: `kvBatchPut`, `kvBatchGet`, `kvBatchDelete`, `kvListPrefix`
- SQLite: `sqliteExec`, `sqliteBatch`
- Lifecycle: `setAlarm`, `startSleep`, `startDestroy`
- Networking: `dispatch` for websocket events
- Runner ack path: `ackHibernatableWebSocketMessage`
- Inline client bridge: `clientCall`

Binary payloads are normalized to `ArrayBuffer` at the host and isolate boundary.

## Security Model

- Each dynamic actor runs in its own sandboxed `NodeProcess`.
- Sandbox permissions deny network and child process access.
- Filesystem access is restricted to dynamic runtime root and read only `node_modules` paths.
- Environment is explicitly injected by host config for the isolate process.

## Module Access Projection

Dynamic actors use secure-exec `moduleAccess` projection to expose a
read-only `/root/node_modules` view into host dependencies (allow-listing
`rivetkit` and transitive packages). We no longer stage a temporary
`node_modules` tree for runtime bootstrap.

## Driver Test Skip Gate

The dynamic registry variant in driver tests has a narrow skip gate for two
cases only:

- secure-exec dist is not available on the local machine
- nested dynamic harness mode is explicitly enabled for tests

This gate is only to avoid invalid test harness setups. Static and dynamic
behavior parity remains the expected target for normal driver test execution.
