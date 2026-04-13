## PR Review: `fix(pegboard-runner): clear terminal tunnel routes`

### Summary

This PR fixes a resource leak in the pegboard-runner's WebSocket-to-tunnel message handler. Previously, tunnel route authorizations (entries in `authorized_tunnel_routes`) were never removed when processing terminal tunnel messages (final HTTP responses, aborted requests, closed WebSocket connections). After this fix, terminal messages also remove the corresponding route entry, preventing stale route entries from accumulating across the lifetime of a runner connection.

The fix is applied to both mk2 and mk1 protocol paths via new pure helper functions `should_clear_tunnel_route_mk2` and `should_clear_tunnel_route_mk1`.

---

### Code Quality

**Positive:**
- Extracting `route` and `clear_route` before consuming `msg` is correct and avoids borrow issues since `msg` is moved into the serialization call.
- `should_clear_tunnel_route_*` helpers are pure functions that clearly express intent.
- Uses `scc::HashMap` async API (`contains_async`, `remove_async`) consistently with the codebase's concurrency model.
- mk1 and mk2 paths are kept at feature parity per the Engine Runner Parity guideline.
- Commit message follows conventional commits format.

---

### Issues

#### Medium: Test helpers are dead code; clearing behavior is untested

The test support file adds several helper constructors (`response_start_message_mk2`, `response_chunk_message_mk2`, `response_start_message_mk1`, `response_chunk_message_mk1`, etc.) that are never called in any test. The updated tests only verify that a `WebSocketMessage` (non-terminal) does **not** clear the route. There are no tests verifying the positive case that terminal messages **do** clear the route.

Missing test cases:
- `ToServerResponseStart` with `stream: false` -> route cleared
- `ToServerResponseStart` with `stream: true` -> route **not** cleared
- `ToServerResponseChunk` with `finish: true` -> route cleared
- `ToServerResponseChunk` with `finish: false` -> route **not** cleared
- `ToServerResponseAbort` -> route cleared
- `ToServerWebSocketClose` -> route cleared
- Symmetric coverage for mk1 variants

The existing `republishes_issued_mk*_tunnel_message_pairs` tests now only exercise the non-clearing path, so there is no test that sends a terminal message and asserts the route entry is subsequently absent.

#### Low: Implicit fallthrough in `should_clear_tunnel_route_*` for future variants

Both `should_clear` functions use `_ => false` as the catch-all. This means any future protocol variant added to the enum would silently default to not clearing the route. Depending on the variant this could be correct (safe default) or a bug. Explicitly enumerating all non-clearing variants or adding a comment would make the intent clear and surface a compile error if a new variant is added without deliberate handling.

#### Low: `DeprecatedTunnelAck` early-return computes `clear_route` unnecessarily

In `handle_tunnel_message_mk1`, `clear_route` is computed before the `DeprecatedTunnelAck` early-return check. Since `should_clear_tunnel_route_mk1` returns `false` for that variant anyway, there is no logic issue, but the value is computed and then immediately discarded. Minor ordering cleanup would eliminate the dead computation.

---

### Security

The fix is directly security-relevant: it enforces the one-request-one-response invariant at the runner level. Without this fix, a route authorization for a completed/aborted request could persist indefinitely, allowing responses to continue being forwarded after the logical request lifecycle has ended. No new concerns are introduced.

---

### Performance

No concerns. `scc::HashMap::remove_async` is O(1) and does not hold a lock across `.await` points.

---

### Potential Edge Case

Route clearing only happens after a **successful** publish. If the NATS publish call fails, the error returns before the `if clear_route` block, leaving the route in place. This is pre-existing behavior and arguably correct (caller can retry), but worth a comment in the code to make the intention explicit.

---

### Summary

| Severity | Finding |
|---|---|
| Medium | Test helpers added but never called; no tests assert terminal messages actually clear the route |
| Low | `_ => false` catchall silently handles unknown future protocol variants |
| Low | `DeprecatedTunnelAck` path computes `clear_route` before the early-return that discards it |
| Info | Failed publish leaves route in place (pre-existing; worth a comment) |
