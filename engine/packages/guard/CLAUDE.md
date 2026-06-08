# Guard package

## Routing function timeouts

- Every `.await` inside a routing function must be covered by `tokio::time::timeout` or `timeout_at` directly or by an enclosing phase fence with a specific, named error type.
- The outer `route_timeout` wrapper in `guard-core/src/proxy_service.rs` is a backstop only; if `guard.request_timeout` appears for `route_resolution`, add a missing per-phase timeout instead of extending the backstop budget.
- Per-phase timeout budgets live in guard config with code defaults and must stay below the outer backstop.
- Per-phase errors live in the routing function's primary domain package, not in `guard-core`; prefer operator-meaningful phase errors over one public error per helper await.
