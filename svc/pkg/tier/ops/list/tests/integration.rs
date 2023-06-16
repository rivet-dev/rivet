use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] tier_list {
		region_ids: vec![Uuid::new_v4().into()]
	})
	.await
	.unwrap();
}
