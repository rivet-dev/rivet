# Driver Test Suite Progress

Started: 2026-04-18T04:53:02Z
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
- [x] actor-onstatechange | Actor State Change Tests
- [x] actor-db | Actor Database
- [x] actor-db-raw | Actor Database Raw Tests
- [x] actor-workflow | Actor Workflow Tests
- [x] actor-error-handling | Actor Error Handling Tests
- [x] actor-queue | Actor Queue Tests
- [x] actor-inline-client | Actor Inline Client Tests
- [x] actor-kv | Actor KV Tests
- [x] actor-stateless | Actor Stateless Tests
- [x] raw-http | raw http
- [x] raw-http-request-properties | raw http request properties
- [x] raw-websocket | raw websocket
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
- [x] hibernatable-websocket-protocol | hibernatable websocket protocol (skipped: feature-gated off for this driver config)
- [x] actor-db-stress | Actor Database Stress Tests

## Excluded

- [ ] actor-agent-os | Actor agentOS Tests (skip unless explicitly requested)
- [ ] cross-backend-vfs | Cross-Backend VFS Compatibility Tests (skip unless explicitly requested)

## Log
- 2026-04-18T04:55:32Z manager-driver: FAIL - multi-part actor keys with slashes collapse into a single escaped key component
- 2026-04-18T05:02:09Z manager-driver: PASS (16 tests, 108.05s)
- 2026-04-18T05:05:35Z actor-conn: FAIL - exit 0
- 2026-04-18T07:33:46Z actor-conn: PASS (23 tests, 157.33s)
- 2026-04-18T07:34:54Z actor-conn-state: PASS (8 tests, 55.75s)
- 2026-04-18T07:37:14Z conn-error-serialization: FAIL - createConnState websocket error lost structured group/code and surfaced actor.js_callback_failed
- 2026-04-18T07:37:14Z conn-error-serialization: PASS (2 tests, 14.47s)
- 2026-04-18T07:48:09Z actor-destroy: FAIL - raw HTTP actor requests kept the guard `/request` prefix, breaking stale getOrCreate fetch after destroy
- 2026-04-18T07:48:09Z actor-destroy: FAIL - transient driver-test setup error (`namespace.not_found`) while upserting runner config
- 2026-04-18T07:48:09Z actor-destroy: PASS (10 tests, 70.77s)
- 2026-04-18T08:01:06Z request-access: FAIL - native contexts dropped `c.request` and stateless HTTP actions skipped `onBeforeConnect`/`createConnState`
- 2026-04-18T08:01:06Z request-access: PASS (4 tests, 27.91s)
- 2026-04-18T08:02:53Z actor-handle: PASS (12 tests, 80.87s)
- 2026-04-18T08:07:51Z action-features: FAIL - native HTTP actions bypassed timeout and message-size enforcement
- 2026-04-18T08:07:51Z action-features: PASS (11 tests, 74.46s)
- 2026-04-18T08:54:15Z access-control: FAIL - transient driver-test setup error (`namespace.not_found`) while upserting runner config
- 2026-04-18T08:54:15Z access-control: PASS (8 tests, 62.68s)
- 2026-04-18T08:55:10Z actor-vars: PASS (5 tests, 37.52s)
- 2026-04-18T08:56:13Z actor-metadata: PASS (6 tests, 46.59s)
- 2026-04-18T09:06:26Z actor-onstatechange: PASS (5 tests, 38.05s)
- 2026-04-18T09:09:24Z actor-db: PASS (16 tests, 130.76s)
- 2026-04-18T09:11:24Z actor-db-raw: FAIL - transient driver-test setup error (`namespace.not_found`) while upserting runner config
- 2026-04-18T09:12:12Z actor-db-raw: FAIL - transient driver-test setup error (`namespace.not_found`) while upserting runner config
- 2026-04-18T09:13:17Z actor-db-raw: PASS (4 tests, 32.34s)
- 2026-04-18T09:16:54Z actor-workflow: FAIL - native workflow runtime never entered the old TypeScript workflow host path, so queue polling, step execution, and onError hooks stayed inert
- 2026-04-18T09:29:48Z actor-workflow: FAIL - transient driver-test setup error (`namespace.not_found`) while upserting runner config after workflow runtime parity fix
- 2026-04-18T09:32:34Z actor-workflow: PASS (19 tests, 150.79s)
- 2026-04-18T09:33:51Z actor-error-handling: FAIL - native callback bridge leaked raw internal exception text instead of RivetKit's safe internal error description
- 2026-04-18T09:39:51Z actor-error-handling: PASS (7 tests, 49.42s)
- 2026-04-18T10:05:18Z actor-queue: PASS (25 tests, 201.40s)
- 2026-04-18T10:06:18Z actor-inline-client: PASS (5 tests, 40.30s)
- 2026-04-18T10:11:07Z actor-kv: FAIL - native user-facing KV adapter returned raw bytes, used inclusive envoy range scans, and leaked internal runtime keys instead of the original TypeScript ActorKv contract
- 2026-04-18T10:12:07Z actor-kv: PASS (3 tests, 23.29s)
- 2026-04-18T10:18:37Z actor-stateless: FAIL - native stateless action contexts still exposed c.state through the direct HTTP action path instead of throwing StateNotEnabled like the original TypeScript runtime
- 2026-04-18T10:20:11Z actor-stateless: PASS (6 tests, 46.64s)
- 2026-04-18T10:24:21Z raw-http: FAIL - native onRequest treated void returns as implicit 204 instead of surfacing the original TypeScript 500 error; the other reported raw-http failure was a transient namespace.not_found setup error
- 2026-04-18T10:25:04Z raw-http: PASS - exact rerun of the previously failing raw-http cases passed after fixing void-return handling
- 2026-04-18T10:24:58Z raw-http-request-properties: PASS (16 tests, 118.92s)
- 2026-04-18T11:23:31Z raw-websocket: PASS (12 tests, 82.54s)
- 2026-04-18T12:01:18Z actor-inspector: PASS (21 tests, 153.11s)
- 2026-04-18T12:01:50Z gateway-query-url: PASS (2 tests, 15.53s)
- 2026-04-18T12:02:28Z actor-db-pragma-migration: PASS (4 tests, 30.86s)
- 2026-04-18T12:02:58Z actor-state-zod-coercion: PASS (3 tests, 24.88s)
- 2026-04-18T12:03:52Z actor-conn-status: PASS (6 tests, 44.91s)
- 2026-04-18T12:04:59Z gateway-routing: PASS (8 tests, 59.31s)
- 2026-04-18T12:05:11Z lifecycle-hooks: FAIL - client ActorHandle.connect() silently dropped explicit conn params, so onBeforeConnect reject paths never saw `{ shouldReject/shouldFail }`
- 2026-04-18T12:06:06Z lifecycle-hooks: PASS (7 tests, 50.17s)
- 2026-04-18T12:06:36Z actor-state: PASS (3 tests, 22.23s)
- 2026-04-18T12:07:20Z actor-schedule: PASS (4 tests, 33.07s)
- 2026-04-18T12:26:00Z actor-sleep: FAIL - one transient namespace.not_found setup miss, plus real raw-websocket timing drift: async message/close handlers now keep the actor awake, but client-side raw websocket close still lands ~105ms late so the 250ms handlers finish before the first 175ms assertion window
- 2026-04-18T12:47:06Z actor-sleep: PASS (22 tests, 185.08s) - fixed raw websocket close timing drift by removing the extra 100ms close linger and hardened slow-suite bootstrap/timeouts against transient engine startup lag
- 2026-04-18T14:40:58Z actor-sleep-db: PASS (24 tests, 217.45s) - fixed actor-connect websocket shutdown parity so server-side conn.disconnect closes the transport instead of leaving zombie sockets during sleep
- 2026-04-18T16:47:36Z actor-lifecycle: PASS (6 tests, 43.27s) - fixed native destroy dispatch timing so concurrent startup teardown no longer leaves stale handlers stuck waiting for ready
- 2026-04-18T17:26:40Z actor-conn-hibernation: PASS (5 tests, 40.45s) - restored wake-time envoy websocket rebinding and native hibernatable inbound-message persistence/acks so the gateway stops replaying stale actor-connect frames after sleep
- 2026-04-18T17:28:22Z actor-run: PASS (8 tests, 64.90s)
- 2026-04-18T17:29:53Z hibernatable-websocket-protocol: SKIP - suite is feature-gated off (`driverTestConfig.features?.hibernatableWebSocketProtocol` is falsy) for this static registry http/bare driver config
- 2026-04-18T17:30:11Z actor-db-stress: PASS (3 tests, 23.20s)
