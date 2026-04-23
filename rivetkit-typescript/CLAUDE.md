# rivetkit-typescript/CLAUDE.md

## Tree-Shaking Boundaries

- Do not import `@rivetkit/workflow-engine` outside the `rivetkit/workflow` entrypoint so it remains tree-shakeable.
- Keep SQLite runtime code on the native `@rivetkit/rivetkit-napi` path. Do not reintroduce WebAssembly SQLite or KV-backed VFS fallbacks.
- Importing `rivetkit/db` is the explicit opt-in for SQLite. Do not lazily load extra SQLite runtimes from that entrypoint.
- Core drivers must remain SQLite-agnostic. Any SQLite-specific wiring belongs behind the native database provider boundary.

## Native SQLite v2

- If `packages/rivetkit` still needs a BARE codec after schema-generator removal, vendor only the live generated modules under `src/common/bare/` and import them from source instead of `dist/schemas/**`.
- The v2 SQLite VFS must reconstruct full 4 KiB pages for partial `xRead` and `xWrite` callbacks because SQLite can issue sub-page header I/O even when commits stay page-based.
- Treat `head_txid` and `db_size_pages` as VFS-owned state. Read-side `get_pages(...)` responses may refresh `max_delta_bytes`, but commit responses plus local `xWrite` or `xTruncate` paths are the only things allowed to advance or shrink those fields.
- `open_database_from_envoy(...)` must dispatch on `sqliteSchemaVersion`, not on whether startup data happens to be present. Schema version `2` should fail closed if startup data is missing.
- When changing Rust under `packages/rivetkit-napi` or `packages/sqlite-native`, rebuild from `packages/rivetkit-napi` with `pnpm build:force` so the native `.node` artifact refreshes.
- Real `sqlite-native` tests that drive the v2 VFS through a direct `SqliteEngine` need a multithread Tokio runtime; `current_thread` is fine for mock transport tests but can stall real engine callbacks.
- Treat any sqlite v2 transport or commit error as fatal for that VFS instance: mark it dead, surface it through `take_last_kv_error()`, and rely on reopen plus takeover instead of trying to limp forward with dirty pages still buffered.
- Keep sqlite v2 fatal commit cleanup in `flush_dirty_pages` and `commit_atomic_write`; callback wrappers should only translate fence mismatches into SQLite I/O return codes.
- If the native SQLite layer exposes new introspection or metrics getters, forward them through `wrapJsNativeDatabase(...)` or actor inspector metrics will silently lose that data.

## Context Types Sync

- Keep the `*ContextOf` types exported from `packages/rivetkit/src/actor/contexts/index.ts` in sync with the two docs locations below when adding, removing, or renaming context types.

- `website/src/content/docs/actors/types.mdx` — public docs page
- `website/src/content/docs/actors/index.mdx` — crash course (Context Types section)

## Gateway Targets

For client-facing gateway operations, use the shared `GatewayTarget` type from `packages/rivetkit/src/engine-client/driver.ts` instead of ad hoc `string | ActorQuery` unions. The engine control client should preserve direct actor ID behavior and resolve `ActorQuery` targets inside the client implementation so higher-level client flows can widen their target type without duplicating query-resolution logic.

Actor-connect protocol `actionId` values are nullable, and `0` is a valid action ID. Treat only `null` as a connection-level error.

Query-backed remote gateway URLs use `rvt-*` query parameters: `/gateway/{name}/{path}?rvt-namespace=...&rvt-method=...&rvt-key=...`. The actor name is a clean path segment, and all routing params are standard query parameters with the `rvt-` prefix. The known `rvt-*` params are: `rvt-namespace`, `rvt-method`, `rvt-runner`, `rvt-key`, `rvt-input`, `rvt-region`, `rvt-crash-policy`, `rvt-token`. `rvt-runner` is required for `getOrCreate` and disallowed for `get`. For multi-component keys, use a single comma-separated `rvt-key` param (e.g. `rvt-key=tenant,room`). Use `URLSearchParams` to build and parse query strings.

Keep `buildGatewayUrl()` query-backed for `get()` and `getOrCreate()` handles instead of pre-resolving to an actor ID. Local `getGatewayUrl()` flows should exercise the shared `actorGateway` query param parser on the served runtime router path, while direct actor ID targets still use `/gateway/{actorId}`.

When parsing query gateway paths in `packages/rivetkit/src/actor-gateway/gateway.ts` or in parity implementations, detect query paths by checking if any query parameter starts with `rvt-`. Use `URLSearchParams` to parse query params. Partition into `rvt-*` params and actor params. Reject raw `@token` syntax, unknown `rvt-*` params, and duplicate scalar `rvt-*` params. Strip all `rvt-*` params from the query string before forwarding to the actor by reconstructing the query string from only the actor params.

Once a query path has been parsed in `packages/rivetkit/src/actor-gateway/gateway.ts`, resolve it to an actor ID inside the shared path-based HTTP and WebSocket gateway helpers before calling `proxyRequest` or `proxyWebSocket`. After resolution, reuse the existing direct-ID proxy flow and preserve the original remaining path with `rvt-*` params stripped.

When adding or validating query input payloads in `packages/rivetkit/src/engine-client/actor-websocket-client.ts`, enforce `ClientConfig.maxInputSize` against the raw CBOR byte length before base64url encoding. This keeps the limit aligned with the actual serialized payload instead of the encoded URL expansion.

For `ClientRaw.get()` and `ClientRaw.getOrCreate()` flows, do not cache a resolved actor ID on `ActorResolutionState`. Key-based handles and connections should resolve fresh for each operation so they do not stay pinned to an older actor selection after a destroy or recreate.

For gateway-facing client helpers in `packages/rivetkit/src/client`, derive the `EngineControlClient` target from `getGatewayTarget()` instead of calling `resolveActorId()` up front. `get()` and `getOrCreate()` handles must pass their `ActorQuery` through to `sendRequest`, `openWebSocket`, and `buildGatewayUrl` so each request and reconnect re-resolves at the gateway. Only `getForId()` and create-backed handles should collapse to a plain actor ID target.

## Raw KV Limits

- Always enforce engine limits when working with raw actor KV.

- Max key size: 2048 bytes.
- Max batch payload size (`kv put`): 976 KiB total across keys + values.
- Max entries per batch (`kv put`): 128 key-value pairs.
- Max total actor KV storage: 10 GiB.

- Design raw KV operations to handle these constraints, and split operations into multiple requests if a per-request limit can be exceeded.
- Treat the total actor KV storage limit (10 GiB) as a hard limit, and fail closed with explicit errors instead of swallowing, truncating, or ignoring KV write failures.
- Update `website/src/content/docs/actors/limits.mdx` in the same change when KV, queue, workflow persistence, SQLite-over-KV, or any limit-related actor behavior changes.

## Startup Logging

Every discrete phase of actor startup must have a corresponding debug log. Use the prefix `perf internal:` for framework/infrastructure phases and `perf user:` for user-code callbacks. For example:

```
DEBUG perf internal: loadStateMs          durationMs=...
DEBUG perf internal: initQueueMs          durationMs=...
DEBUG perf user: onCreateMs               durationMs=...
DEBUG perf user: dbMigrateMs              durationMs=...
```

The log name matches the key in `ActorMetrics.startup`. Internal phases use `perf internal:`, user-code callbacks use `perf user:`. This convention keeps startup logs greppable and makes it easy to separate framework overhead from user-code time. When adding a new startup phase, always add a corresponding log with the appropriate prefix and update the `#userStartupKeys` set in `ActorInstance` if the phase runs user code.

## NAPI Receive Loop

- Keep adapter-owned long-lived task handles (for example the NAPI `run` handler) in `packages/rivetkit-napi/src/napi_actor_events.rs` and expose only sync restart hooks through shared `ActorContext` state; JS-facing restart methods must not depend on async locks.
- Graceful adapter drains in `packages/rivetkit-napi/src/napi_actor_events.rs` should use `while let Some(...) = tasks.join_next().await`; `JoinSet::shutdown()` aborts in-flight work and breaks Sleep/Destroy ordering.
- `Sleep` and `Destroy` must set the shared adapter `end_reason` on both success and error replies; otherwise the outer receive loop keeps consuming queued events after shutdown has already failed.
- On this branch, the native TS actor/conn persistence glue still lives in `packages/rivetkit/src/registry/native.ts`; PRD references to split `state-manager.ts` or `connection-manager.ts` files may be stale, so land equivalent behavior in `registry/native.ts` unless those modules reappear first.
- Public TS actor `onWake` maps to the native callback bag's `onWake`; `onBeforeActorStart` is an internal driver/NAPI startup hook, not public actor config.
- Static actor `state` values in `packages/rivetkit/src/registry/native.ts` must be `structuredClone(...)`d per actor instance; reusing the literal leaks mutations across different keyed actors.
- Every `NativeConnAdapter` construction path in `packages/rivetkit/src/registry/native.ts` must keep the `CONN_STATE_MANAGER_SYMBOL` hookup; hibernatable conn mutations rely on core `ConnHandle::set_state` dirty tracking to request persistence.
- Durable native actor saves in `packages/rivetkit/src/registry/native.ts` must use `ctx.requestSaveAndWait({ immediate: true })`; state bytes are collected only through the `serializeState` callback.
- Reply-bearing TSF dispatches in `packages/rivetkit-napi/src/napi_actor_events.rs` must wrap the callback future in `with_timeout(...)` via a shared timed-spawn helper; raw `spawn_reply(...)` on HTTP or workflow callbacks can leak stuck JS promises until shutdown.

## Sleep Shutdown

- Sleep shutdown should wait for in-flight HTTP action work and pending disconnect callbacks before `onSleep`, but should not block on open hibernatable connections alone because existing connection actions may still complete during the graceful shutdown window.

## Drizzle Compatibility Testing

To test rivetkit's drizzle integration against multiple drizzle-orm versions:

```bash
cd rivetkit-typescript/packages/rivetkit
./scripts/test-drizzle-compat.sh                   # test all default versions
./scripts/test-drizzle-compat.sh 0.44.2 0.45.1     # test specific versions
```

The script installs each drizzle-orm version, runs the drizzle driver tests, and reports pass/fail per version. It restores the original package.json and lockfile on exit. Update the `DEFAULT_VERSIONS` array in the script and `SUPPORTED_DRIZZLE_RANGE` in `packages/rivetkit/src/db/drizzle/mod.ts` when adding support for new drizzle releases.

## Cloudflare Workers Compatibility

Cloudflare Workers forbid `setTimeout`, `fetch`, `connect`, and other async I/O in global scope (outside a request handler). The `Registry` constructor runs in global scope, so it must never call these APIs unconditionally. Any deferred work (e.g., prestarting the runtime) must be gated behind a synchronous config check before scheduling a timer. See `packages/rivetkit/src/registry/index.ts` for the pattern: the outer `if` guards `setTimeout`, and the inner `if` re-checks after the tick to pick up late config mutations.

## Workflow Context Actor Access Guards

- Guard all side-effectful `#runCtx` access in `ActorWorkflowContext` (`packages/rivetkit/src/workflow/context.ts`) with `#ensureActorAccess`; only read-only properties (for example `actorId` and `log`) are exempt.
- Apply `#ensureActorAccess` to any new workflow-context method or property that delegates to `#runCtx` and has side effects.

## Dynamic Actors Architecture Doc

- Reference `docs-internal/rivetkit-typescript/DYNAMIC_ACTORS_ARCHITECTURE.md` when working on dynamic actor behavior, bridge contracts, isolate lifecycle, or runtime sandbox wiring.
- Keep `docs-internal/rivetkit-typescript/DYNAMIC_ACTORS_ARCHITECTURE.md` up to date in the same change whenever dynamic actor architecture, lifecycle, bridge payloads, security behavior, or temporary compatibility paths change.
