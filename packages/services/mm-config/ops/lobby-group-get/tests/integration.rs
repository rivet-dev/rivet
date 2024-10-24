use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let game_version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		..Default::default()
	})
	.await
	.unwrap();
	let version_id = game_version_res.version_id.unwrap();

	let version_res = op!([ctx] mm_config_version_get {
		version_ids: vec![version_id],
	})
	.await
	.unwrap();
	let version_res = &version_res.versions.first().unwrap();
	let lobby_group = version_res
		.config_meta
		.as_ref()
		.unwrap()
		.lobby_groups
		.first()
		.unwrap();

	let res = op!([ctx] mm_config_lobby_group_get {
		lobby_group_ids: vec![lobby_group.lobby_group_id.unwrap(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();

	assert_eq!(1, res.lobby_groups.len());
	let lobby_group_res = res.lobby_groups.first().unwrap();
	assert_eq!(lobby_group_res.lobby_group_id, lobby_group.lobby_group_id);
}
