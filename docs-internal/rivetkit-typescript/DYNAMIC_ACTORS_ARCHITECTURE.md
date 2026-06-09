# Bridged Actors: Dynamic Actors and Worker-Runtime Actors

## Overview

Bridged actors run an actor's user code outside the host process's main
thread, in a dedicated `node:worker_threads` worker per actor instance, while
the Rust core (rivetkit-core via NAPI) keeps owning lifecycle, state
persistence, KV, SQLite, queues, schedules, and envoy transport in the host
process.

Two frontends share the bridge:

- **Dynamic actors** (`dynamicActor({ load, options? })` from
  `rivetkit/dynamic`): the loader runs in the host per actor instance,
  returns source text, and the source executes in the actor's worker. The
  source must export an actor definition as its default export.
- **Worker-runtime actors** (`actor({ options: { runtime: "worker" } })` plus
  registry config `worker: { module, exportName? }`): a statically registered
  definition whose user code executes in a worker. The worker imports the
  definition from the configured registry module, which must be
  side-effect-free (no serve/start calls); pass `import.meta.url` from the
  module that exports the registry.

Both are native-runtime only (`runtime.kind === "napi"`); the wasm runtime
rejects bridged definitions at registry build.

## Main Components (`rivetkit-typescript/packages/rivetkit/src/bridge/`)

- `protocol.ts` — envelope types, handle refs, and the transport
  classification tables for CoreRuntime methods (async rpc, fire-and-forget
  post, blocking sync).
- `host.ts` — `buildBridgedFactory(...)`: registers a proxy callbacks bag with
  the real runtime's `createActorFactory`; `BridgedActorChild` owns one worker
  per actor instance, the handle tables (conns, websockets, cancellation
  tokens, completable queue messages, promise regions), and serves CoreRuntime
  calls from the child against real handles.
- `child-runtime.ts` — `RemoteCoreRuntime implements CoreRuntime` over the
  worker MessagePort. The child runs the regular `buildNativeFactory` glue
  against it, so all actor-facing behavior (context adapters, validation,
  state proxy, inspector handling) is the same code as in-process actors.
- `child-main.ts` — child bootstrap: resolves the definition, builds the
  factory, dispatches host callback envelopes into the captured callbacks bag.
- `child-entry.ts` — production worker entry, bundled and exported as
  `rivetkit/bridge-child`.
- `dev-bundle.ts` — development-only: when the host runs from TypeScript
  source (vitest, tsx), the worker cannot execute the .ts entry (no alias
  resolver in workers; loader hooks break CJS filename bookkeeping there), so
  the host prebundles a per-definition entry with esbuild. Package source
  bundles (including the definition module or dynamic source); every bare
  specifier stays external and loads from node_modules at runtime, exactly
  like production.
- `factory.ts` — assembles the two frontends: callback-surface computation,
  RuntimeActorConfig construction, loader execution, source file writing
  (content-hash directories under `<cwd>/.rivetkit/dynamic-actors/`).
- `sync-channel.ts` — SharedArrayBuffer + Atomics blocking RPC for the few
  genuinely synchronous CoreRuntime reads.

Registry integration: `buildRegistryWithRuntime`
(`src/registry/native.ts`) branches per definition and lazy-imports
`@/bridge/factory` so non-Node platforms never load `node:worker_threads`.

## Lifecycle

1. The Rust core invokes a factory callback (createState, onWake, action,
   ...) on the host proxy bag.
2. The host looks for a child in the actor's runtime-state bag
   (`actorRuntimeState`). The bag resets on same-key recreate, which is the
   generation signal: no child in the bag means a fresh generation, so any
   stale child for the actor id is terminated and a new worker spawns.
3. Spawn resolves the plan first: for dynamic actors the loader runs (with
   `key` and an inline `client()`), the source is written under a
   content-hash path, and (in dev) the child bundle is built. Worker
   `resourceLimits.maxOldGenerationSizeMb` comes from the loader's
   `worker.memoryLimitMb`.
4. The child boots, builds the factory bag, and reports `ready` with the
   callback names the loaded definition registered. The host short-circuits
   callbacks the definition lacks (possible only for dynamic actors, which
   register the full surface) to core-equivalent no-op defaults without a
   round trip.
5. Callbacks forward as `cb:invoke` envelopes; handles in payloads become
   refs with eagerly attached immutable metadata. Errors flatten to
   structured payloads and rethrow host-side as `RivetError`s so NAPI bridge
   encoding behaves exactly as in-process.
6. After `onSleep`/`onDestroy` completes, the host drains in-flight envelopes
   (bounded) and terminates the worker. A worker crash fails all in-flight
   envelopes; the engine's wake retries spawn a fresh generation.

## Wire Design (performance)

- Async CoreRuntime methods (KV, SQL, queue waits, conn disconnect, saves):
  request/response envelopes over the per-actor MessagePort.
- Sync void methods (connSend, broadcast, webSocketSend, schedule, setAlarm,
  requestSave, sleep/destroy): fire-and-forget posts; port FIFO preserves
  ordering; host failures push `evt:postError`, logged in the child.
- Sync reads of immutable data (actor id/name/key/region, queueMaxSize, conn
  id/params/isHibernatable): pushed eagerly with the handle, cached in the
  child.
- `actorState` is read once per wake by the glue; `connState` keeps a
  child-local mirror with write-through (the child is the only conn-state
  writer).
- `actorRuntimeState` is child-local (JS-only cache by design).
- Promise-argument APIs (waitUntil, keepAwake, registerTask) and begin/end
  region APIs mirror through `region:begin`/`region:end`; the host holds a
  deferred promise against the real runtime.
- Remaining genuinely-sync mutable reads (actorConns, queueTryNextBatch,
  inspector codecs, sql metrics, hibernation bookkeeping) block on the
  SharedArrayBuffer channel; all are cold paths. The host never blocks on the
  child, so this cannot deadlock.
- Queue messages flatten to plain data plus a host-side completable handle.
- WebSocket events forward host-to-child; the host registers its forwarder
  before the child ever sees the handle and the child buffers events until
  the glue installs its callback, so no event can be lost.

## Action Dispatch

NAPI binds action callbacks by name at factory creation. Worker-runtime
actors register per-name proxies. Dynamic actors do not know their action
names until load, so the factory registers a `fallbackAction` callback
(rivetkit-napi: unmatched action names route to it) and the child resolves
the handler from the loaded definition, raising `actor.action_not_found` for
unknown names.

## Security Model

Worker threads provide execution isolation (own heap with optional memory
limit, own event loop) but NOT a security sandbox: workers share the process,
filesystem, network, and environment. Dynamic actor source must be trusted at
the same level as deployed code. The previous secure-exec NodeProcess sandbox
was deleted with the all-TS runtime (commit cc1411b91); a sandboxed executor
can be reintroduced behind the same bridge protocol later.

## Testing

Bridged actors are exercised by the existing driver test suite, not a bespoke
smoke. `fixtures/driver-test-suite/registry-worker.ts` and `registry-dynamic.ts`
mirror the static registry, so the full driver suite runs end to end through the
bridge when selected with `RIVETKIT_DRIVER_TEST_REGISTRY=worker` (or `dynamic`).
Bridged variants run native-runtime cells only and are a manual / opt-in pass
(not in the default run or CI). Filter to a variant and file as usual, e.g.:

```bash
RIVETKIT_DRIVER_TEST_REGISTRY=worker RIVETKIT_DRIVER_TEST_RUNTIME=native \
  RIVETKIT_DRIVER_TEST_ENCODING=bare RIVETKIT_DRIVER_TEST_SQLITE=local \
  pnpm test tests/driver/actor-conn.test.ts
```
