# Pegboard Tunnel Retries

_TODO: Clean up this AI slop explanation_

This document explains how retries are coordinated between Guard and Pegboard-based handlers when transient tunnel (UPS) issues occur, for both HTTP and WebSocket traffic.

## HTTP

- Signal: A retryable transient tunnel failure is signaled by returning an HTTP 503 with the `X-RIVET-ERROR` header set.
  - Example (Pegboard Gateway): on tunnel closed (e.g., UPS `request_timeout`), the gateway replies with `503` and `X-RIVET-ERROR: pegboard_gateway.tunnel_closed`.

- Guard behavior
  - Guard considers a response retryable if `status == 503` and the `X-RIVET-ERROR` header is present.
  - Guard then applies exponential backoff (from middleware-config: `max_attempts`, `initial_interval`), re-resolves the route with `ignore_cache = true`, and retries the request.
  - On successful retry, traffic proceeds normally to the new target.
  - If attempts are exhausted, Guard returns an upstream error to the client.

- Notes for implementers
  - For transient tunnel failures, return a 503 with `X-RIVET-ERROR` to trigger Guard retries. Use an empty body or minimal payload as appropriate.
  - Do not 503 for non-transient errors; let the normal error flow apply.

## WebSocket

This section explains how WebSocket retries are coordinated between Guard and Pegboard-based handlers.

## Overview

- Retries are only possible before the client WebSocket is accepted ("opening" stage).
- A retryable transient failure is signaled via the error `guard.websocket_service_unavailable` (WebSocketServiceUnavailable).
- When Guard receives this error during opening, it re-resolves routes (ignoring cache), applies backoff, and retries with the same client socket and a new handler if available.
- After the client socket is accepted ("open"), retries are not possible; the handler must close gracefully on failure.

### Lifecycle Behavior

- Opening (before accept)
  - Source: handler detects a transient UPS/tunnel issue before awaiting the `HyperWebsocket` (e.g., failing to `ups.request(...)` to open, or failing to `ups.subscribe(...)`).
  - Handler contract:
    - Do not await the client websocket yet.
    - Return the untouched `HyperWebsocket` in the error tuple so Guard still owns it: `Err((client_ws, err))`.
    - The outer wrapper maps tunnel-closed UPS errors (e.g., `ups.request_timeout`) to `WebSocketServiceUnavailable`.
  - Guard reaction:
    - Treats `WebSocketServiceUnavailable` as retryable.
    - Re-resolves the route with ignore-cache=true, using middleware-config retry/backoff.
    - Outcomes:
      - Re-resolve → `CustomServe`: reuse the same `client_ws` and retry with the new handler.
      - Re-resolve → `Response`: accept client, send a Close with the response message as the reason.
      - Re-resolve → `Target` (non-CustomServe) or mismatch: accept client, send a Close with a generic message (cannot retry).
      - Attempts exhausted: accept client and send a Close with the original error message.

- Open (after accept)
  - The handler has awaited the client websocket; Guard can no longer retry.
  - Any failures (UPS send/receive, serialization, etc.) should be handled by closing the connection gracefully.

- Closing
  - Best-effort signaling to the server via `ups.request(...)` and to the client via Close frames.
  - Failures are ignored; no retries.

- Closed
  - No further action.

### Implementer Guidance

- Keep the client socket intact for retries:
  - Only return a retryable error (that maps to `WebSocketServiceUnavailable`) before awaiting the client websocket.
  - Return the socket in the error tuple: `Err((client_ws, err))`.

- Map tunnel-closed errors at the wrapper:
  - In the outer `handle_websocket` wrapper, detect tunnel-closed (e.g., `ups.request_timeout`) and map to `WebSocketServiceUnavailable`.
  - `handle_websocket_inner` should return raw errors; do not construct `WebSocketServiceUnavailable` inside the inner function.

- Use `ups.request` for all tunnel operations (open, messages, close):
  - Pre-accept failures should surface as errors with the unconsumed `client_ws` so Guard can retry.
  - Post-accept failures should break streams and close gracefully; do not attempt retries.

- Backoff and attempts:
  - Guard uses middleware-config values for `max_attempts` and `initial_interval` to perform exponential backoff between retries.
  - Routes are re-resolved with ignore-cache=true on each retry to avoid stale targets.

### Rationale

- Returning the untouched `HyperWebsocket` in errors preserves the ability for Guard to re-route and retry without disconnecting the client.
- Mapping tunnel-closed conditions to a single sentinel error (`WebSocketServiceUnavailable`) provides a consistent, guard-specific signal for retryability.
- Restricting retries to pre-accept avoids protocol violations and simplifies resource ownership.
