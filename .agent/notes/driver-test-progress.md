# Driver Test Suite Progress

Started: 2026-04-22
Config: registry (static), client type (http), encoding (bare)

## Fast Tests

- [x] manager-driver | Manager Driver Tests
- [!] actor-conn | Actor Connection Tests
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
- [!] raw-websocket | raw websocket
- [x] actor-inspector | Actor Inspector Tests
- [x] gateway-query-url | Gateway Query URL Tests
- [x] actor-db-pragma-migration | Actor Database Pragma Migration
- [x] actor-state-zod-coercion | Actor State Zod Coercion
- [x] actor-conn-status | Connection Status Changes
- [x] gateway-routing | Gateway Routing
- [x] lifecycle-hooks | Lifecycle Hooks

## Slow Tests

- [x] actor-state | Actor State Tests
- [x] actor-schedule | Actor Schedule Tests
- [x] actor-sleep | Actor Sleep Tests
- [x] actor-sleep-db | Actor Sleep Database Tests
- [x] actor-lifecycle | Actor Lifecycle Tests
- [x] actor-conn-hibernation | Actor Connection Hibernation Tests
- [x] actor-run | Actor Run Tests
- [x] hibernatable-websocket-protocol | hibernatable websocket protocol
- [x] actor-db-stress | Actor Database Stress Tests

## Excluded

- [ ] actor-agent-os | Actor agentOS Tests (skip unless explicitly requested)

## Log

- 2026-04-23T03:45:07.364Z manager-driver: PASS (41.0s)
- 2026-04-23T03:46:11.489Z actor-conn: FAIL - FAIL  tests/driver/actor-conn.test.ts > Actor Conn > static registry > encoding (bare) > Actor Connection Tests > Large Payloads > should reject response exceeding maxOutgoingMessageSize
- 2026-04-23T04:07:04.000Z fast parallel: FAIL (280 passed, 5 failed, 579 skipped)
- 2026-04-23T04:07:04.000Z actor-conn: FAIL - Large Payloads > should reject request exceeding maxIncomingMessageSize timed out in 30000ms
- 2026-04-23T04:07:04.000Z actor-conn: FAIL - Large Payloads > should reject response exceeding maxOutgoingMessageSize timed out in 30000ms
- 2026-04-23T04:07:04.000Z actor-inspector: FAIL - POST /inspector/workflow/replay rejects workflows that are currently in flight timed out in 30000ms
- 2026-04-23T04:07:04.000Z actor-workflow: FAIL - workflow steps can destroy the actor. AssertionError: actor still running: expected true to be falsy
- 2026-04-23T04:07:04.000Z conn-error-serialization: FAIL - error thrown in createConnState preserves group and code through WebSocket serialization timed out in 30000ms
- 2026-04-23T04:36:09.000Z slow parallel: FAIL (65 passed, 1 failed, 168 skipped)
- 2026-04-23T04:36:09.000Z actor-sleep-db: FAIL - schedule.after in onSleep persists and fires on wake. AssertionError: expected startCount 2, got 3
- 2026-04-23T04:36:09.000Z hibernatable-websocket-protocol: SKIP - bare/static encoding filter matched no tests
- 2026-04-23T05:03:34.000Z actor-conn: PASS static/http/bare full file (23 passed, 0 failed, 46 skipped)
- 2026-04-23T05:22:55.000Z actor-conn: PASS static/http/bare full file (23 passed, 0 failed, 46 skipped)
- 2026-04-23T05:26:51.000Z conn-error-serialization: PASS full file (9 passed, 0 failed; includes static/http/bare)
- 2026-04-23T05:33:41.000Z actor-inspector: PASS full file (63 passed, 0 failed; includes static/http/bare)
- 2026-04-23T05:44:46.000Z actor-workflow: PASS full file (54 passed, 0 failed, 3 skipped; includes static/http/bare)
- 2026-04-23T06:18:25.000Z actor-sleep-db: PASS full file (42 passed, 0 failed, 30 skipped; includes static/http/bare)
- 2026-04-23T06:33:39.000Z hibernatable-websocket-protocol: PASS full file (6 passed, 0 failed; static/http/bare enabled; raw-websocket full file also passed 39 passed, 0 failed)
- 2026-04-23T06:38:26.000Z DT-008 full-file check: actor-conn FAIL (2 failed, 67 passed) - bare/cbor `should reject response exceeding maxOutgoingMessageSize` timed out in 30000ms; bare-only targeted recheck passed.
- 2026-04-23T06:38:34.000Z DT-008 full-file check: conn-error-serialization PASS (9 passed, 0 failed).
- 2026-04-23T06:39:32.000Z DT-008 full-file check: actor-inspector PASS (63 passed, 0 failed).
- 2026-04-23T06:40:34.000Z DT-008 full-file check: actor-workflow FAIL (3 failed, 51 passed, 3 skipped) - `workflow steps can destroy the actor` still found actor running.
- 2026-04-23T06:42:17.000Z DT-008 full-file check: actor-sleep-db PASS (42 passed, 0 failed, 30 skipped).
- 2026-04-23T06:43:11.000Z DT-008 full-file check: hibernatable-websocket-protocol FAIL (3 failed, 3 passed) - replay ack state was undefined instead of index 1.
- 2026-04-23T06:43:53.000Z DT-008 targeted recheck: actor-conn bare oversized response PASS; actor-workflow bare destroy FAIL; hibernatable bare replay FAIL.
- 2026-04-23T06:58:43.000Z fast parallel: FAIL (281 passed, 6 failed, 577 skipped)
- 2026-04-23T06:58:43.000Z actor-conn: FAIL - Large Payloads > should reject response exceeding maxOutgoingMessageSize timed out in 30000ms.
- 2026-04-23T06:58:43.000Z actor-queue: FAIL - wait send returns completion response timed out in 30000ms.
- 2026-04-23T06:58:43.000Z actor-workflow: FAIL - workflow steps can destroy the actor. AssertionError: actor still running: expected true to be falsy.
- 2026-04-23T06:58:43.000Z conn-error-serialization: FAIL - error thrown in createConnState preserves group and code through WebSocket serialization timed out in 30000ms.
- 2026-04-23T06:58:43.000Z raw-websocket: FAIL - hibernatable websocket ack state was undefined for indexed and threshold buffered ack tests.
- 2026-04-23T07:02:27.000Z slow parallel: FAIL (67 passed, 1 failed, 166 skipped)
- 2026-04-23T07:02:27.000Z hibernatable-websocket-protocol: FAIL - replays only unacked indexed websocket messages after sleep and wake. Ack state was undefined instead of index 1.
- 2026-04-23T07:02:40.000Z typecheck: PASS (`pnpm -F rivetkit check-types`).
- 2026-04-23T11:57:29.000Z serverless-handler: PASS full file (3 passed, 0 failed; static/http/bare). `/start` uses an actor ID created in the same engine namespace as the serverless envoy headers.
- 2026-04-23T12:14:00.000Z DT-008 full-file recheck: FAIL (239 passed, 4 failed, 33 skipped) - actor-conn bare `onOpen should be called when connection opens`; actor-inspector cbor `POST /inspector/database/execute supports named properties`; conn-error-serialization bare/cbor `createConnState preserves group/code` timed out. Follow-up stories: DT-045, DT-046; DT-014 already covers conn-error-serialization.
- 2026-04-23T12:19:11.000Z actor-conn: PASS DT-011 recheck. Targeted bare oversized response passed; full actor-conn file passed (69 passed, 0 failed); parallel bare actor-conn suite passed (23 passed, 0 failed, 46 skipped).
- 2026-04-23T12:22:37.000Z actor-inspector: PASS DT-046 recheck. Targeted CBOR named-properties inspector database execute passed; full actor-inspector file passed (63 passed, 0 failed).
- 2026-04-23T12:26:06.000Z actor-conn: PASS DT-045 recheck. Targeted bare onOpen passed; full actor-conn file passed (69 passed, 0 failed).
- 2026-04-23T12:40:56.000Z actor-queue: PASS DT-012. Fixed core enqueue-and-wait waiter registration race; targeted bare wait-send passed; full actor-queue file passed (75 passed, 0 failed); parallel bare actor-queue suite passed (25 passed, 0 failed, 50 skipped).
- 2026-04-23T13:06:56.000Z conn-error-serialization: PASS DT-014. Targeted bare createConnState error passed; full conn-error-serialization file passed (9 passed, 0 failed); parallel bare conn-error-serialization suite passed (3 passed, 0 failed, 6 skipped).
- 2026-04-23T13:11:10.000Z actor-workflow: PASS DT-013 recheck. Targeted bare workflow destroy passed; full actor-workflow file passed (54 passed, 0 failed, 3 skipped); parallel bare actor-workflow suite passed (18 passed, 0 failed, 39 skipped).
- 2026-04-23T13:23:10.000Z DT-008 full-file recheck: FAIL (240 passed, 3 failed, 33 skipped) - actor-conn bare `isConnected should be false before connection opens` failed at `tests/driver/actor-conn.test.ts:419`; conn-error-serialization bare/cbor `createConnState preserves group/code` timed out at `tests/driver/conn-error-serialization.test.ts:7`. Follow-up stories: DT-047, DT-048.
- 2026-04-23T13:38:30.000Z conn-error-serialization: PASS DT-048. Root cause was a stale NAPI artifact older than `rivetkit-core/src/registry/websocket.rs`; `pnpm --filter @rivetkit/rivetkit-napi build:force` refreshed the native bridge. Targeted bare and CBOR createConnState error tests passed; full conn-error-serialization file passed (9 passed, 0 failed); six-file DT-008 verifier showed conn-error-serialization passing across bare/CBOR/JSON and remains blocked only by DT-047 actor-conn (242 passed, 1 failed, 33 skipped).
- 2026-04-23T13:51:44.000Z actor-conn: PASS DT-047. Targeted bare `isConnected should be false before connection opens` passed; full actor-conn file passed (69 passed, 0 failed); six-file DT-008 verifier showed actor-conn green across bare/CBOR/JSON.
- 2026-04-23T13:51:44.000Z conn-error-serialization: FAIL - DT-047 six-file verifier failed static/CBOR `createConnState preserves group/code` with `Error: Test timed out in 30000ms`; reopened DT-048.
- 2026-04-23T14:04:08.000Z DT-008 full-file recheck: FAIL (241 passed, 2 failed, 33 skipped) - conn-error-serialization JSON `createConnState preserves group/code` timed out at `tests/driver/conn-error-serialization.test.ts:7`; actor-sleep-db JSON `nested waitUntil inside waitUntil is drained before shutdown` failed at `tests/driver/actor-sleep-db.test.ts:463` with `RivetError: Request timed out after 15 seconds.` Updated DT-048 for JSON coverage and added DT-049.
- 2026-04-23T14:04:08.000Z hibernatable-websocket-protocol: PASS DT-008 recheck (6 passed, 0 failed across bare/CBOR/JSON).
- 2026-04-23T14:21:20.000Z actor-sleep-db: PASS DT-049. Targeted JSON nested waitUntil passed; full actor-sleep-db file passed (42 passed, 0 failed, 30 skipped); six-file DT-008 verifier showed actor-sleep-db green across bare/CBOR/JSON.
- 2026-04-23T14:21:20.000Z actor-workflow: FAIL - static/CBOR `starts child workflows created inside workflow steps` failed at `tests/driver/actor-workflow.test.ts:173`; child result was `timedOut` instead of completed. Follow-up story: DT-050.
- 2026-04-23T14:32:45.000Z DT-008 full-file recheck: FAIL (241 passed, 2 failed, 33 skipped) - conn-error-serialization JSON `createConnState preserves group/code` timed out at `tests/driver/conn-error-serialization.test.ts:7`; actor-workflow JSON `starts child workflows created inside workflow steps` failed at `tests/driver/actor-workflow.test.ts:173` with child result `timedOut` instead of completed. Existing stories cover both failures: DT-048 and DT-050.
- 2026-04-23T14:55:02.000Z DT-008 full-file recheck: FAIL (240 passed, 3 failed, 33 skipped) - conn-error-serialization bare/CBOR/JSON `createConnState preserves group/code` timed out at `tests/driver/conn-error-serialization.test.ts:7`. Existing story DT-048 covers the failure. Actor-workflow passed in this combined verifier run (57 tests, 3 skipped).
- 2026-04-23T15:18:32.000Z conn-error-serialization: PASS DT-048. Targeted bare/CBOR/JSON createConnState error checks passed; full conn-error-serialization file passed (9 passed, 0 failed); six-file DT-008 verifier showed conn-error-serialization green across bare/CBOR/JSON.
- 2026-04-23T15:18:32.000Z actor-conn: FAIL - DT-048 six-file verifier failed static/bare `isConnected should be false before connection opens` at `tests/driver/actor-conn.test.ts:419` with `AssertionError: expected false to be true // Object.is equality`; reopened DT-047.
