# Driver Test Suite Progress

Started: 2026-04-26T14:05:00-07:00
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
- [ ] actor-db | Actor Database
- [ ] actor-db-raw | Actor Database Raw Tests
- [ ] actor-db-init-order | Actor DB Init Order Tests
- [ ] actor-workflow | Actor Workflow Tests
- [ ] actor-error-handling | Actor Error Handling Tests
- [ ] actor-queue | Actor Queue Tests
- [ ] actor-kv | Actor KV Tests
- [ ] actor-stateless | Actor Stateless Tests
- [ ] raw-http | raw http
- [ ] raw-http-request-properties | raw http request properties
- [ ] raw-websocket | raw websocket
- [ ] actor-inspector | Actor Inspector Tests
- [ ] gateway-query-url | Gateway Query URL Tests
- [ ] actor-db-pragma-migration | Actor Database Pragma Migration
- [ ] actor-state-zod-coercion | Actor State Zod Coercion
- [ ] actor-save-state | Actor Save State Tests
- [ ] actor-conn-status | Connection Status Changes
- [ ] gateway-routing | Gateway Routing
- [ ] lifecycle-hooks | Lifecycle Hooks
- [ ] serverless-handler | Serverless Handler Tests

## Slow Tests

- [ ] actor-state | Actor State Tests
- [ ] actor-schedule | Actor Schedule Tests
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

- 2026-04-26T14:06:57-07:00 manager-driver: PASS

- 2026-04-26T14:07:27-07:00 actor-conn: PASS

- 2026-04-26T14:07:37-07:00 actor-conn-state: PASS

- 2026-04-26T14:07:42-07:00 conn-error-serialization: PASS

- 2026-04-26T14:08:14-07:00 actor-destroy: PASS

- 2026-04-26T14:08:19-07:00 request-access: PASS

- 2026-04-26T14:08:31-07:00 actor-handle: PASS

- 2026-04-26T14:08:31-07:00 action-features: PASS

- 2026-04-26T14:08:46-07:00 access-control: PASS

- 2026-04-26T14:08:51-07:00 actor-vars: PASS

- 2026-04-26T14:08:58-07:00 actor-metadata: PASS

- 2026-04-26T14:08:59-07:00 actor-onstatechange: PASS

- 2026-04-26T14:10:59-07:00 actor-db: FAIL (exit 124)

- 2026-04-26T14:12:00-07:00 runner: stale suite-description filters found for action-features, actor-onstatechange, actor-db, gateway-query-url, and likely other renamed suites; switching to per-file bare filter.

- 2026-04-26T14:12:54-07:00 action-features: PASS (bare file filter)

- 2026-04-26T14:12:59-07:00 actor-onstatechange: PASS (bare file filter)

- 2026-04-26T14:17:33-07:00 actor-db: FAIL (exit 1, bare file filter)

- 2026-05-02T00:19:36-07:00 actor-conn-state: FAIL - new onConnect send regression timed out before sender wiring fix.

- 2026-05-02T00:24:56-07:00 actor-conn-state: PASS (static/bare file filter, 9 tests).

- 2026-05-02T02:26:38-07:00 actor-conn-state: PASS (static/bare file filter with c.conns onConnect send, 9 tests).

- 2026-05-02T02:55:45-07:00 actor-conn-state: PASS (static/bare file filter with explicit pre-await onConnect subscription regression, 9 tests).
