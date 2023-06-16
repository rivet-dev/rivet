use proto::backend::{self, cdn::*, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "faker-game-version")]
async fn handle(
	ctx: OperationContext<faker::game_version::Request>,
) -> GlobalResult<faker::game_version::Response> {
	let game_id = internal_unwrap!(ctx.game_id);

	let region_list_res = op!([ctx] region_list {
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
				let site_id = internal_unwrap!(cdn_site_res.site_id);

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
				let build_res = op!([ctx] faker_build {
					game_id: Some(*game_id),
					image: faker::build::Image::MmLobbyAutoReady as i32,
				})
				.await?;
				let build_id = internal_unwrap!(build_res.build_id);

				Some(backend::matchmaker::VersionConfig {
					lobby_groups: ctx.override_lobby_groups.clone().map_or_else(
						|| {
							vec![backend::matchmaker::LobbyGroup {
								name_id: "test-1".into(),

								regions: region_list_res
									.region_ids
									.iter()
									.cloned()
									.map(|region_id| backend::matchmaker::lobby_group::Region {
										region_id: Some(region_id),
										tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
										idle_lobbies: None,
									})
									.collect(),

								max_players_normal: 8,
								max_players_direct: 10,
								max_players_party: 12,

								runtime: Some(
									backend::matchmaker::lobby_runtime::Docker {
										build_id: Some(*build_id),
										args: Vec::new(),
										env_vars: Vec::new(),
										network_mode:
											backend::matchmaker::lobby_runtime::NetworkMode::Bridge
												as i32,
										ports: Vec::new(),
									}
									.into(),
								),
							}]
						},
						|v| v.lobby_groups,
					),
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
		}
	};

	let version_create_res = op!([ctx] cloud_version_publish {
		game_id: Some(*game_id),
		display_name: util::faker::display_name(),
		config: Some(config),
	})
	.await?;

	let version_get = op!([ctx] mm_config_version_get {
		version_ids: vec![version_create_res.version_id.unwrap()],
	})
	.await?;
	let version_data = version_get.versions.first().unwrap();

	Ok(faker::game_version::Response {
		version_id: version_create_res.version_id,
		mm_config: version_data.config.clone(),
		mm_config_meta: version_data.config_meta.clone(),
	})
}
