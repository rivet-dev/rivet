# Driver Test Suite Status

## What works
- rivet-envoy-client (Rust) fully functional
- rivetkit-native NAPI module, TSFN callbacks, envoy lifecycle all work
- Standalone test: create actor + ping = 22-32ms (both test-envoy and native)
- Gateway query path (getOrCreate) works: 112ms
- E2e actor test passes (HTTP ping + WS echo)
- Driver test suite restored (2282 tests), type-checks, loads

## Blocker: engine actor2 workflow doesn't process Running event

### Evidence
- Fresh namespace, fresh pool config, generation 1
- Actor starts in 11ms, Running event sent immediately
- Guard times out after 10s with `actor_ready_timeout`
- The Running event goes: envoy WS → pegboard-envoy → actor_event_demuxer → signal to actor2 workflow
- But actor2 workflow never marks the actor as connectable

### Why test-envoy works but EngineActorDriver doesn't
- test-envoy uses a PERSISTENT envoy on a PERSISTENT pool
- The pool existed before the engine restarted, so the actor workflow may be v1 (not actor2)
- v1 actors process events through the serverless/conn SSE path, which works
- The force-v2 change routes ALL new serverless actors to actor2, where events aren't processed

### Root cause
The engine's v2 actor workflow (`pegboard_actor2`) receives the `Events` signal from `pegboard-envoy`'s `actor_event_demuxer`, but it does not correctly transition to the connectable state. The guard polls `connectable_ts` in the DB which is never set.

### Fix needed (engine side)
Check `engine/packages/pegboard/src/workflows/actor2/mod.rs` - specifically how `process_signal` handles the `Events` signal with `EventActorStateUpdate{Running}`. It should set `connectable_ts` in the DB and transition to `Transition::Running`.
