# Driver Test Suite Progress

Started: 2026-04-21
Config: registry (static), client type (http), encoding (bare)

## Fast Tests

- [x] manager-driver | Manager Driver Tests
- [x] actor-conn | Actor Connection Tests
- [x] actor-conn-state | Actor Connection State Tests
- [x] conn-error-serialization | Connection Error Serialization Tests
- [x] actor-destroy | Actor Destroy Tests
- [x] request-access | Request Access in Lifecycle Hooks
- [x] actor-handle | Actor Handle Tests
- [x] action-features | Action Features
- [x] access-control | access control
- [x] actor-vars | Actor Variables
- [x] actor-metadata | Actor Metadata Tests
- [x] actor-onstatechange | Actor onStateChange Tests
- [x] actor-db | Actor Database
- [x] actor-db-raw | Actor Database (Raw) Tests
- [x] actor-workflow | Actor Workflow Tests
- [x] actor-error-handling | Actor Error Handling Tests
- [x] actor-queue | Actor Queue Tests
- [x] actor-kv | Actor KV Tests
- [x] actor-stateless | Actor Stateless Tests
- [x] raw-http | raw http
- [x] raw-http-request-properties | raw http request properties
- [x] raw-websocket | raw websocket
- [x] actor-inspector | Actor Inspector HTTP API
- [x] gateway-query-url | Gateway Query URLs
- [x] actor-db-pragma-migration | Actor Database PRAGMA Migration Tests
- [x] actor-state-zod-coercion | Actor State Zod Coercion Tests
- [x] actor-conn-status | Connection Status Changes
- [x] gateway-routing | Gateway Routing
- [x] lifecycle-hooks | Lifecycle Hooks

## Slow Tests

- [x] actor-state | Actor State Tests
- [x] actor-schedule | Actor Schedule Tests
- [ ] actor-sleep | Actor Sleep Tests
- [ ] actor-sleep-db | Actor Sleep Database Tests
- [ ] actor-lifecycle | Actor Lifecycle Tests
- [ ] actor-conn-hibernation | Actor Connection Hibernation Tests
- [ ] actor-run | Actor Run Tests
- [ ] hibernatable-websocket-protocol | hibernatable websocket protocol
- [ ] actor-db-stress | Actor Database Stress Tests

## Excluded

- [ ] actor-agent-os | Actor agentOS Tests (skip unless explicitly requested)

## Log

- 2026-04-21 manager-driver: PASS (16 tests, 32 skipped, 23s)
- 2026-04-21 actor-conn: PASS on rerun (23 tests, 46 skipped). Flaky once: `onClose...via dispose` (cold-start waitFor timeout), then `should unsubscribe from events` (waitFor hook timeout). Both pass in isolation; cleared on full-suite rerun.
- 2026-04-21 actor-conn-state: PASS (8 tests, 16 skipped)
- 2026-04-21 conn-error-serialization: PASS (3 tests, 6 skipped)
- 2026-04-21 actor-destroy: PASS (10 tests, 20 skipped)
- 2026-04-21 request-access: PASS (4 tests, 8 skipped)
- 2026-04-21 actor-handle: PASS (12 tests, 24 skipped)
- 2026-04-21 action-features: PASS (11 tests, 22 skipped). Note: suite description is `Action Features`, not `Action Features Tests` — skill mapping is stale.
- 2026-04-21 access-control: PASS (8 tests, 16 skipped)
- 2026-04-21 actor-vars: PASS (5 tests, 10 skipped)
- 2026-04-21 actor-metadata: PASS (6 tests, 12 skipped)
- 2026-04-21 actor-onstatechange: PASS (5 tests, 10 skipped). Note: describe is `Actor onStateChange Tests` (lowercase `on`), not `Actor State Change Tests`.
- 2026-04-21 actor-db: PASS on rerun (16 tests, 32 skipped). Flaky once: `supports shrink and regrow workloads with vacuum` → `An internal error occurred` during `insertPayloadRows`. Passed in isolation and on rerun.
- 2026-04-21 actor-db-raw: PASS (4 tests, 8 skipped). Describe is `Actor Database (Raw) Tests` (parens in name).
- 2026-04-21 actor-workflow: PASS on rerun (18 tests, 39 skipped). Flaky once: `tryStep and try recover terminal workflow failures` → `no_envoys`. Passed in isolation + rerun.
- 2026-04-21 actor-error-handling: PASS (7 tests, 14 skipped)
- 2026-04-21 actor-queue: PASS (25 tests, 50 skipped)
- 2026-04-21 actor-kv: PASS (3 tests, 6 skipped)
- 2026-04-21 actor-stateless: PASS (6 tests, 12 skipped)
- 2026-04-21 raw-http: PASS (15 tests, 30 skipped)
- 2026-04-21 raw-http-request-properties: PASS (16 tests, 32 skipped)
- 2026-04-21 raw-websocket: PASS (11 tests, 28 skipped)
- 2026-04-21 actor-inspector: PASS (21 tests, 42 skipped). Describe is `Actor Inspector HTTP API`.
- 2026-04-21 gateway-query-url: PASS (2 tests, 4 skipped). Describe is `Gateway Query URLs`.
- 2026-04-21 actor-db-pragma-migration: PASS (4 tests, 8 skipped). Describe is `Actor Database PRAGMA Migration Tests`.
- 2026-04-21 actor-state-zod-coercion: PASS (3 tests, 6 skipped)
- 2026-04-21 actor-conn-status: PASS (6 tests, 12 skipped)
- 2026-04-21 gateway-routing: PASS (8 tests, 16 skipped)
- 2026-04-21 lifecycle-hooks: PASS (8 tests, 16 skipped)
- 2026-04-21 FAST TESTS COMPLETE
- 2026-04-21 actor-state: PASS (3 tests, 6 skipped)
- 2026-04-21 actor-schedule: PASS (4 tests, 8 skipped)
- 2026-04-21 actor-sleep: FAIL (4 failed, 17 passed, 45 skipped, 66 total). Re-ran after `pnpm --filter @rivetkit/rivetkit-napi build:force` — same 4 failures:
  - `actor automatically sleeps after timeout` (line 193): sleepCount=0, expected 1
  - `actor automatically sleeps after timeout with connect` (line 222): sleepCount=0, expected 1
  - `alarms wake actors` (line 383): sleepCount=0, expected 1
  - `long running rpcs keep actor awake` (line 427): sleepCount=0, expected 1
  Common pattern: every failing test expects the actor to sleep after SLEEP_TIMEOUT (1000ms) + 250ms of idle time. Actor never calls `onSleep` (sleepCount stays 0). Tests that use explicit keep-awake or preventSleep/noSleep paths all pass. Likely regression in the idle-timer-triggered sleep path introduced by the uncommitted task-model migration changes in `rivetkit-rust/packages/rivetkit-core/src/actor/sleep.rs` + `task.rs`.

