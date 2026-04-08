# Driver Engine Static Test Order

This note breaks the `driver-engine.test.ts` suite into file-name groups for static-only debugging.

Scope:
- `registry (static)` only
- `client type (http)` only unless a specific bug points to inline client behavior
- `encoding (bare)` only unless a specific bug points to CBOR or JSON
- Exclude `agent-os` from the normal pass target
- Exclude `dynamic-reload` from the static pass target

Checklist rules:
- A checkbox is marked only when the entire `*.ts` file has been covered and is fully passing.
- Do not check a file off just because investigation started.
- Start with a single test name, not a whole file-group or suite label.
- After one single test passes, grow scope within that same file until the entire file passes.
- Do not start the next tracked file until the current file is fully passing.
- If a widened file run fails, stop expanding scope and fix that same file before running anything from the next file.
- Record average duration only after the full file is passing.
- The filenames in this note are tracking labels only. `pnpm test ... -t` does not filter by `src/driver-test-suite/tests/<file>.ts`.
- `driver-engine.test.ts` wires everything into nested `describe(...)` blocks, so filter by the description text from the suite, plus the static path text when needed: `registry (static)`, `client type (http)`, and `encoding (bare)`.

## How To Filter

Use `-t` against the `describe(...)` text, not the filename from this note.

Base command shape:

```bash
cd rivetkit-typescript/packages/rivetkit
pnpm test driver-engine.test.ts -t "registry \\(static\\).*client type \\(http\\).*encoding \\(bare\\).*<suite description text>"
```

To narrow to one single test inside that suite, append a stable chunk of the test name:

```bash
cd rivetkit-typescript/packages/rivetkit
pnpm test driver-engine.test.ts -t "registry \\(static\\).*client type \\(http\\).*encoding \\(bare\\).*Actor Driver Tests.*should"
```

Common suite-description mappings:
- `actor-state.ts` -> `Actor State Tests`
- `actor-schedule.ts` -> `Actor Schedule Tests`
- `actor-sleep.ts` -> `Actor Sleep Tests`
- `actor-sleep-db.ts` -> `Actor Sleep Database Tests`
- `actor-lifecycle.ts` -> `Actor Lifecycle Tests`
- `manager-driver.ts` -> `Manager Driver Tests`
- `actor-conn.ts` -> `Actor Connection Tests`
- `actor-conn-state.ts` -> `Actor Connection State Tests`
- `conn-error-serialization.ts` -> `Connection Error Serialization Tests`
- `access-control.ts` -> `access control`
- `actor-vars.ts` -> `Actor Variables`
- `actor-db.ts` -> `Actor Database (raw) Tests`, `Actor Database (drizzle) Tests`, or `Actor Database Lifecycle Cleanup Tests`
- `raw-http.ts` -> `raw http`
- `raw-http-request-properties.ts` -> `raw http request properties`
- `raw-websocket.ts` -> `raw websocket`
- `hibernatable-websocket-protocol.ts` -> `hibernatable websocket protocol`
- `cross-backend-vfs.ts` -> `Cross-Backend VFS Compatibility Tests`
- `actor-agent-os.ts` -> `Actor agentOS Tests`
- `dynamic-reload.ts` -> `Dynamic Actor Reload Tests`
- `actor-conn-status.ts` -> `Connection Status Changes`
- `gateway-routing.ts` -> `Gateway Routing`
- `lifecycle-hooks.ts` -> `Lifecycle Hooks`

Why this order:
- The suite currently pays full per-test harness cost for every test:
  - fresh namespace
  - fresh runner config
  - fresh envoy/driver lifecycle
- Cheap tests are mostly harness overhead
- Slow tests are concentrated in sleep, sandbox, workflow, and DB stress categories
- Wrapper suites that pull in sleep-heavy children should be treated as slow even if the wrapper filename looks generic
- Files that use sleep/hibernation waits or `describe.sequential` should not stay in the fast block

## Fastest First

These are the best initial groups for static-only bring-up.

- [x] `manager-driver.ts` - avg ~10.3s/test over 16 tests, suite 15.1s
- [x] `actor-conn.ts` - avg ~8.4s/test over 23 tests, suite 16.0s
- [x] `actor-conn-state.ts` - avg ~9.3s/test over 8 tests, suite 9.9s
- [x] `conn-error-serialization.ts` - avg ~8.2s/test over 2 tests, suite 8.2s
- [x] `actor-destroy.ts` - avg ~9.8s/test over 10 tests, suite 10.2s
- [x] `request-access.ts` - avg ~9.1s/test over 4 tests, suite 9.1s
- [x] `actor-handle.ts` - avg ~7.7s/test over 12 tests, suite 8.3s
- [x] `action-features.ts` - avg ~8.3s/test over 11 tests, suite 8.8s
- [x] `access-control.ts` - avg ~8.5s/test over 8 tests, suite 8.8s
- [x] `actor-vars.ts` - avg ~8.3s/test over 5 tests, suite 8.5s
- [x] `actor-metadata.ts` - avg ~8.3s/test over 6 tests, suite 8.4s
- [x] `actor-onstatechange.ts` - avg ~8.3s/test over 5 tests, suite 8.3s
- [x] `actor-db.ts` - avg ~9.5s/test over 28 tests, suite 27.0s
- [x] `actor-workflow.ts` - avg ~9.2s/test over 19 tests, suite 11.9s
- [x] `actor-error-handling.ts` - avg ~8.5s/test over 7 tests, suite 8.5s
- [x] `actor-queue.ts` - avg ~9.3s/test over 25 tests, suite 17.5s
- [x] `actor-inline-client.ts` - avg ~9.0s/test over 5 tests, suite 9.8s
- [x] `actor-kv.ts` - avg ~8.4s/test over 3 tests, suite 8.4s
- [x] `actor-stateless.ts` - avg ~8.6s/test over 6 tests, suite 9.1s
- [x] `raw-http.ts` - avg ~8.6s/test over 15 tests, suite 10.1s
- [x] `raw-http-request-properties.ts` - avg ~8.5s/test over 16 tests, suite 9.9s
- [x] `raw-websocket.ts` - avg ~8.9s/test over 13 tests, suite 11.1s
- [x] `actor-inspector.ts` - avg ~9.6s/test over 20 tests, suite 12.1s
- [x] `gateway-query-url.ts` - avg ~8.3s/test over 2 tests, suite 8.3s
- [x] `actor-db-kv-stats.ts` - avg ~9.0s/test over 11 tests, suite 9.9s
- [x] `actor-db-pragma-migration.ts` - avg ~8.8s/test over 4 tests, suite 9.0s
- [x] `actor-state-zod-coercion.ts` - avg ~8.8s/test over 3 tests, suite 8.8s
- [ ] `actor-conn-status.ts`
- [ ] `gateway-routing.ts`
- [ ] `lifecycle-hooks.ts`

## Slow End

These should be last because they are the most likely to dominate wall time.

- [x] `actor-state.ts` - avg ~9.0s/test over 3 tests, suite 9.1s
- [x] `actor-schedule.ts` - avg ~9.9s/test over 4 tests, suite 9.9s
- [ ] `actor-sleep.ts`
- [ ] `actor-sleep-db.ts`
- [ ] `actor-lifecycle.ts`
- [ ] `actor-conn-hibernation.ts`
- [ ] `actor-run.ts`
- [ ] `actor-sandbox.ts`
- [ ] `hibernatable-websocket-protocol.ts`
- [ ] `cross-backend-vfs.ts`
- [ ] `actor-db-stress.ts`

## Not In Static Pass

These should not block the static-only pass target.

- [ ] `actor-agent-os.ts`
  Explicitly allowed to skip for now.
- [ ] `dynamic-reload.ts`
  Dynamic-only path.

## Files Present But Not Wired In `runDriverTests`

- [ ] `raw-http-direct-registry.ts` - intentionally commented out (blocked on gateway actor queries)
- [ ] `raw-websocket-direct-registry.ts` - intentionally commented out (blocked on gateway actor queries)

## Suggested Static-Only Debugging Sequence

Use one single test at a time with `-t`, then grow scope within the same file only after that single test passes.

- [ ] Run one single test from the next unchecked file.
- [ ] Fix the first failing single test before expanding scope.
- [ ] After one test passes, widen to the rest of that file until the entire file passes.
- [ ] Check the file off only after the entire file is passing.
- [ ] After the fast block is clean, run the medium-cost block.
- [ ] Run the slow-end block last.
- [ ] Run `agent-os` separately only if explicitly needed.

## Example Commands

Run one tracked file-group by suite description:

```bash
cd rivetkit-typescript/packages/rivetkit
pnpm test driver-engine.test.ts -t "registry \\(static\\).*client type \\(http\\).*encoding \\(bare\\).*Actor Driver Tests"
```

Run one single test inside that tracked file-group:

```bash
cd rivetkit-typescript/packages/rivetkit
pnpm test driver-engine.test.ts -t "registry \\(static\\).*client type \\(http\\).*encoding \\(bare\\).*Actor Driver Tests.*should create actors"
```

Run a slow group explicitly by suite description:

```bash
cd rivetkit-typescript/packages/rivetkit
pnpm test driver-engine.test.ts -t "registry \\(static\\).*client type \\(http\\).*encoding \\(bare\\).*Actor Sleep Database Tests"
```

Run sandbox only:

```bash
cd rivetkit-typescript/packages/rivetkit
pnpm test driver-engine.test.ts -t "registry \\(static\\).*client type \\(http\\).*encoding \\(bare\\).*Actor Sandbox Tests"
```

## Evidence For Slow Ordering

Observed from the current full-run log:
- cheap tests like raw HTTP property checks are roughly around 1 second end-to-end including teardown
- sandbox tests are about 8.5 to 8.8 seconds each
- sleep and sleep-db groups show repeated alarm/sleep cycles and are consistently the longest-running categories in the log
- `actor-state.ts`, `actor-schedule.ts`, `actor-sleep.ts`, `actor-sleep-db.ts`, and `actor-lifecycle.ts` are all called directly from `mod.ts` and inherit the sleep-heavy cost profile
- `actor-run.ts`, `actor-conn-hibernation.ts`, and `hibernatable-websocket-protocol.ts` all spend real time in sleep or hibernation waits
- the suite-wide average is inflated by the repeated harness lifecycle and these slow categories
