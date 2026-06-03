# Panic Audit

Story: US-095
Date: 2026-04-22 14:53:27 PDT

## Scope

Command required by the story:

```bash
grep -rn '\.expect(\|\.unwrap(\|panic!\|unimplemented!\|todo!' rivetkit-rust/packages/{rivetkit-core,rivetkit,rivetkit-napi}/src
```

## Counts

- Initial grep count: 199
- Initial `expect("lock poisoned")` count: 0
- Final grep count: 165
- Final `expect("lock poisoned")` count: 0
- Final production-source scan: 0 non-test matches

## Changes

- `rivetkit-core/src/actor/metrics.rs`: replaced Prometheus metric creation and registration `expect(...)` calls with fallible construction. Metrics now disable themselves and log a warning if initialization fails.
- `rivetkit-core/src/actor/context.rs`: changed inspector overlay subscription from an `expect(...)` to `Option`, with the registry websocket path closing cleanly if runtime wiring is missing.
- `rivetkit-core/src/actor/task.rs`: replaced shutdown reply `expect(...)` sites with `actor.dropped_reply` errors.
- `rivetkit-core/src/registry/inspector.rs`: replaced inspector CBOR/error-response serialization `expect(...)` sites with logged fallback behavior.
- `rivetkit/src/context.rs`: changed `Ctx::client()` to return `Result<Client>` with structured `actor.not_configured` errors for missing envoy client wiring.
- `rivetkit/src/event.rs`: changed moved HTTP request accessors to `Option` / `Result`, and replaced the checked enum-entry `expect(...)` with an explicit serde error.
- `rivetkit-napi/src/napi_actor_events.rs`: replaced the wake-snapshot `expect(...)` with a structured `napi.invalid_state` error.

## Remaining Matches

All remaining grep matches are under `#[cfg(test)]` inline test modules. They are retained as test assertions, intentional panic probes, or test fixture setup.

```text
20 rivetkit-rust/packages/rivetkit-core/src/actor/connection.rs
 5 rivetkit-rust/packages/rivetkit-core/src/actor/queue.rs
 3 rivetkit-rust/packages/rivetkit-core/src/actor/schedule.rs
 8 rivetkit-rust/packages/rivetkit-core/src/actor/sleep.rs
 1 rivetkit-rust/packages/rivetkit-core/src/actor/work_registry.rs
 5 rivetkit-rust/packages/rivetkit-core/src/registry/envoy_callbacks.rs
11 rivetkit-rust/packages/rivetkit-core/src/registry/http.rs
52 rivetkit-rust/packages/rivetkit/src/event.rs
 1 rivetkit-rust/packages/rivetkit/src/persist.rs
10 rivetkit-rust/packages/rivetkit/src/start.rs
 1 rivetkit-typescript/packages/rivetkit-napi/src/actor_context.rs
 4 rivetkit-typescript/packages/rivetkit-napi/src/actor_factory.rs
44 rivetkit-typescript/packages/rivetkit-napi/src/napi_actor_events.rs
```

## Verification

- `cargo build -p rivetkit-core`: passed.
- `cargo build -p rivetkit`: passed.
- `cargo build -p rivetkit-napi`: passed.
- `cargo test -p rivetkit-core`: passed.
- `cargo test -p rivetkit`: passed.
- `pnpm --filter @rivetkit/rivetkit-napi build:force`: passed.
- `cargo test -p rivetkit-napi --lib`: blocked by the existing standalone NAPI linker issue (`undefined symbol: napi_*`). This is the known reason this repo uses `cargo build -p rivetkit-napi` plus `pnpm --filter @rivetkit/rivetkit-napi build:force` as the NAPI gate.
- `git diff --check`: passed.
