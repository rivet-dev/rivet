# CLAUDE.md

Design constraints, invariants, and reference commands for the Rivet monorepo. For implementation details, wiring, and procedural gotchas, follow the links under [Reference Docs](#reference-docs).

## Important Domain Information

**ALWAYS use `rivet.dev` - NEVER use `rivet.gg`**

**Never claim "zero cold start" or "no cold start" for agentOS or Rivet Actors.** Always say "near-zero cold start" or specify the actual latency (e.g. "~6 ms cold start"). Cold starts are real, just very small.

- API endpoint: `https://api.rivet.dev`
- Cloud API endpoint: `https://cloud-api.rivet.dev`
- Dashboard: `https://hub.rivet.dev`
- Documentation: `https://rivet.dev/docs`

The `rivet.gg` domain is deprecated and should never be used in this codebase.

**Use "sandbox mounting" when referring to the agentOS sandbox integration.** Do not use "sandbox extension" or "sandbox escalation." The feature mounts a sandbox as a filesystem inside the VM.

**ALWAYS use `github.com/rivet-dev/rivet` - NEVER use `rivet-dev/rivetkit` or `rivet-gg/*`**

**Never modify an existing published `*.bare` runner protocol version unless explicitly asked to do so.**

- Add a new versioned schema instead, then migrate `versioned.rs` and related compatibility code to bridge old versions forward.
- When bumping the protocol version, update `PROTOCOL_MK2_VERSION` in `engine/packages/runner-protocol/src/lib.rs` and `PROTOCOL_VERSION` in `rivetkit-typescript/packages/engine-runner/src/mod.ts` together. Both must match the latest schema version.

When talking about "Rivet Actors" make sure to capitalize "Rivet Actor" as a proper noun and lowercase "actor" as a generic noun.

## Commands

### Build + test

```bash
# Check a specific package without producing artifacts (preferred for verification)
cargo check -p package-name

# Build
cargo build
cargo build -p package-name
cargo build --release

# Test
cargo test
cargo test -p package-name
cargo test test_name
cargo test -- --nocapture
```

### Development

```bash
# Run linter (but see "Development warnings" below)
./scripts/cargo/fix.sh

# Check for linting issues
cargo clippy -- -W warnings
```

- Do not run `cargo fmt` automatically. The team runs it at merge time.
- Do not run `./scripts/cargo/fix.sh`. Do not format the code yourself.
- Ensure lefthook is installed and enabled for git hooks (`lefthook install`).

### Docker dev environment

```bash
cd self-host/compose/dev
docker-compose up -d
```

- Do not edit `self-host/compose/dev*` configs directly. Edit the template in `self-host/compose/template/` and rerun (`cd self-host/compose/template && pnpm start`) to regenerate.
- Rebuild publish base images with `scripts/docker-builder-base/build-push.sh <base-name|all> --push`. Update `BASE_TAG` when rebuilding shared builder bases; engine bases are published per commit in `publish.yaml`.

### Git + PRs

- Use conventional commits with a single-line commit message, no co-author: `git commit -m "chore(my-pkg): foo bar"`.
- We use Graphite for stacked PRs. Diff against the parent branch (`gt ls` to see the stack), not `main`.
- To revert a file to the version before this branch's changes, checkout from the first child branch (below in the stack), not from `main` or the parent. Child branches contain the pre-this-branch state of files modified by branches further down the stack.

**Never push to `main` unless explicitly specified by the user.**

## Frontend Routing (TanStack Router)

### Route context vs loader data
- `context()` runs at match creation time — its return value is part of `match.context` and readable via `useRouteContext`. Use it for synchronous context setup (e.g. creating a data provider from params).
- `beforeLoad()` return value goes into `match.__beforeLoadContext` and is **never merged back into `match.context`**. `useRouteContext` will not see it — components reading via `useRouteContext` get a stale snapshot from before `beforeLoad` ran.
- For async-computed values (e.g. a data provider that depends on a fetched namespace), return the value from `loader()` instead and read it in components via `useLoaderData`. The loader receives the full merged context including `beforeLoad` results as a function argument, so it can re-export the computed value into `match.loaderData`.
- Rule of thumb: sync setup → `context()` + `useRouteContext`. Async setup → `beforeLoad` (for child route access) + `loader` return + `useLoaderData` (for component access).

## Dependency Management

### pnpm Workspace
- Use pnpm for all npm-related commands. We're using a pnpm workspace.

### TypeScript Concurrency
- Use `antiox` for TypeScript concurrency primitives instead of ad hoc Promise queues, custom channel wrappers, or event-emitter based coordination.
- Prefer the Tokio-shaped APIs from `antiox` for concurrency needs. For example, use `antiox/sync/mpsc` for `tx` and `rx` channels, `antiox/task` for spawning tasks, and the matching sync and time modules as needed.
- Treat `antiox` as the default choice for any TypeScript concurrency work because it mirrors Rust and Tokio APIs used elsewhere in the codebase.

### RivetKit Type Build Troubleshooting
- If `rivetkit` type or DTS builds fail with missing `@rivetkit/*` declarations, run `pnpm build -F rivetkit` from repo root (Turbo build path) before changing TypeScript `paths`.
- After native `rivetkit-core` changes, use `pnpm --filter @rivetkit/rivetkit-napi build:force` before TS driver tests because the normal N-API build skips when a prebuilt `.node` exists.
- When removing `rivetkit-napi` `JsActorConfig` fields, keep `impl From<JsActorConfig> for FlatActorConfig` explicit and set any wider core-only fields to `None` instead of dropping them from the struct literal.
- Do not add temporary `@rivetkit/*` path aliases in `rivetkit-typescript/packages/rivetkit/tsconfig.json` to work around stale or missing built declarations.
- When trimming `rivetkit` entrypoints, update `package.json` exports, `files`, and `scripts.build` together. `tsup` can still pass while stale exports point at missing dist files.

### RivetKit Test Fixtures
- Keep RivetKit test fixtures scoped to the engine-only runtime.
- Prefer targeted integration tests under `rivetkit-typescript/packages/rivetkit/tests/` over shared multi-driver matrices.
- `rivetkit-typescript/packages/rivetkit/tests/driver/shared-harness.ts` mirrors runtime stderr lines containing `[DBG]`; strip temporary debug instrumentation before timing-sensitive driver reruns or hibernation tests will timeout on log spam.
- `POST /inspector/workflow/replay` can legitimately return an empty workflow-history snapshot when replaying from the beginning because the endpoint clears persisted history before restarting the workflow.
- For inspector replay tests, prove "workflow in flight" via inspector `workflowState` (`pending`/`running`), not `entryMetadata.status` or `runHandlerActive`, because those can lag or disagree across encodings.
- Query-backed inspector endpoints can each hit their own transient `guard/actor_ready_timeout` during actor startup, so active-workflow driver tests should poll the exact endpoint they assert on instead of waiting on one inspector route and doing a single fetch against another.
- When filtering a single driver file with Vitest, include the outer `describeDriverMatrix(...)` suite name before `static registry > encoding (...)` in the `-t` regex or Vitest will happily skip the whole file.
- When moving Rust inline tests out of `src/`, keep a tiny source-owned `#[cfg(test)] #[path = "..."] mod tests;` shim so the moved file still has private module access without widening runtime visibility.
- For RivetKit runtime or parity bugs, use `rivetkit-typescript/packages/rivetkit` driver tests as the primary oracle: reproduce with the TypeScript driver suite first, compare behavior against the original TypeScript implementation at ref `feat/sqlite-vfs-v2`, patch native/Rust to match, then rerun the same TypeScript driver test before adding lower-level native tests.

### SQLite Package
- RivetKit SQLite is native-only: VFS and query execution live in `rivetkit-rust/packages/rivetkit-sqlite/`, core owns lifecycle, and NAPI only marshals JS types.
- The N-API addon lives at `@rivetkit/rivetkit-napi` in `rivetkit-typescript/packages/rivetkit-napi`; keep Docker build targets, publish metadata, examples, and workspace package references in sync when renaming or moving it.
- N-API actor-runtime wrappers should expose `ActorContext` sub-objects as first-class classes, keep raw payloads as `Buffer`, and wrap queue messages as classes so completable receives can call `complete()` back into Rust.
- N-API callback bridges should pass a single request object through `ThreadsafeFunction`, and Promise results that cross back into Rust should deserialize into `#[napi(object)]` structs instead of `JsObject` so the callback future stays `Send`.
- Keep the receive-loop adapter callback registry centralized in `rivetkit-typescript/packages/rivetkit-napi/src/actor_factory.rs`; extend its TSF slots, payload builders, and bridge error helpers there instead of scattering ad hoc JS conversion logic across new dispatch code.
- Keep `rivetkit-typescript/packages/rivetkit-napi/src/napi_actor_events.rs` as the receive-loop execution boundary; `actor_factory.rs` should stay focused on TSF binding setup and bridge helpers, not event-loop control flow.
- `rivetkit-napi` `ActorContextShared` instances are cached by `actor_id`; every fresh `run_adapter_loop` must call `reset_runtime_shared_state()` before reattaching abort/run/task hooks or sleep→wake cycles inherit stale `end_reason` / lifecycle flags and drop post-wake events.
- Receive-loop `SerializeState` handling should stay inline in `napi_actor_events.rs`, reuse the shared `state_deltas_from_payload(...)` converter from `actor_context.rs`, and only cancel the adapter abort token on `Destroy` or final adapter teardown, not on `Sleep`.
- In `rivetkit-typescript/packages/rivetkit/src/registry/native.ts`, late `registerTask(...)` calls during sleep/finalize teardown can legitimately hit `actor task registration is closed` / `not configured`; swallow only that specific bridge error so workflow cleanup does not crash the runtime.
- Bare-workflow `no_envoys` failures should be investigated as possible runtime crashes before being chased as engine scheduling misses; check actor stderr for late `registerTask(...)` / adapter panics first.
- Receive-loop NAPI optional callbacks should preserve the TypeScript runtime defaults: missing `onBeforeSubscribe` allows the subscription, missing workflow callbacks reply `None`, and missing connection lifecycle hooks still accept the connection while leaving the existing empty conn state untouched.
- N-API `ThreadsafeFunction` callbacks using `ErrorStrategy::CalleeHandled` follow Node's error-first JS signature, so internal wrappers must accept `(err, payload)` and rethrow non-null errors explicitly.
- N-API structured errors should cross the JS<->Rust boundary by prefix-encoding `{ group, code, message, metadata }` into `napi::Error.reason`, then normalizing that prefix back into a `RivetError` on the other side.
- `#[napi(object)]` bridge payloads should stay plain-data only; if TypeScript needs to cancel native work, use primitives or JS-side polling instead of trying to pass a `#[napi]` class instance through an object field.
- For non-idempotent native waits like `queue.enqueueAndWait()`, bridge JS `AbortSignal` through a standalone native `CancellationToken`; timeout-slicing is only safe for receive-style polling calls like `waitForNames()`.
- Native queue receive waits should observe the actor abort token, but `enqueue_and_wait` completion waits must ignore actor abort and rely on the tracked user task for shutdown cancellation.
- Core queue receive waits need the `ActorContext`-owned abort `CancellationToken` wired into `Queue::new(...)` and cancelled from `mark_destroy_requested()`; external JS cancel tokens alone will not make `c.queue.next()` abort during destroy.

### RivetKit Package Resolutions
- The root `/package.json` contains `resolutions` that map RivetKit packages to local workspace versions:

```json
{
  "resolutions": {
    "rivetkit": "workspace:*",
    "@rivetkit/react": "workspace:*",
    "@rivetkit/workflow-engine": "workspace:*",
    // ... other @rivetkit/* packages
  }
}
```

- Use `*` as the dependency version when adding RivetKit packages to `/examples/`, because root resolutions map them to local workspace packages:

```json
{
  "dependencies": {
    "rivetkit": "*",
    "@rivetkit/react": "*"
  }
}
```

- Add new internal `@rivetkit/*` packages to root `resolutions` with `"workspace:*"` if missing, and prefer re-exporting internal packages (for example `@rivetkit/workflow-engine`) from `rivetkit` subpaths like `rivetkit/workflow` instead of direct dependencies.

### Dynamic Import Pattern
- For runtime-only dependencies, use dynamic loading so bundlers do not eagerly include them.
- Build the module specifier from string parts (for example with `["pkg", "name"].join("-")` or `["@scope", "pkg"].join("/")`) instead of a single string literal.
- Prefer this pattern for modules like `@rivetkit/rivetkit-napi/wrapper`, `sandboxed-node`, and `isolated-vm`.
- The TypeScript registry's native envoy path should dynamically load `@rivetkit/rivetkit-napi` and `@rivetkit/engine-cli` so browser and serverless bundles do not eagerly pull native-only modules.
- Native actor runner settings in `rivetkit-typescript/packages/rivetkit/src/registry/native.ts` should come from `definition.config.options`, not top-level actor config fields.
- If loading by resolved file path, resolve first and then import via `pathToFileURL(...).href`.

### Fail-By-Default Runtime Behavior
- Avoid silent no-ops for required runtime behavior.
- In `rivetkit-core` `ActorTask::run`, bind inbox `recv()` calls as raw `Option`s and log the closed channel before terminating; `Some(...) = recv()` plus `else => break` hides which inbox died.
- In `rivetkit-typescript/packages/rivetkit/src/common/utils.ts::deconstructError`, only passthrough canonical structured errors (`instanceof RivetError` or tagged `__type: "RivetError"` with full fields); plain-object lookalikes must still be classified and sanitized.
- Do not use optional chaining for required lifecycle and bridge operations (for example sleep, destroy, alarm dispatch, ack, and websocket dispatch paths).
- If a capability is required, validate it and throw an explicit error with actionable context instead of returning early.
- Optional chaining is acceptable only for best-effort diagnostics and cleanup paths (for example logging hooks and dispose/release cleanup).
- Keep scaffolded `rivetkit-core` wrappers `Default`-constructible, but return explicit configuration errors until a real `EnvoyHandle` is wired in.
- Keep foreign-runtime-only `ActorContext` helpers present on the public surface even before NAPI or V8 wires them, and make them fail with explicit configuration errors instead of silently disappearing.
- `rivetkit-core` boxed callback APIs should use `futures::future::BoxFuture<'static, ...>` plus the shared `actor::callbacks::Request` and `Response` wrappers so config and HTTP parsing helpers stay in core for future runtimes.
- `rivetkit-core` actor persistence should keep the BARE snapshot at the single-byte KV key `[1]` so the Rust runtime matches the TypeScript `KEYS.PERSIST_DATA` layout.
- `rivetkit-core` receive-loop persistence should route deferred saves through `ActorContext::request_save(...)` + `ActorEvent::SerializeState { reason: Save, .. }`, while shutdown adapters persist explicitly with `ActorContext::save_state(Vec<StateDelta>)` because `Sleep`/`Destroy` replies are unit-only and direct durability must still clear pending save-request flags after a successful write.
- `rivetkit-core` live inspector state for receive-loop actors now rides `ActorContext::inspector_attach()` / `inspector_detach()` / `subscribe_inspector()`, while `ActorTask` debounces `SerializeState { reason: Inspector, .. }` off request-save hooks; runtime inspector websocket handlers should stream the overlay broadcast instead of trusting `InspectorSignal::StateUpdated` for fresh bytes.
- `rivetkit-core` receive-loop `ActorEvent::Action` dispatch should use `conn: None` for alarm-originated work and `Some(ConnHandle)` for real client connections; do not synthesize placeholder connections for scheduled actions.
- `rivetkit-core` hibernatable websocket connections should persist each connection under KV key prefix `[2] + conn_id` using the TypeScript v4 BARE field order so Rust and TypeScript actors can restore the same connection payloads.
- `rivetkit-core` queue persistence should keep metadata at KV key `[5, 1, 1]` and messages under `[5, 1, 2] + u64be(id)` so FIFO prefix scans match the TypeScript runtime layout.
- `rivetkit-core` actor, connection, and queue persisted payloads should use the vbare-compatible 2-byte little-endian embedded version prefix before the BARE body, matching the TypeScript `serializeWithEmbeddedVersion(...)` format.
- `rivetkit-core` cross-cutting inspector hooks should stay anchored on `ActorContext`, with queue-specific callbacks carrying the current size and connection updates reading the manager count so unconfigured inspectors stay cheap no-ops.
- `rivetkit-core` schedule mutations should update `ActorState` through a single helper, then immediately kick `save_state(immediate = true)` and resync the envoy alarm to the earliest event.
- `rivetkit-core` state mutations from inside `on_state_change` callbacks should fail with `actor/state_mutation_reentrant`; use vars or another non-state side channel for callback-run counters.
- `rivetkit-core` HTTP and WebSocket staging helpers should keep transport failures at the boundary by turning `on_request` errors into HTTP 500 responses and `on_websocket` errors into logged 1011 closes, while `ConnHandle` and `WebSocket` wrappers surface explicit configuration errors through internal `try_*` helpers.
- `rivetkit-core` bulk transport disconnect helpers should sweep every matching connection, remove the successful disconnects, update connection/sleep bookkeeping, and only then aggregate any per-connection failures into the returned error.
- `rivetkit-core` registry startup should build runtime-backed `ActorContext`s with `ActorContext::new_runtime(...)` so state, queue, and connection managers inherit the actor config before lifecycle startup runs.
- Raw `onRequest` HTTP fetches should bypass `maxIncomingMessageSize` / `maxOutgoingMessageSize`; keep those message-size guards on `/action/*` and `/queue/*` HTTP message routes in `rivetkit-typescript/packages/rivetkit/src/registry/native.ts`, not generic `RegistryDispatcher::handle_fetch`.
- `rivetkit-core` sleep readiness should stay centralized in `SleepController`, with queue waits, scheduled internal work, disconnect callbacks, and websocket callbacks reporting activity through `ActorContext` hooks so the idle timer stays accurate.
- `rivetkit-core` startup should load `PersistedActor` into `ActorContext` before factory creation, persist `has_initialized` immediately, set `ready` before the driver hook, and only set `started` after that hook completes.
- `rivetkit-core` startup should resync persisted alarms and restore hibernatable connections before `ready`, then reset the sleep timer, spawn `run` in a detached panic-catching task, and drain overdue scheduled events after `started`.
- `rivetkit-core` sleep shutdown should wait for the tracked `run` task, poll `SleepController` for the idle window and shutdown-task drains, persist hibernatable connections before disconnecting non-hibernatable ones, and finish with an immediate state save.
- `rivetkit-core` sleep shutdown is two-phase now: `SleepGrace` fires `onSleep` immediately and keeps dispatch/save timers live, while only `SleepFinalize` gates dispatch, suspends alarms, and runs teardown.
- Process-global `rivetkit-core` `ActorTask` test hooks (`install_shutdown_cleanup_hook`, lifecycle-event/reply hooks) must be actor-scoped and serialized in tests or parallel `cargo test` runs will cross-wire unrelated actors.
- `rivetkit-core` destroy shutdown should skip the idle-window wait, use `on_destroy_timeout` independently from the shutdown grace period, disconnect every connection, and finish with the same immediate state save and SQLite cleanup path.
- `rivetkit-core` stop shutdown should finish persistence in this order: immediate state save, pending state write wait, alarm write wait, SQLite cleanup, then driver alarm cancellation.
- `rivetkit-core` `ActorConfig` uses `sleep_grace_period_overridden` to distinguish an explicit `sleep_grace_period` from legacy `on_sleep_timeout + wait_until_timeout` fallback behavior.
- `envoy-client` graceful actor teardown should flow through `EnvoyCallbacks::on_actor_stop_with_completion`; the default implementation preserves the old immediate `on_actor_stop` behavior by auto-completing the stop handle after the callback returns.
- `engine/sdks/rust/envoy-client` sync `EnvoyHandle` lookups for live actor state should read the shared `SharedContext.actors` mirror keyed by actor id/generation; blocking back through the envoy task can panic on current-thread Tokio runtimes.
- `rivetkit` typed `Ctx<A>` should stay a stateless wrapper over `rivetkit-core::ActorContext`: actor state lives in the user receive loop, there is no typed vars field, and CBOR encode/decode stays at wrapper method boundaries like `broadcast` and `ConnCtx`.
- `rivetkit` typed `Start<A>` wrappers must rehydrate each `ActorStart.hibernated` state blob back onto the `ConnHandle` before exposing `ConnCtx`, or `conn.state()` stops matching the wake snapshot.

### Rust Dependencies
- New crates under `rivetkit-rust/packages/` that should inherit repo-wide workspace deps must set `[package] workspace = "../../../"` and be added to the root `/Cargo.toml` workspace members.
- The high-level `rivetkit` crate should stay a thin typed wrapper over `rivetkit-core` and re-export shared transport/config types instead of redefining them.
- When `rivetkit` needs ergonomic helpers on a `rivetkit-core` type it re-exports, prefer an extension trait plus `prelude` re-export instead of wrapping and replacing the core type.

## Documentation

- If you need to look at the documentation for a package, visit `https://docs.rs/{package-name}`. For example, serde docs live at https://docs.rs/serde/
- When adding new docs pages, update `website/src/sitemap/mod.ts` so the page appears in the sidebar.
- When changing actor/runtime limits or behavior that affects documented limits (for example KV, queue, SQLite, WebSocket, HTTP, or timeouts), update `website/src/content/docs/actors/limits.mdx` in the same change.

## Code Blocks in Docs

- All TypeScript code blocks in docs are typechecked during the website build. They must be valid, compilable TypeScript.
- Use `<CodeGroup workspace>` only when showing multiple related files together (e.g., `actors.ts` + `client.ts`). For a single file, use a standalone fenced code block.
- Code blocks are extracted and typechecked via `website/src/integrations/typecheck-code-blocks.ts`. Add `@nocheck` to the code fence to skip typechecking for a block.

## Content Frontmatter

### Docs (`website/src/content/docs/**/*.mdx`)

- Required frontmatter fields:

- `title` (string)
- `description` (string)
- `skill` (boolean)

### Blog + Changelog (`website/src/content/posts/**/page.mdx`)

- Required frontmatter fields:

- `title` (string)
- `description` (string)
- `author` (enum: `nathan-flurry`, `nicholas-kissel`, `forest-anderson`)
- `published` (date string)
- `category` (enum: `changelog`, `monthly-update`, `launch-week`, `technical`, `guide`, `frogs`)

- Optional frontmatter fields:

- `keywords` (string array)

## Examples

- All example READMEs in `/examples/` should follow the format defined in `.claude/resources/EXAMPLE_TEMPLATE.md`.

## Agent Working Directory

All agent working files live in `.agent/` at the repo root.

- **Specs**: `.agent/specs/` — design specs and interface definitions for planned work.
- **Research**: `.agent/research/` — research documents on external systems, prior art, and design analysis.
- **Todo**: `.agent/todo/*.md` — deferred work items with context on what needs to be done and why.
- **Notes**: `.agent/notes/` — general notes and tracking.

When the user asks to track something in a note, store it in `.agent/notes/` by default. When something is identified as "do later", add it to `.agent/todo/`. Design documents and interface specs go in `.agent/specs/`.

## RivetKit Layer Architecture

- **Engine** (`packages/core/engine/`, includes Pegboard + Pegboard Envoy) — Orchestration. Manages actor lifecycle, routing, KV, SQLite, alarms. In local dev, the engine is spawned alongside RivetKit.
- **envoy-client** (`engine/sdks/rust/envoy-client/`) — Wire protocol between actors and the engine. BARE serialization, WebSocket transport, KV request/response matching, SQLite protocol dispatch, tunnel routing.
- **rivetkit-core** (`rivetkit-rust/packages/rivetkit-core/`) — Core RivetKit logic in Rust, language-agnostic. Lifecycle state machine, sleep logic, shutdown sequencing, state persistence, action dispatch, event broadcast, queue management, schedule system, inspector, metrics. All callbacks are dynamic closures with opaque bytes. All load-bearing logic must live here. Config conversion helpers and HTTP request/response parsing for foreign runtimes belong here.
- **rivetkit (Rust)** (`rivetkit-rust/packages/rivetkit/`) — Rust-friendly typed API. `Actor` trait, `Ctx<A>`, `Registry` builder, CBOR serde at boundaries. Thin wrapper over rivetkit-core. No load-bearing logic.
- **rivetkit-napi** (`rivetkit-typescript/packages/rivetkit-napi/`) — NAPI bindings only. ThreadsafeFunction wrappers, JS object construction, Promise-to-Future conversion. No load-bearing logic. Must only translate between JS types and rivetkit-core types. Only consumed by `rivetkit-typescript/packages/rivetkit/`.
- **rivetkit (TypeScript)** (`rivetkit-typescript/packages/rivetkit/`) — TypeScript-friendly API. Calls into rivetkit-core via NAPI for lifecycle logic. Owns workflow engine, agent-os, and client library. Zod validation for user-provided schemas runs here.

### Layer constraints

- All actor lifecycle logic, state persistence, sleep/shutdown, action dispatch, event broadcast, queue management, schedule, inspector, and metrics must live in rivetkit-core. No lifecycle logic in TS or NAPI.
- rivetkit-napi must be pure bindings. If code would be duplicated by a future V8 runtime, it belongs in rivetkit-core instead.
- rivetkit-napi serves through `CoreRegistry` + `NapiActorFactory`; do not reintroduce the deleted `BridgeCallbacks` JSON-envelope envoy path or `startEnvoy*Js` exports.
- NAPI `ActorContext.sql()` returns `JsNativeDatabase` directly; do not reintroduce a standalone `SqliteDb` wrapper export.
- rivetkit (Rust) is a thin typed wrapper. If it does more than deserialize, delegate to core, and serialize, the logic should move to rivetkit-core.
- rivetkit (TypeScript) owns only: workflow engine, agent-os, client library, Zod schema validation for user-defined types, and actor definition types.
- Errors use universal `RivetError` (group/code/message/metadata) at all boundaries. No custom error classes in TS.
- CBOR serialization at all cross-language boundaries. JSON only for HTTP inspector endpoints.

### Monorepo orientation

- **Core Engine** (`packages/core/engine/`) — main orchestration service.
- **Workflow Engine** (`packages/common/gasoline/`) — multi-step operations with reliability + observability.
- **Pegboard** (`packages/core/pegboard/`) — actor/server lifecycle management.
- **Pegboard Envoy** (`engine/packages/pegboard-envoy/`) — active actor-to-engine bridge (successor to pegboard-runner).
- **Common packages** (`packages/common/`) — foundation utilities, DB pools, caching, metrics, logging, health, gasoline core.
- **Core packages** (`packages/core/`) — engine executable, pegboard orchestration, workflow workers.
- **Shared libraries** (`shared/{language}/{package}/`) — shared between engine and rivetkit (e.g., `shared/typescript/virtual-websocket/`).
- **Databases**: UniversalDB (distributed state), ClickHouse (analytics/time-series). Connection pooling via `packages/common/pools/`.
- Services communicate via NATS with service discovery.

### Deprecated paths

- `engine/packages/pegboard-runner/`, `engine/sdks/typescript/runner`, `rivetkit-typescript/packages/engine-runner/`, and associated runner workflows are deprecated. All new actor hosting work targets `engine/packages/pegboard-envoy/` exclusively. Do not add features to or fix bugs in the deprecated runner path.

### Engine runner parity

- Keep `engine/sdks/typescript/runner` and `engine/sdks/rust/engine-runner` at feature parity.
- Any behavior, protocol handling, or test coverage added to one runner should be mirrored in the other runner in the same change whenever possible.
- When parity cannot be completed in the same change, explicitly document the gap and add a follow-up task.

## Trust Boundaries

- Treat `client <-> engine` as untrusted.
- Treat `envoy <-> pegboard-envoy` as untrusted.
- Treat traffic inside the engine over `nats`, `fdb`, and other internal backends as trusted.
- Treat `gateway`, `api`, `pegboard-envoy`, `nats`, `fdb`, and similar engine-internal services as one trusted internal boundary once traffic is inside the engine.
- Validate and authorize all client-originated data at the engine edge before it reaches trusted internal systems.
- Validate and authorize all envoy-originated data at `pegboard-envoy` before it reaches trusted internal systems.

## WebSocket Rejection

- Reject WebSocket connections (auth failures, routing errors, any rejection reason) by accepting the upgrade and sending a close frame with a meaningful close code and `<group>.<code>` reason. Do not reject with an HTTP status before the upgrade. Browser clients cannot surface HTTP status on a failed upgrade; they only see `CloseEvent.code` / `.reason`, so pre-upgrade rejection leaves them with no diagnostic. Use close code `1008` (policy violation) for auth failures, matching the `inspector.unauthorized` convention.

## Fail-By-Default Runtime

- Avoid silent no-ops for required runtime behavior. If a capability is required, validate it and throw an explicit error with actionable context instead of returning early.
- Do not use optional chaining for required lifecycle and bridge operations (for example sleep, destroy, alarm dispatch, ack, and websocket dispatch paths).
- Optional chaining is acceptable only for best-effort diagnostics and cleanup paths (for example logging hooks and dispose/release cleanup).
- Keep scaffolded `rivetkit-core` wrappers `Default`-constructible, but return explicit configuration errors until a real `EnvoyHandle` is wired in.
- Keep foreign-runtime-only `ActorContext` helpers present on the public surface even before NAPI or V8 wires them. Make them fail with explicit configuration errors instead of silently disappearing.
- In `rivetkit-core` `ActorTask::run`, bind inbox `recv()` calls as raw `Option`s and log the closed channel before terminating. `Some(...) = recv()` plus `else => break` hides which inbox died.
- In `rivetkit-typescript/packages/rivetkit/src/common/utils.ts::deconstructError`, only passthrough canonical structured errors (`instanceof RivetError` or tagged `__type: "RivetError"` with full fields). Plain-object lookalikes must still be classified and sanitized.
- Actor-owned lifecycle / dispatch / lifecycle-event inbox producers use `try_reserve` helpers and return `actor.overloaded`. Do not await bounded `mpsc::Sender::send`.

## Performance

- Never use `Mutex<HashMap<...>>` or `RwLock<HashMap<...>>`. Use `scc::HashMap` (preferred), `moka::Cache` (for TTL/bounded), or `DashMap` for concurrent maps.
- Use `scc::HashSet` instead of `Mutex<HashSet<...>>` for concurrent sets.
- `scc` async methods do not hold locks across `.await` points. Use `entry_async` for atomic read-then-write.
- Never poll a shared-state counter with `loop { if ready; sleep(Nms).await; }`. Pair the counter with a `tokio::sync::Notify` (or `watch::channel`) that every decrement-to-zero site pings, and wait with `AsyncCounter::wait_zero(deadline)` or an equivalent `notify.notified()` + re-check guard that arms the permit before the check.
- Every shared counter with an awaiter must have a paired `Notify`, `watch`, or permit. Waiters must arm the notification before re-checking the counter so decrement-to-zero cannot race past them.
- Reserve `tokio::time::sleep` for: per-call timeouts via `tokio::select!`, retry/reconnect backoff, deliberate debounce windows, or `sleep_until(deadline)` arms in an event-select loop. If it is inside a `loop { check; sleep }` body, it is polling and should be event-driven instead.
- Never add unexplained wall-clock defers like `sleep(1ms)` to decouple a spawn from its caller. Use `tokio::task::yield_now().await` or rely on the spawn itself.

## Async Rust Locks

- Async Rust code defaults to `tokio::sync::Mutex` / `tokio::sync::RwLock`. Do not use `std::sync::Mutex` / `std::sync::RwLock`.
- Use `parking_lot::Mutex` / `parking_lot::RwLock` only when sync is mandated by the call context: `Drop`, sync traits, FFI/SQLite VFS callbacks, or sync `&self` accessors.
- `rivetkit-napi` sync N-API methods, TSF callback slots, and test `MakeWriter` captures are forced-sync contexts. Use `parking_lot` there and keep guards out of awaits.
- `rivetkit-napi` test-only global serialization should use a real `parking_lot` guard instead of `AtomicBool` spin loops.
- If an external dependency's struct requires `std::sync::Mutex`, keep it at the construction boundary with an explicit forced-std-sync comment.
- Prefer async locks because sync guards can be silently held across `.await`, poisoning creates `.expect("lock poisoned")` boilerplate, and the tiny uncontended-lock win is dwarfed by actor I/O latency.

## Error Handling

- Custom error system at `packages/common/error/` using `#[derive(RivetError)]` on struct definitions. For the full derive example and conventions, see `.claude/reference/error-system.md`.
- Always return anyhow errors from failable functions. Do not glob-import from anyhow. Prefer `.context()` over the `anyhow!` macro.
- `rivetkit-core` should convert callback/action `anyhow::Error` values into transport-safe `group/code/message` payloads with `rivet_error::RivetError::extract` before returning them across runtime boundaries.
- `rivetkit-core` is the single source of truth for cross-boundary error sanitization. The TS bridge must NOT pre-wrap non-structured JS errors into a canonical `RivetError` before bridge-encoding. Pass raw `Error` values through the bridge as unstructured strings so core's `RivetError::extract` hits `build_internal` and produces the sanitized `INTERNAL_ERROR` payload. Only TS errors that never cross into core (HTTP router parsing, Hono middleware) should be sanitized by `common/utils.ts::deconstructError`. The dev-mode toggle that exposes raw messages lives in core (reads env at `build_internal`), not in the TS bridge.
- `envoy-client` actor-scoped HTTP fetch work should stay in a `JoinSet` plus an `Arc<AtomicUsize>` counter so sleep checks can read in-flight request count and shutdown can abort and join the tasks before sending `Stopped`.

## Logging

- Use tracing. Never use `eprintln!` or `println!` for logging in Rust code. Always use `tracing::info!`, `tracing::warn!`, `tracing::error!`, etc.
- Do not format parameters into the main message. Use structured fields: `tracing::info!(?x, "foo")` instead of `tracing::info!("foo {x}")`.
- Log messages should be lowercase unless mentioning specific code symbols. `tracing::info!("inserted UserRow")` instead of `tracing::info!("Inserted UserRow")`.
- `rivetkit-core` runtime logs should include `actor_id` and stable structured fields such as `reason`, `kind`, `delta_count`, byte counts, and timestamp fields instead of payload debug dumps.

## Testing

- **Never use `vi.mock`, `jest.mock`, or module-level mocking.** Write tests against real infrastructure (Docker containers, real databases, real filesystems). For LLM calls, use `@copilotkit/llmock` to run a mock LLM server. For protocol-level test doubles (e.g., ACP adapters), write hand-written scripts that run as real processes. `vi.fn()` for simple callback tracking is acceptable.
- Driver tests that wait for actor sleep must not poll actor actions while waiting; each action counts as activity and can reset the sleep deadline.
- **Never paper over flakes with retry loops or bumped waits.** When a test flakes, (1) root-cause the race, (2) write a deterministic repro using `vi.useFakeTimers()` or event-ordered `Promise` resolution, (3) fix the underlying ordering in core/napi/typescript, (4) delete any flake-workaround note. `vi.waitFor` is acceptable for legitimate "wait for an async event" coordination but never as a retry-until-success masking layer. Every `vi.waitFor` call must have a one-line comment explaining why polling rather than direct awaiting is necessary.
- In `rivetkit-typescript/packages/rivetkit/tests/`, put the `vi.waitFor(...)` justification on the immediately preceding `//` line. `pnpm run check:wait-for-comments` enforces the adjacent comment.
- **Rust tests live under `tests/`, not inline `#[cfg(test)] mod tests` in `src/`.** Move every inline test module in Rust crates to the crate's `tests/` directory. Exceptions must be justified (e.g., testing a private internal that can't be reached from an integration test).
- For running RivetKit tests, Vitest filter gotchas, the driver-test parity workflow, and Rust test layout rules, see `.claude/reference/testing.md`.

## Traces Package

- Keep `@rivetkit/traces` chunk writes under the 128 KiB actor KV value limit. Use 96 KiB chunks unless a multipart reader/writer replaces the single-value format.

## Naming + Data Conventions

- Data structures often include:
  - `id` (uuid)
  - `name` (machine-readable name, must be valid DNS subdomain, convention is using kebab case)
  - `description` (human-readable, if applicable)
- Use UUID (v4) for generating unique identifiers.
- Store dates as i64 epoch timestamps in milliseconds for precise time tracking.
- Timestamps use `*_at` naming with past-tense verbs. For example, `created_at`, `destroyed_at`.

## Code Style

- Hard tabs for Rust formatting (see `rustfmt.toml`).
- Follow existing patterns in neighboring files.
- Always check existing imports and dependencies before adding new ones.
- **Always add imports at the top of the file instead of inline within a function.**

### Comments

- Write comments as normal, complete sentences. Avoid fragmented structures with parentheticals and dashes like `// Spawn engine (if configured) - regardless of start kind`. Instead, write `// Spawn the engine if configured`. Especially avoid dashes (hyphens are OK).
- Do not use em dashes (—). Use periods to separate sentences instead.
- Documenting deltas is not important or useful. A developer who has never worked on the project will not gain extra information if you add a comment stating that something was removed or changed because they don't know what was there before. The only time you would be adding a comment for something NOT being there is if its unintuitive for why its not there in the first place.

## Documentation

- If you need to look at the documentation for a package, visit `https://docs.rs/{package-name}`. For example, serde docs live at `https://docs.rs/serde/`.
- When adding new docs pages, update `website/src/sitemap/mod.ts` so the page appears in the sidebar.
- For the full docs-sync table (limits, config, actor errors, statuses, k8s, landing, sandbox providers, inspector), see `.claude/reference/docs-sync.md`.

## CLAUDE.md conventions

- When adding entries to any CLAUDE.md file, keep them concise. Ideally a single bullet point or minimal bullet points. Do not write paragraphs.
- Only add design constraints, invariants, and non-obvious rules that shape how new code should be written. Do not add general trivia, current implementation wiring, KV-key layouts, module organization, API signatures, ephemeral migration state, or anything a reader can learn by reading the code. That content belongs in module doc-comments, `docs-internal/`, or `.claude/reference/`.
- When the user asks to update any `CLAUDE.md`, add one-line bullet points only, or add a new section containing one-line bullet points.
- Architectural internals and runtime wiring belong in `docs-internal/engine/`. Agent-procedural guides (test-harness gotchas, build troubleshooting, docs-sync tables) belong in `.claude/reference/`. Link them from the [Reference Docs](#reference-docs) index below instead of inlining.

## Reference Docs

Load these only when the task touches the topic.

### Architecture (`docs-internal/engine/`)

- **[rivetkit-core internals](docs-internal/engine/rivetkit-core-internals.md)** — KV-key layout, storage organization on `ActorContextInner`, startup/shutdown sequences, inspector attach plumbing, schedule dirty-flag, registry dispatch. Read before changing state persistence, lifecycle, or registry wiring.
- **[rivetkit-core state management](docs-internal/engine/rivetkit-core-state-management.md)** — `request_save` / `save_state` / `persist_state` / `set_state_initial` semantics. Keep in sync when changing state APIs.
- **[ActorTask dispatch](docs-internal/engine/actor-task-dispatch.md)** — `DispatchCommand::Action`/`Http`/`OpenWebSocket`, `UserTaskKind` children, `ActorTask` migration status. Read before changing actor task routing.
- **[Inspector protocol](docs-internal/engine/inspector-protocol.md)** — HTTP↔WebSocket mirroring rules, wire-version negotiation, `inspector.*_dropped` downgrades, workflow inspector inference. Read before touching inspector endpoints.
- **[NAPI bridge](docs-internal/engine/napi-bridge.md)** — TSF callback slots, `ActorContextShared` cache reset, `#[napi(object)]` payload rules, cancellation token bridging, error prefix encoding. Read before touching `rivetkit-napi`.
- **[BARE protocol crates](docs-internal/engine/bare-protocol-crates.md)** — vbare schema ordering, identity converters, `build.rs` TS codec generation pattern. Read before adding/changing protocol crates.
- **[SQLite VFS parity](docs-internal/engine/sqlite-vfs.md)** — native Rust VFS ↔ WASM TypeScript VFS 1:1 parity rule, v2 storage keys, chunk layout, delete/truncate strategy. Read before touching either VFS.
- **[TLS trust roots](docs-internal/engine/tls-trust-roots.md)** — rustls native+webpki union rationale, which clients use which backend.

### Agent procedural (`.claude/reference/`)

- **[Testing](.claude/reference/testing.md)** — running RivetKit tests, Vitest filter gotchas, driver-test parity workflow, Rust test layout.
- **[Build troubleshooting](.claude/reference/build-troubleshooting.md)** — DTS failures, NAPI rebuild, `JsActorConfig` field churn, tsup stale exports.
- **[Docs sync](.claude/reference/docs-sync.md)** — full table of "when you change X, update docs Y". Consult before finishing a change.
- **[Content frontmatter](.claude/reference/content-frontmatter.md)** — required frontmatter schemas for docs + blog/changelog.
- **[Examples + Vercel](.claude/reference/examples.md)** — example templates, Vercel mirror regen, common errors.
- **[RivetError system](.claude/reference/error-system.md)** — full derive example, artifact commit rule, anyhow usage.
- **[Dependencies](.claude/reference/dependencies.md)** — pnpm resolutions, Rust workspace deps, dynamic imports, version bumps, reqwest pool.
