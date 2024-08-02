use chirp_workflow::prelude::*;
use serde_json::json;

#[workflow_test]
async fn empty(ctx: TestCtx) {
	let tags = json!({
		"amog": "us",
	});

	let id = ctx
		.dispatch_tagged_workflow(&tags, foo_worker::workflows::test::TestInput { x: -2 })
		.await
		.unwrap();

	tokio::time::sleep(std::time::Duration::from_secs(12)).await;

	ctx.tagged_signal(&tags, foo_worker::workflows::test::FooBarSignal { x: 400 })
		.await
		.unwrap();

	let res = ctx
		.wait_for_workflow::<foo_worker::workflows::test::Test>(id)
		.await
		.unwrap();

	tracing::info!(?res);
}
