use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn lobby_closed_set(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	msg!([ctx] mm::msg::lobby_closed_set(lobby_id) -> mm::msg::lobby_closed_set_complete {
		lobby_id: lobby_res.lobby_id,
		is_closed: true,
	})
	.await
	.unwrap();

	let (sql_is_closed,) = sqlx::query_as::<_, (bool,)>(
		"SELECT is_closed FROM db_mm_state.lobbies WHERE lobby_id = $1",
	)
	.bind(lobby_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	assert!(sql_is_closed, "lobby closed");
}
