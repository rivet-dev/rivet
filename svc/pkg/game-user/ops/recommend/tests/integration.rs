use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] game_user_recommend { count: 10 }).await.unwrap();
}
