# RivetKit Runtime Boundary Cleanup Spec

## Goal

Make the TypeScript actor runtime boundary truly portable between NAPI and WebAssembly while keeping the existing actor glue and `CoreRuntime -> NapiCoreRuntime | WasmCoreRuntime` architecture.

The current wasm implementation works end to end, but it reached parity by adapting wasm to a NAPI-shaped boundary. The cleanup goal is to make the shared boundary neutral enough that NAPI, wasm, and any future runtime adapter can implement it without Node-specific shims, hidden globals, or duplicated behavior.

## Problem Summary

The current stack is:

```text
User TypeScript
  setup({ runtime })
    -> rivetkit TypeScript actor glue
    -> CoreRuntime
       -> NapiCoreRuntime -> @rivetkit/rivetkit-napi -> rivetkit-core
       -> WasmCoreRuntime -> @rivetkit/rivetkit-wasm -> rivetkit-core
```

That shape is correct. The problem is the contract under `CoreRuntime` still reflects the original NAPI implementation:

- It uses `Buffer` for runtime bytes.
- SQL types are derived from `JsNativeDatabaseLike`.
- Runtime kind checks use concrete adapter classes instead of `runtime.kind`.
- Wasm package initialization relies on an implicit global binding escape hatch in edge examples.
- NAPI and wasm serverless registry state machines have drifted.
- Some wasm methods are stubs or local adaptations rather than parity implementations.

These differences happened because NAPI existed first and wasm was added as a compatibility adapter. This spec keeps the shared actor glue intact but cleans up the adapter contract.

## Non-Goals

- Do not rewrite the TypeScript actor glue.
- Do not merge `@rivetkit/rivetkit-napi` and `@rivetkit/rivetkit-wasm` into one package.
- Do not reintroduce class-heavy public APIs for user code.
- Do not add local SQLite support to wasm.
- Do not change existing published BARE protocol versions.

## Target Boundary

`CoreRuntime` should be a runtime-neutral TypeScript contract:

```text
CoreRuntime
  bytes: Uint8Array
  handles: opaque runtime handles
  SQL params/results: plain shared structs
  errors: structured RivetError payloads or unknown errors for core sanitization
  lifecycle: identical registry/serverless state semantics
```

Adapters own host-specific conversion:

```text
NapiCoreRuntime
  Node Buffer <-> Uint8Array
  napi-rs classes <-> opaque handles
  native promises/errors <-> CoreRuntime results

WasmCoreRuntime
  wasm-bindgen Uint8Array <-> Uint8Array
  wasm-bindgen classes <-> opaque handles
  JS promises/errors <-> CoreRuntime results
```

The TypeScript actor glue should not need to know which adapter is active.

Tests should use the same public API shape that application developers use. Avoid test-only runtime wiring unless a unit test is specifically isolating a private helper. Edge and packaged-consumer coverage should call `setup({ runtime: "wasm", wasm: { bindings, initInput }, use })` instead of mutating globals, importing private generated paths from app code, or calling lower-level registry builders directly.

## Required Changes

### 1. Replace Boundary Buffers With Portable Bytes

Update `rivetkit-typescript/packages/rivetkit/src/registry/runtime.ts` so runtime byte fields use `Uint8Array` instead of `Buffer`.

This includes:

- HTTP request and response bodies.
- State deltas.
- KV keys and values.
- Queue message bodies and completion payloads.
- SQL blob params and SQL result blobs.
- WebSocket binary messages.
- Conn params and conn state.
- Inspector request and response bytes.

NAPI may still accept and return `Buffer` internally, but the conversion belongs in `NapiCoreRuntime`. Wasm should not need to construct `Buffer` for normal runtime operation.

Acceptance criteria:

- `CoreRuntime` no longer references `Buffer`.
- `NapiCoreRuntime` performs Node-only `Buffer` conversion at its adapter edge.
- `WasmCoreRuntime` does not call `Buffer.from`, `Buffer.alloc`, or `Buffer.isBuffer` for runtime boundary normalization.
- Typecheck passes.
- Tests pass.

### 2. Define Shared SQL Boundary Types

Move the TypeScript SQL runtime types away from `JsNativeDatabaseLike` and define explicit portable types in `runtime.ts` or a small sibling module.

Required shape:

```ts
type RuntimeSqlBindParam =
  | { kind: "null" }
  | { kind: "int"; intValue: number }
  | { kind: "float"; floatValue: number }
  | { kind: "text"; textValue: string }
  | { kind: "blob"; blobValue: Uint8Array };

interface RuntimeSqlQueryResult {
  columns: string[];
  rows: unknown[][];
}

interface RuntimeSqlExecuteResult {
  columns: string[];
  rows: unknown[][];
  changes: number;
  lastInsertRowId?: number | null;
  route: "read" | "write" | "writeFallback";
}

interface RuntimeSqlRunResult {
  changes: number;
}
```

For this cleanup, preserve the current user-facing integer behavior. SQL integer values should continue to surface as numbers where they do today. Do not introduce new public bigint result semantics as part of the boundary cleanup.

Acceptance criteria:

- `runtime.ts` no longer imports `JsNativeDatabaseLike`.
- NAPI and wasm SQL adapters both implement the same explicit SQL result types.
- Existing `wrapJsNativeDatabase` behavior remains unchanged for user-facing database APIs.
- Bigint, boolean, string, number, null, undefined, and `Uint8Array` SQL parameter normalization still works.
- User-facing SQL integer result behavior remains unchanged from the current TypeScript API.
- Typecheck passes.
- Tests pass.

### 3. Make Wasm Initialization First-Class

Remove the need for edge examples to set `globalThis.__rivetkitWasmBindings`.

Use one public wasm package import plus explicit host-provided initialization inputs. This follows the same broad pattern used by Prisma driver adapters and DuckDB-Wasm/sql.js-style initialization: keep the high-level user API stable, but let the host application provide the runtime-specific adapter or asset handle when packaging differs by environment.

Supported configuration should remain:

```ts
setup({
  runtime: "wasm",
  wasm: {
    initInput,
  },
  use: { counter },
});
```

Add an explicit binding override as the first-class escape hatch:

```ts
wasm?: {
  initInput?: WebAssembly.Module | BufferSource | URL | Response;
  bindings?: typeof import("@rivetkit/rivetkit-wasm");
}
```

`bindings` is a documented loader escape hatch, not a hidden global. The default path may still import `@rivetkit/rivetkit-wasm` internally when `bindings` is omitted.

Cloudflare and Supabase should differ only in how they produce `initInput`, not in RivetKit actor/runtime semantics.

Cloudflare example:

```ts
import * as rivetkitWasm from "@rivetkit/rivetkit-wasm";
import wasmModule from "./rivetkit_wasm_bg.wasm";

const registry = setup({
  runtime: "wasm",
  wasm: {
    bindings: rivetkitWasm,
    initInput: wasmModule,
  },
  use: { counter },
});
```

Supabase/Deno example:

```ts
import * as rivetkitWasm from "@rivetkit/rivetkit-wasm";

const wasmBytes = await Deno.readFile(
  new URL("./rivetkit_wasm_bg.wasm", import.meta.url),
);

const registry = setup({
  runtime: "wasm",
  wasm: {
    bindings: rivetkitWasm,
    initInput: wasmBytes,
  },
  use: { counter },
});
```

Do not add `@rivetkit/rivetkit-wasm/cloudflare` or `@rivetkit/rivetkit-wasm/deno` exports in this cleanup unless the single-export plus explicit `bindings`/`initInput` design fails in packaged-consumer tests. If a specialized export becomes necessary, document the packaging failure that requires it.

Acceptance criteria:

- `loadWasmRuntime` does not read `globalThis.__rivetkitWasmBindings`.
- Cloudflare and Supabase examples use either package exports or `wasm.bindings`, not a global.
- `@rivetkit/rivetkit-wasm` exposes one default public import path that can be passed as `wasm.bindings`.
- Cloudflare passes a bundled `WebAssembly.Module` or equivalent as `wasm.initInput`.
- Supabase/Deno passes wasm bytes, URL, `Response`, or equivalent as `wasm.initInput`.
- Packaged-consumer smoke coverage imports only public package exports.
- Typecheck passes.
- Tests pass.

### 4. Align NAPI And Wasm Serverless Registry State

Port the NAPI serverless build state semantics to wasm.

The required state machine is:

```text
Registering
  -> BuildingServerless
  -> Serverless
  -> ShuttingDown
  -> ShutDown
```

Concurrent first serverless requests must share one build or wait for the build to finish. Shutdown during build must cancel or tear down the newly built runtime without orphaning it.

Acceptance criteria:

- Wasm registry has a `BuildingServerless` equivalent.
- Concurrent first requests do not fail with "already serving" while the runtime is building.
- Shutdown during wasm serverless build leaves the registry in a terminal state.
- NAPI and wasm return equivalent wrong-mode/shutdown errors for serve/serverless mode conflicts.
- Add focused tests for concurrent first serverless requests and shutdown during build. These may use a delayed fake runtime builder to make the ordering deterministic.
- Existing workerd and Supabase e2e checks continue to prove the real wasm runtime boots.
- Typecheck passes.
- Tests pass.

### 5. Use `runtime.kind` For Runtime Resolution

Replace concrete adapter checks with the interface discriminator.

Acceptance criteria:

- `loadedRuntimeKind` switches on `runtime.kind`.
- No production runtime selection logic depends on `instanceof NapiCoreRuntime` or `instanceof WasmCoreRuntime`.
- Fake runtimes in tests can use `kind: "napi"` or `kind: "wasm"` without constructing concrete adapter classes.
- Typecheck passes.
- Tests pass.

### 6. Remove Wasm Parity Stubs

Audit `@rivetkit/rivetkit-wasm` for methods that return placeholders or silently diverge from NAPI.

Known issue:

- `WasmQueue.maxSize()` currently returns `0`.

Acceptance criteria:

- `WasmQueue.maxSize()` returns the real core queue max size.
- Add parity coverage for queue max size through both NAPI and wasm adapters.
- Any unsupported wasm runtime method fails with an explicit structured unsupported-runtime error.
- Typecheck passes.
- Tests pass.

### 7. Make Invalid Matrix Cells Visible

The driver matrix should not silently drop an explicitly requested invalid cell.

Acceptance criteria:

- Default matrix excludes `runtime=wasm/sqlite=local`.
- If a user explicitly requests `RIVETKIT_DRIVER_TEST_RUNTIME=wasm` and `RIVETKIT_DRIVER_TEST_SQLITE=local`, the suite fails fast with a clear configuration error.
- Existing valid cells remain native/local/all encodings, native/remote/all encodings, and wasm/remote/all encodings.
- Typecheck passes.
- Tests pass.

### 8. Add Platform Wasm Smoke Coverage

Current workerd and Supabase smoke scripts live under `.agent/` and exercise the kitchen-sink app. Replace that with first-class platform smoke tests under the RivetKit test tree.

Add a new platform test folder:

```text
rivetkit-typescript/packages/rivetkit/tests/platforms/
  shared-registry.ts
  shared-platform-harness.ts
  cloudflare-workers.test.ts
  supabase-functions.test.ts
  deno.test.ts
```

The platform tests should share one minimal registry and actor. Keep it intentionally boring: a SQLite-backed counter actor with `increment` and `getCount` implemented with raw SQL. These tests should validate platform packaging and wasm runtime boot, not duplicate the full driver suite.

Do not run these tests in the default unit or driver test path. Expose them through an explicit script, for example `test:platforms`, or a manual/nightly CI job.

Use pinned `pnpm dlx` commands for platform CLIs. Do not depend on globally installed Wrangler or Supabase CLI versions.

Engine startup should reuse the existing driver-suite shared engine mechanism. If the current helper in `tests/driver/shared-harness.ts` is too driver-specific, extract the engine start/release logic into a shared test utility and have both driver tests and platform tests use it.

Acceptance criteria:

- `tests/platforms/shared-registry.ts` defines the shared SQLite counter actor and registry factory.
- The shared SQLite counter actor uses raw SQL rather than Drizzle.
- Platform tests run from generated temporary app directories or committed minimal fixtures backed by shared source files. Avoid large committed scaffold output.
- Platform tests are not included in the default test command.
- Platform CLIs are launched with pinned `pnpm dlx` versions.
- Cloudflare Workers test runs against real local workerd, for example through pinned `pnpm dlx wrangler@... dev --local`.
- Supabase Functions test runs against real local pinned `pnpm dlx supabase@... functions serve`.
- Deno test runs against plain local Deno without the Supabase CLI wrapper.
- Platform tests reuse the driver-suite shared engine mechanism, or share an extracted engine utility with the driver suite.
- Each platform test imports `rivetkit` and `@rivetkit/rivetkit-wasm` only through public package exports.
- Each platform test uses the same public API shape users should copy: `setup({ runtime: "wasm", wasm: { bindings, initInput }, use })`.
- Do not call lower-level registry builders, mutate `globalThis` loader hooks, or otherwise use test-only wasm runtime wiring in packaged-consumer app code.
- Each platform test exercises the shared SQLite counter with at least one write and one readback.
- Platform coverage includes multiple requests to the same actor, actor sleep and wake, and multiple actors running in parallel on the same platform instance.
- The parallel actor check should use separate actor IDs. It should not rely on concurrent requests to one actor as the only concurrency signal.
- Remote SQLite is used. Local SQLite is not allowed for these wasm platform tests.
- Keep platform smoke intentionally small. Do not test raw HTTP, raw WebSocket, workflows, queues, or the full driver suite here.
- Do not depend on repo-relative imports to `pkg`, `pkg-deno`, or `dist/tsup`.
- Typecheck passes.
- Tests pass.

### 9. Update Public Edge Runtime Docs

Document wasm runtime usage for Cloudflare Workers and Supabase Edge Functions in the public docs.

Required docs:

- Update the quickstart docs to point users at edge/serverless wasm setup.
- Add or update `website/src/content/docs/connect/cloudflare.mdx` for Cloudflare Workers.
- Update `website/src/content/docs/connect/supabase.mdx` with the Supabase Edge Functions setup instead of the placeholder.
- If a new Connect page is added, update the sidebar source used by `website/src/sitemap/mod.ts`.

Docs must use the same public API that tests and users use:

```ts
setup({
  runtime: "wasm",
  wasm: {
    bindings,
    initInput,
  },
  use: { counter },
});
```

Acceptance criteria:

- Cloudflare docs show how to pass the bundled wasm module or equivalent `initInput`.
- Supabase docs show how to load and pass wasm bytes, URL, `Response`, or equivalent `initInput`.
- Docs explain that wasm cannot use local SQLite and defaults to remote SQLite when SQLite config is unset.
- Docs mention `runtime: "wasm"` and the `RIVETKIT_RUNTIME=wasm` env override.
- Docs do not mention hidden globals, private generated paths, or lower-level registry builders.
- Quickstart and Connect pages link to each other where appropriate.

## Risks

- Replacing `Buffer` at the runtime boundary is broad because actor glue currently assumes Node-compatible bytes in many places.
- Edge package export behavior differs between Cloudflare, Deno/Supabase, Node, and bundlers. Keep public exports explicit and tested.
- Serverless state-machine parity is correctness work, not cosmetic cleanup. Treat first-request concurrency as a real bug.
- Some generated wasm-bindgen types may still expose `Uint8Array` or `bigint`; adapters should normalize those at the edge only.

## Validation Plan

Required local checks after the full cleanup:

```bash
pnpm --filter rivetkit check-types
pnpm --filter rivetkit test tests/runtime-selection.test.ts
pnpm --filter rivetkit test tests/driver/shared-matrix.test.ts
scripts/cargo/check-rivetkit-core-wasm.sh
cargo check -p rivetkit-core
```

Required e2e checks:

- Driver suite valid cells for wasm/remote across bare, cbor, and json where wasm support is claimed.
- Cloudflare Workers platform smoke using real local workerd through pinned `pnpm dlx wrangler@...` and the shared SQLite counter registry.
- Supabase Functions platform smoke using real local `supabase functions serve` through pinned `pnpm dlx supabase@...` and the shared SQLite counter registry.
- Deno platform smoke using real local Deno and the shared SQLite counter registry.
