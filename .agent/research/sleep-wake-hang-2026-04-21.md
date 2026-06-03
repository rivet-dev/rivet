# US-108 sleep->wake hang investigation

## Reproducer

- Mandatory reproducer:
  - `cd rivetkit-typescript/packages/rivetkit`
  - `pnpm test tests/driver/actor-db.test.ts -t 'static registry.*encoding \(bare\).*Actor Database.*persists across sleep and wake cycles'`
- Engine run used for investigation:
  - `RUST_LOG='rivet_envoy_client=trace,rivet_engine_runner=debug,pegboard=debug' ./scripts/run/engine-rocksdb.sh`
- Fresh runtime rebuild before reproducing:
  - `pnpm --filter @rivetkit/rivetkit-napi build:force`
  - `pnpm build -F rivetkit`

## What actually happened

- The current worktree no longer reproduces the original 180s hang.
- The mandatory `actor-db` sleep/wake test now passes in ~2.2s:
  - `Test Files  1 passed`
  - `Tests  1 passed | 47 skipped`
- Secondary reruns from the same rebuilt worktree:
  - Passed: `actor-db-pragma-migration` sleep/wake
  - Passed: all 3 `actor-state-zod-coercion` sleep/wake variants
  - Passed: `actor-workflow > workflow onError is not reported again after sleep and wake`
  - Still failing: `actor-workflow > sleeps and resumes between ticks`, but with `Actor failed to start ... "no_envoys"` rather than a sleep/wake hang

## Envoy-client hypothesis check

- The original leading hypothesis was a stale `envoy-client` actor mirror entry caused by the `received_stop` guard in `engine/sdks/rust/envoy-client/src/events.rs`.
- That does **not** match the current fix path:
  - the mandatory reproducer is green without changing the `received_stop` guard
  - the active greening change is in the NAPI adapter, not `envoy-client` stop-event cleanup
- I did **not** reproduce the old engine-side `actor not allocated, ignoring events` / `actor lost` sequence once the rebuilt current worktree was running, so the envoy-mirror theory is no longer the best explanation for the live branch state.

## Confirmed root cause

- `rivetkit-typescript/packages/rivetkit-napi/src/actor_context.rs` caches `ActorContextShared` by `actor_id` in the process-global `ACTOR_CONTEXT_SHARED` map.
- Before the fix, a sleep/destroy cycle left runtime-only state attached to that shared object across the next wake for the same actor id:
  - `end_reason`
  - `ready`
  - `started`
  - abort token
  - run-restart hook
  - registered-task sender
- That stale state is enough to poison the next receive-loop instance:
  - `run_event_loop(...)` breaks whenever `ctx.has_end_reason()` is true
  - a wake-time adapter reusing the same `ActorContextShared` can therefore drop out of the loop early or keep stale lifecycle state from the previous incarnation
- The fix is the new reset call at the top of `run_adapter_loop(...)`:
  - `rivetkit-typescript/packages/rivetkit-napi/src/napi_actor_events.rs`
  - `ctx.reset_runtime_shared_state();`

## Proof used for this conclusion

- Code-path proof:
  - `run_adapter_loop(...)` now clears the stale shared runtime state before reattaching the new abort token / task sender / lifecycle flags.
  - Without that reset, the cached `ActorContextShared` survives purely because it is keyed by actor id, not by actor generation or runtime instance.
- Regression proof:
  - Added Rust unit test `run_adapter_loop_resets_stale_shared_end_reason_before_wake` in `rivetkit-typescript/packages/rivetkit-napi/src/napi_actor_events.rs`
  - The test seeds a stale shared `EndReason::Sleep`, then runs a fresh adapter loop for the same actor id and verifies two queued actions both return `actor/action_not_found` instead of the second one being dropped by a stale end-of-life flag.

## Fix strategy

- Keep the fix scoped to the NAPI receive-loop adapter:
  - clear per-instance runtime-only state at adapter startup
  - keep the cached shared wrapper for cross-boundary object identity, but do not let lifecycle state leak across sleep/wake
- Do **not** touch `envoy-client` `received_stop` handling for this story; that would be a speculative fix against a hypothesis the rebuilt branch no longer supports.
