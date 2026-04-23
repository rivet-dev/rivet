# Actor Queue Flakes

Date: 2026-04-22

Scope: `rivetkit-typescript/packages/rivetkit`, static registry, bare encoding.

## Current Status

- The isolated `wait send returns completion response` path was fixed by DT-012 and is no longer the active bug for this area.
- Remaining queue flake tracking is the high-fan-out child-actor path under fast static/http/bare verification. See DT-051 and DT-056 in `scripts/ralph/prd.json`.

## Repro Commands

```bash
cd /home/nathan/r5/rivetkit-typescript/packages/rivetkit
DRIVER_RUNTIME_LOGS=1 DRIVER_ENGINE_LOGS=1 \
  RUST_LOG=rivetkit_core=debug,rivetkit_napi=debug,rivet_envoy_client=debug,rivet_guard=debug \
  pnpm test tests/driver/actor-queue.test.ts \
  -t "static registry.*encoding \(bare\).*wait send returns completion response" \
  > /tmp/driver-logs/queue-waitsend-run1.log 2>&1
```

The same wrapper was used for:

- `queue-waitsend-run1.log` through `queue-waitsend-run5.log`
- `queue-manychild-run1.log` through `queue-manychild-run3.log`

## Results

- `wait send returns completion response`: 5/5 passed.
- `drains many-queue child actors created from actions while connected`: 1/3 passed, 2/3 failed.

## Finding

The isolated `enqueueAndWait` path did not reproduce. The distinct queue bug is the high-fan-out child actor case while a connection is open.

Source anchors:

- `rivetkit-typescript/packages/rivetkit/tests/driver/actor-queue.test.ts:242` is the isolated wait-send completion test.
- `rivetkit-typescript/packages/rivetkit/tests/driver/actor-queue.test.ts:277` is the high-fan-out child actor test that reproduced.

Failing log anchors:

- `/tmp/driver-logs/queue-manychild-run1.log:354` shows the child actor connection opened through `/gateway/manyQueueChildActor/connect`.
- `/tmp/driver-logs/queue-manychild-run1.log:1285` begins a wave of queue `POST /queue/cmd.*` requests.
- `/tmp/driver-logs/queue-manychild-run1.log:5187` shows at least one queue request completed with `status: 200`.
- `/tmp/driver-logs/queue-manychild-run1.log:5443` begins repeated `ToRivetResponseStart{status: 500, content-length: 75}` responses.
- `/tmp/driver-logs/queue-manychild-run1.log:5661` shows a completed request with `status=500 content_length=75`.
- `/tmp/driver-logs/queue-manychild-run1.log:5663` shows the client received `actor/dropped_reply` with message `Actor reply channel was dropped without a response.`
- `/tmp/driver-logs/queue-manychild-run3.log:6045` and `:6109` show the same dropped reply failure.

## Interpretation

This is not the same as the single `wait send` completion path. Under high fan-out, many queue sends enter the runtime, some complete normally, then a cluster of replies drops and the engine returns 500s. The open WebSocket connection likely amplifies the pressure, but the failure signature is queue/HTTP reply dropping rather than a simple WebSocket open stall.

## Fix Direction

Inspect the core registry HTTP queue path and every error branch that bridges a queue `POST` request to actor dispatch. Each accepted queue request must deterministically send exactly one response or a structured overload/error response. In particular, audit disconnect cleanup and cancellation paths so cleanup cannot drop reply channels after the actor accepted the work.

Add a regression that runs the many-child action drain repeatedly enough to catch the fan-out case, or add deterministic pressure by reducing queue/HTTP concurrency limits in the test fixture.
