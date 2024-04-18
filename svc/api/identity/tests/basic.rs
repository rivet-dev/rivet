// TODO: Rewrite with new api clients

// use std::{str::FromStr, sync::Once};

// use proto::backend::{self, pkg::*};
// use regex::Regex;

// use rivet_claims::ClaimsDecode;
// use rivet_identity::{model, output};
// use rivet_operation::prelude::*;

// const LOBBY_GROUP_NAME_ID: &str = "test";

// static GLOBAL_INIT: Once = Once::new();

// struct Ctx {
// 	op_ctx: OperationContext<()>,
// 	ns_dev_auth_token: String,
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

// 		let pools = rivet_pools::from_env("api-identity-test").await.unwrap();
// 		let cache = rivet_cache::CacheInner::new(
// 			"api-identity-test".to_string(),
// 			std::env::var("RIVET_SOURCE_HASH").unwrap(),
// 			pools.redis_cache().unwrap(),
// 		);
// 		let client = chirp_client::SharedClient::from_env(pools.clone())
// 			.expect("create client")
// 			.wrap_new("api-identity-test");
// 		let conn = rivet_connection::Connection::new(client, pools, cache);
// 		let op_ctx = OperationContext::new(
// 			"api-identity-test".to_string(),
// 			std::time::Duration::from_secs(60),
// 			conn,
// 			Uuid::new_v4(),
// 			Uuid::new_v4(),
// 			util::timestamp::now(),
// 			util::timestamp::now(),
// 			(),
// 			Vec::new(),
// 		);

// 		let (primary_region_id, _) = Self::setup_region(&op_ctx).await;
// 		let (_, _, namespace_id, _, _) = Self::setup_game(&op_ctx, primary_region_id).await;
// 		let ns_dev_auth_token =
// 			Self::setup_dev_token(&op_ctx, namespace_id, "127.0.0.1".to_owned(), Vec::new()).await;

// 		Ctx {
// 			op_ctx,
// 			ns_dev_auth_token,
// 		}
// 	}

// 	fn http_client(&self, bearer_token: String) -> rivet_identity::ClientWrapper {
// 		rivet_identity::Config::builder()
// 			.set_uri("http://traefik.traefik.svc.cluster.local:80/identity")
// 			.set_bearer_token(bearer_token)
// 			.build_client()
// 	}

// 	async fn setup_region(ctx: &OperationContext<()>) -> (Uuid, String) {
// 		tracing::info!("setup region");

// 		let region_res = op!([ctx] faker_region {}).await.unwrap();
// 		let region_id = region_res.region_id.unwrap().as_uuid();

// 		let get_res = op!([ctx] region_get {
// 			region_ids: vec![region_id.into()],
// 		})
// 		.await
// 		.unwrap();
// 		let region_data = get_res.regions.first().unwrap();

// 		(region_id, region_data.name_id.clone())
// 	}

// 	async fn setup_game(
// 		ctx: &OperationContext<()>,
// 		region_id: Uuid,
// 	) -> (
// 		Uuid,
// 		Uuid,
// 		Uuid,
// 		backend::matchmaker::VersionConfig,
// 		backend::matchmaker::VersionConfigMeta,
// 	) {
// 		let game_res = op!([ctx] faker_game {
// 			..Default::default()
// 		})
// 		.await
// 		.unwrap();

// 		let build_res = op!([ctx] faker_build {
// 			game_id: game_res.game_id,
// 			image: backend::faker::Image::MmLobbyAutoReady as i32,
// 		})
// 		.await
// 		.unwrap();

// 		let game_version_res = op!([ctx] faker_game_version {
// 			game_id: game_res.game_id,
// 			override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
// 				lobby_groups: vec![backend::matchmaker::LobbyGroup {
// 					name_id: LOBBY_GROUP_NAME_ID.into(),

// 					regions: vec![backend::matchmaker::lobby_group::Region {
// 						region_id: Some(region_id.into()),
// 						tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
// 						idle_lobbies: None,
// 					}],
// 					max_players_normal: 8,
// 					max_players_direct: 10,
// 					max_players_party: 12,
// 					listable: true,
// 					taggable: false,
// 					allow_dynamic_max_players: false,

// 					runtime: Some(backend::matchmaker::lobby_runtime::Docker {
// 						build_id: build_res.build_id,
// 						args: Vec::new(),
// 						env_vars: Vec::new(),
// 						network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
// 						ports: vec![
// 							backend::matchmaker::lobby_runtime::Port {
// 								label: "test-80-http".into(),
// 								target_port: Some(80),
// 								port_range: None,
// 								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http as i32,
// 								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
// 							},
// 							backend::matchmaker::lobby_runtime::Port {
// 								label: "test-80-https".into(),
// 								target_port: Some(80),
// 								port_range: None,
// 								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
// 								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
// 							},
// 							backend::matchmaker::lobby_runtime::Port {
// 								label: "test-5050-https".into(),
// 								target_port: Some(5050),
// 								port_range: None,
// 								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
// 								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
// 							},
// 						],
// 					}.into()),

// 					actions: None,
// 				}],
// 			}),
// 			..Default::default()
// 		})
// 		.await
// 		.unwrap();

// 		let namespace_res = op!([ctx] faker_game_namespace {
// 			game_id: game_res.game_id,
// 			version_id: game_version_res.version_id,
// 			..Default::default()
// 		})
// 		.await
// 		.unwrap();

// 		(
// 			game_res.game_id.unwrap().as_uuid(),
// 			game_version_res.version_id.unwrap().as_uuid(),
// 			namespace_res.namespace_id.unwrap().as_uuid(),
// 			game_version_res.mm_config.clone().unwrap(),
// 			game_version_res.mm_config_meta.clone().unwrap(),
// 		)
// 	}

// 	async fn setup_dev_token(
// 		ctx: &OperationContext<()>,
// 		namespace_id: Uuid,
// 		hostname: String,
// 		lobby_ports: Vec<backend::matchmaker::lobby_runtime::Port>,
// 	) -> String {
// 		let token_res = op!([ctx] cloud_namespace_token_development_create {
// 			hostname: hostname.to_owned(),
// 			namespace_id: Some(namespace_id.into()),
// 			lobby_ports: lobby_ports,
// 		})
// 		.await
// 		.unwrap();

// 		token_res.token
// 	}

// 	fn chirp(&self) -> &chirp_client::Client {
// 		self.op_ctx.chirp()
// 	}

// 	fn op_ctx(&self) -> &OperationContext<()> {
// 		&self.op_ctx
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn game_identities() {
// 	let ctx = Ctx::init().await;

// 	// MARK: POST /identities
// 	let (_identity_id, game_identity_token) = {
// 		tracing::info!("creating game identity");

// 		let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

// 		let res = http_client.setup_identity().send().await.unwrap();

// 		(
// 			res.identity().unwrap().identity_id().unwrap().to_string(),
// 			res.identity_token().unwrap().to_string(),
// 		)
// 	};

// 	let http_client = ctx.http_client(game_identity_token);

// 	// MARK: GET /identities
// 	{
// 		tracing::info!("getting game identity");

// 		let _res = http_client
// 			.get_identity_self_profile()
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// Create fake new user
// 	let faker_user_res = op!([ctx] faker_user {}).await.unwrap();
// 	let user_id_to_link_to = faker_user_res.user_id.unwrap().as_uuid();

// 	// MARK: POST /identities/links
// 	let (identity_link_token, identity_link_url) = {
// 		tracing::info!("getting identity link");

// 		let res = http_client.prepare_game_link().send().await.unwrap();

// 		(
// 			res.identity_link_token().unwrap().to_string(),
// 			res.identity_link_url().unwrap().to_string(),
// 		)
// 	};

// 	// MARK: GET /identities/links/{}
// 	{
// 		tracing::info!("getting identity link status");

// 		let res = http_client
// 			.get_game_link()
// 			.identity_link_token(&identity_link_token)
// 			.send()
// 			.await
// 			.unwrap();

// 		assert_eq!(
// 			res.status().unwrap(),
// 			&model::GameLinkStatus::Incomplete,
// 			"invalid status"
// 		);
// 	}

// 	// Completing game link
// 	{
// 		let re = Regex::new(r"/link/(.*)$").unwrap();
// 		let re_identity_link_token = re.captures(&identity_link_url).unwrap()[1].to_owned();
// 		assert_eq!(identity_link_token, re_identity_link_token);

// 		tracing::info!(identity_link_token = ?identity_link_token, "completing game identity link");
// 		let token_claims = rivet_claims::decode(&identity_link_token).unwrap().unwrap();
// 		let game_identity_ent = token_claims.as_game_user_link().unwrap();
// 		let token_jti = token_claims.jti.unwrap();

// 		msg!([ctx] game_user::msg::link_complete(game_identity_ent.link_id) -> game_user::msg::link_complete_complete {
// 			user_id: Some(user_id_to_link_to.into()),
// 			link_id: Some(game_identity_ent.link_id.into()),
// 			user_link_jti: Some(token_jti),
// 			resolution: game_user::msg::link_complete::GameUserLinkCompleteResolution::Complete as i32,
// 		})
// 		.await
// 		.unwrap();

// 		let res = http_client
// 			.get_game_link()
// 			.identity_link_token(&identity_link_token)
// 			.send()
// 			.await
// 			.unwrap();

// 		assert_eq!(
// 			res.status().unwrap(),
// 			&model::GameLinkStatus::Complete,
// 			"linking did not complete"
// 		);

// 		let new_identity = res.new_identity().expect("missing new identity");
// 		assert_eq!(
// 			user_id_to_link_to.to_string(),
// 			new_identity.identity().unwrap().identity_id().unwrap(),
// 			"new identity does not match"
// 		);

// 		// Check the provided game user token is valid
// 		let new_identity_http_client =
// 			ctx.http_client(new_identity.identity_token().unwrap().to_string());
// 		let new_profile_self = new_identity_http_client
// 			.get_identity_self_profile()
// 			.send()
// 			.await
// 			.unwrap();
// 		assert_eq!(
// 			user_id_to_link_to.to_string(),
// 			new_profile_self.identity().unwrap().identity_id().unwrap(),
// 			"new fetched identity does not match"
// 		);

// 		// Get the user
// 		let res = new_identity_http_client
// 			.get_identity_profile()
// 			.identity_id(user_id_to_link_to.to_string())
// 			.send()
// 			.await
// 			.unwrap();
// 		assert!(
// 			res.identity().unwrap().is_game_linked().unwrap(),
// 			"linking did not update identity"
// 		);
// 	}

// 	// MARK: GET /events
// 	{
// 		tracing::info!("getting events");

// 		// TODO: Trigger a fake event so this resolves quickly

// 		let events_res_1 = http_client.watch_events().send().await.unwrap();
// 		let _events_res_2 = http_client
// 			.watch_events()
// 			.watch_index(events_res_1.watch().unwrap().index().unwrap())
// 			.send()
// 			.await
// 			.unwrap();
// 	}
// }

// // TODO: Test TOKEN_REVOKED error for token refresh will issue a fresh token
// // safely

// #[tokio::test(flavor = "multi_thread")]
// async fn identities() {
// 	let ctx = Ctx::init().await;

// 	let user_res = op!([ctx] faker_user { }).await.unwrap();
// 	let other_user_id = user_res.user_id.unwrap();

// 	// MARK: POST /identities
// 	let (identity_id, game_identity_token) = {
// 		let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

// 		// Hit identities endpoint multiple times to check refresh token
// 		let mut last_res = Option::<output::SetupIdentityOutput>::None;
// 		let mut identity_token = None;

// 		// TODO: Should this be 1?
// 		for i in 0..1 {
// 			tracing::info!(i, "creating game identity");

// 			let res = http_client
// 				.setup_identity()
// 				.set_existing_identity_token(identity_token)
// 				.send()
// 				.await
// 				.unwrap();

// 			// Check the response state
// 			if i != 0 {
// 				let last_res = last_res.as_ref().unwrap();
// 				assert_eq!(
// 					res.identity_token(),
// 					last_res.identity_token(),
// 					"unnecessary refreshed identity token"
// 				);
// 				assert_eq!(
// 					last_res
// 						.identity()
// 						.unwrap()
// 						.identity_id()
// 						.unwrap()
// 						.to_string(),
// 					res.identity().unwrap().identity_id().unwrap().to_string(),
// 					"identity id changed"
// 				);
// 			}

// 			// Check that the token is valid
// 			{
// 				tracing::info!("checking identity token is valid");

// 				let get_self_res = ctx
// 					.http_client(res.identity_token().unwrap().to_string())
// 					.get_identity_self_profile()
// 					.send()
// 					.await
// 					.unwrap();

// 				assert_eq!(
// 					res.identity().unwrap().identity_id().unwrap(),
// 					get_self_res.identity().unwrap().identity_id().unwrap(),
// 					"token represents wrong identity"
// 				);
// 			}

// 			// Save state for next round
// 			identity_token = Some(res.identity_token().unwrap().to_string());
// 			last_res = Some(res);
// 		}

// 		let res = last_res.unwrap();
// 		(
// 			res.identity().unwrap().identity_id().unwrap().to_string(),
// 			res.identity_token().unwrap().to_string(),
// 		)
// 	};

// 	let http_client = ctx.http_client(game_identity_token);

// 	// MARK: GET /identities/{}
// 	{
// 		tracing::info!("getting identity");

// 		let _res = http_client
// 			.get_identity_profile()
// 			.identity_id(&identity_id)
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /identities/self/activity
// 	{
// 		tracing::info!("setting identity game activity");

// 		let _res = http_client
// 			.set_identity_game_activity()
// 			.game_activity(
// 				model::update_identity_game_activity::Builder::default()
// 					.message("test message")
// 					.build(),
// 			)
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: DELETE /identities/self/activity
// 	{
// 		tracing::info!("removing identity game activity");

// 		let _res = http_client
// 			.remove_identity_game_activity()
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /identities/self/status
// 	{
// 		tracing::info!("setting identity status");

// 		let _res = http_client
// 			.update_identity_status()
// 			.status(model::IdentityStatus::Away)
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /identities/{}/follow
// 	{
// 		tracing::info!("friending identity");

// 		let _res = http_client
// 			.follow_identity()
// 			.identity_id(other_user_id.to_string())
// 			.send()
// 			.await
// 			.unwrap();

// 		let res = http_client
// 			.get_identity_profile()
// 			.identity_id(other_user_id.to_string())
// 			.send()
// 			.await
// 			.unwrap();

// 		tracing::info!(?other_user_id);

// 		assert!(
// 			res.identity().unwrap().following().unwrap(),
// 			"friending failed"
// 		);
// 	}

// 	// MARK: DELETE /identities/{}/follow
// 	{
// 		tracing::info!("unfriending identity");

// 		let _res = http_client
// 			.unfollow_identity()
// 			.identity_id(other_user_id.to_string())
// 			.send()
// 			.await
// 			.unwrap();

// 		let res = http_client
// 			.get_identity_profile()
// 			.identity_id(other_user_id.to_string())
// 			.send()
// 			.await
// 			.unwrap();

// 		assert!(
// 			!res.identity().unwrap().following().unwrap(),
// 			"unfriending failed"
// 		);
// 	}

// 	// MARK: GET /identities/search
// 	{
// 		tracing::info!("searching identities");

// 		let _res = http_client
// 			.search_identities()
// 			.query(util::faker::ident())
// 			.limit(10)
// 			.send()
// 			.await
// 			.unwrap();
// 	}
// }
// #[tokio::test(flavor = "multi_thread")]
// async fn activities() {
// 	let ctx = Ctx::init().await;

// 	// MARK: POST /identities
// 	let (identity_id, game_identity_token) = {
// 		tracing::info!("creating game identity");

// 		let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

// 		let res = http_client.setup_identity().send().await.unwrap();

// 		(
// 			res.identity().unwrap().identity_id().unwrap().to_string(),
// 			res.identity_token().unwrap().to_string(),
// 		)
// 	};

// 	let http_client = ctx.http_client(game_identity_token);
// 	let user_res = op!([ctx] faker_user { }).await.unwrap();
// 	let other_user_id = user_res.user_id.unwrap();

// 	op!([ctx] user_presence_touch {
// 		user_id: user_res.user_id,
// 	})
// 	.await
// 	.unwrap();

// 	op!([ctx] user_follow_toggle {
// 		follower_user_id: Some(other_user_id),
// 		following_user_id: Some(Uuid::from_str(&identity_id).unwrap().into()),
// 		active: true,
// 	})
// 	.await
// 	.unwrap();

// 	// MARK: POST /identities/{}/follow
// 	{
// 		tracing::info!("tailing activities and friending identity");

// 		let (activities_res, _) = tokio::join!(
// 			async {
// 				let activities_req = http_client
// 					.list_activities()
// 					.watch_index(util::timestamp::now().to_string())
// 					.send();

// 				let activities_res = util::macros::select_with_timeout!([3 SEC] {
// 					activities_res = activities_req => {
// 						Some(activities_res.unwrap())
// 					}
// 				});

// 				activities_res.expect("no message from activities endpoint")
// 			},
// 			async {
// 				// Wait before putting
// 				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

// 				http_client
// 					.follow_identity()
// 					.identity_id(other_user_id.to_string())
// 					.send()
// 					.await
// 					.unwrap()
// 			}
// 		);

// 		let identities = activities_res.identities().unwrap();
// 		tracing::info!(?identities);
// 		assert!(
// 			!identities.is_empty(),
// 			"followed identity not present in activities"
// 		);
// 	}

// 	// MARK: GET /activities
// 	{
// 		tracing::info!("tailing activities and unfriending identity");

// 		let (activities_res, _) = tokio::join!(
// 			async {
// 				let activities_req = http_client
// 					.list_activities()
// 					.watch_index(util::timestamp::now().to_string())
// 					.send();

// 				let activities_res = util::macros::select_with_timeout!([3 SEC] {
// 					activities_res = activities_req => {
// 						Some(activities_res.unwrap())
// 					}
// 				});

// 				activities_res.expect("no message from activities endpoint")
// 			},
// 			async {
// 				// Wait before putting
// 				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

// 				http_client
// 					.unfollow_identity()
// 					.identity_id(other_user_id.to_string())
// 					.send()
// 					.await
// 					.unwrap()
// 			}
// 		);

// 		let identities = activities_res.identities().unwrap();
// 		assert!(
// 			identities.is_empty(),
// 			"unfollowed identity not removed from activities"
// 		);
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn invalid_identity_refresh() {
// 	let ctx = Ctx::init().await;

// 	// MARK: POST /identities
// 	{
// 		tracing::info!("creating game identity");

// 		let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

// 		let _res = http_client
// 			.setup_identity()
// 			.existing_identity_token("~~invalid token~~")
// 			.send()
// 			.await
// 			.unwrap();
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn revoked_token_issues_new_token() {
// 	let ctx = Ctx::init().await;

// 	let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

// 	// MARK: POST /identities
// 	let (original_identity_id, token) = {
// 		tracing::info!("creating game identity");

// 		let res = http_client.setup_identity().send().await.unwrap();

// 		(
// 			res.identity().unwrap().identity_id().unwrap().to_string(),
// 			res.identity_token().unwrap().to_string(),
// 		)
// 	};

// 	// Revoke the identity
// 	{
// 		let claims = rivet_claims::decode(&token).unwrap().unwrap();

// 		let jti = claims.jti.unwrap();
// 		op!([ctx] token_revoke {
// 			jtis: vec![jti],
// 		})
// 		.await
// 		.unwrap();
// 	}

// 	// MARK: POST /identities
// 	{
// 		tracing::info!("creating another game identity");

// 		let res = http_client
// 			.setup_identity()
// 			.existing_identity_token(token)
// 			.send()
// 			.await
// 			.unwrap();
// 		let new_identity_id = res.identity().unwrap().identity_id().unwrap();

// 		assert_ne!(
// 			original_identity_id, new_identity_id,
// 			"new token should have been issued from revoked token"
// 		);
// 	}
// }

// // TODO: Add remaining identity endpoints
