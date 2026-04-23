# Restoring Serverless Support (`.handler()` / `.serve()`)

Status: proposal
Supersedes: `/home/nathan/r5/.agent/specs/handler-serve-restoration.md` (delete on approval)
Scope: `rivetkit-rust/packages/rivetkit-core`, `rivetkit-typescript/packages/rivetkit-napi`, `rivetkit-typescript/packages/rivetkit`

## Why the earlier spec was wrong

The prior `handler-serve-restoration.md` treated `.handler()` as a user-traffic gateway and recommended a TS reverse proxy to the engine. Reading the actual `feat/sqlite-vfs-v2` code shows `.handler()` is not a user-traffic gateway. It is the **serverless runner endpoint** that the engine calls to wake a runner inside a serverless function's request lifespan. The old surface is a four-route fixed router:

- `GET  /api/rivet/`
- `GET  /api/rivet/health`
- `GET  /api/rivet/metadata`
- `POST /api/rivet/start` — the meaningful one

User actor traffic never flows through `.handler()`. It flows through the engine gateway, which decides to invoke the serverless function by POSTing to `/start`.

## The old `POST /start` flow (reference branch)

`feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/drivers/engine/actor-driver.ts:788`:

```ts
async serverlessHandleStart(c: HonoContext): Promise<Response> {
  let payload = await c.req.arrayBuffer();
  return streamSSE(c, async (stream) => {
    await this.#envoyStarted.promise;
    if (this.#isShuttingDown) return;
    await this.#envoy.startServerlessActor(payload);
    while (true) {
      if (this.#isEnvoyStopped) break;
      if (stream.closed || stream.aborted) break;
      await stream.writeSSE({ event: "ping", data: "" });
      await stream.sleep(ENVOY_SSE_PING_INTERVAL);
    }
  });
}
```

Router shim (`feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/serverless/router.ts`):

```ts
router.post("/start", async (c) => {
  const { endpoint, token, poolName, namespace } = parseHeaders(c);
  if (config.endpoint) {
    if (!endpointsMatch(endpoint, config.endpoint)) throw new EndpointMismatch(...);
    if (namespace !== config.namespace) throw new NamespaceMismatch(...);
  }
  const actorDriver = new EngineActorDriver(runnerConfig, engineClient, client);
  return await actorDriver.serverlessHandleStart(c);
});
```

The request body is the engine's binary envoy-protocol startup payload. The SSE stream is kept alive with pings until either the envoy stops or the engine aborts the HTTP request. The stream lifetime **is** the serverless function's lifetime.

## Verified parity requirements

These are the source-of-truth behaviors from `feat/sqlite-vfs-v2` that the restoration must preserve unless a security fix explicitly overrides them:

- `POST /start` reads `x-rivet-endpoint`, optional `x-rivet-token`, `x-rivet-pool-name`, and `x-rivet-namespace-name`. The namespace header is **not** `x-rivet-namespace`.
- Missing start headers use the same messages as `ServerlessStartHeadersSchema`: `x-rivet-endpoint header is required`, `x-rivet-pool-name header is required`, and `x-rivet-namespace-name header is required`. These should return structured `request/invalid` errors.
- `/metadata` returns `runtime`, TS package `version`, `envoy.kind`, `envoy.version`, `envoyProtocolVersion`, `actorNames`, `clientEndpoint`, `clientNamespace`, and `clientToken`. The values for package version, public client fields, and actor metadata are TS-derived inputs that must be passed into core or handled in TS.
- Parsed config resolves `endpoint`, `namespace`, and `token` from URL auth syntax and env vars. `serverless.publicEndpoint` also supports URL auth syntax and feeds `publicEndpoint`, `publicNamespace`, and `publicToken`.
- `Runtime.startServerless()` calls `configureServerlessPool(config)` when `configurePool` is set. That flow calls `getDatacenters` and `updateRunnerConfig` with the serverless URL, headers, request lifespan default `900`, metadata poll interval default `1000`, `max_runners: 100_000`, `drain_on_version_upgrade: true`, and custom metadata.
- `ENVOY_SSE_PING_INTERVAL` is `1000` ms. The SSE ping frame is `event: ping` with empty `data`.
- The old abort handler only logs and exits the SSE loop when `stream.closed` or `stream.aborted` is observed. Abort must not be documented as shutting down the shared envoy.

## The Rust side already has the core piece

`engine/sdks/rust/envoy-client/src/handle.rs:484` already exposes:

```rust
pub async fn start_serverless_actor(&self, payload: &[u8]) -> anyhow::Result<()>
```

It validates the protocol version, decodes as `ToEnvoy`, asserts exactly one `CommandStartActor`, waits for envoy readiness, and injects into `envoy_tx`. This is the hard part. What is missing is:

1. A `rivetkit-core` routing layer that turns an HTTP request into a dispatch decision over the 4 routes.
2. A NAPI bridge that carries `Request` in and a streaming SSE `Response` out.
3. TS-side `Registry.handler()` / `.serve()` that wrap the NAPI call.
4. TS-side config plumbing for metadata, public client fields, `configurePool`, and body-size limits.

## Architecture

Push the request into Rust. The routing logic, endpoint validation, SSE pump, and envoy coordination all live in `rivetkit-core`. TypeScript is thin glue.

### Layer split

- **`rivetkit-core` (Rust)** gains `serverless::handle_request(req: ServerlessRequest) -> ServerlessResponseStream`. Owns URL routing for `/api/rivet/*`, header parsing, endpoint/namespace validation, auth validation, envoy startup, `envoy.start_serverless_actor(payload)`, and the SSE ping loop. Takes config-provided `base_path`, metadata response fields, configured endpoint/namespace/token, and body-size limits.
- **`rivetkit-napi`** exposes one request method with positional arguments: `CoreRegistry.handleServerlessRequest(req, onStreamEvent, cancelToken, serveConfig)`. The stream callback is a single `ThreadsafeFunction<StreamEvent>` tagged union so chunk and end ordering is FIFO on one TSF.
- **`rivetkit-typescript`** registry `handler(req, opts?)` method. It enforces `maxStartPayloadBytes` before NAPI marshal, wires `Request.signal` to the native `CancellationToken`, feeds a `ReadableStream` from the single stream-event callback, and calls `configureServerlessPool` once when `configurePool` is set.

### Why not proxy to the engine instead

The engine is the *caller* of `/start`, not its target. A TS proxy would mean the runner function proxies the engine's call back to the engine, which is a pointless round trip. The runner needs to actually invoke `envoy.start_serverless_actor(payload)` on a local in-process envoy.

### Why not keep the routing in TS

- `/` and `/health` are trivial. `/metadata` is not trivial because package version, public client fields, and actor metadata originate in TypeScript. Prefer core-owned routing with those fields passed in `ServeConfig`; keep only the metadata assembly in TS if passing all fields cleanly becomes uglier than the route is worth.
- Endpoint/namespace validation and `endpointsMatch` URL normalization already have incentive to move to core (they inform the envoy boot decision). Putting the thin routing layer in the same place is the natural home.
- A future V8-only runtime binding would have to reimplement the TS-side routing. Keeping it in core means V8 reuses the same logic via different bindings.

## NAPI bridge design

### Single entrypoint

```typescript
// rivetkit-typescript/packages/rivetkit-napi/ (pseudo d.ts)
interface CoreRegistry {
  handleServerlessRequest(
    req: {
      method: string,
      url: string,
      headers: Record<string, string>,
      body: Buffer,     // null/empty Buffer if none
    },
    onStreamEvent: (event:
      | { kind: "chunk"; chunk: Buffer }
      | { kind: "end"; error?: { group: string; code: string; message: string } }
    ) => Promise<void>,
    cancelToken: CancellationToken,
    serveConfig: JsServeConfig,
  ): {
    status: number,
    headers: Record<string, string>,
  };
}
```

The method returns status+headers after core validates the route, start headers, auth token, endpoint/namespace gates, and body size. The response body is delivered asynchronously through `onStreamEvent`.

### Streaming direction (Rust -> JS)

`onStreamEvent` is a single `ThreadsafeFunction<StreamEvent>` exposed to core through an `ActorContext`-style plumbing struct. Core writes SSE chunks (`event: ping\ndata:\n\n`) via `kind: "chunk"` and closes via `kind: "end"`. Use `call_async::<Promise<()>>` so TypeScript can apply `ReadableStream` backpressure and so final `end` cannot be dropped behind a full libuv queue. Post-end chunks are ignored in core before they cross NAPI.

### Body direction (JS -> Rust)

Request body is a single `Buffer`. `/start` payloads are bounded by `serverless.maxStartPayloadBytes` with a default of `1_048_576` bytes before NAPI marshal. Return `413` on overflow. No per-chunk streaming for inbound bodies in v1.

### Abort propagation

TS forwards `req.signal` into a native `CancellationToken`. Core observes the token while waiting for envoy readiness and while running the SSE ping loop. Cancellation ends the response stream for that request; it must not be described as shutting down a shared envoy. If the chosen lifecycle is per-request envoy, request cancellation may drop that request-owned handle after the stream closes.

### Error handling

Non-streaming errors (bad headers, auth failure, endpoint mismatch, namespace mismatch, body too large) return `status >= 400` with a structured-error JSON body. Post-header errors from envoy readiness or `start_serverless_actor` route through `kind: "end", error`, because HTTP status is already committed. Both paths use `RivetError::extract`; unstructured errors sanitize through `build_internal`.

## TypeScript surface

```typescript
// rivetkit-typescript/packages/rivetkit/src/registry/index.ts

export type FetchHandler = (req: Request, ...args: unknown[]) => Promise<Response>;
export interface ServerlessHandler { fetch: FetchHandler }

class Registry<A extends RegistryActors> {
  // Receives an engine-POSTed /api/rivet/* request, returns the handler response
  // (SSE for /start, JSON/text for others).
  async handler(request: Request): Promise<Response> {
    // Lazily builds native registry/core serverless runtime.
    return this.#handleNativeServerlessRequest(request);
  }

  // Convenience for `export default registry.serve()` in serverless entrypoints.
  serve(): ServerlessHandler {
    return { fetch: (req) => this.handler(req) };
  }

  // Starts the native serverful path. It only spawns the engine when startEngine:true.
  startEnvoy(): void { /* native serve path */ }

  // Convenience: startEnvoy() plus the `welcome` printout; see section on start().
  start(): void { /* see below */ }
}
```

### Wiring examples

Node / Hono (the common case):

```ts
import { Hono } from "hono";
import { serve } from "@hono/node-server";

const registry = setup({
  use: { counter },
  endpoint: "https://api.rivet.dev",
  serverless: { basePath: "/api/rivet" },
});

const app = new Hono();
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
serve({ fetch: app.fetch, port: 3000 });
```

Cloudflare Workers is **out of scope for v1** because NAPI does not load on V8-only runtimes. Document that CF Workers requires a future V8 binding; until then, CF users host the runner elsewhere.

## `registry.start()` — high-level standalone

`registry.start()` is not a three-line wrapper. In the reference branch it defaulted `staticDir` to `public`, ensured the local HTTP server, served static files, started envoy, and printed welcome from private `Runtime.#printWelcome()`.

The native restoration must choose one explicit behavior:

1. Rebuild the local HTTP/static wrapper in TypeScript around the native runtime router, preserving old `registry.start()` behavior.
2. Ship `start()` as a documented alias for `startEnvoy()` and explicitly remove built-in static serving from this surface.

There is no Rust `staticDir` pass-through today. Do not claim the engine subprocess serves static files unless that flag exists and is tested.

## Out of scope (v1)

- **CF Workers / V8-only runtimes.** Requires a V8 binding for rivetkit-core that does not exist.
- **Deno / Bun parity testing.** Node is primary; Bun *should* work since it supports NAPI and `fetch`/`Response` are spec-standard. Add Bun to tests as a follow-up if demand.
- **Custom auth hooks between `.handler()` and `/start`.** The built-in `/start` path authenticates `x-rivet-token` against configured `config.token` using constant-time comparison. User middleware is optional extra defense, not the primary auth boundary.
- **Multi-namespace routing.** One `Registry` instance per namespace (same as current `startEnvoy`).
- **Request-body streaming into core.** `/start` payload is bounded and read-once; we do not need chunked inbound bodies. If a future route needs it, add a second `handleServerlessRequest` overload.
- **HTTP trailers, HTTP/2 push.** Not attempted.
- **Non-SSE response streaming.** If a future route returns a non-SSE stream, the same TSF callback approach works; core just writes raw bytes instead of SSE framing.

## Open questions / out of scope clarifications

- **`staticDir` support.** There is no Rust engine-process `staticDir` flag today. Preserve static serving in TS or explicitly remove it from `registry.start()`.
- **Endpoint normalization helpers.** The old `normalizeEndpointUrl` + `endpointsMatch` + regional-hostname logic must port to Rust with identical behavior. Write as a pure function in `rivetkit-core` with unit tests mirroring the old TS unit tests.
- **Envoy lifecycle on serverless.** The reference branch constructs a new `EngineActorDriver` for each `/start` request, so different concurrent start headers can coexist. A shared process-global envoy is a behavior change and must either be proven safe or replaced with a per-request/refcounted lifecycle.

## Implementation story breakdown

Each item is one Ralph iteration unless noted.

1. **This spec lands.**
2. `rivetkit-core`: add `serverless` module with `handle_request(...)`, URL router for the 4 paths, start-header parser, auth/endpoint/namespace gates, endpoint-match/normalize helpers ported from TS with unit tests, request body limit handling, metadata response assembly from config-provided fields, and SSE chunker.
3. `rivetkit-napi`: `CoreRegistry.handleServerlessRequest(req, onStreamEvent, cancelToken, serveConfig)` using one stream-event TSF, positional args, `CancellationToken`, and Promise-backed backpressure.
4. `rivetkit-typescript`: `Registry.handler(req)` + `.serve()` implementation. Drop the `removedLegacyRoutingError` throw, enforce `maxStartPayloadBytes`, pass metadata/public fields into `ServeConfig`, wire `configureServerlessPool`, and build the `ReadableStream` around stream events.
5. Driver-test coverage: new `tests/driver/serverless-handler.test.ts` boots a local engine, POSTs a realistic `/start` payload through `registry.handler(req)`, asserts the SSE stream stays open, a `CommandStartActor` reaches the envoy, and aborting the request ends the stream without hanging. Cover `/health`, `/metadata`, `/`, missing headers, bad token, namespace mismatch, and body too large.
6. `registry.start()`: preserve the old HTTP/static behavior or intentionally document the removal. Do not pretend `startEnvoy()` binds user-facing HTTP/static ports.
7. Docs: `website/src/content/docs/actors/serverless.mdx` with Hono/Next.js/etc examples, CHANGELOG, and `.claude/reference/docs-sync.md` note so future changes to the surface get mirrored.
8. (Follow-up) Bun matrix job in CI running the same driver tests against Bun.
9. (Follow-up, out of scope for now) V8 binding for CF Workers.

Estimated total: 4 meaningful implementation stories (2-5 above), plus docs and the follow-ups. Keeps CLAUDE.md layer rules clean: no load-bearing logic lands in TS or NAPI; core owns everything.
