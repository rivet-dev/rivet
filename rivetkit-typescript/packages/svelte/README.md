# @rivetkit/svelte

Official Svelte 5 adapter for [RivetKit](https://rivet.gg) actors.

`@rivetkit/svelte` keeps the core RivetKit client model intact while giving Svelte apps a first-class DX for:

- app-local typed context in layouts
- shared client reuse across components and ViewModels
- reactive actor state via Svelte runes
- mixed reactive/raw connection handling when low-level control still matters

Built on a small package-local, Apache-2.0-derived Rivet framework bridge and shaped for Svelte patterns that feel familiar if you already use TanStack Query for shared clients, Runed for getter ergonomics, Bits UI for composable primitives, or Better Auth for app-owned auth wiring. Shipping the bridge with the adapter guarantees that `getParams` reaches every connect/reconnect in standalone installs.

## Install

```bash
npm install @rivetkit/svelte rivetkit
```

## Migration Note

The package no longer exports package-global default-context helpers. Existing apps should move to an app-local typed context created with `createRivetContext()` and provide it from a layout or other provider component.

## Choose A Setup Pattern

### Simple app: app-local typed context in your layout

```ts
// lib/rivet.ts
import { createRivetContext } from "@rivetkit/svelte";
import type { AppRegistry } from "./registry";

export const rivetContext = createRivetContext<AppRegistry>("AppRivet");
```

```svelte
<script lang="ts">
  import { rivetContext } from '$lib/rivet';
  import type { AppRegistry } from './registry';

  let { children } = $props();

  rivetContext.setup('http://localhost:3000');
</script>

{@render children()}
```

### Shared client app: one transport, one wrapper, many consumers

This is the recommended pattern when you want component-level `useActor()` and app-level ViewModels to share the same client.

```ts
// lib/rivet.ts
import {
  createClient,
  createRivetContext,
  createSharedRivetKit,
} from "@rivetkit/svelte";
import type { AppRegistry } from "./registry";

export const rivetContext = createRivetContext<AppRegistry>("AppRivet");

const getClient = (() => {
  let client: ReturnType<typeof createClient<AppRegistry>> | null = null;

  return () => {
    if (!client) {
      client = createClient<AppRegistry>({
        endpoint: "http://localhost:3000",
        devtools: false,
      });
    }

    return client;
  };
})();

export const getRivet = createSharedRivetKit<AppRegistry>(getClient);
```

```svelte
<script lang="ts">
  import { rivetContext, getRivet } from '$lib/rivet';

  let { children } = $props();

  rivetContext.set(getRivet());
</script>

{@render children()}
```

That shared-client mental model mirrors how TanStack Query centralizes one client instance at the provider boundary, but keeps RivetKit actor connections and transport ownership explicit.

## Picking The Right Primitive

| Primitive                          | Best for                                                              | Lifecycle                                |
| ---------------------------------- | --------------------------------------------------------------------- | ---------------------------------------- |
| `useActor()`                       | Components that render live actor state                               | Automatic via `$effect`                  |
| `createReactiveActor()`            | ViewModels, singletons, manual connection ownership                   | Cheap construction; connect on `mount()` |
| shared raw client (`createClient`) | One-off actions, low-level handles, custom orchestration              | App-owned                                |
| `createReactiveConnection()`       | Bridging a raw connection into reactive connection status/error state | App-owned                                |

A good rule of thumb:

- use `useActor()` or `createReactiveActor()` when UI needs reactive connection state
- use the shared raw client for one-off operations and direct handles
- keep auth refresh, org switching, and app-specific orchestration outside the package

## Core APIs

### `createRivetContext<Registry>()`

Creates a typed Svelte context helper with `set`, `get`, `has`, `setup`, and `setupWithClient`.

```ts
import { createRivetContext } from "@rivetkit/svelte";

export const rivetContext = createRivetContext<AppRegistry>("AppRivet");
```

This follows the typed context style that is common in modern Svelte libraries instead of pushing consumers toward ad-hoc string keys or package-global defaults.

### `createSharedRivetKit<Registry>(getClient, opts?)`

Lazily creates one RivetKit wrapper around a shared `rivetkit/client` instance and reuses it.

```ts
const getRivet = createSharedRivetKit<AppRegistry>(() => getClient());

const a = getRivet();
const b = getRivet();
// a === b
```

Use this when you already have a shared raw client and want one obvious wrapper for `useActor()` and `createReactiveActor()`.

### `withActorParams(base, params)`

Merges actor options with static or reactive params.

```ts
import { withActorParams } from "@rivetkit/svelte";

const getActorOptions = withActorParams(
  { name: "chatRoom", key: ["room-123"] },
  () => ({ token: session.actorToken, orgId: session.orgId }),
);
```

This is intentionally generic. If your app uses Better Auth or another auth layer, keep refresh/session rules in the app and pass the resolved token into actor params from there.

### `createReactiveConnection(source)`

Wraps an existing raw connection source in reactive connection state.

```ts
import { createReactiveConnection } from "@rivetkit/svelte";

const reactive = createReactiveConnection({
  connect: () => handle.connect(),
});

reactive.connect();
reactive.connStatus;
reactive.error;
reactive.isConnected;
```

This is useful when a low-level handle should stay low-level, but the UI still wants Svelte-friendly `connStatus` and `error` reads. `disconnect()` closes the current socket but retains event registrations for a later `connect()`. For backward compatibility, `dispose()` remains an alias of `disconnect()` rather than permanently invalidating the wrapper. Both detach reactive connection state and settle connection waiters immediately, even if transport teardown is slow or fails; concurrent teardown calls share the same promise.

### `connectionInspector` (opt-in)

A RivetKit instance can expose a live registry of **distinct package-managed sockets** — the connections opened by `useActor()`, `createReactiveActor()`, and `preConnect()`. `warmUp()` is HTTP-only and is not listed. `createReactiveConnection()` is a raw-path bridge and is also not listed.

The registry is **off by default**. Enable it only for local-dev overlays:

```ts
const rivet = createRivetKitWithClient(client, {
  connectionInspector: true,
});

rivet.connectionInspector?.snapshot();
// [{ name: "page", key: ["page-1"], hash, connStatus: "connected", hasConnection: true }]
rivet.connectionInspector?.connectedCount();
rivet.connectionInspector?.revision; // read inside $derived to refresh
```

Rows use opaque identifiers scoped to the inspector; framework hashes and connection parameters are never exposed. Two handles that share a socket share one row; disposing one handle cannot drop the other handle's row. A snapshot copies only `name`, `key`, `hash`, `connStatus`, and `hasConnection`.

Enable the inspector in a development-only client configuration when building a connection status overlay.

## Core Factories And Utilities

These APIs remain part of the public surface:

- `createRivetKit()`
- `createRivetKitWithClient()`
- `useActor()`
- `createReactiveActor()`
- `createConnectionHealth()`
- `createConnectionInspector()` — opt-in live registry of package-managed sockets
- `extract()`, `Getter`, `MaybeGetter`

## `useActor()`

```svelte
<script lang="ts">
  import { rivetContext } from '$lib/rivet';
  import type { AppRegistry } from './registry';

  let { roomId } = $props<{ roomId: string }>();

  const { useActor } = rivetContext.get();

  const chat = useActor(() => ({
    name: 'chatRoom',
    key: [roomId],
  }));
</script>

{#if chat.isConnected}
  <button onclick={() => chat.sendMessage({ text: 'Hello' })}>Send</button>
{:else if chat.error}
  <p>{chat.error.message}</p>
{/if}
```

`useActor()` accepts a `MaybeGetter`, so reactive reads inside the getter re-subscribe automatically when inputs change.

Returned reactive metadata includes:

- `connection`
- `handle`
- `connStatus`
- `error`
- `lastError`
- `isConnected`
- `hasEverConnected`
- `hash`
- `onEvent()`
- `isMutating` — true when any action is in-flight (requires `actionDefaults`)
- `pendingActions` — count of concurrent in-flight actions (requires `actionDefaults`)
- `lastActionError` — most recent action error (requires `actionDefaults`)
- `lastAction` — name of the last called action (requires `actionDefaults`)
- `resetActionState()` — clear error/action state (requires `actionDefaults`)
- proxied actor methods

## `createReactiveActor()`

```ts
import { createRivetKit } from "@rivetkit/svelte";
import type { AppRegistry } from "./registry";

const { createReactiveActor } = createRivetKit<AppRegistry>(
  "http://localhost:3000",
);

export class ChatViewModel {
  actor = createReactiveActor({
    name: "chatRoom",
    key: ["room-123"],
  });

  draft = $state("");

  async send() {
    await this.actor.sendMessage({ text: this.draft });
    this.draft = "";
  }
}
```

`createReactiveActor()` is the right primitive when the app wants ref counting, token refresh, lazy secondary connections, or other orchestration on top.

Construction is side-effect-light: it creates the Svelte-facing proxy, but does not subscribe to the framework core or open a connection until `mount()` runs. `dispose()` releases active `mount()` refs, removes package subscriptions/listeners, resolves pending `whenConnected()` waiters with `false`, and immediately resets connection/action state so retained handlers cannot call a detached socket.

### `reconnect()` — replace a zombie socket

```ts
// Recovery sweep (online / tab-focus / watchdog): a liveness probe failed, so
// force a brand-new connection even though connStatus still reads "connected".
if (!(await probeLiveness())) actor.reconnect();
```

`reconnect()` tears down the current connection — even a half-open **zombie** socket (NAT/LB idle cull, half-open TCP) that still reports `connStatus === "connected"` — and opens a fresh one, re-running `getParams` for a new auth token. Event subscriptions registered via `onEvent()` are automatically re-bound onto the new connection.

It drives the framework core's `enabled` toggle (disable → dispose + reset to `idle` → re-enable → create from `idle`). The adapter removes `enabled` before invoking a custom `hashFunction`, so both phases always address the same framework entry. A plain `dispose()` + `mount()` cannot do this: the core only creates a connection from `idle`, and a zombie never leaves `"connected"`, so the dead socket would be reused. `reconnect()` is a no-op if the actor was never mounted.

## Action Middleware

Use `timeoutByAction: { getSnapshot: 15_000 }` alongside a longer default
`timeout` when reads and mutations have different budgets. Finite positive
overrides settle adapter counters on their own deadline. Timeout and disposal
settle the adapter caller and forward an abort signal when supported. RivetKit
2.3.17 does not cancel its internal action waiter from that signal; server work
also continues. Disposed in-flight actions follow `throwOnError` rather than
returning late results.

Both `useActor()` and `createReactiveActor()` accept an `actionDefaults` option that wraps every proxied action call with built-in middleware — timeout, error capture, loading tracking, and connection guard. No manual wrapping needed.

The action hot path keeps non-reactive mirrors of connection status and pending action count. That avoids accidental Svelte effect subscriptions when a proxied method is called from an effect, without wrapping every action dispatch in `untrack()`. Action functions are stable, recursively proxied, and resolve the current connection only when invoked, so destructured event handlers survive reconnects and nested actions such as `actor.admin.reset()` work. The actor proxy is explicitly non-thenable.

### Quick Start

```ts
const rivet = createRivetKit<AppRegistry>("http://localhost:3000", {
  actionDefaults: { timeout: 30_000 },
});

const actor = rivet.createReactiveActor({
  name: "counter",
  key: ["main"],
});

actor.mount();

// Direct action call — errors captured, loading tracked, timeout enforced
await actor.increment(5);
```

### Reactive State In Templates

```svelte
<script lang="ts">
  const { useActor } = rivetContext.get();

  const counter = useActor({
    name: 'counter',
    key: ['main'],
    actionDefaults: { timeout: 10_000 },
  });
</script>

<button
  onclick={() => counter.increment(1)}
  disabled={counter.isMutating}
>
  {counter.isMutating ? 'Saving...' : 'Increment'}
</button>

{#if counter.lastActionError}
  <p class="error">{counter.lastActionError.message}</p>
  <button onclick={() => counter.resetActionState()}>Dismiss</button>
{/if}
```

### Cascade Configuration

Client-level defaults are shallow-merged with actor-level overrides. Actor-level wins.

```ts
const rivet = createRivetKit<AppRegistry>("http://localhost:3000", {
  // Every actor gets these defaults
  actionDefaults: {
    timeout: 30_000,
    onActionError: (err, name) => telemetry.captureError(name, err),
  },
});

// This actor overrides timeout but inherits onActionError
const actor = rivet.createReactiveActor({
  name: "chatRoom",
  key: ["room-1"],
  actionDefaults: { timeout: 60_000 },
});
```

### Direct Action Calls

Configure middleware on an actor handle and call its actions directly:

```ts
const notifications = rivet.createReactiveActor({
  name: "notifications",
  key: [userId],
  actionDefaults: { timeout: 30_000, throwOnError: true },
});

const unmount = notifications.mount();
try {
  await notifications.markAsRead({ ids: ["message-1"] });
} finally {
  unmount();
  notifications.dispose();
}
```

### Optimistic UI With Rollback

Use `throwOnError: true` for optimistic actions, since a successful void action
can also return `undefined`:

```ts
const actor = rivet.createReactiveActor({
  name: "document",
  key: [documentId],
  actionDefaults: { throwOnError: true },
});

async function togglePin() {
  const previous = pinned;
  pinned = !pinned;
  try {
    await actor.setPinned(pinned);
  } catch (error) {
    pinned = previous;
    throw error;
  }
}
```

### Concurrent Action Tracking

`pendingActions` tracks how many actions are in-flight simultaneously.

```svelte
<script lang="ts">
  const actor = useActor({
    name: 'batchProcessor',
    key: ['main'],
    actionDefaults: { timeout: 60_000 },
  });

  async function processAll(items: string[]) {
    // Fire all in parallel — pendingActions increments for each
    await Promise.all(items.map((id) => actor.process(id)));
  }
</script>

{#if actor.isMutating}
  <p>Processing {actor.pendingActions} items...</p>
{/if}
```

### Lifecycle Callbacks

Callbacks fire at the definition level — useful for telemetry, logging, and global error handling.

```ts
const actor = rivet.createReactiveActor({
  name: "user",
  key: ["user", userId],
  actionDefaults: {
    timeout: 30_000,

    onActionStart: (name, args) => {
      console.log(`[${name}] started`, args);
    },

    onActionSuccess: (name, data) => {
      console.log(`[${name}] completed`, data);
    },

    onActionError: (err, name) => {
      // Send to error tracking service
      errorReporter.capture(err, { action: name, actor: "user" });
    },

    onActionSettled: (name) => {
      // Always fires — useful for cleanup
      console.log(`[${name}] settled`);
    },
  },
});
```

### Connection Guard

By default, a **lost** socket (`disconnected`) fails immediately. A first-paint handshake (`idle` / `connecting` after token mint) **waits** for `whenConnected` (capped at 30s) and then dispatches — a 200 mint is not a connected actor. The error is captured to `lastActionError` if the wait times out or the socket is gone.

```ts
// guardConnection: true (default) — wait through handshake, fail if lost
const actor = rivet.createReactiveActor({
  name: "counter",
  key: ["main"],
  actionDefaults: { guardConnection: true },
});

// During connecting: waits, then increment runs
await actor.increment(5);
// After disconnect: resolves to undefined immediately
// actor.lastActionError.code === 'ACTOR_NOT_YET_CONNECTED'
// or actor.lastActionError.code === 'ACTOR_DISCONNECTED'

// Disable guard — let the action attempt even when disconnected
// (useful if you want the WebSocket queue to handle it)
const actor2 = rivet.createReactiveActor({
  name: "counter",
  key: ["main"],
  actionDefaults: { guardConnection: false },
});
```

### `throwOnError` Modes

Control whether errors reject the promise or only land in reactive state.

```ts
// Mode 1: false (default) — errors captured, not thrown
const actor = rivet.createReactiveActor({
  name: "counter",
  key: ["main"],
  actionDefaults: { throwOnError: false },
});

const result = await actor.riskyAction(); // resolves to undefined on error
// actor.lastActionError has the Error object

// Mode 2: true — errors captured AND re-thrown
const actor2 = rivet.createReactiveActor({
  name: "counter",
  key: ["main"],
  actionDefaults: { throwOnError: true },
});

try {
  await actor2.riskyAction();
} catch (err) {
  // err is the original Error
  // actor2.lastActionError also has it
}

// Mode 3: function — decide per error
const actor3 = rivet.createReactiveActor({
  name: "counter",
  key: ["main"],
  actionDefaults: {
    throwOnError: (err, actionName) => {
      // Only throw for auth errors — swallow everything else
      return err.message.includes("AUTH_");
    },
  },
});
```

### Without `actionDefaults` — Zero Behavior Change

When `actionDefaults` is not set, everything works exactly as before. Actions are plain pass-through calls on the Proxy with no interception.

```ts
// No actionDefaults — same behavior as before the feature existed
const actor = rivet.createReactiveActor({
  name: "counter",
  key: ["main"],
});

// isMutating/lastActionError exist but stay at defaults (false/null)
// Actions throw on error, no timeout, no tracking
await actor.increment(5); // raw pass-through
```

### `ActionDefaults` Reference

| Option            | Type                                | Default | Description                                             |
| ----------------- | ----------------------------------- | ------- | ------------------------------------------------------- |
| `timeout`         | `number`                            | none    | Action timeout in milliseconds                          |
| `throwOnError`    | `boolean \| (err, name) => boolean` | `false` | Whether to re-throw captured errors                     |
| `guardConnection` | `boolean`                           | `true`  | Wait through idle/connecting, then reject if still down |
| `onActionStart`   | `(name, args) => void`              | —       | Fires when an action call starts                        |
| `onActionSuccess` | `(name, data) => void`              | —       | Fires on successful completion                          |
| `onActionError`   | `(error, name) => void`             | —       | Fires on failure (timeout, network, etc)                |
| `onActionSettled` | `(name) => void`                    | —       | Fires after success or failure                          |

The timeout bounds the local caller promise; it does not cancel work already running in the actor. Waiting for the initial connection counts toward `pendingActions`/`isMutating`. If disposal or a reactive `useActor()` key change cancels that wait, the cancellation runs through `onActionError`, `onActionSettled`, and the configured `throwOnError` policy (`false` resolves `undefined`; `true` rejects). Lifecycle callbacks are observational: if one throws, internal counters are still settled and the returned promise rejects with the callback error regardless of `throwOnError`.

### Reactive State Reference

| Property           | Type             | Description                                                      |
| ------------------ | ---------------- | ---------------------------------------------------------------- |
| `isMutating`       | `boolean`        | `true` when any action is in-flight                              |
| `pendingActions`   | `number`         | Count of concurrent in-flight actions                            |
| `lastActionError`  | `Error \| null`  | Most recent action error (cleared on next success or reset)      |
| `lastAction`       | `string \| null` | Name of the last action called                                   |
| `resetActionState` | `() => void`     | Clear `lastActionError` and `lastAction` (return to clean state) |

## `whenConnected()`

Both `useActor()` and `createReactiveActor()` expose a promise-based ready signal that eliminates manual polling loops:

```ts
const actor = rivet.createReactiveActor({
  name: "chatRoom",
  key: ["room-123"],
});

actor.mount();

// Wait up to 10 seconds for the connection
const connected = await actor.whenConnected(10_000);
if (!connected) {
  console.warn("Connection timed out");
  return;
}

// Safe to call actions
await actor.sendMessage({ text: "Hello" });
```

`whenConnected()` resolves immediately if already connected. The default timeout is 30 seconds. Returns `false` on timeout — never rejects.

Also available on `createReactiveConnection()` for raw connection wrappers.

## `getActionError()`

Structured error extraction for any actor handle's `lastActionError`:

```ts
import { getActionError } from "@rivetkit/svelte";

const error = getActionError(threadHandle);
if (error) {
  showToast(error.message ?? "Something went wrong");
  if (error.code === "RATE_LIMITED") retryLater();
}
```

Returns `{ message, code, isActorError }` or `null` when there is no error. Detection uses RivetKit's structural guard, so modern `__type: "RivetError"`, legacy `__type: "ActorError"`, and serialized cross-realm error shapes are supported.

## Auth And Params Guidance

Keep framework-specific auth rules in your app, not in the package.

```ts
import { withActorParams } from "@rivetkit/svelte";

const getChatActorOptions = withActorParams(
  {
    name: "chatRoom",
    key: ["room-123"],
  },
  () => ({
    token: actorToken.current,
    orgId: activeOrgId.current,
  }),
);

const chat = rivet.createReactiveActor(getChatActorOptions());
```

That pattern stays flexible whether your token came from Better Auth, a custom server session, or another auth system entirely.

If connection params contain short-lived values such as actor tokens, provide a `hashFunction` that hashes actor identity (`name`, `key`, `noCreate`) but excludes volatile params. The adapter shares actor connections by hash; including an expiring token can split one actor instance into multiple WebSockets after refresh. A reactive options refresh with the same hash preserves the current actor state and pending initial-connection actions.

## Connection Sharing And Performance

`@rivetkit/svelte` is optimized for the common “one shared transport, many actor consumers” shape:

- multiple `useActor()` calls with the same actor identity share the underlying connection through the ref-counted framework core
- `createSharedRivetKit()` prevents duplicate wrapper creation when the app already centralizes a raw client
- proxied actor methods are cached per actor handle, keep stable identity across reconnects, and lazily cache only the current connection's nested invoker; repeated reads and reconnect-safe destructured handlers do not allocate fresh bound functions
- `createReactiveActor()` construction does not subscribe or connect until `mount()`, keeping module-level ViewModel construction cheap
- `warmUp()` uses raw Rivet client `getOrCreate(key, opts).resolve()` to warm actor resolution without a WebSocket, forwards `createWithInput`/`createInRegion`, supports `noCreate` via `get(key).resolve()`, and uses a collision-safe length-prefixed hash for the common no-input path instead of `JSON.stringify()`. Cyclic and BigInt-containing initialization values fall back to a non-throwing identity hash. Concurrent resolves for one identity are deduplicated, but completion clears that in-flight key so a later hover can wake an actor that has slept again. `createInRegion` is Rivet datacenter selection for newly created actors only; it does not move existing actors. (`preloadActor()` is a deprecated alias.) For the heavier tier that opens a real WebSocket ahead of time, `preConnect()` returns a caller-disposed `{ dispose() }` handle and forces `enabled: true` — use it only for high-intent signals; broad hover should prefer `warmUp()`.
- `lastError` and `hasEverConnected` make reconnect UX easier without forcing app code to track extra flags; internal nonreactive mirrors prevent those public runes from feeding lifecycle effects back into themselves
- concurrent action settlements are invocation-ordered, so an older completion cannot erase or replace the newest action error
- one action timeout deadline covers both the initial connection wait and dispatch instead of granting each phase a fresh budget

Run `bun run --filter @rivetkit/svelte bench` to benchmark proxy reads, forwarded action calls, 32-actor fan-out reads, construct/mount/unmount, subscription pushes, preload hashing, concurrent actions, and `whenConnected()`.

## SSR Safety

- `useActor()` is SSR-safe by default because `$effect` is the browser lifecycle boundary
- `createReactiveActor()` can be created anywhere because construction does not subscribe or connect, but `mount()` should still happen in a browser lifecycle
- prefer app-local typed context over mutable request-time globals in SvelteKit code that can run during SSR

## Testing

Test helpers live under `@rivetkit/svelte/testing`:

```ts
import { describe, expect } from "vitest";
import { testWithEffect } from "@rivetkit/svelte/testing";

describe("runes", () => {
  testWithEffect("runs inside an effect root", () => {
    let count = $state(0);
    expect(count).toBe(0);
  });
});
```

## Type Safety Note

Actor method calls (e.g. `actor.sendMessage(...)`) are **untyped at the package level**. The `ProxiedActorMethods` type uses `Record<string, (...args: any[]) => any>` because RivetKit's deeply nested conditional types inside `ActorConn` exceed TypeScript's instantiation depth limit (TS2589) when wrapped in `Omit` or mapped types.

All reactive state properties (`connStatus`, `error`, `isMutating`, etc.) remain fully typed. For type-safe actor method calls, use typed client interfaces from your actor registry at the call site.

## Familiar Mental Models

The package does not depend on these libraries, but its DX intentionally lines up with maintained Svelte adapters at similar state/lifecycle seams:

- [TanStack Svelte Query](https://tanstack.com/query/latest/docs/framework/svelte/overview): shared-client/provider ownership, reactive result state, and stable mutation functions that dispatch through the current observer
- [XState Svelte](https://stately.ai/docs/xstate-svelte): subscription-owned snapshots with a stable `send` function and selector-oriented updates
- [urql Svelte](https://urql.dev/docs/basics/svelte/): a deliberately thin Svelte binding over a framework-neutral core client
- Runed: `Getter` and `MaybeGetter` ergonomics for reactive inputs
- Bits UI: composable primitives instead of rigid framework wrappers
- Better Auth: auth stays app-owned, while the package only consumes resolved params

The connection lifecycle also follows Rivet's JavaScript client contract: stateful handles own a connection that must be disposed when no longer needed. See the [Rivet JavaScript client](https://rivet.dev/actors/docs/clients/javascript/), [actor lifecycle](https://rivet.dev/actors/docs/lifecycle/), and [production checklist](https://rivet.dev/actors/docs/general/production-checklist/).

## Requirements

- Svelte `^5.57.0`
- RivetKit `2.3.17`

The published artifact is self-contained: its internal framework bridge forwards `getParams` through both `get()` and `getOrCreate()`. It does not rely on a consuming repository's `patchedDependencies` configuration.

## License

Apache-2.0. See the repository [LICENSE](../../../LICENSE).
