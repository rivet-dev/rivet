use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

// TODO: Test player limits
// TODO: Test all failure cases

struct TestLobbyGroup {
	lobby_group_id: Uuid,
	#[allow(unused)]
	version_id: Uuid,
	namespace_id: Uuid,
	region_id: Uuid,
}

async fn create_lobby_group(ctx: &TestCtx, image: Option<faker::build::Image>) -> TestLobbyGroup {
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = game_res.namespace_ids.first().unwrap().as_uuid();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: image.unwrap_or(faker::build::Image::MmLobbyAutoReady) as i32,
	})
	.await
	.unwrap();

	let game_version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
			lobby_groups: vec![backend::matchmaker::LobbyGroup {
				name_id: "faker-lg".into(),

				regions: vec![backend::matchmaker::lobby_group::Region {
					region_id: region_res.region_id,
					tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
					idle_lobbies: None,
				}],
				max_players_normal: 8,
				max_players_direct: 10,
				max_players_party: 12,

				runtime: Some(backend::matchmaker::lobby_runtime::Docker {
					// We can't use `curlimages/curl` here because it doesn't allow for
					// variable interpolation, so we need a container that has a proper shell
					// that we can inject variables with.
					build_id: build_res.build_id,
					args: vec![],
					env_vars: vec![backend::matchmaker::lobby_runtime::EnvVar {
						key: "HELLO".into(),
						value: "world".into(),
					}],
					network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
					ports: vec![backend::matchmaker::lobby_runtime::Port {
						label: "1234".into(),
						target_port: Some(1234),
						port_range: None,
						proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
						proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
					}],
				}.into()),

				find_config: None,
				join_config: None,
			}],
		}),
		..Default::default()
	})
	.await
	.unwrap();
	let version_id = game_version_res.version_id.as_ref().unwrap().as_uuid();

	let version_get_res = op!([ctx] mm_config_version_get {
		version_ids: vec![version_id.into()],
	})
	.await
	.unwrap();
	let version = version_get_res.versions.first().unwrap();
	let config_meta = version.config_meta.as_ref().unwrap();
	let lobby_group = config_meta.lobby_groups.first().unwrap();
	let lobby_group_id = lobby_group.lobby_group_id.as_ref().unwrap().as_uuid();

	op!([ctx] game_namespace_version_set {
		namespace_id: Some(namespace_id.into()),
		version_id: Some(version_id.into()),
	})
	.await
	.unwrap();

	TestLobbyGroup {
		lobby_group_id,
		version_id,
		namespace_id,
		region_id,
	}
}

#[worker_test]
async fn direct(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let find_res = find(
		&ctx,
		lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
		gen_players(1),
		mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: lobby_res.lobby_id,
		}),
	)
	.await
	.unwrap();

	assert_eq!(lobby_res.lobby_id, find_res.lobby_id);
}

#[worker_test]
async fn lobby_group_existing(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let find_res = find(
		&ctx,
		lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
		gen_players(1),
		mm::msg::lobby_find::message::Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
			lobby_group_ids: vec![lobby_res.lobby_group_id.unwrap()],
			region_ids: vec![lobby_res.region_id.unwrap()],
			auto_create: Some(backend::matchmaker::query::AutoCreate {
				lobby_group_id: lobby_res.lobby_group_id,
				region_id: lobby_res.region_id,
			}),
		}),
	)
	.await
	.unwrap();

	assert_eq!(lobby_res.lobby_id, find_res.lobby_id, "fond wrong lobby");
}

#[worker_test]
async fn direct_closed(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	msg!([ctx] mm::msg::lobby_closed_set(lobby_id) -> mm::msg::lobby_closed_set_complete {
		lobby_id: Some(lobby_id.into()),
		is_closed: true,
	})
	.await
	.unwrap();

	let err = find(
		&ctx,
		lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
		gen_players(1),
		mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: lobby_res.lobby_id,
		}),
	)
	.await
	.unwrap_err();

	assert_eq!(
		mm::msg::lobby_find_fail::ErrorCode::LobbyClosed as i32,
		err.error_code
	);
}

#[worker_test]
async fn lobby_group_closed(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	msg!([ctx] mm::msg::lobby_closed_set(lobby_id) -> mm::msg::lobby_closed_set_complete {
		lobby_id: Some(lobby_id.into()),
		is_closed: true,
	})
	.await
	.unwrap();

	let err = find(
		&ctx,
		lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
		gen_players(1),
		mm::msg::lobby_find::message::Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
			lobby_group_ids: vec![lobby_res.lobby_group_id.unwrap()],
			region_ids: vec![lobby_res.region_id.unwrap()],
			auto_create: None,
		}),
	)
	.await
	.unwrap_err();

	assert_eq!(
		mm::msg::lobby_find_fail::ErrorCode::NoAvailableLobbies as i32,
		err.error_code
	);
}

#[worker_test]
async fn lobby_crash_immediate(ctx: TestCtx) {
	let lobby_group = create_lobby_group(&ctx, Some(faker::build::Image::FailImmediately)).await;

	let err = find(
		&ctx,
		lobby_group.namespace_id,
		gen_players(1),
		mm::msg::lobby_find::message::Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
			lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
			region_ids: vec![lobby_group.region_id.into()],
			auto_create: Some(backend::matchmaker::query::AutoCreate {
				lobby_group_id: Some(lobby_group.lobby_group_id.into()),
				region_id: Some(lobby_group.region_id.into()),
			}),
		}),
	)
	.await
	.unwrap_err();

	assert_eq!(
		mm::msg::lobby_find_fail::ErrorCode::LobbyStoppedPrematurely as i32,
		err.error_code
	);
}

#[worker_test]
async fn max_players_per_client(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let max_players_per_client = 3;
	op!([ctx] mm_config_namespace_config_set {
		namespace_id: lobby_res.namespace_id,
		lobby_count_max: 4,
		max_players_per_client: max_players_per_client,
		max_players_per_client_vpn: max_players_per_client,
		max_players_per_client_proxy: max_players_per_client,
		max_players_per_client_tor: max_players_per_client,
		max_players_per_client_hosting: max_players_per_client,
	})
	.await
	.unwrap();

	let fake_ip = util::faker::ip_addr_v4();
	tracing::info!(%fake_ip, "fake ip");

	for i in 0..(max_players_per_client + 2) {
		tracing::info!(i, "find iter");

		let res = find(
			&ctx,
			lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
			vec![mm::msg::lobby_find::Player {
				player_id: Some(Uuid::new_v4().into()),
				token_session_id: Some(Uuid::new_v4().into()),
				client_info: Some(backend::net::ClientInfo {
					user_agent: Some("Test".into()),
					remote_address: Some(fake_ip.to_string()),
				}),
			}],
			mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
				lobby_id: lobby_res.lobby_id,
			}),
		)
		.await;
		if i >= max_players_per_client {
			let err = res.unwrap_err();
			assert_eq!(
				mm::msg::lobby_find_fail::ErrorCode::TooManyPlayersFromSource as i32,
				err.error_code
			);
		} else {
			let _ = res.unwrap();
		}
	}
}

#[worker_test]
async fn lobby_group_auto_create(ctx: TestCtx) {
	let lobby_group = create_lobby_group(&ctx, None).await;

	find(
		&ctx,
		lobby_group.namespace_id,
		gen_players(1),
		mm::msg::lobby_find::message::Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
			lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
			region_ids: vec![lobby_group.region_id.into()],
			auto_create: Some(backend::matchmaker::query::AutoCreate {
				lobby_group_id: Some(lobby_group.lobby_group_id.into()),
				region_id: Some(lobby_group.region_id.into()),
			}),
		}),
	)
	.await
	.unwrap();
}

#[worker_test]
async fn lobby_group_no_auto_create(ctx: TestCtx) {
	let lobby_group = create_lobby_group(&ctx, None).await;

	let err = find(
		&ctx,
		lobby_group.namespace_id,
		gen_players(1),
		mm::msg::lobby_find::message::Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
			lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
			region_ids: vec![lobby_group.region_id.into()],
			auto_create: None,
		}),
	)
	.await
	.unwrap_err();

	assert_eq!(
		mm::msg::lobby_find_fail::ErrorCode::NoAvailableLobbies as i32,
		err.error_code
	);
}

fn gen_players(count: usize) -> Vec<mm::msg::lobby_find::Player> {
	let mut players = Vec::new();
	for _ in 0..count {
		players.push(mm::msg::lobby_find::Player {
			player_id: Some(Uuid::new_v4().into()),
			token_session_id: Some(Uuid::new_v4().into()),
			client_info: Some(backend::net::ClientInfo {
				user_agent: Some("Test".into()),
				remote_address: Some(util::faker::ip_addr_v4().to_string()),
			}),
		});
	}
	players
}

async fn find(
	ctx: &TestCtx,
	namespace_id: Uuid,
	players: Vec<mm::msg::lobby_find::Player>,
	query: mm::msg::lobby_find::message::Query,
) -> Result<
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_complete::Message>,
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_fail::Message>,
> {
	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: players,
		query: Some(query),
		verification_data_json: None,
	})
	.await
	.unwrap()
}
