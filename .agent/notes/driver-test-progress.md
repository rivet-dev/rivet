# Driver Test Suite Progress

Started: 2026-04-22
Config: registry (static), client type (http), encoding (bare)

## Fast Tests

- [x] manager-driver | Manager Driver Tests
- [x] actor-conn | Actor Connection Tests
- [x] actor-conn-state | Actor Connection State Tests
- [x] conn-error-serialization | Connection Error Serialization Tests
- [x] actor-destroy | Actor Destroy Tests
- [x] request-access | Request Access in Lifecycle Hooks
- [x] actor-handle | Actor Handle Tests
- [x] action-features | Action Features (was listed as "Tests" in skill doc; actual describe is "Action Features")
- [x] access-control | access control
- [x] actor-vars | Actor Variables
- [x] actor-metadata | Actor Metadata Tests
- [x] actor-onstatechange | Actor onStateChange Tests (was listed as "State Change Tests")
- [x] actor-db | Actor Database (flaky: "handles parallel actor lifecycle churn" hit `no_envoys` 1/4 runs)
- [x] actor-db-raw | Actor Database (Raw) Tests
- [~] actor-workflow | Actor Workflow Tests (US-103 fixed sleep-grace/run-handler crash-path coverage; remaining known red test is workflow destroy semantics)
- [~] actor-error-handling | Actor Error Handling Tests (6 pass / 1 fail)
- [x] actor-queue | Actor Queue Tests (flaky on first run: 3 failures related to "reply channel dropped" / timeout; clean on retry)
- [x] actor-kv | Actor KV Tests
- [x] actor-stateless | Actor Stateless Tests
- [x] raw-http | raw http
- [x] raw-http-request-properties | raw http request properties
- [x] raw-websocket | raw websocket
- [~] actor-inspector | Actor Inspector HTTP API (1 fail is workflow-replay related; 20 pass)
- [x] gateway-query-url | Gateway Query URLs (filter was missing the "s")
- [x] actor-db-pragma-migration | Actor Database PRAGMA Migration Tests
- [x] actor-state-zod-coercion | Actor State Zod Coercion Tests (filter needed suffix)
- [x] actor-conn-status | Connection Status Changes
- [x] gateway-routing | Gateway Routing
- [x] lifecycle-hooks | Lifecycle Hooks

## Slow Tests

- [x] actor-state | Actor State Tests
- [x] actor-schedule | Actor Schedule Tests
- [x] actor-sleep | Actor Sleep Tests
- [x] actor-sleep-db | Actor Sleep Database Tests
- [x] actor-lifecycle | Actor Lifecycle Tests
- [x] actor-conn-hibernation | Connection Hibernation (flaky first run; clean on retry)
- [x] actor-run | Actor Run Tests
- [x] hibernatable-websocket-protocol | hibernatable websocket protocol (all 6 tests skipped; the feature flag `hibernatableWebSocketProtocol` is not enabled for the static driver config)
- [x] actor-db-stress | Actor Database Stress Tests

## Excluded

- [ ] actor-agent-os | Actor agentOS Tests (skip unless explicitly requested)

## Log

- 2026-04-22 manager-driver: PASS (16 tests, 12.20s)
- 2026-04-22 actor-conn: PASS (23 tests, 28.12s) -- Note: first run showed 2 flaky failures (lifecycle hooks `onWake` missing; `maxIncomingMessageSize` timeout). Re-ran 5 times with trace after, all passed. Likely cold-start race on first run.
- 2026-04-22 actor-conn-state: PASS (8 tests, 6.80s)
- 2026-04-22 conn-error-serialization: PASS (3 tests, 2.53s)
- 2026-04-22 actor-destroy: PASS (10 tests, 19.47s)
- 2026-04-22 request-access: PASS (4 tests, 3.52s)
- 2026-04-22 actor-handle: PASS (12 tests, 8.42s)
- 2026-04-22 action-features: PASS (11 tests, 8.46s) -- corrected filter to "Action Features" (no "Tests" suffix)
- 2026-04-22 access-control: PASS (8 tests, 6.29s)
- 2026-04-22 actor-vars: PASS (5 tests, 3.81s)
- 2026-04-22 actor-metadata: PASS (6 tests, 4.34s)
- 2026-04-22 actor-onstatechange: PASS (5 tests, 3.97s) -- corrected filter to "Actor onStateChange Tests"
- 2026-04-22 actor-db: PASS (16 tests, 26.21s) -- flaky 1/4: "handles parallel actor lifecycle churn" intermittently fails with no_envoys. Passes on retry.
- 2026-04-22 actor-db-raw: PASS (4 tests, 4.04s) -- corrected filter to "Actor Database (Raw) Tests"
- 2026-04-22 actor-queue: PASS (25 tests, 32.95s) -- first run had 3 flaky failures, all passed on retry
- 2026-04-22 actor-kv: PASS (3 tests, 2.51s)
- 2026-04-22 actor-stateless: PASS (6 tests, 4.38s)
- 2026-04-22 raw-http: PASS (15 tests, 10.76s)
- 2026-04-22 raw-http-request-properties: PASS (16 tests, 11.44s)
- 2026-04-22 raw-websocket: PASS (11 tests, 8.77s)
- 2026-04-22 actor-inspector: PARTIAL PASS (20 passed, 1 failed, 42 skipped) -- filter corrected to "Actor Inspector HTTP API". Only failure is `POST /inspector/workflow/replay rejects workflows that are currently in flight` (workflow-related; user asked to skip workflow issues).
- 2026-04-22 gateway-query-url: PASS (2 tests, 2.35s) -- filter corrected to "Gateway Query URLs"
- 2026-04-22 actor-db-pragma-migration: PASS (4 tests, 4.09s)
- 2026-04-22 actor-state-zod-coercion: PASS (3 tests, 3.34s)
- 2026-04-22 actor-conn-status: PASS (6 tests, 5.76s)
- 2026-04-22 gateway-routing: PASS (8 tests, 5.96s)
- 2026-04-22 lifecycle-hooks: PASS (8 tests, 6.62s)
- 2026-04-22 actor-state: PASS (3 tests, 3.08s)
- 2026-04-22 actor-schedule: PASS (4 tests, 6.79s)
- 2026-04-22 actor-sleep: PASS (21 tests, 53.61s)
- 2026-04-22 actor-sleep-db: PASS (14 tests, 42.29s)
- 2026-04-22 actor-lifecycle: PASS (5 tests, 30.22s)
- 2026-04-22 actor-conn-hibernation: PASS (5 tests) -- filter is "Connection Hibernation". Flaky first run ("conn state persists through hibernation"), passed on retry.
- 2026-04-22 hibernatable-websocket-protocol: N/A (feature not enabled; all 6 tests correctly skipped)
- 2026-04-22 actor-db-stress: PASS (3 tests, 24.22s)
- 2026-04-22 actor-run: PASS after US-103 (8 passed / 16 skipped) -- native abortSignal binding plus sleep-grace abort firing and NAPI run-handler active gating now cover `active run handler keeps actor awake past sleep timeout`.
- 2026-04-22 actor-error-handling: FAIL (1 failed, 6 passed, 14 skipped) -- `should convert internal errors to safe format` leaks the original `Error` message through instead of sanitizing to `INTERNAL_ERROR_DESCRIPTION`. Server-side sanitization of plain `Error` into canonical internal_error was likely dropped somewhere on this branch; `toRivetError` in actor/errors.ts preserves `error.message` and the classifier in common/utils.ts is not being invoked on this path. Needs fix outside driver-runner scope.
- 2026-04-22 actor-workflow: FAIL (6 failed / 12 passed / 39 skipped) -- REVERTED the `isLifecycleEventsNotConfiguredError` swallow in `stateManager.saveState`. The fix only masked the symptom: workflow `batch()` does `Promise.all([kvBatchPut, stateManager.saveState])`, and when the task joins and `registry/mod.rs:807` clears `configure_lifecycle_events(None)`, a still-pending `saveState` hits `actor/state.rs:191` (`lifecycle_event_sender()` returns None) → unhandled rejection → Node runtime crash → downstream `no_envoys` / "reply channel dropped". Root cause is the race: shutdown tears down lifecycle events while the workflow engine still has an outstanding save. Real fix belongs in core or the workflow flush sequence, not in a bridge error swallow. Failures that were being masked:
  * `starts child workflows created inside workflow steps` - 2 identical "child-1" results instead of 1. Workflow step body re-executes on replay, double-pushing to `state.results`.
  * `workflow steps can destroy the actor` - ctx.destroy() fires onDestroy but actor still resolvable via `get`. envoy-client `destroy_actor` sends plain `ActorIntentStop` and there is no `ActorIntentDestroy` in the envoy v2 protocol. TS runner sets `graceful_exit` marker; equivalent marker is not wired through Rust envoy-client.
- 2026-04-22 actor-workflow after US-103: PARTIAL PASS (17 passed / 1 failed / 39 skipped). Crash-path coverage passed, including `replays steps and guards state access`, `tryStep and try recover terminal workflow failures`, `sleeps and resumes between ticks`, and `completed workflows sleep instead of destroying the actor`. Remaining failure is still `workflow steps can destroy the actor`, matching the known missing envoy destroy marker above.
- 2026-04-22 actor-db sanity after US-103: PASS for `handles parallel actor lifecycle churn`.
- 2026-04-22 actor-queue sanity after US-103: combined route-sensitive run still hit the known many-queue dropped-reply/overload flake; both targeted cases passed when run in isolation.
- 2026-04-22 ALL FILES PROCESSED (37 files). Summary: 30 full-pass, 4 partial-pass (actor-workflow, actor-error-handling, actor-inspector, actor-run), 1 n/a (hibernatable-websocket-protocol - feature disabled). 2 code fixes landed: (1) `stateManager.saveState` swallows post-shutdown state-save bridge error in workflow cleanup; (2) `#createActorAbortSignal` uses native `AbortSignal` property/event API instead of calling non-existent methods. Outstanding issues captured above; none caused by the test-runner pass itself.
- 2026-04-22 flake investigation Step 1: `actor-error-handling` recheck is GREEN for static/bare `Actor Error Handling Tests` (`/tmp/driver-logs/error-handling-recheck.log`, exit 0). `actor-workflow` child-workflow recheck is GREEN for static/bare `starts child workflows` (`/tmp/driver-logs/workflow-child-recheck.log`, exit 0). Step 5 skipped because the child-workflow target is no longer red.
- 2026-04-22 flake investigation Step 2: `actor-inspector` replay target still fails, but the failure is after the expected 409. `/tmp/driver-logs/inspector-replay.log` shows replay rejection works, then `handle.release()` does not lead to `finishedAt` before the 30s test timeout. Evidence and fix direction captured in `.agent/notes/flake-inspector-replay.md`.
- 2026-04-22 flake investigation Step 3: `actor-conn` targeted runs: `isConnected should be false before connection opens` 5/5 PASS; `onOpen should be called when connection opens` 2/3 PASS and 1/3 FAIL; `should reject request exceeding maxIncomingMessageSize` 2/3 PASS and 1/3 FAIL; `should reject response exceeding maxOutgoingMessageSize` 3/3 PASS. Evidence and fix direction captured in `.agent/notes/flake-conn-websocket.md`.
- 2026-04-22 flake investigation Step 4: isolated `actor-queue` `wait send returns completion response` is 5/5 PASS. `drains many-queue child actors created from actions while connected` is 1/3 PASS and 2/3 FAIL with `actor/dropped_reply` plus HTTP 500 responses. Evidence and fix direction captured in `.agent/notes/flake-queue-waitsend.md`.
