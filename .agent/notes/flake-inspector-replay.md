# Inspector Workflow Replay Flake

Date: 2026-04-22

Scope: `rivetkit-typescript/packages/rivetkit`, static registry, bare encoding.

## Repro

```bash
cd /home/nathan/r5/rivetkit-typescript/packages/rivetkit
DRIVER_RUNTIME_LOGS=1 DRIVER_ENGINE_LOGS=1 \
  RUST_LOG=rivetkit_core=debug,rivetkit_napi=debug,rivet_envoy_client=debug,rivet_guard=debug \
  pnpm test tests/driver/actor-inspector.test.ts \
  -t "static registry.*encoding \(bare\).*rejects workflows that are currently in flight" \
  > /tmp/driver-logs/inspector-replay.log 2>&1
```

Exit: 1.

## Finding

The test failure is not the expected 409 assertion. The replay request does return 409 with the expected structured `workflow_in_flight` error, then the test times out after `handle.release()` while waiting for `finishedAt`.

Source anchors:

- `rivetkit-typescript/packages/rivetkit/tests/driver/actor-inspector.test.ts:596` waits for inspector `workflowState` to be `pending` or `running`.
- `rivetkit-typescript/packages/rivetkit/tests/driver/actor-inspector.test.ts:607` posts `/inspector/workflow/replay` and expects 409.
- `rivetkit-typescript/packages/rivetkit/tests/driver/actor-inspector.test.ts:641` calls `handle.release()`, then waits for `finishedAt`.
- `rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/workflow.ts:769` blocks the workflow on a deferred.
- `rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/workflow.ts:784` resolves that deferred from the `release` action.
- `rivetkit-typescript/packages/rivetkit/src/workflow/mod.ts:199` installs the replay guard and checks `actor.isRunHandlerActive()` plus workflow state before calling `replayWorkflowFromStep`.
- `rivetkit-typescript/packages/rivetkit/src/registry/native.ts:3416` handles `POST /inspector/workflow/replay`.

Log anchors:

- `/tmp/driver-logs/inspector-replay.log:360` shows the live workflow storage loaded as `state=pending`.
- `/tmp/driver-logs/inspector-replay.log` contains the `POST /inspector/workflow/replay` request and a `status=409 content_length=147` response.
- `/tmp/driver-logs/inspector-replay.log:11724` shows sleep stayed blocked because of `reason=ActiveRunHandler`.
- `/tmp/driver-logs/inspector-replay.log:13382` reports the test timeout at 30012 ms.
- `/tmp/driver-logs/inspector-replay.log:13437` reports `Error: Test timed out in 30000ms`.

## Interpretation

The server is rejecting replay correctly, but the failed replay appears to leave the live in-flight workflow stranded. The most likely bug is that the replay path touches workflow storage or replay control state before, during, or despite the in-flight guard. Another possibility is that the guard's active-run state and the live workflow's release path disagree under native execution.

## Fix Direction

Make the in-flight replay guard happen before any replay storage or control mutation. Then add a regression test that proves a rejected replay does not affect the live workflow: after the 409, `release()` must still let the `block` step continue and the `finish` step set `finishedAt`.

Also inspect whether `actor.isRunHandlerActive()` and `workflowInspector.adapter.getState()` can temporarily disagree around this path. If they can, prefer a single authoritative in-flight state for replay rejection.
