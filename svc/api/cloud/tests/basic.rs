// TODO: Rewrite in fern api

// use std::{str::FromStr, sync::Once};

// use proto::{
// 	backend::{self, pkg::*},
// 	common,
// };
// use rivet_api::models;
// use rivet_operation::prelude::*;

// static GLOBAL_INIT: Once = Once::new();

// struct Ctx {
// 	op_ctx: OperationContext<()>,
// 	auth_token: String,
// 	team_id: Uuid,
// }

// impl Ctx {
// 	async fn init() -> Ctx {
// 		GLOBAL_INIT.call_once(|| {
// 			tracing_subscriber::fmt()
// 				.pretty()
// 				.with_max_level(tracing::Level::INFO)
// 				.with_target(false)
// 				.init();
// 		});

// 		let pools = rivet_pools::from_env("api-cloud-test").await.unwrap();
// 		let cache = rivet_cache::CacheInner::new(
// 			"api-cloud-test".to_string(),
// 			std::env::var("RIVET_SOURCE_HASH").unwrap(),
// 			pools.redis_cache().unwrap(),
// 		);
// 		let client = chirp_client::SharedClient::from_env(pools.clone())
// 			.expect("create client")
// 			.wrap_new("api-cloud-test");
// 		let conn = rivet_connection::Connection::new(client, pools, cache);
// 		let op_ctx = OperationContext::new(
// 			"api-cloud-test".to_string(),
// 			std::time::Duration::from_secs(60),
// 			conn,
// 			Uuid::new_v4(),
// 			Uuid::new_v4(),
// 			util::timestamp::now(),
// 			util::timestamp::now(),
// 			(),
// 			Vec::new(),
// 		);

// 		// Create temp team
// 		let (team_id, _team_user_ids, primary_user_id) = {
// 			// Create team
// 			tracing::info!("setup team");
// 			let create_res = op!([op_ctx] faker_team {
// 				is_dev: true,
// 				..Default::default()
// 			})
// 			.await
// 			.unwrap();
// 			let team_id = create_res.team_id.expect("team id").as_uuid();
// 			let member_user_ids = create_res
// 				.member_user_ids
// 				.iter()
// 				.map(common::Uuid::as_uuid)
// 				.collect::<Vec<_>>();
// 			let primary_user_id = member_user_ids.first().cloned().unwrap();

// 			// Register user
// 			op!([op_ctx] user_identity_create {
// 				user_id: Some(primary_user_id.into()),
// 				identity: Some(backend::user_identity::Identity {
// 					kind: Some(backend::user_identity::identity::Kind::Email(backend::user_identity::identity::Email {
// 						email: util::faker::email()
// 					}))
// 				})
// 			})
// 			.await
// 			.unwrap();

// 			(team_id, member_user_ids, primary_user_id)
// 		};

// 		// MARK: Setup auth
// 		// Encode user token
// 		let auth_token = {
// 			let token_res = op!([op_ctx] token_create {
// 				issuer: "test".into(),
// 				token_config: Some(token::create::request::TokenConfig {
// 					ttl: util::duration::hours(1),
// 				}),
// 				refresh_token_config: None,
// 				client: Some(backend::net::ClientInfo {
// 					user_agent: Some("Test".into()),
// 					remote_address: Some("0.0.0.0".into()),
// 				}),
// 				kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
// 					entitlements: vec![
// 						proto::claims::Entitlement {
// 							kind: Some(
// 								proto::claims::entitlement::Kind::User(proto::claims::entitlement::User {
// 									user_id: Some(primary_user_id.into()),
// 								})
// 							)
// 						}
// 					],
// 				})),
// 				label: Some("usr".into()),
// 				..Default::default()
// 			})
// 			.await
// 			.unwrap();
// 			let token = token_res.token.unwrap();

// 			token.token
// 		};

// 		Ctx {
// 			op_ctx,
// 			team_id,
// 			auth_token,
// 		}
// 	}

// 	fn http_client(&self, bearer_token: String) -> rivet_cloud::ClientWrapper {
// 		rivet_cloud::Config::builder()
// 			.set_uri("http://traefik.traefik.svc.cluster.local:80/cloud")
// 			.set_bearer_token(bearer_token)
// 			.build_client()
// 	}

// 	fn chirp(&self) -> &chirp_client::Client {
// 		self.op_ctx.chirp()
// 	}

// 	fn op_ctx(&self) -> &OperationContext<()> {
// 		&self.op_ctx
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn generic() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.auth_token.clone());

// 	// MARK: Setup
// 	// Create temp region
// 	let primary_region_id = {
// 		tracing::info!("setup region");

// 		let region_res = op!([ctx] faker_region {}).await.unwrap();
// 		let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

// 		region_id
// 	};

// 	// MARK: GET /auth/inspect
// 	{
// 		tracing::info!("testing auth inspect");

// 		let res = http_client.inspect().send().await.unwrap();

// 		tracing::info!(agent = ?res.agent, "auth agent");
// 	}

// 	// MARK: POST /games
// 	let game_id = {
// 		tracing::info!("creating game");

// 		let name_id = util::faker::ident();
// 		let display_name = util::faker::display_name();

// 		let res = http_client
// 			.create_game()
// 			.name_id(&name_id)
// 			.display_name(&display_name)
// 			.developer_group_id(ctx.team_id.to_string())
// 			.send()
// 			.await
// 			.unwrap();

// 		let game_res = op!([ctx] game_get {
// 			game_ids: vec![Uuid::from_str(res.game_id().unwrap()).unwrap().into()],
// 		})
// 		.await
// 		.unwrap();
// 		let game_data = game_res.games.first().unwrap();
// 		assert_eq!(game_data.name_id, name_id);
// 		assert_eq!(game_data.display_name, display_name);
// 		assert_eq!(
// 			game_data.developer_team_id.as_ref().unwrap().as_uuid(),
// 			ctx.team_id,
// 		);

// 		res.game_id().unwrap().to_string()
// 	};

// 	// MARK: GET /games
// 	{
// 		tracing::info!("listing games");

// 		let res = http_client.get_games().send().await.unwrap();

// 		assert_eq!(1, res.games().unwrap().len(), "wrong number of games");
// 		let game_res = res.games().unwrap().first().unwrap();
// 		assert_eq!(game_id, game_res.game_id().unwrap(), "returned wrong game");
// 	}

// 	// MARK: GET /games/{}
// 	{
// 		tracing::info!("getting full game");

// 		let res = http_client
// 			.get_game_by_id()
// 			.game_id(&game_id)
// 			.send()
// 			.await
// 			.unwrap();

// 		let game = res.game().unwrap();
// 		assert_eq!(game_id, game.game_id().unwrap(), "returned wrong game");
// 		assert_eq!(
// 			ctx.team_id.to_string(),
// 			game.developer_group_id().unwrap(),
// 			"game belongs to wrong group"
// 		);

// 		assert_eq!(2, game.namespaces().unwrap().len());
// 		let prod_ns_res = &game.namespaces().unwrap()[0];
// 		let staging_ns_res = &game.namespaces().unwrap()[1];
// 		assert_eq!("Production", prod_ns_res.display_name().unwrap());
// 		assert_eq!("Staging", staging_ns_res.display_name().unwrap());

// 		assert_eq!(1, game.versions().unwrap().len());
// 		let default_version_res = &game.versions().unwrap()[0];
// 		assert_eq!("0.0.1", default_version_res.display_name().unwrap());

// 		assert!(
// 			!game.available_regions().unwrap().is_empty(),
// 			"missing regions"
// 		);
// 		assert!(
// 			game.available_regions()
// 				.unwrap()
// 				.iter()
// 				.any(|x| x.region_id().unwrap() == primary_region_id.to_string()),
// 			"created region doesn't exist"
// 		);
// 	}

// 	// MARK: POST /games/{}/cdn/sites
// 	let site_id = {
// 		tracing::info!("uploading cdn site");

// 		const INDEX_BODY: &[u8] = b"Hello, world!";

// 		let res = {
// 			let display_name = util::faker::display_name();

// 			http_client
// 				.create_game_cdn_site()
// 				.game_id(&game_id)
// 				.files(
// 					model::upload_prepare_file::Builder::default()
// 						.path("index.html")
// 						.content_type("text/html")
// 						.content_length(INDEX_BODY.len() as i64)
// 						.build(),
// 				)
// 				.display_name(display_name)
// 				.send()
// 				.await
// 				.unwrap()
// 		};

// 		{
// 			tracing::info!("uploading file");

// 			let index_presigned_req = res.presigned_requests().unwrap().first().unwrap();
// 			let res = reqwest::Client::new()
// 				.put(index_presigned_req.url().unwrap())
// 				.header(http::header::CONTENT_TYPE, "text/html")
// 				.header(http::header::CONTENT_LENGTH, INDEX_BODY.len())
// 				.body(INDEX_BODY)
// 				.send()
// 				.await
// 				.unwrap();
// 			tracing::info!(status = %res.status(), "upload response");
// 			assert!(res.status().is_success(), "failed to upload site");
// 		}

// 		{
// 			tracing::info!("completing upload");

// 			let _res = http_client
// 				.complete_upload()
// 				.upload_id(res.upload_id().unwrap())
// 				.send()
// 				.await
// 				.unwrap();
// 		}

// 		res.site_id().unwrap().to_string()
// 	};

// 	// MARK: POST /games/{}/builds
// 	let build_id = {
// 		tracing::info!("creating game build");

// 		let (image_tag, image_body) = fetch_test_image().await;

// 		let display_name = util::faker::display_name();

// 		let res = http_client
// 			.create_game_build()
// 			.game_id(&game_id)
// 			.display_name(display_name)
// 			.image_tag(image_tag)
// 			.image_file(
// 				model::upload_prepare_file::Builder::default()
// 					.path("image.tar")
// 					.content_type("application/x-tar")
// 					.content_length(image_body.len() as i64)
// 					.build(),
// 			)
// 			.send()
// 			.await
// 			.unwrap();

// 		{
// 			tracing::info!("uploading file");

// 			let res = reqwest::Client::new()
// 				.put(res.image_presigned_request().unwrap().url().unwrap())
// 				.header(http::header::CONTENT_TYPE, "application/x-tar")
// 				.header(http::header::CONTENT_LENGTH, image_body.len())
// 				.body(image_body)
// 				.send()
// 				.await
// 				.unwrap();
// 			tracing::info!(status = %res.status(), "upload response");
// 			assert!(res.status().is_success(), "failed to upload build");
// 		}

// 		{
// 			tracing::info!("completing upload");

// 			let _res = http_client
// 				.complete_upload()
// 				.upload_id(res.upload_id().unwrap())
// 				.send()
// 				.await
// 				.unwrap();
// 		}

// 		res.build_id().unwrap().to_string()
// 	};

// 	// MARK: GET /games/{}/cdn/sites
// 	{
// 		tracing::info!("listing cdn");

// 		let res = http_client
// 			.list_game_cdn_sites()
// 			.game_id(&game_id)
// 			.send()
// 			.await
// 			.unwrap();

// 		assert!(
// 			res.sites()
// 				.unwrap()
// 				.iter()
// 				.any(|u| u.site_id().unwrap() == site_id),
// 			"did not return upload"
// 		);
// 	}

// 	// MARK: GET /games/{}/builds
// 	{
// 		tracing::info!("listing builds");

// 		let res = http_client
// 			.list_game_builds()
// 			.game_id(&game_id)
// 			.send()
// 			.await
// 			.unwrap();

// 		assert!(
// 			res.builds()
// 				.unwrap()
// 				.iter()
// 				.any(|u| u.build_id().unwrap() == build_id),
// 			"did not return upload"
// 		);
// 	}

// 	// MARK: POST /games/{}/versions
// 	let (version_id, mm_version_data) = {
// 		tracing::info!("creating version");

// 		let res = http_client
// 			.create_game_version()
// 			.game_id(&game_id)
// 			.display_name("test-version")
// 			.config(build_test_version_config(
// 				primary_region_id.to_string(),
// 				site_id,
// 				build_id,
// 			))
// 			.send()
// 			.await
// 			.unwrap();

// 		let version_get_res = op!([ctx] mm_config_version_get {
// 			version_ids: vec![Uuid::from_str(res.version_id().unwrap()).unwrap().into()],
// 		})
// 		.await
// 		.unwrap();
// 		let version_data = version_get_res.versions.first().unwrap().clone();

// 		(res.version_id().unwrap().to_string(), version_data)
// 	};

// 	// MARK: GET /games/{}/versions/{}
// 	{
// 		tracing::info!("reading version");

// 		let res = http_client
// 			.get_game_version_by_id()
// 			.game_id(&game_id)
// 			.version_id(&version_id)
// 			.send()
// 			.await
// 			.unwrap();

// 		assert_eq!(
// 			"test-version",
// 			res.version().unwrap().display_name().unwrap()
// 		);
// 	}

// 	// MARK: POST /games/{}/namespaces
// 	let namespace_id = {
// 		tracing::info!("creating namespace");

// 		let res = http_client
// 			.create_game_namespace()
// 			.game_id(&game_id)
// 			.display_name("Test Namespace")
// 			.version_id(&version_id)
// 			.name_id("test-ns")
// 			.send()
// 			.await
// 			.unwrap();

// 		res.namespace_id().unwrap().to_string()
// 	};

// 	// MARK: GET /games/{}/namespaces/{}
// 	{
// 		tracing::info!("reading namespaces");

// 		let _res = http_client
// 			.get_game_namespace_by_id()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: GET /games/{}/namespaces/{}/analytics/matchmaker/live (no matchmaker)
// 	{
// 		tracing::info!("reading matchmaker analytics without matchmaker");

// 		let res = http_client
// 			.get_namespace_analytics_matchmaker_live()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.send()
// 			.await
// 			.unwrap();

// 		assert!(res.lobbies().unwrap().is_empty());
// 	}

// 	// MARK: PUT /games/{}/namespaces/{}/version
// 	{
// 		tracing::info!("setting namespace version");

// 		let _res = http_client
// 			.update_game_namespace_version()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.version_id(&version_id)
// 			.send()
// 			.await
// 			.unwrap();

// 		let ns_get_res = op!([ctx] game_namespace_get {
// 			namespace_ids: vec![Uuid::from_str(namespace_id.as_str()).unwrap().into()],
// 		})
// 		.await
// 		.unwrap();
// 		let ns_res = ns_get_res.namespaces.first().expect("ns doesn't exist");
// 		assert_eq!(
// 			version_id,
// 			ns_res.version_id.as_ref().unwrap().as_uuid().to_string(),
// 			"version not updated"
// 		);
// 	}

// 	// MARK: GET /games/{}/namespaces/{}/analytics/matchmaker/live (with matchmaker)
// 	{
// 		tracing::info!("reading matchmaker analytics with matchmaker");

// 		let lobby_groups = &mm_version_data.config_meta.as_ref().unwrap().lobby_groups;
// 		let mut all_lobby_ids = Vec::new();
// 		for lobby_group in lobby_groups {
// 			let lobby_id = Uuid::new_v4();
// 			msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
// 				lobby_id: Some(lobby_id.into()),
// 				namespace_id: Some(Uuid::from_str(namespace_id.as_str()).unwrap().into()),
// 				lobby_group_id: lobby_group.lobby_group_id,
// 				region_id: Some(primary_region_id.into()),
// 				create_ray_id: None,
// 				preemptively_created: false,

// 				creator_user_id: None,
// 				is_custom: false,
// 				publicity: None,
// 				lobby_config_json: None,
// 				dynamic_max_players: None,
// 			})
// 			.await
// 			.unwrap();

// 			all_lobby_ids.push(lobby_id);
// 		}

// 		let res = http_client
// 			.get_namespace_analytics_matchmaker_live()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.send()
// 			.await
// 			.unwrap();

// 		for lobby_id in &all_lobby_ids {
// 			assert!(
// 				res.lobbies()
// 					.unwrap()
// 					.iter()
// 					.any(|l| l.lobby_id().unwrap() == lobby_id.to_string()),
// 				"did not list created lobby"
// 			);
// 		}
// 	}

// 	// MARK: POST /games/{}/namespaces/{}/tokens/public
// 	{
// 		tracing::info!("creating public namespace token");

// 		let _res = http_client
// 			.create_game_namespace_token_public()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /games/{}/namespaces/{}/tokens/development
// 	{
// 		tracing::info!("creating development namespace token");

// 		let _res = http_client
// 			.create_game_namespace_token_development()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.hostname("127.0.0.1")
// 			.lobby_ports(
// 				model::lobby_group_runtime_docker_port::Builder::default()
// 					.label("test")
// 					.target_port(80)
// 					.proxy_protocol(model::ProxyProtocol::Https)
// 					.build(),
// 			)
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /games/{}/tokens/cloud
// 	let cloud_token_auth_token = {
// 		tracing::info!("creating cloud token");

// 		let res = http_client
// 			.create_cloud_token()
// 			.game_id(&game_id)
// 			.send()
// 			.await
// 			.unwrap();

// 		res.token().unwrap().to_string()
// 	};

// 	// MARK: GET /games/{} (with cloud token)
// 	{
// 		tracing::info!("getting full game with cloud token");

// 		let http_client = ctx.http_client(cloud_token_auth_token);

// 		let _res = http_client
// 			.get_game_by_id()
// 			.game_id(&game_id)
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /games/{}/version/validate
// 	{
// 		tracing::info!("validating game version");

// 		let res = http_client
// 			.validate_game_version()
// 			.game_id(&game_id)
// 			.display_name("   bad name")
// 			.config(
// 				model::cloud_version_config::Builder::default()
// 					.cdn(
// 						model::cdn_version_config::Builder::default()
// 							.set_site_id(None)
// 							.build(),
// 					)
// 					.matchmaker(
// 						model::matchmaker_version_config::Builder::default()
// 							.set_lobby_groups(Some(vec![
// 								model::lobby_group::Builder::default()
// 									.name_id("name".to_owned())
// 									.set_regions(Some(Vec::new()))
// 									.max_players_normal(33)
// 									.max_players_direct(0)
// 									.max_players_party(16)
// 									.runtime(model::LobbyGroupRuntime::Docker(
// 										model::lobby_group_runtime_docker::Builder::default()
// 											.set_build_id(None)
// 											.set_args(Some(Vec::new()))
// 											.set_ports(Some(vec![
// 												model::lobby_group_runtime_docker_port::Builder::default()
// 										.label("http")
// 										.target_port(80)
// 										.proxy_protocol(model::ProxyProtocol::Http)
// 										.build(),
// 										model::lobby_group_runtime_docker_port::Builder::default()
// 										.label("https")
// 										.target_port(80)
// 										.proxy_protocol(model::ProxyProtocol::Https)
// 										.build(),
// 											]))
// 											.set_env_vars(Some(vec![
// 												model::lobby_group_runtime_docker_env_var::Builder::default()
// 										.key("key")
// 										.value("value")
// 										.build(),
// 										model::lobby_group_runtime_docker_env_var::Builder::default()
// 										.key("key")
// 										.value("value")
// 										.build(),
// 											]))
// 											.build(),
// 									))
// 									.build(),
// 								model::lobby_group::Builder::default()
// 									.name_id("name".to_owned())
// 									.regions(
// 										model::lobby_group_region::Builder::default()
// 											.region_id(Uuid::new_v4().to_string())
// 											.tier_name_id(util_mm::test::TIER_NAME_ID)
// 											.build(),
// 									)
// 									.max_players_normal(33)
// 									.max_players_direct(0)
// 									.max_players_party(16)
// 									.runtime(model::LobbyGroupRuntime::Docker(
// 										model::lobby_group_runtime_docker::Builder::default()
// 											.set_build_id(None)
// 											.set_args(Some(Vec::new()))
// 											.set_ports(Some(Vec::new()))
// 											.set_env_vars(Some(Vec::new()))
// 											.build(),
// 									))
// 									.build(),
// 							]))
// 							.set_captcha(None)
// 							.build(),
// 					)
// 					.build(),
// 			)
// 			.send()
// 			.await
// 			.unwrap();

// 		assert_eq!(res.errors().unwrap().len(), 11, "validation failed");
// 	}

// 	// MARK: POST /games/{}/namespace/validate
// 	{
// 		tracing::info!("validating game namespace");

// 		let res = http_client
// 			.validate_game_namespace()
// 			.game_id(&game_id)
// 			.name_id(" bad-name-id")
// 			.display_name(util::faker::display_name())
// 			.send()
// 			.await
// 			.unwrap();

// 		assert_eq!(res.errors().unwrap().len(), 1, "validation failed");
// 	}

// 	// MARK: POST /games/validate
// 	{
// 		tracing::info!("validating game");

// 		let res = http_client
// 			.validate_game()
// 			.name_id(" bad-name-id")
// 			.display_name(util::faker::display_name())
// 			.send()
// 			.await
// 			.unwrap();

// 		assert_eq!(res.errors().unwrap().len(), 1, "validation failed");
// 	}

// 	// MARK: POST /groups/validate
// 	{
// 		tracing::info!("validating group");

// 		let res = http_client
// 			.validate_group()
// 			.display_name("bad display   name")
// 			.send()
// 			.await
// 			.unwrap();

// 		assert_eq!(res.errors().unwrap().len(), 1, "validation failed");
// 	}

// 	// MARK: POST /games/{}/namespaces/{}/domains
// 	{
// 		tracing::info!("adding domain");

// 		let _res = http_client
// 			.add_namespace_domain()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.domain("example.com")
// 			.send()
// 			.await
// 			.unwrap();

// 		let ns_res = op!([ctx] cloud_namespace_get {
// 			namespace_ids: vec![Uuid::from_str(namespace_id.as_str()).unwrap().into()],
// 		})
// 		.await
// 		.unwrap();

// 		assert!(
// 			ns_res
// 				.namespaces
// 				.first()
// 				.unwrap()
// 				.config
// 				.clone()
// 				.unwrap()
// 				.cdn
// 				.unwrap()
// 				.domains
// 				.iter()
// 				.any(|domain| domain.domain == "example.com"),
// 			"domain was not added"
// 		);
// 	}

// 	// MARK: DELETE /games/{}/namespaces/{}/domains/{}
// 	{
// 		tracing::info!("removing domain");

// 		let _res = http_client
// 			.remove_namespace_domain()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.domain("example.com")
// 			.send()
// 			.await
// 			.unwrap();

// 		let ns_res = op!([ctx] cloud_namespace_get {
// 			namespace_ids: vec![Uuid::from_str(namespace_id.as_str()).unwrap().into()],
// 		})
// 		.await
// 		.unwrap();

// 		assert!(
// 			!ns_res
// 				.namespaces
// 				.first()
// 				.unwrap()
// 				.config
// 				.clone()
// 				.unwrap()
// 				.cdn
// 				.unwrap()
// 				.domains
// 				.iter()
// 				.any(|domain| domain.domain == "example.com"),
// 			"domain was not removed"
// 		);
// 	}

// 	// MARK: POST /games/{}/namespaces/{}/auth-user
// 	{
// 		tracing::info!("adding auth user");

// 		let _res = http_client
// 			.update_namespace_cdn_auth_user()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.user("test-user")
// 			.password(util::faker::bcrypt().1)
// 			.send()
// 			.await
// 			.unwrap();

// 		let ns_res = op!([ctx] cloud_namespace_get {
// 			namespace_ids: vec![Uuid::from_str(namespace_id.as_str()).unwrap().into()],
// 		})
// 		.await
// 		.unwrap();

// 		assert!(
// 			ns_res
// 				.namespaces
// 				.first()
// 				.unwrap()
// 				.config
// 				.clone()
// 				.unwrap()
// 				.cdn
// 				.unwrap()
// 				.auth_user_list
// 				.iter()
// 				.any(|user| user.user == "test-user"),
// 			"auth user was not added"
// 		);
// 	}

// 	// MARK: DELETE /games/{}/namespaces/{}/auth-user/{}
// 	{
// 		tracing::info!("removing auth user");

// 		let _res = http_client
// 			.remove_namespace_cdn_auth_user()
// 			.game_id(&game_id)
// 			.namespace_id(&namespace_id)
// 			.user("test-user")
// 			.send()
// 			.await
// 			.unwrap();

// 		let ns_res = op!([ctx] cloud_namespace_get {
// 			namespace_ids: vec![Uuid::from_str(namespace_id.as_str()).unwrap().into()],
// 		})
// 		.await
// 		.unwrap();

// 		assert!(
// 			!ns_res
// 				.namespaces
// 				.first()
// 				.unwrap()
// 				.config
// 				.clone()
// 				.unwrap()
// 				.cdn
// 				.unwrap()
// 				.auth_user_list
// 				.iter()
// 				.any(|user| user.user == "test-user"),
// 			"auth user was not removed"
// 		);
// 	}

// 	// TODO: Test forbidden
// 	// TODO: Test game cloud token
// }

// fn build_test_version_config(
// 	region_id: String,
// 	site_id: String,
// 	build_id: String,
// ) -> model::CloudVersionConfig {
// 	model::cloud_version_config::Builder::default()
// 		.cdn(
// 			model::cdn_version_config::Builder::default()
// 				.site_id(site_id)
// 				.build(),
// 		)
// 		.matchmaker(
// 			model::matchmaker_version_config::Builder::default()
// 				.set_lobby_groups(Some(vec![
// 					model::lobby_group::Builder::default()
// 						.name_id("test-1")
// 						.regions(
// 							model::lobby_group_region::Builder::default()
// 								.region_id(&region_id)
// 								.tier_name_id(util_mm::test::TIER_NAME_ID)
// 								.build(),
// 						)
// 						.max_players_normal(8)
// 						.max_players_direct(10)
// 						.max_players_party(12)
// 						.runtime(model::LobbyGroupRuntime::Docker(
// 							model::lobby_group_runtime_docker::Builder::default()
// 								.build_id(&build_id)
// 								.set_args(Some(Vec::new()))
// 								.env_vars(
// 									model::lobby_group_runtime_docker_env_var::Builder::default()
// 										.key("HELLO")
// 										.value("world")
// 										.build(),
// 								)
// 								.network_mode(model::NetworkMode::Bridge)
// 								.ports(
// 									model::lobby_group_runtime_docker_port::Builder::default()
// 										.label("http")
// 										.target_port(80)
// 										.proxy_protocol(model::ProxyProtocol::Http)
// 										.build(),
// 								)
// 								.build(),
// 						))
// 						.build(),
// 					model::lobby_group::Builder::default()
// 						.name_id("test-2")
// 						.regions(
// 							model::lobby_group_region::Builder::default()
// 								.region_id(&region_id)
// 								.tier_name_id(util_mm::test::TIER_NAME_ID)
// 								.build(),
// 						)
// 						.max_players_normal(8)
// 						.max_players_direct(10)
// 						.max_players_party(12)
// 						.runtime(model::LobbyGroupRuntime::Docker(
// 							model::lobby_group_runtime_docker::Builder::default()
// 								.build_id(&build_id)
// 								.set_args(Some(Vec::new()))
// 								.env_vars(
// 									model::lobby_group_runtime_docker_env_var::Builder::default()
// 										.key("HELLO")
// 										.value("world")
// 										.build(),
// 								)
// 								.network_mode(model::NetworkMode::Host)
// 								.ports(
// 									model::lobby_group_runtime_docker_port::Builder::default()
// 										.label("http")
// 										.target_port(80)
// 										.proxy_protocol(model::ProxyProtocol::Http)
// 										.build(),
// 								)
// 								.ports(
// 									model::lobby_group_runtime_docker_port::Builder::default()
// 										.label("udp-range")
// 										.port_range(
// 											model::port_range::Builder::default()
// 												.min(26000)
// 												.max(27000)
// 												.build(),
// 										)
// 										.proxy_protocol(model::ProxyProtocol::Udp)
// 										.build(),
// 								)
// 								.ports(
// 									model::lobby_group_runtime_docker_port::Builder::default()
// 										.label("udp-single")
// 										.port_range(
// 											model::port_range::Builder::default()
// 												.min(28000)
// 												.max(28000)
// 												.build(),
// 										)
// 										.proxy_protocol(model::ProxyProtocol::Udp)
// 										.build(),
// 								)
// 								.build(),
// 						))
// 						.build(),
// 				]))
// 				.build(),
// 		)
// 		.build()
// }

// async fn fetch_test_image() -> (String, bytes::Bytes) {
// 	todo!()
// }
