use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] mm_dev_player_token_create {
		namespace_id: Some(Uuid::new_v4().into()),
		player_id: Some(Uuid::new_v4().into()),
	})
	.await
	.unwrap();
}
