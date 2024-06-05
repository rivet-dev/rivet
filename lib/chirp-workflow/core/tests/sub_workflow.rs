use anyhow::*;
use futures_util::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use wf::*;

mod common;

use crate::common::MyActivityInput;

#[tokio::test]
async fn sub_workflow() -> Result<()> {
	common::setup();

	let db =
		DatabasePostgres::new("postgres://root@127.0.0.1:26257/postgres?sslmode=disable").await?;

	let mut registry = Registry::new();
	registry.register_workflow::<MyWorkflow>();
	registry.register_workflow::<MySubWorkflow>();
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
			let output = ctx.workflow(MyWorkflowInput { x: 5 }).await?;
			assert_eq!(60, output.y);

			Ok(())
		})
		.buffer_unordered(100)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}

// MARK: Parallelized workflow
#[derive(Debug, Serialize, Deserialize)]
pub struct MyWorkflowInput {
	x: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MyWorkflowOutput {
	y: i64,
}

// GOTCHA: "returning this value requires that `'life1` must outlive `'static`" error comes from trying to use
// the `input` variable of a workflow in a closure without cloning it
#[macros::workflow(MyWorkflow)]
async fn my_workflow(ctx: &mut WorkflowCtx, input: &MyWorkflowInput) -> Result<MyWorkflowOutput> {
	let a = ctx.activity(MyActivityInput { x: input.x }).await?;

	let b = ctx.workflow(MySubWorkflowInput { x: input.x }).await?;

	let c = ctx.activity(MyActivityInput { x: input.x }).await?;

	Ok(MyWorkflowOutput { y: a.y + b.y + c.y })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MySubWorkflowInput {
	x: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MySubWorkflowOutput {
	y: i64,
}

// GOTCHA: "returning this value requires that `'life1` must outlive `'static`" error comes from trying to use
// the `input` variable of a workflow in a closure without cloning it
#[macros::workflow(MySubWorkflow)]
async fn my_sub_workflow(
	ctx: &mut WorkflowCtx,
	input: &MySubWorkflowInput,
) -> Result<MySubWorkflowOutput> {
	let a = ctx.activity(MyActivityInput { x: input.x }).await?;

	Ok(MySubWorkflowOutput { y: a.y * 4 })
}
