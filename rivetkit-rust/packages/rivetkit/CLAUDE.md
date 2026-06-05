# rivetkit (Rust)

Thin typed wrapper over `rivetkit-core`. Keep parity with `rivetkit-typescript` on a best-effort basis. See the root [RivetKit Layer Architecture](../../../CLAUDE.md#rivetkit-layer-architecture) for the layer hierarchy.

## Design constraints

- No `vars` / ephemeral-variable API. The TypeScript `ctx.vars` exists because user state lives in the framework. In the Rust event-loop model the user owns their actor struct, so ephemeral state is just fields on that struct. Do not add a `vars` accessor to mirror TypeScript.
- The wrapper is an event-loop API: lifecycle is delivered as `Event` variants plus the `Start` snapshot, not as config hooks. TypeScript hooks like `onCreate`/`onWake`/`createState`/`createConnState` have no 1:1 method; they are expressed through the `Start` snapshot and the event loop.
- `onStateChange` is exposed as `Ctx::on_state_change(future)` (and `begin_state_change()` for the guard form), which runs reaction logic inside core's on-state-change region so sleep/shutdown tracking matches TypeScript. Call it after mutating state.
- Keep `Ctx<A>` methods as thin pass-throughs to `ActorContext`; do not add load-bearing logic. New wrapper methods should only encode/decode CBOR at the boundary and delegate.
- No workflow events. The workflow engine is owned by `rivetkit-typescript`, so Rust actors never host workflows. Do not expose `Event` variants or types for `ActorEvent::WorkflowHistoryRequested` / `WorkflowReplayRequested`; handle those core variants with `unreachable!` in `Event::from_core`.
