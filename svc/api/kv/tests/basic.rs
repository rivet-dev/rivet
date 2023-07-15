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

		let pools = rivet_pools::from_env("api-kv-test").await.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-kv-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-kv-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-kv-test".to_string(),
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
		let (game_id, _, namespace_id, _, mm_config_meta) =
			Self::setup_game(&op_ctx, primary_region_id).await;

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
			base_path: util::env::svc_router_url("api-kv"),
			bearer_access_token: Some(bearer_token.to_string()),
			..Default::default()
		}
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
			creator_user_id: None,
			lobby_config_json: None,
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

	async fn setup_game(
		ctx: &OperationContext<()>,
		region_id: Uuid,
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
					listable: true,

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

					find_config: None,
					join_config: None,
					create_config: None,
				}],
			}),
			..Default::default()
		})
		.await
		.unwrap();

		let namespace_res = op!([ctx] faker_game_namespace {
			game_id: game_res.game_id,
			version_id: game_version_res.version_id,
			..Default::default()
		})
		.await
		.unwrap();

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
async fn generic() {
	let ctx = Ctx::init().await;
	let lobby_token = ctx.issue_lobby_token().await;

	// MARK: GET /entries
	{
		tracing::info!("get empty value");

		let res = kv_api::kv_get(&ctx.config(&lobby_token), "non/existent/value", None, None)
			.await
			.unwrap();

		assert!(
			matches!(res.value, None | Some(Value::Null)),
			"invalid get response"
		);
	}

	// MARK: PUT /entries
	let value1 = {
		tracing::info!("put value");

		let value = json!({
			"likes": 12
		});

		kv_api::kv_put(
			&ctx.config(&lobby_token),
			models::KvPutRequest {
				key: "some/value".into(),
				value: Some(value.clone()),
				namespace_id: None,
			},
		)
		.await
		.unwrap();

		value
	};

	// MARK: GET /entries
	{
		tracing::info!("get value");

		let res = kv_api::kv_get(&ctx.config(&lobby_token), "some/value", None, None)
			.await
			.unwrap();

		assert_eq!(res.value.unwrap(), value1, "wrong value received");
	}

	// MARK: PUT /entries
	let value2 = {
		tracing::info!("put another value");

		let value = json!({
			"follows": 100
		});

		kv_api::kv_put(
			&ctx.config(&lobby_token),
			models::KvPutRequest {
				key: "some/other-value".into(),
				value: Some(value.clone()),
				namespace_id: None,
			},
		)
		.await
		.unwrap();

		value
	};

	// MARK: GET /entries/batch
	{
		tracing::info!("get multiple values");

		// Individual keys
		{
			// OpenAPI generator does not support repeated query strings
			let res = reqwest::Client::new()
				.get(format!(
					"{}/entries/batch",
					util::env::svc_router_url("api-kv")
				))
				.bearer_auth(&lobby_token)
				.query(&[
					("keys", "some/value"),
					("keys", "some/other-value"),
					("keys", "non/existent"),
				])
				.send()
				.await
				.unwrap()
				.error_for_status()
				.unwrap()
				.json::<models::KvGetBatchResponse>()
				.await
				.unwrap();

			assert_eq!(res.entries.len(), 2, "wrong key count");

			let entry1 = res.entries.iter().find(|x| x.key == "some/value").unwrap();
			assert_eq!(*entry1.value.as_ref().unwrap(), value1);

			let entry2 = res
				.entries
				.iter()
				.find(|x| x.key == "some/other-value")
				.unwrap();
			assert_eq!(*entry2.value.as_ref().unwrap(), value2);
		}
	}

	// MARK: GET /entries
	{
		tracing::info!("watch value");

		let value = json!({
			"update": true
		});

		let (res, _) = tokio::join!(
			async {
				kv_api::kv_get(
					&ctx.config(&lobby_token),
					"some/value",
					Some(&util::timestamp::now().to_string()),
					None,
				)
				.await
				.unwrap()
			},
			async {
				// Wait before putting
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

				kv_api::kv_put(
					&ctx.config(&lobby_token),
					models::KvPutRequest {
						key: "some/value".into(),
						value: Some(value.clone()),
						namespace_id: None,
					},
				)
				.await
				.unwrap();
			}
		);

		assert_eq!(res.value.unwrap(), value, "wrong value received");
	}

	// MARK: GET /entries/batch
	{
		tracing::info!("watch multiple keys");

		let value = json!({
			"update": true
		});

		let (res, _) = tokio::join!(
			async {
				// OpenAPI generator does not support repeated query strings
				reqwest::Client::new()
					.get(format!(
						"{}/entries/batch",
						util::env::svc_router_url("api-kv")
					))
					.bearer_auth(&lobby_token)
					.query(&[
						("keys", "some/value"),
						("keys", "some/other-value"),
						("watch_index", &util::timestamp::now().to_string()),
					])
					.send()
					.await
					.unwrap()
					.error_for_status()
					.unwrap()
					.json::<models::KvGetBatchResponse>()
					.await
					.unwrap()
			},
			async {
				// Wait before putting
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

				kv_api::kv_put(
					&ctx.config(&lobby_token),
					models::KvPutRequest {
						key: "some/value".into(),
						value: Some(value.clone()),
						namespace_id: None,
					},
				)
				.await
				.unwrap();
			}
		);

		// Ordered alphabetically
		assert_eq!(res.entries.len(), 1, "wrong key count");

		let first = res.entries.first().unwrap();
		assert_eq!(first.key, "some/value", "wrong key");
		assert_eq!(*first.value.as_ref().unwrap(), value, "wrong value");
	}

	// MARK: PUT /put
	{
		tracing::info!("put value");

		let value = json!({
			"likes": 12
		});

		kv_api::kv_put(
			&ctx.config(&lobby_token),
			models::KvPutRequest {
				key: "some/value".into(),
				value: Some(value.clone()),
				namespace_id: None,
			},
		)
		.await
		.unwrap();
	};

	// MARK: DELETE /delete
	{
		tracing::info!("delete value");

		kv_api::kv_delete(&ctx.config(&lobby_token), "some/value", None)
			.await
			.unwrap();
	}

	// MARK: GET /get
	{
		tracing::info!("get value");

		let res = kv_api::kv_get(&ctx.config(&lobby_token), "some/value", None, None)
			.await
			.unwrap();

		assert!(
			matches!(res.value, Some(serde_json::Value::Null) | None),
			"wrong value received"
		);
	}

	// TODO:
	// - watch tests with specific anchor
	// - tests with cloud token
	// - write tests for put batch and delete batch
	// - max key length
	// - max value length
}

#[tokio::test(flavor = "multi_thread")]
async fn list() {
	let ctx = Ctx::init().await;
	let cloud_token = ctx.issue_cloud_token().await;

	let values = (0..12).map(|i| json!({ "idx": i })).collect::<Vec<_>>();

	// Write data
	for (i, value) in values.iter().enumerate() {
		kv_api::kv_put(
			&ctx.config(&cloud_token),
			models::KvPutRequest {
				key: format!("root-{i}"),
				value: Some(value.clone()),
				namespace_id: Some(ctx.namespace_id),
			},
		)
		.await
		.unwrap();

		kv_api::kv_put(
			&ctx.config(&cloud_token),
			models::KvPutRequest {
				key: format!("value/idx-{i}"),
				value: Some(value.clone()),
				namespace_id: Some(ctx.namespace_id),
			},
		)
		.await
		.unwrap();
	}

	// kv-list is not consistent for performance
	tokio::time::sleep(Duration::from_secs(2)).await;

	// List root
	let res = kv_api::kv_list(&ctx.config(&cloud_token), "", &ctx.namespace_id.to_string())
		.await
		.unwrap();

	assert_eq!(
		values.len(),
		res.entries.len(),
		"wrong value count returned"
	);

	// List subdir
	let res = kv_api::kv_list(
		&ctx.config(&cloud_token),
		"value",
		&ctx.namespace_id.to_string(),
	)
	.await
	.unwrap();

	assert_eq!(
		values.len(),
		res.entries.len(),
		"wrong value count returned"
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
