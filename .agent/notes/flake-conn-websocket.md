# Actor Connection WebSocket Flakes

Date: 2026-04-22

Scope: `rivetkit-typescript/packages/rivetkit`, static registry, bare encoding.

## Repro Commands

```bash
cd /home/nathan/r5/rivetkit-typescript/packages/rivetkit
DRIVER_RUNTIME_LOGS=1 DRIVER_ENGINE_LOGS=1 \
  RUST_LOG=rivetkit_core=debug,rivetkit_napi=debug,rivet_envoy_client=debug,rivet_guard=debug \
  pnpm test tests/driver/actor-conn.test.ts \
  -t "static registry.*encoding \(bare\).*isConnected should be false before connection opens" \
  > /tmp/driver-logs/conn-isconnected-run1.log 2>&1
```

The same wrapper was used for:

- `conn-isconnected-run1.log` through `conn-isconnected-run5.log`
- `conn-onopen-run1.log` through `conn-onopen-run3.log`
- `conn-large-incoming-run1.log` through `conn-large-incoming-run3.log`
- `conn-large-outgoing-run1.log` through `conn-large-outgoing-run3.log`

## Results

- `isConnected should be false before connection opens`: 5/5 passed.
- `onOpen should be called when connection opens`: 2/3 passed, 1/3 failed.
- `should reject request exceeding maxIncomingMessageSize`: 2/3 passed, 1/3 failed.
- `should reject response exceeding maxOutgoingMessageSize`: 3/3 passed.

## Finding 1: `onOpen` Can Beat The Test Timeout

Source anchor:

- `rivetkit-typescript/packages/rivetkit/tests/driver/actor-conn.test.ts:433` creates a connection and waits for `openCount` with default `vi.waitFor` timing.

Failing log:

- `/tmp/driver-logs/conn-onopen-run2.log:213` shows the engine accepted the envoy WebSocket connect.
- `/tmp/driver-logs/conn-onopen-run2.log:234` shows the runtime received `ToEnvoyInit`.
- `/tmp/driver-logs/conn-onopen-run2.log:314` reports `expected +0 to be 1`.
- `/tmp/driver-logs/conn-onopen-run2.log:374` repeats the assertion failure.

Classification: Bucket B-ish from the plan. The gateway and envoy connection are alive, but the actor-side `/connect` open does not complete before the short default wait. This may be a test timeout that is too aggressive for the native path, or it may expose a slow route/start window that should be instrumented.

Fix direction: change this test to use an explicit longer wait, matching adjacent connection-state tests, and add route-to-open timing logs if it still flakes.

## Finding 2: Incoming Oversize Close Does Not Reject The Pending RPC

Source anchor:

- `rivetkit-typescript/packages/rivetkit/tests/driver/actor-conn.test.ts:652` calls `connection.processLargeRequest(...)` and expects the promise to reject.

Failing log:

- `/tmp/driver-logs/conn-large-incoming-run2.log:330` shows the connection state was created.
- `/tmp/driver-logs/conn-large-incoming-run2.log:344` shows core sent `ToRivetWebSocketClose{code: 1011, reason: "message.incoming_too_long"}`.
- `/tmp/driver-logs/conn-large-incoming-run2.log:440` reports `Test timed out in 30000ms`.
- `/tmp/driver-logs/conn-large-incoming-run2.log:494` repeats the 30 second timeout.

Classification: distinct transport close propagation bug. The actor/runtime emits the close frame for `message.incoming_too_long`, but the client-side pending RPC is not rejected promptly. The promise only unwinds after disposal noise, which is why the test hangs.

Fix direction: inspect `rivetkit-typescript/packages/rivetkit/src/client/actor-conn.ts` and `rivetkit-typescript/packages/rivetkit/src/engine-client/actor-websocket-client.ts`. Pending actor-connection RPCs must be rejected when the underlying WebSocket closes for protocol, size, or abnormal reasons.

## PRD Split

Track the `onOpen` timing as a test hardening and instrumentation item inside the actor-conn story. Track the oversize close behavior as the real production-facing bug because it can leave user RPC promises stuck.
