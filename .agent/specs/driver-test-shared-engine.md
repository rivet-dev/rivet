# Driver Test Shared Engine Proposal

## Problem

The RivetKit TypeScript driver suite currently looks like it disables per-runtime engine startup, but still spawns an engine per test.

- `tests/fixtures/driver-test-suite-runtime.ts` sets `registry.config.startEngine = false`.
- The same fixture then sets `serveConfig.engineBinaryPath = resolveEngineBinaryPath()`.
- `rivetkit-napi` forwards `engineBinaryPath` to `rivetkit-core`.
- `rivetkit-core` treats `engine_binary_path` as the signal to call `EngineProcessManager::start(...)`.

The result is heavier than intended:

- Fresh runtime process per test.
- Fresh engine process per test.
- Shared `default` namespace.
- Unique pool name per runtime.
- Sequential execution across the full encoding matrix.

This makes full driver-suite reruns slow as hell and hides the intended isolation model.

## Goal

Use one shared engine process for the driver suite and isolate each test with its own namespace.

The first implementation should keep runtime process isolation unchanged. The big win is removing per-test engine startup, not rewriting the whole suite at once.

Target model:

- One engine process per driver-suite run or per registry variant.
- Fresh runtime process per test, initially unchanged.
- Unique namespace per test.
- Unique pool name per test.
- No `engineBinaryPath` passed into runtime serve config.
- Keep the suite sequential for the first diff.

## Non-Goals

- Do not parallelize the full suite in the first pass.
- Do not share a registry runtime between tests in the first pass.
- Do not change actor fixture semantics unless namespace isolation exposes a real bug.
- Do not add mocking or fake infrastructure.

## Proposed Design

### Shared Engine

Start the engine once from `tests/driver-test-suite.test.ts` before running the static registry variant.

The shared engine should provide:

- `endpoint`
- `token`
- lifecycle cleanup after the suite exits

The existing engine binary resolution logic can remain in the test harness, but it should move out of `driver-test-suite-runtime.ts`.

### Runtime Fixture

Update `tests/fixtures/driver-test-suite-runtime.ts` so it only starts the registry/envoy runtime against an existing engine endpoint.

Required changes:

- Keep `registry.config.startEngine = false`.
- Keep `registry.config.endpoint = endpoint`.
- Keep `registry.config.namespace = namespace`.
- Keep `registry.config.envoy.poolName = poolName`.
- Remove `serveConfig.engineBinaryPath = resolveEngineBinaryPath()`.

This prevents `nativeRegistry.serve(serveConfig)` from spawning an engine.

### Per-Test Namespace

Generate a unique namespace for every test setup.

Suggested format:

```ts
const namespace = `driver-${crypto.randomUUID()}`;
```

Thread this namespace through:

- `startNativeDriverRuntime(...)`
- spawned runtime env var `RIVET_NAMESPACE`
- returned `DriverDeployOutput.namespace`
- client config in `setupDriverTest(...)`

Keep `poolName` unique per test as it is today.

### Runner Config

`upsertNormalRunnerConfig(...)` should operate against the per-test namespace.

If namespaces must be created explicitly before runner config upsert, add a small `ensureNamespace(...)` helper in the test harness.

If the engine lazily creates namespaces today, document that assumption in the helper.

### Cleanup

First pass can rely on process-level engine teardown to discard per-test namespaces.

If namespace accumulation becomes a problem inside one run, add best-effort namespace deletion in `cleanup()`.

Cleanup must still stop the per-test runtime process.

## Implementation Plan

1. Move engine binary resolution and engine process startup into `tests/driver-test-suite.test.ts`.
2. Start the engine once for the static registry suite.
3. Pass the shared engine endpoint into `startNativeDriverRuntime(...)`.
4. Generate a unique namespace inside `startNativeDriverRuntime(...)`.
5. Remove `serveConfig.engineBinaryPath = resolveEngineBinaryPath()` from `driver-test-suite-runtime.ts`.
6. Update runner config setup to use the per-test namespace.
7. Run a narrow TS driver test to verify a runtime can register against the shared engine.
8. Run a broader `bare` subset.
9. Run the full driver suite after the harness change is stable.

## Parallelism Follow-Up

Only consider parallelism after the shared-engine model is green.

Recommended sequence:

- Phase 1: Shared engine, per-test namespace, per-test runtime, sequential suite.
- Phase 2: Parallelize by worker or file chunk, still using unique namespaces and pool names.
- Phase 3: Consider one runtime per worker if startup cost is still high.

Separate-process mode is useful only after Phase 1. Before that, it just makes per-test engine spawn more chaotic.

## Risks

- Tests may implicitly assume the namespace is `default`.
- Engine APIs may require explicit namespace creation before runner config upsert.
- Some resources may be engine-global rather than namespace-scoped.
- Parallelizing too early could introduce flaky envoy registration and cleanup races.
- Per-test runtime spawn may still be slow, just less stupid than per-test engine spawn.

## Success Criteria

- No driver runtime fixture passes `engineBinaryPath` to native serve config.
- A full driver-suite run uses one engine process instead of one engine process per test.
- Each test uses a unique namespace.
- Existing targeted RivetKit driver tests still pass.
- Full suite runtime drops materially before any parallelism is introduced.
