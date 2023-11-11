use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "faker-mm-lobby")]
async fn handle(
	ctx: OperationContext<faker::mm_lobby::Request>,
) -> GlobalResult<faker::mm_lobby::Response> {
	let max_players_normal = if ctx.max_players_normal != 0 {
		ctx.max_players_normal
	} else {
		8
	};
	let max_players_direct = if ctx.max_players_direct != 0 {
		ctx.max_players_direct
	} else {
		10
	};
	let max_players_party = if ctx.max_players_party != 0 {
		ctx.max_players_party
	} else {
		12
	};

	let region_res = op!([ctx] faker_region {}).await?;

	let (game_id, namespace_id) = if let Some(namespace_id) = ctx.namespace_id.as_ref() {
		let game_res = op!([ctx] game_resolve_namespace_id {
			namespace_ids: vec![*namespace_id],
		})
		.await?;
		let game = unwrap!(game_res.games.first());

		(unwrap_ref!(game.game_id).as_uuid(), namespace_id.as_uuid())
	} else {
		let game_res = op!([ctx] faker_game {
			..Default::default()
		})
		.await?;

		(
			unwrap_ref!(game_res.game_id).as_uuid(),
			unwrap_ref!(game_res.namespace_ids.first()).as_uuid(),
		)
	};

	let version_id = if let Some(version_id) = ctx.version_id.as_ref() {
		version_id.as_uuid()
	} else {
		let build_res = op!([ctx] faker_build {
			game_id: Some(game_id.into()),
			image: if let Some(image) = ctx.image {
				image
			} else if ctx.skip_set_ready {
				faker::build::Image::HangIndefinitely as i32
			} else {
				faker::build::Image::MmLobbyEcho as i32
			},
		})
		.await?;

		let game_version_res = op!([ctx] faker_game_version {
			game_id: Some(game_id.into()),
			override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
				lobby_groups: vec![backend::matchmaker::LobbyGroup {
					name_id: "faker-lg".into(),

					regions: vec![backend::matchmaker::lobby_group::Region {
						region_id: region_res.region_id,
						tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
						idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
							min_idle_lobbies: 0,
							// Set a high max lobby count in case this is
							// coming from a test that test mm-lobby-create
							// without creating an associated player
							max_idle_lobbies: 32,
						}),
					}],
					max_players_normal,
					max_players_direct,
					max_players_party,
					listable: true,

					runtime: Some(backend::matchmaker::lobby_runtime::Docker {
						build_id: build_res.build_id,
						args: Vec::new(),
						env_vars: Vec::new(),
						network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
						ports: vec![
							backend::matchmaker::lobby_runtime::Port {
								label: "test-http".into(),
								target_port: Some(8001),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-tcp".into(),
								target_port: Some(8002),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-udp".into(),
								target_port: Some(8002),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
						],
					}.into()),

					find_config: None,
					join_config: None,
					create_config: None,
				}],
			}),
			..Default::default()
		})
		.await?;
		unwrap_ref!(game_version_res.version_id).as_uuid()
	};

	let version_get_res = op!([ctx] mm_config_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;
	let version = version_get_res.versions.first();
	let version = unwrap_ref!(version);
	let config_meta = unwrap_ref!(version.config_meta);
	let lobby_group = config_meta.lobby_groups.first();
	let lobby_group = unwrap_ref!(lobby_group);
	let lobby_group_id = unwrap_ref!(lobby_group.lobby_group_id).as_uuid();

	op!([ctx] game_namespace_version_set {
		namespace_id: Some(namespace_id.into()),
		version_id: Some(version_id.into()),
	})
	.await?;

	let lobby_id = Uuid::new_v4();
	let subs = if !ctx.skip_set_ready {
		Some((
			subscribe!([ctx] mm::msg::lobby_ready_complete(lobby_id)).await?,
			subscribe!([ctx] mm::msg::lobby_create_fail(lobby_id)).await?,
			subscribe!([ctx] mm::msg::lobby_cleanup(lobby_id)).await?,
		))
	} else {
		None
	};

	let complete_msg =
		msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
			lobby_id: Some(lobby_id.into()),
			namespace_id: Some(namespace_id.into()),
			lobby_group_id: Some(lobby_group_id.into()),
			region_id: region_res.region_id,
			create_ray_id: Some(ctx.ray_id().into()),
			preemptively_created: false,

			creator_user_id: ctx.creator_user_id,
			is_custom: ctx.is_custom,
			publicity: ctx.publicity,
			lobby_config_json: ctx.lobby_config_json.clone(),
		})
		.await?;
	let run_id = unwrap_ref!(complete_msg.run_id).as_uuid();

	// TODO: Allow enabling fast-ready instead of waiting for the lobby itself to call ready
	// endpoint
	// if !ctx.skip_set_ready {
	// 	msg!([ctx] mm::msg::lobby_ready(lobby_id) {
	// 		lobby_id: Some(lobby_id.into()),
	// 	})
	// 	.await?;
	// }

	if let Some((mut ready_sub, mut fail_sub, mut cleanup_sub)) = subs {
		tokio::select! {
			msg = ready_sub.next() => {
				let _ = msg?;
				tracing::info!("ready");
			}
			msg = fail_sub.next() => {
				let msg = msg?;
				tracing::error!(?msg, "lobby create failed");
				rivet_operation::prelude::bail!("lobby create failed");
			}
			msg = cleanup_sub.next() => {
				let msg = msg?;
				tracing::error!(?msg, "lobby being cleaned up early");
				rivet_operation::prelude::bail!("lobby being cleaned up early");
			}
		}
	}

	Ok(faker::mm_lobby::Response {
		lobby_id: Some(lobby_id.into()),
		lobby_group_id: Some(lobby_group_id.into()),
		game_id: Some(game_id.into()),
		version_id: Some(version_id.into()),
		namespace_id: Some(namespace_id.into()),
		region_id: region_res.region_id,
		run_id: Some(run_id.into()),
	})
}
