use chirp_workflow::prelude::*;

#[workflow_test]
async fn empty(ctx: TestCtx) {
	let res = ctx
		.workflow(foo_worker::workflows::test::TestInput { x: 12 })
		.await
		.unwrap();

	tracing::info!(?res);
}
