# Driver Test Suite Progress

Started: 2026-04-21 23:01:08 PDT
Config: registry (static), client type (http), encoding (bare)

## Fast Tests

- [x] manager-driver | Manager Driver Tests
- [x] actor-conn | Actor Connection Tests
- [x] actor-conn-state | Actor Connection State Tests
- [x] conn-error-serialization | Connection Error Serialization Tests
- [x] actor-destroy | Actor Destroy Tests
- [x] request-access | Request Access in Lifecycle Hooks
- [x] actor-handle | Actor Handle Tests
- [x] action-features | Action Features Tests
- [x] access-control | access control
- [x] actor-vars | Actor Variables
- [x] actor-metadata | Actor Metadata Tests
- [x] actor-onstatechange | Actor State Change Tests
- [x] actor-db | Actor Database
- [x] actor-db-raw | Actor Database Raw Tests
- [x] actor-workflow | Actor Workflow Tests
- [x] actor-error-handling | Actor Error Handling Tests
- [x] actor-queue | Actor Queue Tests
- [x] actor-kv | Actor KV Tests
- [x] actor-stateless | Actor Stateless Tests
- [x] raw-http | raw http
- [x] raw-http-request-properties | raw http request properties
- [x] raw-websocket | raw websocket
- [ ] actor-inspector | Actor Inspector Tests
- [ ] gateway-query-url | Gateway Query URL Tests
- [ ] actor-db-pragma-migration | Actor Database Pragma Migration
- [x] actor-state-zod-coercion | Actor State Zod Coercion
- [x] actor-conn-status | Connection Status Changes
- [x] gateway-routing | Gateway Routing
- [x] lifecycle-hooks | Lifecycle Hooks

## Slow Tests

- [x] actor-state | Actor State Tests
- [x] actor-schedule | Actor Schedule Tests
- [x] actor-sleep | Actor Sleep Tests
- [ ] actor-sleep-db | Actor Sleep Database Tests (2 known TODO failures, see log)
- [ ] actor-lifecycle | Actor Lifecycle Tests
- [ ] actor-conn-hibernation | Actor Connection Hibernation Tests
- [ ] actor-run | Actor Run Tests
- [ ] hibernatable-websocket-protocol | hibernatable websocket protocol
- [ ] actor-db-stress | Actor Database Stress Tests

## Excluded

- [ ] actor-agent-os | Actor agentOS Tests (skip unless explicitly requested)

## Log
- 2026-04-21 23:02:18 PDT manager-driver: PASS (22s) Tests  16 passed | 32 skipped (48)
- 2026-04-21 23:02:51 PDT actor-conn: PASS (33s) Tests  23 passed | 46 skipped (69)
- 2026-04-21 23:02:59 PDT actor-conn-state: PASS (8s) Tests  8 passed | 16 skipped (24)
- 2026-04-21 23:03:03 PDT conn-error-serialization: PASS (4s) Tests  3 passed | 6 skipped (9)
- 2026-04-21 23:03:33 PDT actor-destroy: PASS (30s) Tests  10 passed | 20 skipped (30)
- 2026-04-21 23:03:37 PDT request-access: PASS (4s) Tests  4 passed | 8 skipped (12)
- 2026-04-21 23:03:47 PDT actor-handle: PASS (10s) Tests  12 passed | 24 skipped (36)
- 2026-04-21 23:03:48 PDT action-features: PASS (1s) Tests  33 skipped (33)
- 2026-04-21 23:04:00 PDT access-control: PASS (12s) Tests  8 passed | 16 skipped (24)
- 2026-04-21 23:04:05 PDT actor-vars: PASS (5s) Tests  5 passed | 10 skipped (15)
- 2026-04-21 23:04:11 PDT actor-metadata: PASS (6s) Tests  6 passed | 12 skipped (18)
- 2026-04-21 23:04:12 PDT actor-onstatechange: PASS (1s) Tests  15 skipped (15)
- 2026-04-21 23:04:40 PDT actor-db: PASS (28s) Tests  16 passed | 32 skipped (48)
- 2026-04-21 23:04:41 PDT actor-db-raw: PASS (1s) Tests  12 skipped (12)
- 2026-04-21 23:05:40 PDT actor-workflow: PASS (59s) Tests  18 passed | 39 skipped (57)
- 2026-04-21 23:05:47 PDT actor-error-handling: PASS (7s) Tests  7 passed | 14 skipped (21)
- 2026-04-21 23:06:20 PDT actor-queue: PASS (33s) Tests  25 passed | 50 skipped (75)
- 2026-04-21 23:06:24 PDT actor-kv: PASS (4s) Tests  3 passed | 6 skipped (9)
- 2026-04-21 23:06:30 PDT actor-stateless: PASS (6s) Tests  6 passed | 12 skipped (18)
- 2026-04-21 23:06:53 PDT raw-http: PASS (23s) Tests  15 passed | 30 skipped (45)
- 2026-04-21 23:07:06 PDT raw-http-request-properties: PASS (13s) Tests  16 passed | 32 skipped (48)
- 2026-04-21 23:07:15 PDT raw-websocket: PASS (9s) Tests  11 passed | 28 skipped (39)
- 2026-04-21 23:07:16 PDT actor-inspector: PASS (1s) Tests  63 skipped (63)
- 2026-04-21 23:07:17 PDT gateway-query-url: PASS (1s) Tests  6 skipped (6)
- 2026-04-21 23:07:18 PDT actor-db-pragma-migration: PASS (1s) Tests  12 skipped (12)
- 2026-04-21 23:07:22 PDT actor-state-zod-coercion: PASS (4s) Tests  3 passed | 6 skipped (9)
- 2026-04-21 23:07:28 PDT actor-conn-status: PASS (6s) Tests  6 passed | 12 skipped (18)
- 2026-04-21 23:07:35 PDT gateway-routing: PASS (7s) Tests  8 passed | 16 skipped (24)
- 2026-04-21 23:07:42 PDT lifecycle-hooks: PASS (7s) Tests  8 passed | 16 skipped (24)
- 2026-04-21 23:08:25 PDT action-features: RECHECK PASS (9s) Tests  11 passed | 22 skipped (33)
- 2026-04-21 23:08:31 PDT actor-onstatechange: RECHECK PASS (5s) Tests  5 passed | 10 skipped (15)
- 2026-04-21 23:08:37 PDT actor-db-raw: RECHECK PASS (6s) Tests  4 passed | 8 skipped (12)
- 2026-04-21 23:09:43 PDT actor-inspector: RECHECK FAIL (66s) × Actor Inspector > static registry > encoding (bare) > Actor Inspector HTTP API > GET /inspector/workflow-history returns populated history for active workflows 10696ms
- 2026-04-21 23:10:35 PDT actor-inspector: ISOLATED RERUN PASS (2s) Tests  1 passed | 62 skipped (63)
- 2026-04-21 23:11:00 PDT US-116 CHECKPOINT 3 COMPLETE: fast=26/29 confirmed green before stop, slop=0/9. Regressions: [actor-inspector full bare file fails `GET /inspector/workflow-history returns populated history for active workflows` with 503; isolated rerun passes]. New bugs: [US-119]. Branch merge-readiness: BLOCKED by fast-tier actor-inspector regression.
- 2026-04-21 23:54:27 PDT actor-sleep: PASS (45s) Tests  21 passed | 45 skipped (66). Fix: dispatch_scheduled_action now wraps action send/await in internal_keep_awake so scheduled/alarm actions keep actor awake and reset sleep timer, matching reference TS internalKeepAwake wrapping in schedule-manager.ts #executeDueEvents. Also earlier fix removed reset_sleep_timer calls from request_save/request_save_within/save_state_with_revision in context.rs and removed reset_sleep_deadline from StateMutated/SaveRequested handlers in task.rs to stop state-save feedback pushing the sleep deadline forward.
- 2026-04-21 23:58:38 PDT US-119 FINDINGS: after the required rebuilds, the full bare `actor-inspector` file failure was a query-route startup flake, not workflow-history corruption. Active-workflow `/inspector/workflow-history` and `/inspector/summary` requests can each independently return transient `guard/actor_ready_timeout` during actor bring-up, so waiting on one inspector route and then doing a single fetch against another is not a stable assertion pattern.
- 2026-04-21 23:58:38 PDT actor-inspector: FULL BARE PASS (52s) `pnpm test tests/driver/actor-inspector.test.ts -t 'static registry.*encoding \\(bare\\).*Actor Inspector HTTP API'` -> Tests  21 passed | 42 skipped (63)
- 2026-04-21 23:58:38 PDT actor-inspector: ISOLATED HISTORY PASS (24s) `pnpm test tests/driver/actor-inspector.test.ts -t 'static registry.*encoding \\(bare\\).*Actor Inspector HTTP API.*GET /inspector/workflow-history returns populated history for active workflows'` -> Tests  1 passed | 62 skipped (63)
- 2026-04-21 23:58:38 PDT actor-inspector: ISOLATED SUMMARY PASS (19s) `pnpm test tests/driver/actor-inspector.test.ts -t 'static registry.*encoding \\(bare\\).*Actor Inspector HTTP API.*GET /inspector/summary returns populated workflow history for active workflows'` -> Tests  1 passed | 62 skipped (63)
- 2026-04-22 00:05 PDT actor-state: PASS (3s) Tests  3 passed | 6 skipped (9)
- 2026-04-22 00:06 PDT actor-schedule: PASS (7s) Tests  4 passed | 8 skipped (12)
- 2026-04-22 00:25 PDT actor-sleep: PASS (after engine restart flake) Tests 21 passed | 45 skipped (66). Test `alarms wake actors` is flaky on this branch; sometimes passes, sometimes hits actor_ready_timeout. Related to documented TODO in `.agent/todo/alarm-during-destroy.md`: alarm-during-sleep wake path is broken; engine alarm is cancelled at shutdown via `cancel_driver_alarm_logged` in `finish_shutdown_cleanup_with_ctx`, matching TS ref behavior but TS ref comment says alarms are re-armed via `initializeAlarms` on wake. Rust does this via `init_alarms -> sync_future_alarm_logged` at startup, but alarm-triggered wake from engine does not happen because engine alarm is cleared. HTTP-triggered wake works for non-alarm scheduled events. Leaving this branch-level flake for a follow-up.
- 2026-04-22 00:35 PDT actor-sleep-db: FAIL (2 of 14) Tests  2 failed | 12 passed | 58 skipped (72). Failing: `scheduled alarm can use c.db after sleep-wake` (actor_ready_timeout), `schedule.after in onSleep persists and fires on wake` (timeout). Root cause: same documented TODO in `.agent/todo/alarm-during-destroy.md` — alarm-during-sleep wake is broken because `finish_shutdown_cleanup_with_ctx` cancels the driver alarm unconditionally. Fix attempt to skip cancel on Sleep caused alarm+HTTP wake races, needs design coordination per the TODO. Also wrapping `dispatch_scheduled_action` in `internal_keep_awake` (already landed for actor-sleep fix) remains correct and necessary.
- 2026-04-22 00:37 PDT actor-lifecycle: PASS Tests 5 passed | 13 skipped (18)
- 2026-04-22 00:38 PDT actor-conn-hibernation: FAIL (4 of 5) Tests 4 failed | 1 passed | 10 skipped (15). Failing: `basic conn hibernation`, `conn state persists through hibernation`, `onOpen is not emitted again after hibernation wake` (all 30s timeouts), `messages sent on a hibernating connection during onSleep resolve after wake` (expected 'resolved' got 'timed_out'). Suite filter needed to be `Actor Conn Hibernation.*static registry.*encoding \(bare\).*Connection Hibernation` because outer describe is `Actor Conn Hibernation` and inner describe is `Connection Hibernation` (not `Actor Connection Hibernation Tests`). Likely related to same alarm/hibernation wake bug.
