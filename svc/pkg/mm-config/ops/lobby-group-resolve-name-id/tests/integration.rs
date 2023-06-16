use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let region_res = op!([ctx] faker_region {}).await.unwrap();

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: faker::build::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();

	let name_ids = vec!["test-1", "test-2", "test-3"];
	let game_version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
			lobby_groups: name_ids
				.iter()
				.map(|name_id| backend::matchmaker::LobbyGroup {
					name_id: name_id.to_string(),

					regions: vec![backend::matchmaker::lobby_group::Region {
						region_id: region_res.region_id,
						tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
						idle_lobbies: None,
					}],
					max_players_normal: 1,
					max_players_direct: 1,
					max_players_party: 1,

					runtime: Some(backend::matchmaker::lobby_runtime::Docker {
						build_id: build_res.build_id,
						args: Vec::new(),
						env_vars: Vec::new(),
						network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
						ports: Vec::new(),
					}.into()),
				})
				.collect(),
		}),
		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] mm_config_lobby_group_resolve_name_id {
		version_id: game_version_res.version_id,
		name_ids: name_ids
			.iter()
			.map(|v| v.to_string())
			.chain(vec!["nonexistent".to_string()].into_iter())
			.collect(),
	})
	.await
	.unwrap();

	assert_eq!(name_ids.len(), res.lobby_groups.len());
}
