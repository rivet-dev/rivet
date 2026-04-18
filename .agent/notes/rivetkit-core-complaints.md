# RivetKit Core Implementation Complaints

Tracking issues and complaints about the current rivetkit-core / rivetkit Rust implementation.

---

1. **Actor key ser/de should be in its own file** — Move actor key serialization/deserialization to `utils/key.rs` instead of wherever it currently lives.

2. **Request and Response structs need their own file** — Move both `Request`, `Response`, and all related utilities to a dedicated file.

3. **Rename `callbacks` to `lifecycle_hooks`** — `actor/callbacks.rs` should be `actor/lifecycle_hooks.rs` (and the module name accordingly).

4. **Rename `FlatActorConfig` to `ActorConfigInput`** — Add doc comment: "Sparse, serialization-friendly actor configuration. All fields are optional with millisecond integers instead of Duration. Used at runtime boundaries (NAPI, config files). Convert to ActorConfig via ActorConfig::from_input()." Rename `from_flat()` to `from_input()`.

6. **TODO: Investigate `context.rs` issues** — Need to look closer at: (a) `build()` passes `ActorConfig::default()` to Queue and ConnectionManager instead of the actual config — possible bug, (b) 5 public methods that unconditionally error with no way to configure them (`db_exec`, `db_query`, `db_run`, `client_call`, `ack_hibernatable_websocket_message`), (c) `sleep()` spawns fire-and-forget task with untracked JoinHandle, (d) `Default` impl creates empty context with `actor_id: ""` — footgun, (e) ~15 `#[allow(dead_code)]` methods — incomplete wiring or compiler can't see call paths.

7. **TODO: Review `lifecycle.rs` more closely** — At 474 lines of code (1421 with tests) it's the biggest file and the state machine for startup/sleep/destroy sequencing. Need to verify correctness of the sequencing, error handling in each phase, and whether it matches TS behavior.

8. **Remove all `#[allow(dead_code)]`** — Methods called by NAPI should be `pub`, not `pub(crate)` with dead_code suppressed. Methods truly unused should be deleted.

9. **Move `kv.rs` and `sqlite.rs` out of top-level `src/`** — They're actor subsystems, not standalone crate features. Move to `src/actor/kv.rs` and `src/actor/sqlite.rs`, or `src/kv/mod.rs` and `src/sqlite/mod.rs` if they need room to grow.

10. **TODO: Review testability and KV in-memory backend** — Why does KV need an in-memory backend at all? State, queue, connections, schedule all use KV as their storage layer and the in-memory backend lets them be unit tested without an engine. But is this the right testing approach? Should we use a trait-based KV backend instead? Or should these subsystems be tested via integration tests only? Need to evaluate whether the in-memory KV is the right abstraction or a testing smell.

11. **Action timeout/size enforcement lives in TS instead of Rust** — `native.ts` enforces `withTimeout()` and `maxIncomingMessageSize`/`maxOutgoingMessageSize` for HTTP actions because `handle_fetch` bypasses rivetkit-core's `actor/event.rs` dispatch. Should be consolidated into Rust, either by routing HTTP actions through `event.rs` or by adding enforcement in `handle_fetch` in `registry.rs`.

5. **Env var parity gap** — Rust rivetkit-core only reads: `RIVET_REGION`, `RIVET_ENVOY_VERSION`, `RIVET_ENDPOINT`, `RIVET_TOKEN`, `RIVET_NAMESPACE`, `RIVET_POOL_NAME`, `RIVET_ENGINE_BINARY_PATH`. Missing from TS parity: `RIVET_ENGINE`, `RIVET_RUN_ENGINE`, `RIVET_RUN_ENGINE_VERSION`, `RIVET_TOTAL_SLOTS`, `RIVET_ENVOY_KIND`, `RIVET_PUBLIC_ENDPOINT`, `RIVET_PUBLIC_TOKEN`, `RIVET_INSPECTOR_TOKEN`, `RIVET_INSPECTOR_DISABLE`, `RIVET_LOG_LEVEL`, `RIVET_LOG_TARGET`, `RIVET_LOG_TIMESTAMP`. Some may be TS-only (logging/inspector), but `RIVET_PUBLIC_ENDPOINT`, `RIVET_ENVOY_KIND`, and `RIVET_TOTAL_SLOTS` look like real gaps.

