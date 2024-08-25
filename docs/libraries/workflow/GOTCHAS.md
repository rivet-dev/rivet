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

## Stale data

When fetching data for use in a workflow, you will most often put it in an activity for retryability. However,
depending on how much later the data from the activity is used, it may become stale. Make sure to add another
activity where needed when you need more up-to-date info.

## `WorkflowCtx::spawn`

`WorkflowCtx::spawn` allows you to run workflow steps in a different thread and returns its join handle. Be
**very careful** when using it because it is the developers responsibility to make sure it's result is handled
correctly. If a spawn thread errors but its result is not handled, the main thread may continue as though no
error occurred. This will result in a corrupt workflow state and a divergent history.

Also see [Consistency with concurrency](#consistency-with-concurrency).

## Consistency with concurrency

When you need to run multiple workflow events (like activities or signals) in parallel, be careful that you
ensure the state of the context is consistent between replays.

Take this example trying to concurrently run multiple activities:

```rust
let iter = actions.into_iter().map(|action| {
	let ctx = ctx.clone();

	async move {
		ctx.activity(MyActivityInput {
			action,
		}).await?;
	}
	.boxed()
});

futures_util::stream::iter(iter)
	.buffer_unordered(16)
	.try_collect::<Vec<_>>()
	.await?;
```

This will error because of the `ctx.clone()`; each activity has the same internal location because none of the
ctx's know about each other\*.

Instead, you can increment the location preemptively with `ctx.step()`:

```rust
let iter = actions.into_iter().map(|action| {
	let ctx = ctx.step();

	async move {
		ctx.activity(MyActivityInput {
			action,
		}).await?;
	}
	.boxed()
});

futures_util::stream::iter(iter)
	.buffer_unordered(16)
	.try_collect::<Vec<_>>()
	.await?;
```

If you plan on running more than one workflow step in each future, use a branch instead:

```rust
let iter = actions.into_iter().map(|action| {
	let ctx = ctx.branch();

	async move {
		ctx.activity(MyActivityInput {
			action,
		}).await?;
	}
	.boxed()
});

futures_util::stream::iter(iter)
	.buffer_unordered(16)
	.try_collect::<Vec<_>>()
	.await?;
```

Note that the first example would also work with a branch, but its a bit overkill as it creates a new layer in
the internal location.

> **\*** Even if they did know about each other via atomics, there is no guarantee of consistency from
> `buffer_unordered`. Preemptively incrementing the location ensures consistency regardless of the order or
> completion time of the futures.

## Hashmaps in activity inputs/outputs

`std::collections::HashMap` does not implement `Hash`. To get around this, use `util::HashableMap`:

```rust
use util::AsHashableExt;

ctx
	.activity(MyActivityInput {
		map: input.map.as_hashable(),
	})
	.await?;
```

## Nested options with serde

Nested options do not serialize/deserialize consistently with serde.

```rust
Some(Some(1234)) -> "1234" -> Some(Some(1234))
Some(None)		 -> "null" -> None
None			 -> "null" -> None
```

Be careful when writing your struct definitions.

## Force waking a sleeping workflow

When force waking a sleeping workflow by setting `wake_immediate = true`, know that if the workflow is
currently on a `sleep` step it will go back to sleep if it has not reached its `wake_deadline` yet. For all
other steps, the workflow will continue normally (usually just go back to sleep).
