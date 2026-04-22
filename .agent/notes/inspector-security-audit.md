# Inspector Security Audit

Date: 2026-04-22
Story: US-094
Source: `.agent/notes/production-review-complaints.md` #19

## Scope

Audited the native Rust inspector HTTP/WebSocket surface in `rivetkit-rust/packages/rivetkit-core/src/registry/inspector.rs`, `inspector_ws.rs`, and `http.rs` against the TypeScript native runtime surface in `rivetkit-typescript/packages/rivetkit/src/registry/native.ts`.

## Auth Model

- Rust HTTP `/inspector/*` routes call `InspectorAuth::verify(...)` before route dispatch.
- Rust inspector WebSocket `/inspector/connect` verifies the `rivet_inspector_token.*` websocket protocol token first, then falls back to `Authorization: Bearer ...`.
- TypeScript native HTTP `/inspector/*` routes call `ctx.verifyInspectorAuth(...)`, which delegates to the same core `InspectorAuth`.
- `InspectorAuth` prefers `RIVET_INSPECTOR_TOKEN` when configured. If absent, it falls back to the per-actor KV token at key `[3]`.
- Fixed in US-094: Rust bearer parsing now matches TS more closely by accepting case-insensitive `Bearer` and arbitrary whitespace after the scheme.

## HTTP Endpoint Matrix

| Endpoint | Rust auth | Rust response | TS counterpart | Mutation |
|---|---|---|---|---|
| `GET /inspector/state` | `InspectorAuth` | `{ state, isStateEnabled }` | Same | Read-only |
| `PATCH /inspector/state` | `InspectorAuth` | `{ ok: true }` | Same | Intended state replacement |
| `GET /inspector/connections` | `InspectorAuth` | `{ connections: [{ type, id, details }] }` | Same after US-094 | Read-only |
| `GET /inspector/rpcs` | `InspectorAuth` | `{ rpcs: [] }` | TS returns action names | Read-only |
| `POST /inspector/action/{name}` | `InspectorAuth` | `{ output }` or structured action error | Same | Intended action execution |
| `GET /inspector/queue?limit=` | `InspectorAuth` | `{ size, maxSize, truncated, messages }` | TS returns size/max/truncated but no messages | Read-only |
| `GET /inspector/workflow-history` | `InspectorAuth` | `{ history, isWorkflowEnabled }` | TS also returns `workflowState` | Read-only, but dispatches workflow inspector request |
| `POST /inspector/workflow/replay` | `InspectorAuth` | `{ history, isWorkflowEnabled }` | TS also returns `workflowState` | Intended workflow replay mutation |
| `GET /inspector/traces` | `InspectorAuth` | `{ otlp: [], clamped: false }` | Same placeholder | Read-only |
| `GET /inspector/database/schema` | `InspectorAuth` | `{ schema: { tables } }` | Same | Read-only SQL/PRAGMA queries |
| `GET /inspector/database/rows?table=&limit=&offset=` | `InspectorAuth` | `{ rows }` or structured invalid request | Same success shape | Read-only SQL query |
| `POST /inspector/database/execute` | `InspectorAuth` | `{ rows }` | Same success shape | Intended SQL execution; can mutate |
| `GET /inspector/summary` | `InspectorAuth` | state/connections/rpcs/queue/database/workflow snapshot | TS also returns `workflowState` | Read-only, but dispatches workflow inspector request |
| `GET /inspector/metrics` | Missing in Rust core HTTP | TS returns JSON actor metrics | TS-only today | Read-only |

## WebSocket Message Matrix

All Rust inspector WebSocket messages are gated by the authenticated `/inspector/connect` handshake.

| Message | Rust response | TS/WebSocket counterpart | Mutation |
|---|---|---|---|
| `PatchStateRequest` | No response on success | Same protocol message | Intended state replacement |
| `StateRequest` | `StateResponse` | Same | Read-only |
| `ConnectionsRequest` | `ConnectionsResponse` | Same | Read-only |
| `ActionRequest` | `ActionResponse` | Same | Intended action execution |
| `RpcsListRequest` | `RpcsListResponse` | Same, but Rust names are empty | Read-only |
| `TraceQueryRequest` | Empty `TraceQueryResponse` | Same placeholder | Read-only |
| `QueueRequest` | `QueueResponse` with message summaries | Same protocol message | Read-only |
| `WorkflowHistoryRequest` | `WorkflowHistoryResponse` | Same | Read-only, but dispatches workflow inspector request |
| `WorkflowReplayRequest` | `WorkflowReplayResponse` | Same | Intended workflow replay mutation |
| `DatabaseSchemaRequest` | `DatabaseSchemaResponse` | Same | Read-only SQL/PRAGMA queries |
| `DatabaseTableRowsRequest` | `DatabaseTableRowsResponse` | Same | Read-only SQL query |

## Findings Fixed In US-094

- Rust auth parsing was too strict. It only accepted exactly `Bearer ` while TS accepted case-insensitive bearer schemes with flexible whitespace. Fixed by sharing a tolerant parser across inspector HTTP, WebSocket bearer fallback, and `/metrics`.
- Rust HTTP connection payloads did not match TS/docs. Fixed to return `{ type, id, details: { type, params, stateEnabled, state, subscriptions, isHibernatable } }`.
- Rust `POST /inspector/database/execute` accepted both `args` and `properties` and silently preferred `properties`. Fixed to reject the ambiguous request with `inspector.invalid_request`.

## No Unintended Read Mutations Found

- State, connections, RPC list, queue, traces, database schema, database rows, and summary reads do not directly write actor state.
- Workflow history and summary reads dispatch a workflow-inspector request to the actor runtime. That can run user/runtime workflow inspector code, but it is the existing workflow inspector read contract rather than an actor state write.
- Database schema and rows use quoted identifiers and parameterized `LIMIT`/`OFFSET`.

## Follow-Up Stories

- **Inspector RPC list parity**: Teach core/`ActorConfig` the action name list so Rust `/inspector/rpcs`, summary, and WebSocket `RpcsListResponse` match TS instead of returning `[]`.
- **Inspector workflow state parity**: Add `workflowState` to Rust HTTP `/inspector/workflow-history`, `/inspector/workflow/replay`, and `/inspector/summary`, or remove it from TS/docs if it is intentionally runtime-only.
- **Inspector metrics parity**: Decide whether Rust core should expose JSON `GET /inspector/metrics` to match TS, or whether docs/tests should describe Rust `/metrics` Prometheus text as the only core metrics endpoint.
- **Inspector queue message parity**: Either expose queue message summaries through the TS native runtime or intentionally document that only Rust core HTTP includes queue message summaries.
- **Inspector error shape parity**: Align TS inspector validation errors with Rust structured `{ group, code, message, metadata }` errors instead of ad hoc `{ error }` bodies.
