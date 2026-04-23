# Serverless remediation pass

Status: proposal
Prereq: `/home/nathan/r5/.agent/specs/serverless-restoration.md` (v1) is implemented and merged.
Scope: fix parity, security, NAPI, and architecture gaps found in the v1 spec review. No full rewrite of v1 work; each item is a targeted correction or addition.

## Context

The v1 spec restored `Registry.handler()` / `.serve()` by putting `/api/rivet/*` routing + SSE pump in `rivetkit-core`, a thin NAPI streaming bridge, and TS glue. Reviews identified 12 blockers and 17 should-fixes verified against current code and the `feat/sqlite-vfs-v2` reference branch. This spec enumerates each and specifies the remediation. Items are standalone; they can land in any order unless noted.

Sections mirror the four review angles so work can be parallelized by file area.

---

## 1. Parity corrections

v1 deviated from `feat/sqlite-vfs-v2` in ways that would break the wire protocol or lose behavior.

### 1.1 Header name is `x-rivet-namespace-name`, not `x-rivet-namespace`

Source: `feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/serverless/router.ts:33`.

Fix: in core's `/start` header parser, read `x-rivet-namespace-name`. Grep v1 for `x-rivet-namespace` and rename.

Success: engine POSTing to `/api/rivet/start` with `x-rivet-namespace-name: <ns>` is accepted.

### 1.2 `ServerlessStartHeadersSchema` validation rules

Old schema (`feat/sqlite-vfs-v2:src/runtime-router/router-schema.ts`):

- `endpoint` — required string, error `"x-rivet-endpoint header is required"`.
- `token` — optional string, error `"x-rivet-token header must be a string"`.
- `poolName` — required string, error `"x-rivet-pool-name header is required"`.
- `namespace` — required string, error `"x-rivet-namespace-name header is required"`.

Fix: port to Rust as an explicit validation routine. On failure, return HTTP 400 with body `{group: "serverless", code: "invalid_request", message: <the exact message above>}`. Preserve message passthrough via the first-failed-field semantics that the old TS `parseResult.error.issues[0]?.message` used.

### 1.3 `/metadata` is the full `MetadataResponse`

v1 called this "trivial JSON". It is not. From `feat/sqlite-vfs-v2:src/common/router.ts:114-166`:

```
MetadataResponse = {
  runtime: "rivetkit",
  version: VERSION,                                // pkg version
  envoy: { kind: {serverless:{}} | {normal:{}}, version?: number },
  envoyProtocolVersion: envoyProtocol.VERSION,     // @rivetkit/engine-envoy-protocol
  actorNames: buildActorNames(config),
  clientEndpoint?: string,
  clientNamespace?: string,
  clientToken?: string,
}
```

Fix: implement in core. Since the response depends on TS-only data (pkg VERSION, envoy-protocol VERSION, `buildActorNames` output from Zod-validated user config), pass those into core via the registry NAPI config bag at construction time. Core assembles the response.

Rationale for not moving `buildActorNames` to Rust: it walks the Zod-typed user `actors` config; Zod validation must stay in TS per CLAUDE.md layer rules. Pre-computing on TS side and handing a `Vec<String>` (or equivalent structured array) to core keeps the boundary clean.

### 1.4 `publicEndpoint` / `publicNamespace` / `publicToken` resolution chain

From `feat/sqlite-vfs-v2:src/registry/config/index.ts:238-258`:

```ts
const publicEndpoint =
  parsedPublicEndpoint?.endpoint ??
  (isDevEnv && config.startEngine ? ENGINE_ENDPOINT : undefined);
const publicNamespace = parsedPublicEndpoint?.namespace;
const publicToken = parsedPublicEndpoint?.token ?? config.serverless.publicToken;
```

`parsedPublicEndpoint` parses the `https://namespace:token@host` URL auth syntax from `config.serverless.publicEndpoint`.

Fix: port the resolution to TS (it touches Zod + env-var coalescing + `isDevEnv`, so it stays on the TS side). Pass the three resolved fields into core via the NAPI config bag. Core uses them in `/metadata`.

### 1.5 `configureServerlessPool` is load-bearing

From `feat/sqlite-vfs-v2:runtime/index.ts:228`, `startServerless()` calls `configureServerlessPool(config)` when `config.configurePool` is set. That function POSTs to engine `PUT /runner-configs/{poolName}` with:

```
serverless: {
  url: customConfig.url,
  headers: customConfig.headers ?? {},
  request_lifespan: customConfig.requestLifespan ?? 900,         // seconds (15 min)
  metadata_poll_interval: customConfig.metadataPollInterval ?? 1000,
  max_runners: 100_000,
  min_runners: 0,
  runners_margin: 0,
  slots_per_runner: 1,
},
metadata: customConfig.metadata ?? {},
drain_on_version_upgrade: customConfig.drainOnVersionUpgrade ?? true,
```

Fix: wire into the core-owned `update_runner_config` path (already being moved to core by DT-041). Call from the serverless startup flow when `config.configurePool` is set. Failures log at error level but do NOT throw (matches old behavior — the old `configure.ts` wrapped in try/catch with a "restart this process" log and continued).

### 1.6 Token dual assignment: keep `runnerConfig.token` and `clientConfig.token` separate

`feat/sqlite-vfs-v2:src/serverless/router.ts:74-83`:

```ts
const runnerConfig: RegistryConfig = { ...sharedConfig, token: config.token ?? token };
const clientConfig: RegistryConfig = {
  ...sharedConfig,
  // Preserve the configured application token for actor-to-actor calls.
  // The start token is only needed for the runner connection and may not
  // have gateway permissions.
  token: config.token ?? token,
};
```

Both configs get the same `config.token ?? token` fallback but are kept as distinct fields. The comment explicitly says they serve different purposes and may diverge.

Fix: in core, maintain two distinct resolved tokens (`runner_token`, `client_token`). Do not collapse into one even when their current values are identical. The runner token is used for the `/start` runner handshake; the client token is used for actor-to-actor calls.

### 1.7 Constants

Define in core:

- `ENVOY_SSE_PING_INTERVAL: Duration = Duration::from_millis(1000);` (from `actor-driver.ts:81`)
- `ENVOY_STOP_WAIT_MS: Duration = Duration::from_millis(15_000);` (from `actor-driver.ts:82`)

Ping loop uses the former. Shutdown coordination uses the latter.

### 1.8 Abort behavior: stop pinging, do NOT shut down envoy

v1 spec said abort "shuts down the envoy start path". **Wrong.** Old handler (`actor-driver.ts:795-828`):

```ts
c.req.raw.signal.addEventListener("abort", () => {
  logger().debug("SSE aborted");
});
// ... later, inside ping loop:
if (stream.closed || stream.aborted) { /* log + break */ }
```

Envoy continues running after the SSE stream aborts. The stream's lifetime decouples from the envoy's.

Fix: on abort, core terminates ONLY the ping loop. Do NOT call envoy shutdown. Emit one debug log `"SSE aborted"` (match old string for log-grep parity).

### 1.9 Endpoint + namespace validation are both gated on `config.endpoint`

`feat/sqlite-vfs-v2:src/serverless/router.ts:56-66`: both `endpointsMatch(endpoint, config.endpoint)` AND `namespace !== config.namespace` checks are inside `if (config.endpoint) { ... }`. If `config.endpoint` is unset, neither validation runs.

Fix: port as a two-level conditional. **Security note:** §2.5 tightens this — namespace check becomes unconditional when `config.namespace` is set.

### 1.10 `/` landing text is a literal

Exact string: `"This is a RivetKit server.\n\nLearn more at https://rivet.dev"`. Match exactly for compatibility with log-scraping / uptime checks.

### 1.11 `normalizeEndpointUrl` parity test table

Port with unit tests covering every branch from `feat/sqlite-vfs-v2:src/serverless/router.ts:112-199`:

| Input | Expected |
|---|---|
| Invalid URL | `None` from normalize; `endpoints_match` falls back to string equality |
| Pathname `/` | preserved |
| Pathname `/foo/` or `/foo///` | `/foo` (strip trailing `/+`) |
| Host `127.0.0.1`, `0.0.0.0`, `::1`, `[::1]` | → `localhost` |
| `api-us-west-1.rivet.dev` | → `api.rivet.dev` |
| `api-lax.staging.rivet.dev` | → `api.staging.rivet.dev` |
| `api.rivet.dev` | unchanged |
| `api-us-west-1.example.com` | unchanged (not rivet.dev) |
| `foo-bar.rivet.dev` | unchanged (no `api-` prefix) |
| Port preserved | yes |
| Protocol preserved | yes |

Comment in the Rust impl: "HACK: regional-hostname normalization is specific to Rivet Cloud and will not work for self-hosted engines with different regional naming conventions" (verbatim from old TS, for continuity).

---

## 2. Security hardening

v1 deferred `/api/rivet/*` authentication to the user's HTTP edge. Per `CLAUDE.md` "Trust Boundaries" (client↔engine is untrusted) + "Fail-By-Default Runtime", the runner itself must validate. The OLD TS code had the same gap — this is a security improvement, not pure parity.

### 2.1 Require `x-rivet-token` match `config.token` (constant-time)

Fix: in the core `/start` handler, before any envoy work:

```rust
if let Some(cfg_token) = &config.token {
    let header_token = headers.get("x-rivet-token").unwrap_or("");
    if !constant_time_eq(header_token.as_bytes(), cfg_token.as_bytes()) {
        return Response::status(401).body(structured_error(
            "serverless", "unauthorized", "x-rivet-token mismatch"
        ));
    }
}
```

Use `subtle::ConstantTimeEq` or the equivalent Rust primitive. Never string-compare tokens.

### 2.2 Fail-closed when `config.token` is unset

If `config.token` is `None`, reject `/start` with 401 UNLESS a new config flag `config.serverless.unauthenticated: true` is explicitly set. Default is secure. The flag exists for dev/test loopbacks only and should be documented as such.

### 2.3 `endpointsMatch` is a misconfig guard, not auth

Add a code comment on the endpoint-match branch: `// NOTE: endpoint match guards against misconfiguration, not attackers. config.endpoint is typically public. Use x-rivet-token (§2.1) for authentication.` Include the same clarification in the error message returned on mismatch.

### 2.4 `maxStartPayloadBytes` enforcement

Add `config.serverless.maxStartPayloadBytes` (default 1 MiB = 1_048_576). Enforce:

- **TS side**: before calling NAPI, check `request.headers.get("content-length")` (if present) and reject with 413 before buffering. For chunked requests, buffer up to the cap and abort if exceeded.
- **Core side**: defense in depth — after receiving the `Buffer`, verify `len() <= max`, return 413 otherwise.

### 2.5 Namespace gate unconditional when `config.namespace` is set

Change from v1.9's conditional (gated on `config.endpoint`) to: if `config.namespace` is set, require `x-rivet-namespace-name` to equal it. Independent of `config.endpoint`. Fail-closed.

### 2.6 Concurrency cap

Add `config.serverless.maxConcurrentStarts` (default 10). Core tracks active `/start` SSE streams. When at cap, return 429 with `Retry-After: 1`. Structured error: `{group: "serverless", code: "too_many_concurrent_starts"}`.

### 2.7 `start_serverless_actor` errors are structured

`envoy.start_serverless_actor(payload)` returns `anyhow::Result<()>` from `engine/sdks/rust/envoy-client/src/handle.rs:484`. Its failures (bad protocol version, `ToEnvoy` decode, not-exactly-one-command, not-`CommandStartActor`) must:

- Wrap through `rivet_error::RivetError::extract` → `build_internal` path.
- Before headers are sent (see §3.4): resolve the NAPI Promise with 400 + structured JSON body.
- After headers are sent: terminate via `endStream({group, code, message})`.
- Never `panic!`. Never abort the runner process.

### 2.8 CORS: reject browser access

`/api/rivet/*` is server-to-server (engine → runner) only. Fix:

- No `Access-Control-Allow-*` headers emitted on any response.
- `OPTIONS` on `/api/rivet/*` returns 405 Method Not Allowed.

### 2.9 `x-forwarded-for`: log but don't trust

Log if present (for debug-trace fidelity). Never use for auth, rate-limit keying, or any policy decision.

### 2.10 `poolName` DNS-subdomain validation

Per `CLAUDE.md` "Naming + Data Conventions". Validate `x-rivet-pool-name` matches `^[a-z0-9][a-z0-9-]{0,62}$` (lowercase letters, digits, hyphens, ≤63 chars, no leading hyphen). Return 400 on mismatch with `{group: "serverless", code: "invalid_pool_name"}`.

### 2.11 No request-header content in SSE output

Defensive: audit that no code path reflects request-header content into SSE frame bodies. Just a response-splitting prevention guard; relevant if someone later adds an echo or a body-based error surface.

---

## 3. NAPI bridge corrections

v1 hand-waved streaming mechanics. Concrete fixes:

### 3.1 Backpressure: `onEvent` returns `Promise<void>`

Change v1 signature from fire-and-forget callbacks to an awaitable one. Rust `await`s via `ThreadsafeFunction::call_async` (pattern from `rivetkit-napi/src/actor_factory.rs` — grep for `call_async`). TS resolves the Promise from inside the `ReadableStream.pull` handler, gated on `controller.desiredSize > 0`.

Result: Rust core is paced by the JS consumer. No unbounded TSF queue, no dropped events.

### 3.2 Single TSF dispatch — collapse `writeChunk` + `endStream`

Two TSFs break cross-TSF ordering (libuv guarantees per-TSF FIFO, not cross-TSF). Collapse to one tagged-union TSF:

```rust
#[napi(object)]
pub struct StreamEvent {
    pub kind: String, // "chunk" | "end"
    pub chunk: Option<Buffer>,
    pub error: Option<StructuredError>,
}
```

TS handler: `(event) => { if (event.kind === "chunk") ... else handleEnd(event.error) }`. Matches the existing `rivetkit-napi/src/websocket.rs:~23` `WebSocketEvent` pattern.

### 3.3 Abort race: `tokio::select!` on cancel token at every `await`

Core's `handle_request` body runs inside `tokio::select!` against the cancel token. Every `.await` (including `envoy.started()`, the ping loop's `sleep`, and any TSF `call_async`) is interruptible.

- Cancel BEFORE Promise resolves: resolve Promise with HTTP 499 Client Closed Request + structured body `{group: "serverless", code: "cancelled"}`.
- Cancel DURING streaming: exit ping loop per §1.8, emit `endStream(None)` with clean termination, do NOT call envoy shutdown.

### 3.4 Pre-stream vs mid-stream error boundary

Defer Promise resolve until **all** pre-stream validation succeeds. Specifically, in this order:

1. Header parse + Zod-equivalent validation (§1.2) — sync; fail → 400.
2. Token auth (§2.1) — sync; fail → 401.
3. Endpoint/namespace match (§1.9, §2.5) — sync; fail → 400 (EndpointMismatch / NamespaceMismatch).
4. Concurrency cap check (§2.6) — sync; fail → 429.
5. Payload size check (§2.4) — sync; fail → 413.
6. `envoy.start_serverless_actor(payload)` payload decode/validation (§2.7) — near-sync; fail → 400.
7. `envoy.started()` await — async but usually instant; fail → 503.
8. All pass → resolve Promise with 200 + `Content-Type: text/event-stream`. Enter ping loop.

Any post-(8) failure (write fails, envoy stops) terminates via `endStream({group, code, message})`.

This gives the engine a real HTTP status for every startup failure instead of a truncated stream.

### 3.5 Post-`endStream` chunks must no-op

After `endStream` fires (normal end, error, or cancel), subsequent `writeChunk` from core must be dropped silently on the JS side. The TS `ReadableStream` controller is already closed/errored. Core should not call `writeChunk` post-`endStream`, but defend against the race on the JS side too.

### 3.6 Use `CancellationToken` class, NOT raw `AbortSignal` through NAPI

Per `docs-internal/engine/napi-bridge.md`: `#[napi(object)]` fields are plain data only; `JsFunction`/TSF inside is forbidden; `AbortSignal` cannot cross `#[napi(object)]`.

Fix: TS creates `new CancellationToken()` per request, wires `req.signal.addEventListener("abort", () => token.cancel())`, passes `token` as a distinct positional NAPI argument. Core awaits `token.cancelled()` in select arms.

### 3.7 Flatten callbacks to positional args

Per `napi-bridge.md` conventions. Final signature:

```
handleServerlessRequest(
  req: { method, url, headers, body: Buffer },
  onEvent: (event: StreamEvent) => Promise<void>,
  cancelToken: CancellationToken,
): Promise<{ status: number, headers: Record<string, string> }>
```

No inline callbacks object.

### 3.8 Buffer copy: bounded to `/start`, documented

`handleServerlessRequest` copies the request body once (NAPI Buffer → Rust `Vec<u8>`). For `/start`'s ≤1 MiB one-shot payload this is acceptable. Document in core code + the public d.ts:

> `handleServerlessRequest` is for engine-initiated runner start only. It is NOT a general-purpose HTTP dispatch entrypoint; per-request user data-plane traffic must go through the engine gateway.

---

## 4. Architecture

### 4.1 `registry.start()` must force `startEngine: true`

Current `buildServeConfig` at `rivetkit-typescript/packages/rivetkit/src/registry/native.ts:4479` only wires `engineBinaryPath` when `config.startEngine` is truthy. v1's three-line `start()` silently does nothing in the serverful case.

Fix: `registry.start()` sets `config.startEngine = true` if not already set. Matches old `feat/sqlite-vfs-v2` behavior — `.start()` always assumed a local engine. Fail loudly with a structured `RivetError` if the engine binary cannot be resolved (`@rivetkit/engine-cli`'s `getEnginePath()` throws).

### 4.2 `printWelcome`: make public on `Runtime`

v1 called `rt.printWelcome()`. Current `runtime/index.ts:125,128` has `#printWelcome()` private. Fix: rename to public `printWelcome()`. `registry.start()` calls it after `startEnvoy()` resolves.

### 4.3 `staticDir` serving: documented gap, not wired

Grep confirmed zero `staticDir` / `static_dir` references in `/home/nathan/r5/rivetkit-rust/`. The engine binary has no static-dir flag to wire to.

Fix: `registry.start()` ignores `config.staticDir` for now. If `staticDir` is set, emit a one-time warning log: `"staticDir is not yet wired in the native engine runtime; see TODO(issue-TBD)"`. CHANGELOG entry documents the gap as a known regression from `feat/sqlite-vfs-v2`.

A separate story (NOT this one) adds the engine-side flag.

### 4.4 Envoy lifecycle: process-global, shared across `/start` requests

Resolve v1's open question. Decision: **one process-global envoy** owned by the registry runtime, shared across all concurrent `/start` SSE streams. Matches old `this.#envoyStarted.promise` semantics. All `handle_request` invocations ensure it's running via the existing idempotent boot path.

No per-request envoy. No refcounting.

### 4.5 Zod / core config boundary: explicit

- **TS owns**: env-var reading (`RIVET_ENDPOINT`, `RIVET_TOKEN`, etc.), Zod parsing, dev-mode detection (`isDevEnv`), `publicEndpoint` URL auth-syntax parse, defaults.
- **Core owns**: post-parsed primitive values only. Never reads env vars. Never imports Zod.

The NAPI registry-config bag carries post-parsed fields: `endpoint`, `namespace`, `runner_token`, `client_token`, `serverless.{base_path, max_start_payload_bytes, max_concurrent_starts, unauthenticated, public_endpoint, public_namespace, public_token, configure_pool}`.

Add this boundary to `rivetkit-typescript/CLAUDE.md` under a new `## Config Boundary` section if not already captured.

### 4.6 NAPI streaming state is core-owned

Restate and enforce: core owns event ordering (§3.2 single TSF), backpressure pacing (§3.1 Promise), post-endStream drop defense (§3.5), per-request cancel token (§3.6). NAPI holds only the TSF handle and `CancellationToken` forwarder — zero state machine.

Update `docs-internal/engine/napi-bridge.md` with the streaming-response pattern if it isn't already documented.

---

## 5. Misc cleanups

### 5.1 `startServerless` idempotency

Old `startServerless()` (`feat/sqlite-vfs-v2:runtime/index.ts:217-220`) is idempotent: early-return if already started; assert not serverful. `Registry.handler()` calls this per request.

Fix: preserve same shape in the new code. `ensureRuntime()` is per-request-idempotent (same as old behavior). Document at the top of `handler()`.

### 5.2 Story breakdown split

v1 step 2 bundled router + parser + endpoint-match + SSE pump + tests. Split for parallelism:

- **2a**: endpoint-match helpers + parity unit tests from §1.11 table. Pure functions. No dependencies on the NAPI bridge.
- **2b**: URL router + header parser (§1.1, §1.2) + `/health` + `/` + `/metadata` wiring (§1.3, §1.4). Synchronous endpoints only.
- **2c**: `/start` SSE pump + envoy coordination (§1.7, §1.8, §2.7, §3.4). Depends on 2a.
- **3**: NAPI bridge (§3). Depends on 2c's signatures.
- **4**: TS `Registry.handler()` + `.serve()` + `registry.start()` (§4.1-4.3). Depends on 3.

Lets 2a and 3's Rust-side infrastructure proceed in parallel.

### 5.3 Error sanitization via `build_internal`

Core's mid-stream errors via `endStream` go through `rivet_error::RivetError::extract` + `build_internal` path per `CLAUDE.md` "rivetkit-core is the single source of truth for cross-boundary error sanitization". TS bridge must not re-wrap — forward structured errors unchanged.

### 5.4 `normalizeEndpointUrl` is reimplementation, not port

Zero live TS callers on the current branch (grep verified). Source of truth is the deleted `feat/sqlite-vfs-v2` branch code. Frame the work in the implementing commit as reimplementation; no TS duplication required.

---

## 6. Out of scope (confirmed deferrals)

- **Cloudflare Workers / Deno Deploy / Vercel Edge** — no NAPI, requires a future V8 binding for `rivetkit-core`.
- **Inbound request-body streaming** — `/start` is bounded one-shot; `maxStartPayloadBytes` caps it.
- **Non-SSE response streaming** — same NAPI plumbing works; add when a route needs it.
- **Per-IP rate limiting** — user's HTTP edge. Core enforces total-concurrency cap only (§2.6).
- **HTTP trailers / HTTP/2 push.**
- **CORS support for browsers** — explicitly rejected (§2.8).
- **`staticDir` serving in `registry.start()`** — documented gap (§4.3); tracked separately.
- **Runtime auth schemes beyond `x-rivet-token`** (JWT, OAuth, mTLS) — not needed; runner is behind the user's authenticated edge.

---

## 7. Acceptance checklist

A v2 implementation is complete when:

### Parity (§1)
- [ ] Header name `x-rivet-namespace-name` (§1.1)
- [ ] Header validation returns exact old Zod error strings (§1.2)
- [ ] `/metadata` returns full `MetadataResponse` shape (§1.3)
- [ ] `publicEndpoint`/`publicNamespace`/`publicToken` resolved in TS, passed to core (§1.4)
- [ ] `configureServerlessPool` called when `configurePool` set, uses core-owned `update_runner_config` (§1.5)
- [ ] `runner_token` and `client_token` kept distinct in core (§1.6)
- [ ] `ENVOY_SSE_PING_INTERVAL=1000ms`, `ENVOY_STOP_WAIT_MS=15000ms` (§1.7)
- [ ] Abort stops ping loop only, envoy untouched, debug log matches `"SSE aborted"` (§1.8)
- [ ] Endpoint-match gate is conditional on `config.endpoint` (§1.9)
- [ ] `/` body matches literal `"This is a RivetKit server.\n\nLearn more at https://rivet.dev"` (§1.10)
- [ ] `normalize_endpoint_url` passes every row of the §1.11 table

### Security (§2)
- [ ] Constant-time `x-rivet-token` vs `config.token` compare (§2.1)
- [ ] Fail-closed when `config.token` unset, unless `serverless.unauthenticated: true` (§2.2)
- [ ] Endpoint-mismatch error includes "not authentication" clarification (§2.3)
- [ ] `maxStartPayloadBytes` enforced on both TS and Rust sides (§2.4)
- [ ] Namespace gate unconditional when `config.namespace` set (§2.5)
- [ ] `maxConcurrentStarts` cap returns 429 with `Retry-After: 1` (§2.6)
- [ ] `start_serverless_actor` errors routed through `RivetError::extract` (§2.7)
- [ ] CORS headers absent; `OPTIONS` returns 405 (§2.8)
- [ ] `x-forwarded-for` logged but never trusted (§2.9)
- [ ] `poolName` DNS-subdomain regex enforced (§2.10)
- [ ] No request-header content echoed into SSE frames (§2.11)

### NAPI (§3)
- [ ] `onEvent` returns `Promise<void>`, awaited in core (§3.1)
- [ ] Single TSF with tagged `StreamEvent` (§3.2)
- [ ] All awaits inside `tokio::select!` against cancel token (§3.3)
- [ ] Pre-stream validation deferred-resolve contract matches §3.4 ordering
- [ ] Post-`endStream` chunks are no-ops on JS side (§3.5)
- [ ] `CancellationToken` class used, not raw `AbortSignal` (§3.6)
- [ ] Positional NAPI args, not inline callbacks object (§3.7)
- [ ] Buffer-copy documented as bounded to `/start` (§3.8)

### Architecture (§4)
- [ ] `registry.start()` forces `startEngine: true`, fails loudly on missing binary (§4.1)
- [ ] `printWelcome` public on `Runtime`, called by `start()` (§4.2)
- [ ] `staticDir` warning logged when set, CHANGELOG entry (§4.3)
- [ ] Envoy is process-global singleton, shared across concurrent `/start` (§4.4)
- [ ] Config-boundary rule in `rivetkit-typescript/CLAUDE.md` (§4.5)
- [ ] NAPI streaming state zero (§4.6)

### Misc (§5)
- [ ] `handler()` idempotency comment (§5.1)
- [ ] Implementation stories split per §5.2
- [ ] Mid-stream errors via `build_internal` (§5.3)

### Tests
- [ ] `tests/driver/serverless-handler.test.ts` covers: token auth pass/fail, endpoint mismatch, namespace mismatch (both conditional and unconditional paths), payload-size reject, concurrency reject, full metadata shape, SSE ping arrival + timing, abort tears down stream but not envoy, `/health`, `/metadata`, `/`, CORS reject, invalid `poolName`.
- [ ] Rust unit tests cover: every §1.11 normalize row, header validation error message passthrough, constant-time token compare (timing-attack test — optional).
