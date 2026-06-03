# Error Standardization Audit

Date: 2026-04-22
Story: US-096

## Scope

Audited raw `anyhow!(...)`, `anyhow::bail!(...)`, `anyhow::anyhow!(...)`, and public string-backed NAPI errors across:

- `rivetkit-rust/packages/rivetkit-core/src`
- `rivetkit-rust/packages/rivetkit/src`
- `rivetkit-typescript/packages/rivetkit-napi/src`

## Converted

- `rivetkit-core`: added shared structured errors for actor runtime, protocol parsing, SQLite runtime, and engine process failures in `src/error.rs`.
- `rivetkit-core`: converted KV, SQLite, engine process, callback request/response conversion, persistence decode, actor-connect decode, websocket decode, registry dispatch, connection, queue, inspector, schedule, state, and actor-task panic paths to `RivetError`-backed errors.
- `rivetkit-core`: preserved structured errors when cloning/forwarding existing `anyhow::Error` values with `RivetError::extract` instead of lossy `to_string()` reconstruction.
- `rivetkit`: converted missing actor input and typed action decode failures to shared `actor.*` structured errors.
- `rivetkit`: added a typed `QueueSend` event wrapper while fixing the exhaustive event conversion exposed by the build.
- `rivetkit-napi`: added `napi.invalid_argument` and `napi.invalid_state` errors and routed public validation/state failures through `napi_anyhow_error(...)` so they cross the JS boundary with the structured prefix.
- Artifacts: generated new error JSON files under `rivetkit-rust/engine/artifacts/errors/` for core/Rust wrapper errors and under `engine/artifacts/errors/` for NAPI errors generated from the TypeScript package crate.

## Remaining Raw Sites

- `rivetkit-rust/packages/rivetkit/src/event.rs`: three `anyhow::anyhow!` calls remain in unit tests as synthetic rejection payloads.
- `rivetkit-rust/packages/rivetkit-core/src/actor/schedule.rs`: two `anyhow::bail!` calls remain in tests as fail-fast sentinels for code paths that must not run.
- `rivetkit-typescript/packages/rivetkit-napi/src/lib.rs`: one `napi::Error::from_reason(...)` remains intentionally inside `napi_anyhow_error(...)`; this helper is the structured bridge encoder for `RivetError` metadata.

## Checks

- Passed: `cargo build -p rivetkit-core`
- Passed: `cargo test -p rivetkit-core --lib actor::queue`
- Passed: `cargo test -p rivetkit-core --lib actor::state`
- Passed: `cargo test -p rivetkit-core --lib registry::http`
- Passed: `cargo build --manifest-path rivetkit-rust/packages/rivetkit/Cargo.toml`
- Passed: `cargo test --manifest-path rivetkit-rust/packages/rivetkit/Cargo.toml --lib event::`
- Passed: `cargo build -p rivetkit-napi`
- Passed: `pnpm --filter @rivetkit/rivetkit-napi build:force`
- Passed: `pnpm build -F rivetkit`
- Limited: `cargo test -p rivetkit-napi --lib` compiles Rust test code but fails at final link outside Node because NAPI symbols like `napi_create_reference` are unresolved. The native package build is the usable gate for this crate.
