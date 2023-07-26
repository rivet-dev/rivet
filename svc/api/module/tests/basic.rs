use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde_json::{json, Value};
use std::{str::FromStr, sync::Once, time::Duration};

use rivet_api::{
	apis::{configuration::Configuration, *},
	models,
};

const LOBBY_GROUP_NAME_ID: &str = "test";

static GLOBAL_INIT: Once = Once::new();

struct Ctx {
	op_ctx: OperationContext<()>,
	game_id: Uuid,
	primary_region_id: Uuid,
	namespace_id: Uuid,
	mm_config_meta: backend::matchmaker::VersionConfigMeta,
}

impl Ctx {
	async fn init() -> Ctx {
		GLOBAL_INIT.call_once(|| {
			tracing_subscriber::fmt()
				.pretty()
				.with_max_level(tracing::Level::INFO)
				.with_target(false)
				.without_time()
				.init();
		});

		let pools = rivet_pools::from_env("api-module-test").await.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-module-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-module-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-module-test".to_string(),
			std::time::Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			util::timestamp::now(),
			util::timestamp::now(),
			(),
			Vec::new(),
		);

		let (primary_region_id, _) = Self::setup_region(&op_ctx).await;
		let (_, module_version_id) = Self::setup_module(&op_ctx).await;
		let (game_id, _, namespace_id, _, mm_config_meta) =
			Self::setup_game(&op_ctx, primary_region_id, module_version_id).await;

		Ctx {
			op_ctx,
			game_id,
			primary_region_id,
			namespace_id,
			mm_config_meta,
		}
	}

	fn config(&self, bearer_token: &str) -> Configuration {
		Configuration {
			base_path: util::env::svc_router_url("api-module"),
			bearer_access_token: Some(bearer_token.to_string()),
			..Default::default()
		}
	}

	async fn issue_ns_token(&self) -> String {
		let token_res = op!([self.op_ctx] cloud_namespace_token_public_create {
			namespace_id: Some(self.namespace_id.into()),
		})
		.await
		.unwrap();

		token_res.token
	}

	async fn issue_lobby_token(&self) -> String {
		// Create lobby
		let lobby_group_meta = &self.mm_config_meta.lobby_groups[0];
		let lobby_id = Uuid::new_v4();

		msg!([self.op_ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
			lobby_id: Some(lobby_id.into()),
			namespace_id: Some(self.namespace_id.into()),
			lobby_group_id: lobby_group_meta.lobby_group_id,
			region_id: Some(self.primary_region_id.into()),
			create_ray_id: None,
			preemptively_created: false,
		})
		.await
		.unwrap();

		lobby_token(&self.op_ctx, lobby_id.to_string().as_str()).await
	}

	async fn issue_cloud_token(&self) -> String {
		let res = op!([self.op_ctx] cloud_game_token_create {
			game_id: Some(self.game_id.into()),
		})
		.await
		.unwrap();

		res.token
	}

	async fn setup_region(ctx: &OperationContext<()>) -> (Uuid, String) {
		tracing::info!("setup region");

		let region_res = op!([ctx] faker_region {}).await.unwrap();
		let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

		let get_res = op!([ctx] region_get {
			region_ids: vec![region_id.into()],
		})
		.await
		.unwrap();
		let region_data = get_res.regions.first().unwrap();

		(region_id, region_data.name_id.clone())
	}

	async fn setup_module(ctx: &OperationContext<()>) -> (Uuid, Uuid) {
		let module_id = Uuid::new_v4();
		let version_id = Uuid::new_v4();

		msg!([ctx] module::msg::create(module_id) -> module::msg::create_complete {
			module_id: Some(module_id.into()),
			name_id: "test".into(),
			team_id: Some(Uuid::new_v4().into()),
			creator_user_id: None,
		})
		.await
		.unwrap();

		msg!([ctx] module::msg::version_create(version_id) -> module::msg::version_create_complete {
			version_id: Some(version_id.into()),
			module_id: Some(module_id.into()),
			creator_user_id: None,

			major: 1,
			minor: 0,
			patch: 0,

			functions: vec![
				backend::module::Function {
					name: "foo".into(),
					request_schema: "{}".into(),
					response_schema: "{}".into(),
					callable: Some(backend::module::function::Callable {}),
				},
			],

			image: Some(module::msg::version_create::message::Image::Docker(module::msg::version_create::message::Docker {
				image_tag: "ghcr.io/rivet-gg/rivet-module-hello-world:0.0.1".into(),
			})),
		}).await.unwrap();

		(module_id, version_id)
	}

	async fn setup_game(
		ctx: &OperationContext<()>,
		region_id: Uuid,
		module_version_id: Uuid,
	) -> (
		Uuid,
		Uuid,
		Uuid,
		backend::matchmaker::VersionConfig,
		backend::matchmaker::VersionConfigMeta,
	) {
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

		let game_version_res = op!([ctx] faker_game_version {
			game_id: game_res.game_id,
			override_cdn_config: Some(faker::game_version::request::OverrideCdnConfig {
				config: None,
			}),
			override_module_config: Some(faker::game_version::request::OverrideModuleConfig {
				config: Some(backend::module::GameVersionConfig {
					dependencies: vec![
						backend::module::game_version_config::Dependency {
							key: "hello-world".into(),
							module_version_id: Some(module_version_id.into()),
						}
					]
				})
			}),
			override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
				lobby_groups: vec![backend::matchmaker::LobbyGroup {
					name_id: LOBBY_GROUP_NAME_ID.into(),

					regions: vec![backend::matchmaker::lobby_group::Region {
						region_id: Some(region_id.into()),
						tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
						idle_lobbies: None,
					}],
					max_players_normal: 8,
					max_players_direct: 10,
					max_players_party: 12,

					runtime: Some(backend::matchmaker::lobby_runtime::Docker {
						build_id: build_res.build_id,
						args: Vec::new(),
						env_vars: Vec::new(),
						network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
						ports: vec![
							backend::matchmaker::lobby_runtime::Port {
								label: "test-80-http".into(),
								target_port: Some(80),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-80-https".into(),
								target_port: Some(80),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-5050-https".into(),
								target_port: Some(5050),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
						],
					}.into()),
				}],
			}),
			..Default::default()
		})
		.await
		.unwrap();

		let mut module_ns_version_set_complete =
			subscribe!([ctx] module::msg::ns_version_set_complete("*"))
				.await
				.unwrap();
		let namespace_res = op!([ctx] faker_game_namespace {
			game_id: game_res.game_id,
			version_id: game_version_res.version_id,
			..Default::default()
		})
		.await
		.unwrap();
		module_ns_version_set_complete.next().await.unwrap();

		(
			game_res.game_id.as_ref().unwrap().as_uuid(),
			game_version_res.version_id.as_ref().unwrap().as_uuid(),
			namespace_res.namespace_id.as_ref().unwrap().as_uuid(),
			game_version_res.mm_config.clone().unwrap(),
			game_version_res.mm_config_meta.clone().unwrap(),
		)
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn call() {
	let ctx = Ctx::init().await;
	let token = ctx.issue_ns_token().await;

	let res = module_api::module_call(
		&ctx.config(&token),
		"hello-world",
		"foo",
		models::ModuleCallRequest {
			namespace_id: None,
			data: Some(json!({
				"x": 5
			})),
		},
		None,
	)
	.await
	.unwrap();
	assert_eq!(
		10,
		res.data
			.unwrap()
			.as_object()
			.unwrap()
			.get("y")
			.unwrap()
			.as_i64()
			.unwrap()
	);
}

/// Issues a testing lobby token. We use this since we can't access the lobby token issued
/// on the lobby creation.
async fn lobby_token(ctx: &OperationContext<()>, lobby_id: &str) -> String {
	let token_res = op!([ctx] token_create {
		issuer: "test".into(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::days(365),
		}),
		refresh_token_config: None,
		client: None,
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::MatchmakerLobby(proto::claims::entitlement::MatchmakerLobby {
							lobby_id: Some(Uuid::from_str(lobby_id).unwrap().into()),
						})
					)
				}
			],
		})),
		label: Some("lobby".into()),
		..Default::default()
	})
	.await
	.unwrap();

	token_res.token.as_ref().unwrap().token.clone()
}
