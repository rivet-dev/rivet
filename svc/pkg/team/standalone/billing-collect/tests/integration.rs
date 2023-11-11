use chirp_worker::prelude::*;
use indoc::indoc;
use proto::backend::pkg::*;

use std::time::Duration;

use ::team_billing_collect::run_from_env;

#[tokio::test]
async fn all() {
	// Run tests sequentially
	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	let ctx = TestCtx::from_env("all").await.unwrap();
	let crdb_pool = ctx.crdb().await.unwrap();

	test(ctx.clone(), crdb_pool.clone()).await;
}

// NACK: The balance will always be 1 in this case since it rounds up from a sub 1 cent value
async fn test(ctx: TestCtx, crdb: CrdbPool) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	tokio::time::sleep(Duration::from_secs(1)).await;

	let game_get_res = op!([ctx] game_get {
		game_ids: vec![lobby_res.game_id.unwrap()]
	})
	.await
	.unwrap();
	let team_id = game_get_res.games.first().unwrap().developer_team_id;

	tokio::time::sleep(std::time::Duration::from_secs(5)).await;

	op!([ctx] team_billing_aggregate {
		teams: vec![
			team::billing_aggregate::request::TeamBillingRequest {
				team_id,
				query_start: 0,
				query_end: util::timestamp::now(),
			}]
	})
	.await
	.unwrap();

	run_from_env(util::timestamp::now()).await.unwrap();

	let (last_collection_ts,) = sqlx::query_as::<_, (i64,)>(indoc!(
		"
		SELECT last_collection_ts
		FROM db_team_dev.dev_teams
		WHERE team_id = $1
		"
	))
	.bind(team_id.unwrap().as_uuid())
	.fetch_one(&crdb)
	.await
	.unwrap();

	assert_ne!(last_collection_ts, 0, "collection failed");

	// TODO: Check stripe event was created
}
