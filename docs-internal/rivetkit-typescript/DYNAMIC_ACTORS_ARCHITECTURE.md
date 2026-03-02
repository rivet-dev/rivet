# Dynamic Actors Architecture

## Overview

Dynamic actors let a registry entry resolve actor source code at actor start time.

Dynamic actors are represented by `dynamicActor(loader, config)` and still
participate in normal registry routing and actor lifecycle.

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
  Runs inside the isolate and exports envelope handlers.
- Bridge contract:
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

## Temporary Compatibility Layer

Current implementation materializes a runtime `node_modules` tree under `/tmp`
and patches specific dependencies to CJS safe output.

This is temporary. Remove this path when sandboxed-node can load required
RivetKit runtime dependencies without package source patching.

## Driver Test Skip Gate

The dynamic registry variant in driver tests has a narrow skip gate for two
cases only:

- secure-exec dist is not available on the local machine
- nested dynamic harness mode is explicitly enabled for tests

This gate is only to avoid invalid test harness setups. Static and dynamic
behavior parity remains the expected target for normal driver test execution.
