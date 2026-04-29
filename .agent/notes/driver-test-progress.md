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

- 2026-04-28T03:01:07-07:00 actor-sleep: FAIL focused repro `waitUntil accepts promises that resolve to undefined`. Native NAPI logs `actor wait_until promise rejected` with `InvalidArg: undefined cannot be represented as a serde_json::Value` after `triggerWaitUntilVoid`; `triggerWaitUntilWithValue` did not reproduce locally with this checkout.

- 2026-04-28T03:02:10-07:00 actor-sleep: FAIL focused repro updated to exact `counterWaitUntilProbe` shape. Failure occurs on first action `triggerWaitUntilVoid`, before the value and rejection controls run.

- 2026-04-28T03:05:41-07:00 actor-sleep: PASS after native waitUntil bridge normalization. Focused `waitUntil`/`keepAwake` bridge tests pass, and full bare `Actor Sleep Tests` passed (21 passed, 45 skipped).

- 2026-04-28T03:59:04-07:00 raw-websocket: PASS focused native `onWebSocket` connection-context repro after passing raw websocket `ConnHandle` through core/NAPI/TS. Full bare raw-websocket run had one `guard.request_timeout` on `should establish raw WebSocket connection`; isolated rerun passed.

- 2026-04-28T05:18:50-07:00 raw-websocket: PASS focused `/actors/{id}/sleep` repro for non-hibernatable raw websocket disconnect after making non-HWS actor stop terminal in pegboard gateway retry handling. Bare raw-websocket slice passed (16 passed, 32 skipped). Checks passed: `cargo check -p pegboard-gateway2`, `cargo check -p pegboard-gateway`, `pnpm check-types`.
