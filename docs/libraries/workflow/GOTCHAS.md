# Gotchas

## Timestamps

Use timestamps with care when passing them between activity inputs/outputs. Because activity inputs need to be
consistent for replays, use `util::timestamp::now()` only within activities and not workflow bodies.

If you need a timestamp in a workflow body, use `ctx.create_ts()` for the creation of the workflow. Using
`ctx.ts()` is also inconsistent because it marks the start of the current workflow run (which is different
between replays).

If you need a consistent current timestamp, create a new activity that just returns `util::timestamp::now()`.
This will be the current timestamp on the first execution of the activity and won't change on replay.

> **When an activity's input doesn't produce the same hash as the first time it was executed (i.e. its input
> changed), the entire workflow will error with "History Diverged" and will not restart.**

## Randomly generated content

Randomly generated content like UUIDs should be placed in activities for consistent history.
