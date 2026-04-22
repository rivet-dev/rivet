# Alarm During Sleep Wake Fix

## Problem

Actors can schedule durable alarms while awake or during shutdown. If sleep cleanup clears the
engine-side driver alarm, no host-side timer remains to wake the sleeping actor, so `schedule.after`
work only runs after another external request wakes the actor.

## Reference Behavior

The TypeScript runtime at `feat/sqlite-vfs-v2` persists scheduled events and re-arms future alarms
from `initializeAlarms()` on startup. If scheduled work becomes due while an instance is stopping,
that work stays persisted and the next instance drains overdue events after startup.

## Runtime Contract

- Sleep shutdown preserves the engine-side alarm for the next scheduled event.
- Sleep shutdown cancels only local Tokio alarm timeouts owned by the terminating instance.
- Destroy shutdown clears the engine-side alarm because there is no next actor instance to wake.
- Alarm dispatch during `SleepGrace`, `SleepFinalize`, `Destroying`, or `Terminated` must not consume
  scheduled events. The persisted event remains for the next startup drain unless the actor is
  destroyed.
- Startup calls `init_alarms()` before accepting normal work. `init_alarms()` only arms future alarms;
  overdue events are handled by the existing startup drain.

## Race Handling

- **Alarm vs. sleep finalize**: once finalization begins, alarm dispatch is suspended and the local
  callback is removed. The persisted engine alarm remains armed on sleep, so the host can wake a new
  generation when the timestamp fires.
- **Alarm vs. destroy**: destroy cleanup cancels both local timeouts and the engine alarm after state
  writes have settled. Any alarm event already dispatched before destroy is tracked actor work and is
  drained by the normal shutdown sequence.
- **HTTP wake vs. alarm wake**: either wake path starts a fresh generation. Startup reloads persisted
  schedule state, re-syncs future alarm state, then drains overdue scheduled events. Duplicate wake
  signals are safe because scheduled events are removed only after dispatch completion.

## Regression Coverage

- `fire_due_alarms_defers_overdue_work_during_sleep_grace` proves in-flight sleep does not consume
  overdue scheduled events.
- `sleep_shutdown_preserves_driver_alarm_after_cleanup` proves sleep cleanup does not clear the
  engine alarm.
- `destroy_shutdown_still_clears_driver_alarm_after_cleanup` proves destroy cleanup still clears it.
- Driver coverage is the targeted `actor-sleep-db`, `actor-conn-hibernation`, and `actor-sleep`
  alarm-wake cases.
