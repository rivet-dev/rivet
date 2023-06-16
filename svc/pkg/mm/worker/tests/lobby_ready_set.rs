use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		skip_set_ready: true,
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	msg!([ctx] mm::msg::lobby_ready(lobby_id) -> mm::msg::lobby_ready_complete {
		lobby_id: Some(lobby_id.into()),
	})
	.await
	.unwrap();

	let (ready_ts,) =
		sqlx::query_as::<_, (Option<i64>,)>("SELECT ready_ts FROM lobbies WHERE lobby_id = $1")
			.bind(lobby_id)
			.fetch_one(&ctx.crdb("db-mm-state").await.unwrap())
			.await
			.unwrap();
	assert!(ready_ts.is_some());
}
