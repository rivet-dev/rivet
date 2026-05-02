# Driver Test Suite Progress

Started: 2026-05-02
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
- [x] [-] actor-db | Actor Database (wasm: incomplete - pegboard-envoy SQL exec handlers stubbed)
- [x] [-] actor-db-raw | Actor Database Raw Tests (wasm: same)
- [x] [-] actor-db-init-order | Actor Db Init Order (wasm: same)
- [x] [-] actor-workflow | Actor Workflow Tests (wasm: 4 fail - workflow uses SQLite, gated on remote SQL exec)
- [x] [x] actor-error-handling | Actor Error Handling Tests
- [x] [x] actor-queue | Actor Queue Tests
- [x] [x] actor-kv | Actor KV Tests
- [x] [x] actor-stateless | Actor Stateless Tests
- [x] [x] raw-http | raw http
- [x] [x] raw-http-request-properties | raw http request properties
- [x] [x] raw-websocket | raw websocket
- [x] [-] actor-inspector | Actor Inspector Tests (wasm: 5/20 fail on database/* endpoints)
- [x] [x] gateway-query-url | Gateway Query URL Tests
- [x] [-] actor-db-pragma-migration | Actor Database Pragma Migration (wasm: same)
- [x] [x] actor-state-zod-coercion | Actor State Zod Coercion
- [x] [x] actor-conn-status | Connection Status Changes
- [x] [x] gateway-routing | Gateway Routing
- [x] [x] lifecycle-hooks | Lifecycle Hooks
- [-] [-] serverless-handler | Serverless Handler Tests (1/3 fails on both: streaming pings test internal error)

## Slow Tests

- [x] [x] actor-state | Actor State Tests
- [x] [x] actor-save-state | Actor Save State Tests
- [x] [-] actor-schedule | Actor Schedule Tests (wasm: 1 fail - "scheduled action can use c.db")
- [x] [-] actor-sleep | Actor Sleep Tests (wasm: 17/22 fail - sleep tests use SQLite for restore checks)
- [-] [-] actor-sleep-db | Actor Sleep Database Tests (native: 1/24 fail - "ws handler exceeding grace period should still complete db writes")
- [x] [-] actor-lifecycle | Actor Lifecycle Tests (wasm: 2/11 fail)
- [-] [-] actor-conn-hibernation | Actor Connection Hibernation Tests (native: 4/5 fail - hibernation timeouts)
- [x] [-] actor-run | Actor Run Tests (wasm: 2/8 fail)
- [-] [ ] hibernatable-websocket-protocol | hibernatable websocket protocol (native: 1/2 fail - replays only unacked indexed websocket messages)
- [x] [-] actor-db-stress | Actor Database Stress Tests (wasm: 4/5 fail - SQL exec gap)

## Excluded

- [ ] [ ] actor-agent-os | Actor agentOS Tests (skip unless explicitly requested)

## Notes

- Vitest `-t` filter triggers parallel-test timeouts on this branch; running test files without a `-t` filter works fine. Each file's outer suite is already scoped to one runtime/sqlite/encoding cell via env vars.

## Log

- 2026-05-02 manager-driver [native]: PASS (16/16, 19.8s)
- 2026-05-02 manager-driver [wasm]: PASS (16/16, 15.0s)
- 2026-05-02 actor-conn [native]: PASS (24/24)
- 2026-05-02 actor-conn [wasm]: PASS (24/24)
- 2026-05-02 actor-conn-state [native]: PASS (9/9, 7.4s) after stubbing preload-hint code in `rivetkit-core/src/actor/sqlite.rs` and rebuilding NAPI.
- 2026-05-02 actor-conn-state [wasm]: PASS (9/9, 16.7s)
- 2026-05-02 conn-error-serialization [native]: PASS (3/3)

## Final summary

**Native (NAPI / sqlite-local)**: 37/42 entries pass, 4 fail, and 1 is excluded. Failing files:
- `actor-conn-hibernation` — 4/5 fail (hibernation regression)
- `hibernatable-websocket-protocol` — 1/2 fail (replay regression)
- `actor-sleep-db` — 1/24 fail (grace-period race)
- `serverless-handler` — 1/3 fail (streaming pings internal error)

**Wasm (rivetkit-wasm / sqlite-remote)**: 26/42 entries pass, 13 fail, and 3 are unrun or excluded. Failing files are dominated by remote-SQLite execute paths being stubbed in `pegboard-envoy/src/ws_to_tunnel_task.rs`:
- `actor-db`, `actor-db-raw`, `actor-db-init-order`, `actor-db-pragma-migration`, `actor-db-stress`
- `actor-workflow` (workflow uses SQLite)
- `actor-inspector` (5/20 — DB endpoints only)
- `actor-schedule` (1/4 — DB scheduled action)
- `actor-sleep` (17/22 — sleep tests use SQLite restore)
- `actor-sleep-db`
- `actor-lifecycle` (2/11)
- `actor-run` (2/8)
- `serverless-handler` (1/3)

Follow-up branches in this stack address the actionable failures from this snapshot.
