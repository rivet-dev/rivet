# rivetkit (Rust)

Thin typed wrapper over `rivetkit-core`. Keep parity with `rivetkit-typescript` on a best-effort basis. See the root [RivetKit Layer Architecture](../../../CLAUDE.md#rivetkit-layer-architecture) for the layer hierarchy.

## Design constraints

- No `vars` / ephemeral-variable API. The TypeScript `ctx.vars` exists because user state lives in the framework. In Rust, ephemeral state is ordinary fields on the actor struct, while persisted state is `Actor::State` and is saved by the framework. Do not add a `vars` accessor to mirror TypeScript.
- The wrapper's primary API is `Actor` + one `Handles<M>` impl per action. Keep lifecycle and dispatch logic in `run_actor`; user actors should register with `register_actor` / `register_actor_with`.
- The legacy event-loop API (`Start`, `RuntimeEvent`, `Registry::register`, `Registry::register_with`) stays available for compatibility, but new examples and docs should use the trait API.
- State changes through `Ctx::state_mut()` / `Ctx::set_state()` mark persisted state dirty automatically. Use `request_save()` only when an explicit save point is needed.
- Keep `Ctx<A>` methods as thin pass-throughs to `ActorContext`; do not add load-bearing logic. New wrapper methods should only encode/decode CBOR at the boundary and delegate.
- No workflow events. The workflow engine is owned by `rivetkit-typescript`, so Rust actors never host workflows. Do not expose `Event` variants or types for `ActorEvent::WorkflowHistoryRequested` / `WorkflowReplayRequested`; handle those core variants with `unreachable!` in `Event::from_core`.
