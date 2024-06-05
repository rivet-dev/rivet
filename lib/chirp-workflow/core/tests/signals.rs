use anyhow::*;
use futures_util::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use wf::*;

mod common;

use crate::common::MyActivityInput;

#[tokio::test]
async fn signals() -> Result<()> {
	common::setup();

	let db =
		DatabasePostgres::new("postgres://root@127.0.0.1:26257/postgres?sslmode=disable").await?;

	let mut registry = Registry::new();
	registry.register_workflow::<MySignalWorkflow>();
	let registry = registry.handle();

	let worker = Worker::new(registry.clone(), db.clone());
	tokio::spawn(async move {
		if let Err(err) = worker.start().await {
			tracing::error!(?err, "worker failed");
		}
	});

	let ctx = common::TestCtx::new(db.clone());

	// Run 20 workflows at once
	futures_util::stream::iter(0..20)
		.map(|_| async {
			let workflow_id = ctx
				.dispatch_workflow(MySignalWorkflowInput { x: 5 })
				.await?;

			ctx.signal(workflow_id, MySignal { x: 12 }).await?;

			tokio::time::sleep(std::time::Duration::from_secs(5)).await;
			ctx.signal(workflow_id, MySignal2 { y: 3 }).await?;

			let output = ctx
				.wait_for_workflow::<MySignalWorkflow>(workflow_id)
				.await?;
			assert_eq!(25, output.y);

			Ok(())
		})
		.buffer_unordered(100)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MySignalWorkflowInput {
	x: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MySignalWorkflowOutput {
	y: i64,
}

#[macros::workflow(MySignalWorkflow)]
async fn my_signal_workflow(
	ctx: &mut WorkflowCtx,
	input: &MySignalWorkflowInput,
) -> Result<MySignalWorkflowOutput> {
	let a = ctx.activity(MyActivityInput { x: input.x }).await?;

	let b = ctx.listen::<MySignal>().await?;

	let c = match ctx.listen::<Join>().await? {
		Join::MySignal(sig) => sig.x,
		Join::MySignal2(sig) => sig.y,
	};

	Ok(MySignalWorkflowOutput { y: a.y + b.x + c })
}

#[macros::signal("my-signal")]
struct MySignal {
	x: i64,
}

#[macros::signal("my-signal2")]
struct MySignal2 {
	y: i64,
}

join_signal!(Join, [MySignal, MySignal2]);
