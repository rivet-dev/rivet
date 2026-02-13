# Workflow History

To provide durable execution, workflows store their steps (aka events) in a database. These are stored in order of event location.

## Location

All events have a __location__ consisting of a set of __coordinates__. Each __coordinate__ is a set of __ordinates__ which are positive integers.

Locations look like:

- `{1}` - the first event
- `{1, 4}` - the fourth child of the first event
- `{0.1}` - the first inserted event before the first event
- `{4, 0.3.1, 0.6}` - the 6th inserted event before the first event of the parent, which is the first inserted event between 0.3 and 0.4, which is a child of the fourth event in the root

This may look confusing, but it allows dynamic location assignment without prior knowledge of the entire list of steps of a workflow, as well as modifying an existing workflow which already has some history.

## Calculating Location

Location is determined both by the events before it and the events after it (if the location is for an inserted event).

For a new workflow with no history, location is determined by incrementing the final coordinate (which consists of a single ordinate). Location `{2}` follows `{1}`, etc.

Coordinates start at `1`, not `0`. This is important to allow for inserting events before location `{1}` without negative numbers.

### Branches

When a branch (used internally for steps like loops and closures) is encountered, a new coordinate is added to the current location. So events that are children of a branch at location `{1}` would start at `{1, 1}`.

### Inserting Events

If a workflow has already executed up to location `{4}` but you want to add a new activity before location `{2}`, you can use __versioned workflow steps__ to make this happen.

By default all steps inherit the version of the branch they come from, which for the root of the workflow is version 1.

If you were to add a new step before location `{2}` with version 1 (denoted as `{2}v1` or `{2} v1`), the workflow would fail when it replays with the error `HistoryDiverged`. The version of inserted events must always be higher than the version of the step that comes AFTER it.

When we add a new step before location `{2}` with a version 2, it will be assigned the location `{1.1}` because it is the first inserted event after location `{1}`. All subsequent new events we add will increment this final ordinate: `{1.2}`, `{1.3}`, etc.

If we want to add an event between `{1.1}` and `{1.2}` (which are both version 2 events), we will need to set the event's version to 3. The new event's location will be `{1.1.1}`.

Events inserted before the event at location `{1}` will start with a 0 (`{0.1}`). Continuing to insert events before this new event will prepend more 0's: `{0.0.1}`, `{0.0.0.1}`, etc.

Note that inserting can be done at any root, so an event between `{2, 11, 4}` and `{2, 11, 5}` will have the location `{2, 11, 4.1}`.

### Removing events

Removing events requires replacing the event with a `ctx.removed::<_>()` call. This is a durable step that will either:

- For workflows that have already executed the step that should be removed: will not manipulate the database but will skip the step when replaying.
- For workflows that have not executed the step yet: will insert a `removed` event into history

This keeps locations consistent between the two cases.

### Inserting Events Conditionally Based On Version

Sometimes you may want to keep the history of existing workflows the same while modifying only new workflows. You can do this with `ctx.check_version(N)` where `N` is the version that will be used when the history does not exist yet (i.e. for a new workflow).

Given a workflow with the history:

- `{1}v1` activity foo
- `{2}v1` activity bar
- `{3}v1` sleep

If we want this workflow to remain the same but new workflows to execute a different activity instead of `bar` (perhaps a newer version), we can do:

```rust
// Activity foo
ctx.activity(...).await?;

match ctx.check_version(2).await? {
	// The existing workflow will always match this path because the next event (activity bar) has version 1
	1 => {
		// Here we need to keep the workflow steps as expected by the history, run activity bar
		ctx.activity(...).await?;
	}
	// This will be `2` because that is the value of `N`
	_latest => {
		// Activity bar_fast
		ctx.v(2).activity(...).await?;
	}
}

ctx.sleep().await?;
```

Version checks are durable because if history already exists at the location they are added then they do not manipulate the database and read the version of that history event. But if the version check is at the end of the current branch of events (as in a new workflow), it will be inserted as an event itself. This means the workflow history for both workflows will look like this:

- Existing workflow history:
	- `{1}v1` activity foo
	- `{2}v1` activity bar
	- `{3}v1` sleep
- New workflow history:
	- `{1}v1` activity foo
	- `{2}v2` version check
	- `{3}v2` activity bar_fast
	- `{4}v1` sleep

Note that you can also manipulate the existing workflow history in the `1` branch just like you would without `check_version`. We could insert a new activity after activity `bar` with a `v2` or remove activity `bar`.

## Loops

Loops structure event history with 2 nested branches:

A loop at location `{2}` will have each iteration on a separate branch: `{2, 1}`, `{2, 2}`, ... `{2, iteration}`.

Events in each iteration will be a child of the iteration branch: `{2, 2, 1}`, `{2, 2, 2}` are the first two events in the second iteration of the loop at location `{2}`.

Loops are often used to create state machines out of workflows. Because state machines can technically run forever based on their loop configuration, Gasoline moves all complete iteration's event history into a different place in the database known as forgotten event history.

Forgotten events will not be pulled from the database when the workflow is replayed. This will not cause issues for the workflow because we know which iteration is the current one and previous iterations should not influence the current history as each iteration is separate.
