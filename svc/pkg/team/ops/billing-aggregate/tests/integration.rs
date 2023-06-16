use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let region_res = op!([ctx] faker_region {}).await.unwrap();

	let version_get_res = op!([ctx] mm_config_version_get {
		version_ids: game_res.version_ids.clone(),
	})
	.await
	.unwrap();
	let version = version_get_res.versions.first().unwrap();
	let config_meta = version.config_meta.as_ref().unwrap();
	let lobby_group = config_meta.lobby_groups.first().unwrap();
	let lobby_group_id = lobby_group.lobby_group_id.unwrap().as_uuid();

	let lobby_id = Uuid::new_v4();
	op!([ctx] faker_mm_lobby_row {
		lobby_id: Some(lobby_id.into()),
		namespace_id: Some(*game_res.namespace_ids.first().unwrap()),
		lobby_group_id: Some(lobby_group_id.into()),
		region_id: region_res.region_id,
		run_id: Some(Uuid::new_v4().into()),
		create_ts: Some(0),
		stop_ts: Some(util::duration::days(30)),
	})
	.await
	.unwrap();

	let game_get_res = op!([ctx] game_get {
		game_ids: vec![game_res.game_id.unwrap()]
	})
	.await
	.unwrap();
	let team_id = game_get_res.games.first().unwrap().developer_team_id;

	tokio::time::sleep(std::time::Duration::from_secs(5)).await;

	let res = op!([ctx] team_billing_aggregate {
		teams: vec![team::billing_aggregate::request::TeamBillingRequest {
			team_id,
			query_start: 0,
			query_end: util::timestamp::now(),
		}]
	})
	.await
	.unwrap();

	let metrics = res
		.teams
		.first()
		.unwrap()
		.games
		.first()
		.unwrap()
		.metrics
		.first()
		.unwrap();

	assert_eq!(
		metrics.uptime,
		util::duration::days(30) / 1000,
		"uptime wrong"
	);
}
