use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] faker_mm_lobby_row {
		lobby_id: Some(Uuid::new_v4().into()),
		namespace_id: Some(Uuid::new_v4().into()),
		lobby_group_id: Some(Uuid::new_v4().into()),
		region_id: Some(Uuid::new_v4().into()),
		run_id: Some(Uuid::new_v4().into()),
		create_ts: Some(100),
		stop_ts: Some(200),
	})
	.await
	.unwrap();
}
