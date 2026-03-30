# Packages / rivetkit-svelte

**Parent:** [Root](../../AGENTS.md)

Official Svelte 5 adapter for RivetKit actors. Thin adapter over `@rivetkit/framework-base`, with Svelte-first ergonomics for app-owned typed context, shared clients, and reactive actor handles.

---

## Workspace

|             |                                                                      |
| ----------- | -------------------------------------------------------------------- |
| Package     | `@rivetkit/svelte`                                                   |
| Scripts     | `build`, `check-types`, `test`                                       |
| Depends on  | `@rivetkit/framework-base`, `rivetkit`, `esm-env`                    |
| Peer deps   | `svelte` ^5.0.0                                                      |
| Dev deps    | `vitest`, `jsdom`, `@sveltejs/package`, `svelte-check`, `typescript` |
| Consumed by | `apps/web`, `apps/connectivity-source`, and external SvelteKit apps  |

## Architecture

The package supports two primary shapes:

```text
Provider pattern:
  app-local module
    → createRivetContext()
    → layout calls context.set(...) or context.setup(...)
    → descendants call appLocalContext.get().useActor(...)

Shared-client pattern:
  app-local createClient(...)
    → createSharedRivetKit(() => client)
    → shared wrapper reused by ViewModels and provider setup
```

Reactive actor state is still powered by framework-base subscriptions bridged into Svelte runes:

```text
useActor(opts | () => opts)
  → extract(MaybeGetter)
  → framework-base getOrCreateActor()
  → $effect subscription
  → getter-backed object + Proxy-forwarded actor methods

createReactiveActor(opts)
  → framework-base getOrCreateActor()
  → manual subscription lifecycle
  → getter-backed object + Proxy-forwarded actor methods

preloadActor(opts)
  → BROWSER guard (no-op during SSR)
  → dedup check (Set<string> keyed by name:key)
  → client Proxy accessor → getOrCreate(key)
  → handle.resolve() (single HTTP PUT, no WebSocket)
  → fire-and-forget (catch removes from dedup on failure)
```

## Structure

```text
packages/rivetkit-svelte/
├── package.json
├── tsconfig.json
├── AGENTS.md
├── README.md
└── src/
    ├── check-types/
    │   └── noop.svelte                # Keeps svelte-check happy for the package workspace
    └── lib/
        ├── index.ts                   # Main barrel exports
        ├── rivetkit.svelte.ts         # createRivetKit, createReactiveActor, useActor, ActionDefaults
        ├── shared.svelte.ts           # createSharedRivetKit, withActorParams, createReactiveConnection
        ├── context.ts                 # createRivetContext
        ├── connection-health.svelte.ts # createConnectionHealth aggregate health
        ├── internal/
        │   ├── types.ts               # Getter, MaybeGetter
        │   └── extract.ts             # extract(MaybeGetter)
        ├── testing/
        │   ├── index.ts
        │   └── test-helpers.svelte.ts
        └── __tests__/
            ├── action-middleware.test.ts # Action middleware interceptor tests (12 tests)
            ├── context.test.ts
            ├── reactive-actor.test.ts
            ├── shared.test.ts
            ├── helpers.ts
            └── runes-shim.ts
```

## Public API

### Factory Functions

- `createRivetKit<Registry>(endpoint?, opts?)`
- `createRivetKitWithClient<Registry>(client, opts?)`

Both return `{ useActor, createReactiveActor, preloadActor }`.

### Context Helper

- `createRivetContext<Registry>()` — typed context helper with `set`, `get`, `has`, `setup`, `setupWithClient`

Apps are expected to create and own their own context instance. The package no longer exports default-context helpers.

### Shared-Client / Mixed-Mode Helpers

- `createSharedRivetKit<Registry>(getClient, opts?)` — lazily reuse one RivetKit wrapper for a shared client factory
- `withActorParams(base, params)` — merge actor options with static or reactive params
- `createReactiveConnection(source)` — bridge raw connection handling into reactive `connStatus` / `error` state

### Action Middleware (`actionDefaults`)

Both `useActor` and `createReactiveActor` accept an `actionDefaults` option (also configurable at the client level via `SvelteRivetKitOptions`). When provided, every proxied action call is wrapped with built-in middleware:

- **Timeout** — configurable per-actor or per-client
- **Error capture** — errors captured to `lastActionError` reactive state (not thrown by default)
- **Loading tracking** — `isMutating`, `pendingActions` counters updated automatically
- **Connection guard** — rejects immediately if disconnected (configurable)
- **Lifecycle callbacks** — `onActionStart`, `onActionSuccess`, `onActionError`, `onActionSettled`

```typescript
const actor = rivet.createReactiveActor({
  name: "user",
  key: ["user", userId],
  actionDefaults: {
    timeout: 30_000,
    throwOnError: false, // default — errors captured reactively
    onActionError: (err, name) => console.error(name, err),
  },
});

// Direct action call — no manual wrapping needed
await actor.updateProfile({ name: "New" });

// Reactive tracking (all $state-backed)
actor.isMutating; // boolean
actor.pendingActions; // number
actor.lastActionError; // Error | null
actor.lastAction; // string | null
actor.resetActionState(); // clear error state
```

**Cascade:** Client-level `actionDefaults` are shallow-merged with actor-level overrides. Actor-level wins.

**Types:** `ActionDefaults`, `SvelteRivetKitOptions` exported from `@rivetkit/svelte`.

### Reactive Actor Primitives

- `useActor<ActorName>(opts: MaybeGetter<ActorOptions>)`
  - component initialization only
  - accepts static options or a getter thunk
  - returns getter-backed reactive metadata plus proxied actor methods
  - exposes `lastError` and `hasEverConnected` in addition to `connection`, `handle`, `connStatus`, `error`, `isConnected`, `hash`, `onEvent`
  - when `actionDefaults` provided: also exposes `isMutating`, `pendingActions`, `lastActionError`, `lastAction`, `resetActionState()`

- `createReactiveActor<ActorName>(opts)`
  - safe in modules and `.svelte.ts` classes
  - manual lifecycle via `mount()` and `dispose()`
  - `onEvent()` rebinds listeners when the underlying connection changes
  - proxied actor methods are cached per connection instance for stable repeated reads
  - when `actionDefaults` provided: also exposes `isMutating`, `pendingActions`, `lastActionError`, `lastAction`, `resetActionState()`

### Actor Preloading

- `preloadActor<ActorName>(opts: PreloadActorOptions)`
  - wakes an actor via a single HTTP `resolve()` call — no WebSocket connection
  - analogous to SvelteKit's `data-sveltekit-preload-data` for routes
  - deduplicates: same actor (name + key) is only resolved once per RivetKit lifetime
  - fire-and-forget: errors are silently caught; failed attempts removed from dedup set for retry
  - SSR-safe: no-ops when `BROWSER` is false (via `esm-env`)
  - intended for hover-based preloading to eliminate cold-start latency

```typescript
// Preload a document actor on hover
rivet.preloadActor({ name: "document", key: ["doc", docId] });
```

### Other Exports

- `createConnectionHealth<K>(getSources)`
- `extract()`
- `Getter<T>` / `MaybeGetter<T>`
- `PreloadActorOptions` type
- `ActionDefaults` type
- `SvelteRivetKitOptions` type
- `createClient` re-export from `rivetkit/client`
- `ActorConnStatus`, `ActorOptions`, `AnyActorRegistry` types

### Testing Subpath

`@rivetkit/svelte/testing` exports:

- `testWithEffect(name, fn)`
- `effectRootScope(fn)`

## Design Notes

### App-Owned Typed Context

The preferred provider-level API is `createRivetContext()`. Each app should own a local context instance rather than depending on a package-global default context.

### Shared Client Ownership Is Explicit

The package does not hide the raw `rivetkit/client` model. Apps that want a single transport should own that client locally and wrap it with `createSharedRivetKit()`.

### App-Owned Auth

Auth stays outside the package. `withActorParams()` exists to make token/org/session params ergonomic without baking Better Auth, Layerr token refresh, or framework-specific session logic into the adapter.

### Familiar Svelte Conventions

- `Getter` / `MaybeGetter` follow the same ergonomic direction teams will recognize from Runed and Bits UI
- provider/shared-client setup maps well to the mental model teams already have from TanStack Query
- composable primitives are preferred over monolithic app-framework wrappers

### Closure-Based Rune State

`createReactiveActor()` uses closure-based `$state` instead of class-field state so Proxy forwarding works correctly. Svelte class-field runes compile to private fields, and private fields do not cooperate with JS `Proxy`.

### Action Middleware Architecture

The action interceptor is built from `ActionDefaults` and passed to `proxyWithConnection()`. Every proxied method call flows through the interceptor, which:

1. Checks connection guard (fail-fast if disconnected)
2. Increments `pendingActions` / sets `isMutating`
3. Races the action against timeout (if configured)
4. On success: clears `lastActionError`, fires `onActionSuccess`
5. On failure: captures error to `lastActionError`, fires `onActionError`. With `throwOnError: false` (default), resolves to `undefined` instead of rejecting.
6. Decrements `pendingActions` / clears `isMutating` when all actions complete

The interceptor is a closure that captures `$state` variables directly — same pattern as the existing connection state tracking. No class fields, no double-proxy.

### SSR Safety

- `useActor()` is SSR-safe because `$effect` is the browser-only lifecycle boundary
- `createReactiveActor()` can be constructed during SSR, but `mount()` still belongs in a browser lifecycle
- `preloadActor()` is SSR-safe — guarded by `BROWSER` from `esm-env`; no-ops during SSR to avoid wasteful HTTP calls
- prefer app-local typed context and browser-owned singletons over request-time mutable globals in SvelteKit code

## Integration With apps/web

The web app now uses:

- `apps/web/src/lib/context/actor-rivet-context.ts` for the app-local typed Rivet context
- `apps/web/src/lib/clients/actor-client.ts` for shared raw client and shared wrapper ownership

That composition point owns:

- `getActorClient(endpoint)` — shared raw client per endpoint
- `getActorRivet(endpoint)` — shared wrapper via `createSharedRivetKit()`
- `setupActorRivetContext(endpoint)` — typed-context provider setup

`BaseActorViewModel` consumes `getActorRivet(endpoint)` and `withActorParams(...)`, so primary reactive actors and any lazy actor handles continue sharing one transport while Layerr-specific token refresh remains in app code.

`BaseActorViewModel._createAndMount()` passes `actionDefaults` with `timeout: 30_000`, `throwOnError: false`, `guardConnection: true`, and callback bridges that sync the package's action tracking state to the ViewModel's reactive `isMutating` and `error` fields. Subclass ViewModels call `this.actor.someAction()` directly — no `callAction()` wrapper needed. The `callAction` method still exists as a thin adapter for legacy code that expects the `T | null` return shape.

## Verification Expectations

When changing this package, verify at minimum:

- `bun run --filter @rivetkit/svelte check-types`
- `bun run --filter @rivetkit/svelte test`
- `bun run --filter @rivetkit/svelte build` when public exports or generated declarations changed
- consumer typecheck for `apps/web` and any other in-repo Svelte consumer if package surface or inferred types changed

## Related

[README.md](./README.md) | [Root AGENTS.md](../../AGENTS.md) | [Web App](../../apps/web/AGENTS.md) | [Real-Time Architecture](../../docs/architecture/Real-Time%20Architecture.md)
