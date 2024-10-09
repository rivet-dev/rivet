use chirp_workflow::prelude::*;

#[workflow_test]
async fn usage_get(ctx: &TestCtx) {
	ctx.op(pegboard::ops::client::usage_get::Input {
		client_ids: vec![util::uuid::parse("1bbd29ce-1a8f-4644-9c58-64db8efb7b10").unwrap()],
	})
	.await
	.unwrap();
}
