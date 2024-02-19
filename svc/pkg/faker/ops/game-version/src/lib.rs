use proto::backend::{self, cdn::*, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "faker-game-version")]
async fn handle(
	ctx: OperationContext<faker::game_version::Request>,
) -> GlobalResult<faker::game_version::Response> {
	let game_id = unwrap_ref!(ctx.game_id);

	let region_res = op!([ctx] faker_region {
		..Default::default()
	})
	.await?;

	let config = if let Some(config) = ctx.override_config.clone() {
		config
	} else {
		backend::cloud::VersionConfig {
			cdn: if let Some(config) = ctx.override_cdn_config.clone() {
				config.config
			} else {
				let cdn_site_res = op!([ctx] faker_cdn_site {
					game_id: Some(*game_id),
				})
				.await?;
				let site_id = unwrap_ref!(cdn_site_res.site_id);

				Some(VersionConfig {
					site_id: Some(*site_id),
					routes: vec![
						Route {
							glob: Some(util::glob::Glob::parse("test-glob")?.into()),
							priority: 0,
							middlewares: vec![Middleware {
								kind: Some(middleware::Kind::CustomHeaders(
									CustomHeadersMiddleware {
										headers: vec![custom_headers_middleware::Header {
											name: "header-name".to_string(),
											value: "header-value".to_string(),
										}],
									},
								)),
							}],
						},
						Route {
							glob: Some(util::glob::Glob::parse("test-glob2")?.into()),
							priority: 1,
							middlewares: vec![Middleware {
								kind: Some(middleware::Kind::CustomHeaders(
									CustomHeadersMiddleware {
										headers: vec![custom_headers_middleware::Header {
											name: "header-name2".to_string(),
											value: "header-value2".to_string(),
										}],
									},
								)),
							}],
						},
					],
				})
			},
			matchmaker: if let Some(config) = ctx.override_mm_config.clone() {
				config.config
			} else {
				let lobby_groups = if let Some(lobby_groups) = &ctx.override_lobby_groups {
					lobby_groups.lobby_groups.clone()
				} else {
					let build_res = op!([ctx] faker_build {
						game_id: Some(*game_id),
						image: backend::faker::Image::MmLobbyAutoReady as i32,
					})
					.await?;
					let build_id = unwrap_ref!(build_res.build_id);

					vec![backend::matchmaker::LobbyGroup {
						name_id: "test-1".into(),

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

						max_players_normal: 8,
						max_players_direct: 10,
						max_players_party: 12,
						listable: true,
						taggable: true,
						allow_dynamic_max_players: false,

						runtime: Some(
							backend::matchmaker::lobby_runtime::Docker {
								build_id: Some(*build_id),
								args: Vec::new(),
								env_vars: Vec::new(),
								network_mode:
									backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
								ports: Vec::new(),
							}
							.into(),
						),

						actions: None,
					}]
				};

				Some(backend::matchmaker::VersionConfig {
					lobby_groups,
					captcha: ctx
						.override_captcha
						.clone()
						.map_or_else(|| None, |config| config.captcha_config),
				})
			},
			kv: if let Some(config) = ctx.override_kv_config.clone() {
				config.config
			} else {
				Some(backend::kv::VersionConfig {})
			},
			identity: if let Some(config) = ctx.override_identity_config.clone() {
				config.config
			} else {
				Some(backend::identity::VersionConfig {
					custom_display_names: vec![backend::identity::CustomDisplayName {
						display_name: "Guest".to_string(),
					}],
					custom_avatars: Vec::new(),
				})
			},
			module: if let Some(config) = ctx.override_module_config.clone() {
				config.config
			} else {
				Some(backend::module::GameVersionConfig {
					dependencies: Vec::new(),
				})
			},
		}
	};

	let version_create_res = op!([ctx] cloud_version_publish {
		game_id: Some(*game_id),
		display_name: util::faker::display_name(),
		config: Some(config),
	})
	.await?;

	// Automatically deploy version
	if let Some(namespace_id) = ctx.deploy_to_namespace_id {
		op!([ctx] game_namespace_version_set {
			namespace_id: Some(namespace_id),
			version_id: version_create_res.version_id,
		})
		.await?;
	}

	let version_get = op!([ctx] mm_config_version_get {
		version_ids: vec![version_create_res.version_id.unwrap()],
	})
	.await?;
	let version_data = unwrap!(version_get.versions.first());

	Ok(faker::game_version::Response {
		version_id: version_create_res.version_id,
		mm_config: version_data.config.clone(),
		mm_config_meta: version_data.config_meta.clone(),
	})
}
