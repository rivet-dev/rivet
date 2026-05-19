# Engine Restart Serverless Investigation

## Scope

This documents the local reproduction work for serverless Rivet Actor access after restarting `rivet-engine`.

The harness runs:

- `tests/driver/engine-restart-serverless.ts`
- `tests/fixtures/engine-restart-serverless-runtime.ts`

The runtime is a normal Node process, not Vitest. It listens on a port with `serverless.basePath = "/api/rivet"`. The engine is started as a real `rivet-engine` binary and configured with a serverless runner config pointing at that runtime.

## Main Finding

The bug is not SQLite-specific.

After `rivet-engine` restarts, there is a short window where gateway traffic to already-existing warmed actors can hang. This affects:

- action calls through the client
- HTTP actor requests through `/gateway/{actor}/request/...`
- raw actor WebSockets through `/gateway/{actor}/websocket/...`

New actor keys work immediately while warmed existing keys can hang, so this looks like stale or not-yet-settled gateway/serverless actor routing state for existing actors.

## Timing Window

Timing is measured from the moment engine `/health` returns after restart.

Observed local window:

- Unsafe: `0-2400ms`
- Flaky edge: roughly `2500-3250ms`
- Local minimum to trust: `3000ms`
- Conservative diagnostic delay: `5000ms`

This is not a crisp timer. The edge moves run-to-run.

## Baseline Actor Action Results

These probes use the same existing actor key and a new key after engine restart.

No heartbeat, immediate probe:

- same-handle `getCount`: timed out
- same-handle `tick`: timed out
- fresh handle with same key `getCount`: timed out
- new key `tick`: succeeded

This reproduced the "actor bricked" symptom:

```text
bricked actor symptom reproduced. mode=idle failedPostRestartProbes=3 before=0
```

No heartbeat, delayed probe:

- same existing key succeeded
- new key succeeded

This means the actor is not permanently broken in the local harness. It is unreachable through the existing-key gateway path during the post-restart race window.

## Heartbeat Results

The runtime supports `RIVETKIT_HEARTBEAT_MODE=none|sqlite|kv`.

Immediate post-restart probes still reproduced the brick with:

- no heartbeat
- SQLite heartbeat
- KV heartbeat

Both SQLite and KV actor-originated heartbeat traffic could recover after restart while gateway/client traffic to the same warmed existing actor key still hung.

Conclusion: heartbeat success does not prove gateway readiness, and SQLite is not special.

## HTTP Gateway Health Sweep

The actor exposes an `onRequest` health endpoint at:

```text
/gateway/{actor}/request/health
```

The sweep warms one existing actor key per delay before restart, then probes each key once after engine `/health` returns. This avoids an early failed probe poisoning later delay measurements.

Coarse sweep:

```text
0ms: timeout
250ms: timeout
500ms: timeout
1000ms: timeout
2000ms: timeout
3000ms: 200 OK
5000ms: 200 OK
8000ms: 200 OK
12000ms: 200 OK
```

Narrow sweeps:

```text
2000ms: timeout
2250ms: timeout
2500ms: timeout in one run, success in another
2750ms: success in one run
3000ms: success
3250ms: success
```

Low repeat:

```text
1000ms: timeout
1500ms: timeout
1800ms: timeout
2000ms: timeout
2200ms: timeout
2400ms: timeout
```

HTTP conclusion: gateway HTTP actor requests become reliably usable around `3s` locally, with `5s` as the safer diagnostic delay.

## WebSocket Ping/Pong Sweep

The actor exposes `onWebSocket` ping/pong at:

```text
/gateway/{actor}/websocket/ping
```

The client opens the WebSocket with Rivet gateway subprotocols:

```text
rivet
rivet_encoding.bare
```

Then it sends:

```json
{"type":"ping","sentAt":...}
```

And waits for:

```json
{"type":"pong","sentAt":...}
```

Coarse sweep:

```text
0ms: timeout
250ms: timeout
500ms: timeout
1000ms: timeout
2000ms: timeout
3000ms: pong
5000ms: pong
8000ms: pong
12000ms: pong
```

Narrow sweeps:

```text
2000ms: timeout
2250ms: timeout
2500ms: timeout
2750ms: timeout in one run
3000ms: timeout in one run, success in another
3250ms: pong
```

Edge repeat:

```text
2800ms: pong
3000ms: pong
3200ms: pong
3400ms: pong
3600ms: pong
```

Low repeat:

```text
2200ms: timeout
2400ms: timeout
2600ms: timeout
2800ms: pong
```

WebSocket conclusion: raw actor WebSocket ping/pong has the same post-restart readiness window as HTTP gateway traffic. It is not action-specific.

## Commit During Restart

The harness can coordinate an actor action that starts a SQLite transaction, signals the driver, then attempts `COMMIT` after the engine is stopped.

Immediate post-restart probes after this failed commit reproduced the same shape:

- failed in-flight operation
- same existing key probes timed out
- new key succeeded

Delayed post-restart probes passed.

This is consistent with the gateway/serverless actor routing race, not durable SQLite session poisoning.

## Important Corrections

Earlier, it looked like a SQLite heartbeat caused later gateway probes to pass. That was wrong. A no-heartbeat delayed control also passed.

The actual variable was delay after engine health, not SQLite activity.

## Useful Logs

Representative local logs:

```text
/tmp/rivet-idle-none-immediate.log
/tmp/rivet-idle-kv-immediate.log
/tmp/rivet-idle-sqlite-immediate.log
/tmp/rivet-idle-none-after.log
/tmp/rivet-idle-kv-after.log
/tmp/rivet-idle-sqlite-after.log
/tmp/rivet-commit-none-immediate.log
/tmp/rivet-commit-none-after.log
/tmp/rivet-gateway-health-sweep.log
/tmp/rivet-gateway-health-sweep-narrow.log
/tmp/rivet-gateway-health-sweep-edge.log
/tmp/rivet-gateway-health-sweep-low.log
/tmp/rivet-gateway-websocket-sweep.log
/tmp/rivet-gateway-websocket-sweep-narrow.log
/tmp/rivet-gateway-websocket-sweep-edge.log
/tmp/rivet-gateway-websocket-sweep-low.log
```

## Commands

Action/client repro:

```bash
RIVETKIT_ENGINE_RESTART_MODE=idle \
RIVETKIT_HEARTBEAT_MODE=none \
RIVETKIT_POST_RESTART_PROBE_TIMING=immediate \
node --import tsx tests/driver/engine-restart-serverless.ts
```

HTTP health sweep:

```bash
RIVETKIT_ENGINE_RESTART_MODE=idle \
RIVETKIT_HEARTBEAT_MODE=none \
RIVETKIT_GATEWAY_HEALTH_DELAYS_MS=0,250,500,1000,2000,3000,5000,8000,12000 \
node --import tsx tests/driver/engine-restart-serverless.ts
```

WebSocket ping/pong sweep:

```bash
RIVETKIT_ENGINE_RESTART_MODE=idle \
RIVETKIT_HEARTBEAT_MODE=none \
RIVETKIT_GATEWAY_WEBSOCKET_DELAYS_MS=0,250,500,1000,2000,3000,5000,8000,12000 \
node --import tsx tests/driver/engine-restart-serverless.ts
```

## Current Theory

Engine `/health` returning does not mean gateway/serverless routing for previously warmed existing actors is fully settled.

Requests in the first few seconds can attach to stale or incomplete actor routing/request state. Those requests hang until the caller timeout. A new key succeeds because it follows a fresh actor allocation path instead of the stale existing-key path.

The likely fix area is the gateway/serverless/pegboard-envoy readiness and stale actor state path after engine restart, especially around existing actor generation, in-flight wake, stopped state, and pending request routing.
