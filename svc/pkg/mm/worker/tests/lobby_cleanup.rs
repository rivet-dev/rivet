use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	msg!([ctx] mm::msg::lobby_cleanup(lobby_id) -> mm::msg::lobby_cleanup_complete {
		lobby_id: Some(lobby_id.into()),
	})
	.await
	.unwrap();

	let crdb = ctx.crdb("db-mm-state").await.unwrap();

	let (stop_ts,) =
		sqlx::query_as::<_, (Option<i64>,)>("SELECT stop_ts FROM lobbies WHERE lobby_id = $1")
			.bind(lobby_id)
			.fetch_one(&crdb)
			.await
			.unwrap();
	assert!(stop_ts.is_some(), "lobby not removed");

	let players =
		sqlx::query_as::<_, (Option<i64>,)>("SELECT remove_ts FROM players WHERE lobby_id = $1")
			.bind(lobby_id)
			.fetch_all(&crdb)
			.await
			.unwrap();
	for (remove_ts,) in players {
		assert!(remove_ts.is_some(), "player not removed");
	}
}
