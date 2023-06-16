use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
}
