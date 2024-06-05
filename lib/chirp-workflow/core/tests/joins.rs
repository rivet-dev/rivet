use anyhow::*;
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use wf::*;

mod common;

use crate::common::MyActivityInput;

#[tokio::test]
async fn joins() -> Result<()> {
	common::setup();

	let db =
		DatabasePostgres::new("postgres://root@127.0.0.1:26257/postgres?sslmode=disable").await?;

	let mut registry = Registry::new();
	registry.register_workflow::<MyParallelWorkflow>();
	let registry = registry.handle();

	let worker = Worker::new(registry.clone(), db.clone());
	tokio::spawn(async move {
		if let Err(err) = worker.start().await {
			tracing::error!(?err, "worker failed");
		}
	});

	let ctx = common::TestCtx::new(db);

	// Run 20 workflows at once
	futures_util::stream::iter(0..20)
		.map(|_| async {
			let output = ctx.workflow(MyParallelWorkflowInput { x: 5 }).await?;
			assert_eq!(138, output.y);

			Ok(())
		})
		.buffer_unordered(100)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MyParallelWorkflowInput {
	x: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MyParallelWorkflowOutput {
	y: i64,
}

// GOTCHA: "returning this value requires that `'life1` must outlive `'static`" error comes from trying to use
// the `input` variable of a workflow in a closure without cloning it
#[macros::workflow(MyParallelWorkflow)]
async fn my_parallel_workflow(
	ctx: &mut WorkflowCtx,
	input: &MyParallelWorkflowInput,
) -> Result<MyParallelWorkflowOutput> {
	let a = ctx.activity(MyActivityInput { x: input.x }).await?;

	let (b, c, d) = ctx
		.join((
			MyActivityInput { x: input.x },
			MyActivityInput { x: 12 },
			closure(|ctx: &mut WorkflowCtx| {
				async move {
					let mut sum = 0;

					for i in 0..5 {
						sum += ctx.activity(MyActivityInput { x: i }).await?.y;
					}

					let (e, f) = ctx
						.join((MyActivityInput { x: 3 }, MyActivityInput { x: 34 }))
						.await?;

					WorkflowResult::Ok(sum + e.y + f.y)
				}
				.boxed()
			}),
		))
		.await?;

	Ok(MyParallelWorkflowOutput {
		y: a.y + b.y + c.y + d,
	})
}
