use anyhow::*;
use futures_util::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use wf::*;

mod common;

use crate::common::MyActivityInput;

#[tokio::test]
async fn basic() -> Result<()> {
	common::setup();

	let db =
		DatabasePostgres::new("postgres://root@127.0.0.1:26257/postgres?sslmode=disable").await?;

	let mut registry = Registry::new();
	registry.register_workflow::<MyWorkflow>();
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
			assert_eq!(20, output.y);

			Ok(())
		})
		.buffer_unordered(100)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MyWorkflowInput {
	x: i64,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MyWorkflowOutput {
	y: i64,
}

#[macros::workflow(MyWorkflow)]
async fn my_workflow(ctx: &mut WorkflowCtx, input: &MyWorkflowInput) -> Result<MyWorkflowOutput> {
	let a = ctx.activity(MyActivityInput { x: input.x }).await?;
	let b = ctx.activity(MyActivityInput { x: a.y }).await?;

	let my_num = if b.y > 7 {
		ctx.activity(MyActivityInput { x: 10 }).await?.y
	} else {
		ctx.activity(MyActivityInput { x: 20 }).await?.y
	};

	Ok(MyWorkflowOutput { y: my_num })
}
