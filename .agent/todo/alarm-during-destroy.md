# Alarm fire during actor destroy

## Context

The new lifecycle architecture (`.agent/specs/rivetkit-task-architecture.md`) establishes the
invariant that incoming work arriving during `Sleeping` / `Destroying` / `Terminated` fails fast
and does not wait for the next actor instance.

That invariant is correct for one-shot work (HTTP requests, action calls, WebSocket opens). It is
**not** correct for alarms.

## Why alarms differ

Alarms are durable scheduled events persisted in the actor's state. A scheduled alarm with a fire
time during a destroy/sleep window must still execute eventually. Failing it fast loses the
scheduled work entirely, which is a different correctness model from "client retry."

The TS reference behavior: alarms that would fire during shutdown are deferred to the next actor
instance startup (handled via the "drain overdue scheduled events" step at the end of startup).

## What needs to be specified

- **Detection**: when an alarm fires while `lifecycle != Started`, the actor task must not run it
  through the normal `dispatch_action` path (which would reject it).
- **Persistence**: the alarm must remain on disk (not be consumed) so the next instance startup
  picks it up.
- **Coordination with destroy**: if the actor is being destroyed permanently (not just sleeping),
  is there an instance to "next"? If the actor is destroyed-with-no-restart, the alarm is silently
  abandoned. Confirm semantics with TS reference.
- **In-flight alarm during destroy**: an alarm whose dispatch already started before destroy was
  requested is tracked work and must drain (covered by the existing tracked-work invariant).

## Files likely affected

- `rivetkit-rust/packages/rivetkit-core/src/actor/schedule.rs`
- `rivetkit-rust/packages/rivetkit-core/src/actor/lifecycle.rs`
- The `LifecycleState` check inside the lifecycle task's alarm-fire arm.

## Action items

- [ ] Read TS reference (`rivetkit-typescript/packages/rivetkit/src/actor/instance/schedule-manager.ts`
      or equivalent) to confirm "alarm during destroy → next instance" semantics, including the
      destroy-with-no-restart edge case.
- [ ] Update `.agent/specs/rivetkit-task-architecture.md` "Invariants" section to carve out alarms
      explicitly — e.g., "alarms remain persisted across instance lifetimes; one-shot work does not."
- [ ] Decide how the alarm scheduler in the dying instance signals to the *next* instance that an
      overdue alarm exists. Most likely no signal needed because the next instance's startup step
      13 already drains overdue alarms — but verify the alarm row is still on disk when the dying
      instance exits.
