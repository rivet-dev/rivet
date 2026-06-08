# Driver Test Suite Progress

Started: 2026-06-03T09:23:00-07:00
Config: registry (static), encoding (bare), runtimes (native, wasm)

Each row: `[native] [wasm] <file> | <suite description>`

## Fast Tests

- [x] [x] manager-driver | Manager Driver Tests
- [x] [x] actor-conn | Actor Connection Tests
- [x] [x] actor-conn-state | Actor Connection State Tests
- [x] [x] conn-error-serialization | Connection Error Serialization Tests
- [x] [x] actor-destroy | Actor Destroy Tests
- [x] [x] request-access | Request Access in Lifecycle Hooks
- [x] [x] actor-handle | Actor Handle Tests
- [x] [x] action-features | Action Features Tests
- [x] [x] access-control | access control
- [x] [x] actor-vars | Actor Variables
- [x] [x] actor-metadata | Actor Metadata Tests
- [x] [x] actor-onstatechange | Actor State Change Tests
- [x] [x] actor-db | Actor Database
- [x] [x] actor-db-raw | Actor Database Raw Tests
- [x] [x] actor-db-init-order | Actor Db Init Order
- [x] [x] actor-workflow | Actor Workflow Tests
- [x] [x] actor-error-handling | Actor Error Handling Tests
- [x] [x] actor-queue | Actor Queue Tests
- [x] [x] actor-kv | Actor KV Tests
- [x] [x] actor-stateless | Actor Stateless Tests
- [x] [x] raw-http | raw http
- [x] [x] raw-http-request-properties | raw http request properties
- [x] [x] raw-websocket | raw websocket
- [x] [x] actor-inspector | Actor Inspector Tests
- [x] [x] gateway-query-url | Gateway Query URL Tests
- [x] [x] actor-db-pragma-migration | Actor Database Pragma Migration
- [x] [x] actor-state-zod-coercion | Actor State Zod Coercion
- [x] [x] actor-conn-status | Connection Status Changes
- [x] [x] gateway-routing | Gateway Routing
- [x] [x] lifecycle-hooks | Lifecycle Hooks
- [x] [x] serverless-handler | Serverless Handler Tests

## Slow Tests

- [x] [x] actor-state | Actor State Tests
- [x] [x] actor-save-state | Actor Save State Tests
- [x] [x] actor-schedule | Actor Schedule Tests
- [x] [x] actor-sleep | Actor Sleep Tests
- [x] [x] actor-sleep-db | Actor Sleep Database Tests
- [x] [x] actor-lifecycle | Actor Lifecycle Tests
- [x] [x] actor-conn-hibernation | Actor Connection Hibernation Tests
- [x] [x] actor-run | Actor Run Tests
- [x] [x] hibernatable-websocket-protocol | hibernatable websocket protocol
- [x] [x] actor-db-stress | Actor Database Stress Tests

## Excluded

- [ ] [ ] actor-agent-os | Actor agentOS Tests (skip unless explicitly requested)

## Log

- 2026-06-03T09:15:45-07:00 manager-driver [native]: PASS (16 tests, 16.7s)

- 2026-06-03T09:16:18-07:00 manager-driver [wasm]: PASS (16 tests, 13.9s)

- 2026-06-03T09:16:50-07:00 actor-conn [native]: PASS (32s)

- 2026-06-03T09:17:19-07:00 actor-conn [wasm]: PASS (29s)

- 2026-06-03T09:17:34-07:00 actor-conn-state [native]: PASS (15s)

- 2026-06-03T09:17:46-07:00 actor-conn-state [wasm]: PASS (12s)

- 2026-06-03T09:17:51-07:00 conn-error-serialization [native]: FAIL - exit 1 after 5s

- 2026-06-03T09:19:32-07:00 conn-error-serialization [native]: PASS after user error public-boundary fix

- 2026-06-03T09:20:22-07:00 conn-error-serialization [wasm]: PASS after user error public-boundary fix

- 2026-06-03T09:20:50-07:00 actor-destroy [native]: FAIL - exit 1 after 28s

- 2026-06-03T09:22:25-07:00 actor-destroy [native]: PASS after raw websocket dynamic query-target fix

- 2026-06-03T09:23:07-07:00 actor-destroy [wasm]: PASS

- 2026-06-03T09:23:13-07:00 request-access [native]: PASS (6s)

- 2026-06-03T09:23:19-07:00 request-access [wasm]: PASS (6s)

- 2026-06-03T09:23:33-07:00 actor-handle [native]: PASS (14s)

- 2026-06-03T09:23:44-07:00 actor-handle [wasm]: PASS (11s)

- 2026-06-03T09:23:58-07:00 action-features [native]: PASS (14s)

- 2026-06-03T09:24:10-07:00 action-features [wasm]: PASS (12s)

- 2026-06-03T09:24:25-07:00 access-control [native]: PASS (15s)

- 2026-06-03T09:24:38-07:00 access-control [wasm]: PASS (13s)

- 2026-06-03T09:24:44-07:00 actor-vars [native]: PASS (6s)

- 2026-06-03T09:24:50-07:00 actor-vars [wasm]: PASS (6s)

- 2026-06-03T09:24:58-07:00 actor-metadata [native]: PASS (8s)

- 2026-06-03T09:25:05-07:00 actor-metadata [wasm]: PASS (7s)

- 2026-06-03T09:25:14-07:00 actor-onstatechange [native]: PASS (9s)

- 2026-06-03T09:25:22-07:00 actor-onstatechange [wasm]: PASS (8s)

- 2026-06-03T09:25:41-07:00 actor-db [native]: PASS (19s)

- 2026-06-03T09:25:58-07:00 actor-db [wasm]: PASS (17s)

- 2026-06-03T09:26:05-07:00 actor-db-raw [native]: PASS (7s)

- 2026-06-03T09:26:11-07:00 actor-db-raw [wasm]: PASS (6s)

- 2026-06-03T09:26:19-07:00 actor-db-init-order [native]: PASS (8s)

- 2026-06-03T09:26:26-07:00 actor-db-init-order [wasm]: PASS (7s)

- 2026-06-03T09:27:10-07:00 actor-workflow [native]: FAIL - exit 1 after 44s

- 2026-06-03T09:30:20-07:00 actor-workflow [native]: PASS after restoring workflow step state semantics

- 2026-06-03T09:31:00-07:00 actor-workflow [wasm]: PASS

- 2026-06-03T09:31:10-07:00 actor-error-handling [native]: PASS (10s)

- 2026-06-03T09:31:18-07:00 actor-error-handling [wasm]: PASS (8s)

- 2026-06-03T09:31:56-07:00 actor-queue [native]: PASS (38s)

- 2026-06-03T09:32:35-07:00 actor-queue [wasm]: PASS (39s)

- 2026-06-03T09:32:40-07:00 actor-kv [native]: PASS (5s)

- 2026-06-03T09:32:44-07:00 actor-kv [wasm]: PASS (4s)

- 2026-06-03T09:32:51-07:00 actor-stateless [native]: PASS (7s)

- 2026-06-03T09:32:57-07:00 actor-stateless [wasm]: PASS (6s)

- 2026-06-03T09:33:14-07:00 raw-http [native]: PASS (17s)

- 2026-06-03T09:33:27-07:00 raw-http [wasm]: PASS (13s)

- 2026-06-03T09:33:44-07:00 raw-http-request-properties [native]: PASS (17s)

- 2026-06-03T09:33:58-07:00 raw-http-request-properties [wasm]: PASS (14s)

- 2026-06-03T09:34:16-07:00 raw-websocket [native]: PASS (18s)

- 2026-06-03T09:34:29-07:00 raw-websocket [wasm]: PASS (13s)

- 2026-06-03T09:34:52-07:00 actor-inspector [native]: PASS (23s)

- 2026-06-03T09:35:12-07:00 actor-inspector [wasm]: PASS (20s)

- 2026-06-03T09:35:15-07:00 gateway-query-url [native]: PASS (3s)

- 2026-06-03T09:35:18-07:00 gateway-query-url [wasm]: PASS (3s)

- 2026-06-03T09:35:24-07:00 actor-db-pragma-migration [native]: PASS (6s)

- 2026-06-03T09:35:29-07:00 actor-db-pragma-migration [wasm]: PASS (5s)

- 2026-06-03T09:35:33-07:00 actor-state-zod-coercion [native]: FAIL - exit 1 after 4s

- 2026-06-03T09:43:47-07:00 actor-state-zod-coercion [native]: PASS after final onSleep save fix

- 2026-06-03T09:43:58-07:00 actor-state-zod-coercion [wasm]: PASS after final onSleep save fix

- 2026-06-03T09:44:36-07:00 actor-conn-status [native]: PASS (8s)

- 2026-06-03T09:44:43-07:00 actor-conn-status [wasm]: PASS (7s)

- 2026-06-03T09:44:53-07:00 gateway-routing [native]: PASS (10s)

- 2026-06-03T09:45:02-07:00 gateway-routing [wasm]: PASS (9s)

- 2026-06-03T09:45:11-07:00 lifecycle-hooks [native]: PASS (9s)

- 2026-06-03T09:45:19-07:00 lifecycle-hooks [wasm]: PASS (8s)

- 2026-06-03T09:45:22-07:00 serverless-handler [native]: PASS (3s)

- 2026-06-03T09:45:25-07:00 serverless-handler [wasm]: PASS (3s)

- 2026-06-03T09:45:45-07:00 actor-state [native]: PASS (6s)

- 2026-06-03T09:45:48-07:00 actor-state [wasm]: PASS (3s)

- 2026-06-03T09:45:52-07:00 actor-save-state [native]: PASS (4s)

- 2026-06-03T09:45:55-07:00 actor-save-state [wasm]: PASS (3s)

- 2026-06-03T09:46:03-07:00 actor-schedule [native]: PASS (8s)

- 2026-06-03T09:46:11-07:00 actor-schedule [wasm]: PASS (8s)

- 2026-06-03T09:47:14-07:00 actor-sleep [native]: FAIL - exit 1 after 63s

- 2026-06-03T09:53:06-07:00 actor-sleep [native]: PASS after deferred sleep cleanup/save fix

- 2026-06-03T09:54:01-07:00 actor-sleep [wasm]: PASS (55s)

- 2026-06-03T09:55:08-07:00 actor-sleep-db [native]: FAIL - exit 1 after 67s

- 2026-06-03T09:57:10-07:00 actor-sleep-db [native]: PASS after deferred state cleanup but immediate DB close fix

- 2026-06-03T09:58:27-07:00 actor-sleep-db [wasm]: PASS (77s)

- 2026-06-03T09:59:05-07:00 actor-lifecycle [native]: PASS (38s)

- 2026-06-03T09:59:39-07:00 actor-lifecycle [wasm]: PASS (34s)

- 2026-06-03T09:59:51-07:00 actor-conn-hibernation [native]: FAIL - exit 1 after 12s

- 2026-06-03T10:03:58-07:00 actor-conn-hibernation [native] passed after deferring hibernatable websocket actions before dispatch once sleep is requested.

- 2026-06-03T10:04:23-07:00 actor-conn-hibernation [wasm] passed.

- 2026-06-03T10:04:52-07:00 actor-run [native] passed.

- 2026-06-03T10:05:09-07:00 actor-run [wasm] passed.

- 2026-06-03T10:05:16-07:00 hibernatable-websocket-protocol [native] passed.

- 2026-06-03T10:05:17-07:00 hibernatable-websocket-protocol [wasm] passed.

- 2026-06-03T10:05:56-07:00 actor-db-stress [native] passed.

- 2026-06-03T10:07:05-07:00 actor-db-stress [wasm] passed.
