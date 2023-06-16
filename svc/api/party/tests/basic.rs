use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use rivet_party::model;
use std::sync::Once;

const LOBBY_GROUP_NAME_ID: &str = "test";

static GLOBAL_INIT: Once = Once::new();

struct Ctx {
	op_ctx: OperationContext<()>,
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

		let pools = rivet_pools::from_env("api-party-test").await.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-party-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-party-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-party-test".to_string(),
			std::time::Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			util::timestamp::now(),
			util::timestamp::now(),
			(),
			Vec::new(),
		);

		Ctx { op_ctx }
	}

	fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
	}
}

struct UserCtx {
	user_id: Uuid,
	http_client: rivet_party::ClientWrapper,
}

impl UserCtx {
	async fn init_with_token(user_id: Uuid, token: String) -> UserCtx {
		let http_client = rivet_party::Config::builder()
			.set_uri(util::env::svc_router_url("api-party"))
			.set_bearer_token(token)
			.build_client();

		UserCtx {
			user_id,
			http_client,
		}
	}

	async fn init_user(ctx: &Ctx) -> UserCtx {
		let (user_id, user_token) = Self::issue_user(&ctx.op_ctx).await;
		Self::init_with_token(user_id, user_token).await
	}

	async fn init_game_user(ctx: &Ctx, namespace_id: Uuid) -> UserCtx {
		let (user_id, game_user_token) = Self::issue_game_user(&ctx.op_ctx, namespace_id).await;
		Self::init_with_token(user_id, game_user_token).await
	}

	async fn issue_user(ctx: &OperationContext<()>) -> (Uuid, String) {
		let user = op!([ctx] faker_user {
		})
		.await
		.unwrap();
		let user_id = user.user_id.as_ref().unwrap().as_uuid();

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
							proto::claims::entitlement::Kind::User(proto::claims::entitlement::User {
								user_id: Some(user_id.into()),
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

		(user_id, token_res.token.as_ref().unwrap().token.clone())
	}

	async fn issue_game_user(ctx: &OperationContext<()>, namespace_id: Uuid) -> (Uuid, String) {
		let user = op!([ctx] faker_user {
		})
		.await
		.unwrap();
		let user_id = user.user_id.as_ref().unwrap().as_uuid();

		let game_user = op!([ctx] game_user_create {
			namespace_id: Some(namespace_id.into()),
			user_id: Some(user_id.into()),
		})
		.await
		.unwrap();

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
							proto::claims::entitlement::Kind::GameUser(proto::claims::entitlement::GameUser {
								game_user_id: game_user.game_user_id,
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

		(user_id, token_res.token.as_ref().unwrap().token.clone())
	}
}

#[allow(unused)]
struct TestGame {
	region_id: Uuid,
	region_name_id: String,
	game_id: Uuid,
	version_id: Uuid,
	namespace_id: Uuid,
	mm_config: backend::matchmaker::VersionConfig,
	mm_config_meta: backend::matchmaker::VersionConfigMeta,
}

impl TestGame {
	async fn init(ctx: &OperationContext<()>) -> Self {
		let region_res = op!([ctx] faker_region {}).await.unwrap();
		let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

		let get_res = op!([ctx] region_get {
			region_ids: vec![region_id.into()],
		})
		.await
		.unwrap();
		let region_data = get_res.regions.first().unwrap();

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

		let namespace_res = op!([ctx] faker_game_namespace {
			game_id: game_res.game_id,
			version_id: game_version_res.version_id,
			..Default::default()
		})
		.await
		.unwrap();

		TestGame {
			region_id,
			region_name_id: region_data.name_id.clone(),
			game_id: game_res.game_id.as_ref().unwrap().as_uuid(),
			version_id: game_version_res.version_id.as_ref().unwrap().as_uuid(),
			namespace_id: namespace_res.namespace_id.as_ref().unwrap().as_uuid(),
			mm_config: game_version_res.mm_config.clone().unwrap(),
			mm_config_meta: game_version_res.mm_config_meta.clone().unwrap(),
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn get_party() {
	let ctx = Ctx::init().await;
	let user_a_ctx = UserCtx::init_user(&ctx).await;
	let _user_b_ctx = UserCtx::init_user(&ctx).await;

	// Create party
	let create_res = user_a_ctx
		.http_client
		.create_party()
		.party_size(4)
		.send()
		.await
		.unwrap();
	let party_id = create_res.party_id().unwrap();

	// Get party
	let _get_res = user_a_ctx
		.http_client
		.get_party_summary()
		.party_id(party_id)
		.send()
		.await
		.unwrap();

	// Get party self
	let _get_res = user_a_ctx
		.http_client
		.get_party_self_summary()
		.send()
		.await
		.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn leave_party() {
	let ctx = Ctx::init().await;
	let user_a_ctx = UserCtx::init_user(&ctx).await;

	// Create party
	let _create_res = user_a_ctx
		.http_client
		.create_party()
		.party_size(4)
		.send()
		.await
		.unwrap();

	// Leave party
	let _ = user_a_ctx.http_client.leave_party().send().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn transfer_party_ownership() {
	let ctx = Ctx::init().await;
	let user_a_ctx = UserCtx::init_user(&ctx).await;
	let user_b_ctx = UserCtx::init_user(&ctx).await;

	// Create party
	let create_res = user_a_ctx
		.http_client
		.create_party()
		.party_size(4)
		.send()
		.await
		.unwrap();
	let _party_id = create_res.party_id().unwrap();

	// Create invite
	let create_invite_res = user_a_ctx
		.http_client
		.create_party_invite()
		.send()
		.await
		.unwrap();
	let token = create_invite_res
		.invite()
		.unwrap()
		.token()
		.unwrap()
		.to_string();
	let decoded_token = rivet_claims::decode(&token).unwrap().unwrap();
	tracing::info!(?token, ?decoded_token, "created invite");

	// Create new party member
	let _ = user_b_ctx
		.http_client
		.join_party()
		.invite(model::JoinPartyInvite::Token(token))
		.send()
		.await
		.unwrap();

	// Transfer party
	let _ = user_a_ctx
		.http_client
		.transfer_party_ownership()
		.identity_id(user_b_ctx.user_id.to_string())
		.send()
		.await
		.unwrap();

	// Leave party
	let _ = user_a_ctx.http_client.leave_party().send().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn kick_member() {
	let ctx = Ctx::init().await;
	let user_a_ctx = UserCtx::init_user(&ctx).await;
	let user_b_ctx = UserCtx::init_user(&ctx).await;

	// Create party
	let create_res = user_a_ctx
		.http_client
		.create_party()
		.party_size(4)
		.send()
		.await
		.unwrap();
	let _party_id = create_res.party_id().unwrap();

	// Create invite
	let create_invite_res = user_a_ctx
		.http_client
		.create_party_invite()
		.send()
		.await
		.unwrap();
	let token = create_invite_res
		.invite()
		.unwrap()
		.token()
		.unwrap()
		.to_string();
	let decoded_token = rivet_claims::decode(&token).unwrap().unwrap();
	tracing::info!(?token, ?decoded_token, "created invite");

	// Create new party member
	let _ = user_b_ctx
		.http_client
		.join_party()
		.invite(model::JoinPartyInvite::Token(token))
		.send()
		.await
		.unwrap();

	// Transfer party
	let _ = user_a_ctx
		.http_client
		.kick_member()
		.identity_id(user_b_ctx.user_id.to_string())
		.send()
		.await
		.unwrap();

	// Leave party
	let _ = user_a_ctx.http_client.leave_party().send().await.unwrap();
}

// TODO: Use chat API instead
// #[tokio::test(flavor = "multi_thread")]
// async fn invite_identity() {
// 	let ctx = Ctx::init().await;
// 	let user_a_ctx = UserCtx::init_user(&ctx).await;
// 	let user_b_ctx = UserCtx::init_user(&ctx).await;

// 	// Create party
// 	let create_res = user_a_ctx
// 		.http_client
// 		.create_party()
// 		.party_size(4)
// 		.send()
// 		.await
// 		.unwrap();
// 	let party_id = create_res.party_id().unwrap();

// 	// Create invite
// 	let create_invite_res = user_a_ctx
// 		.http_client
// 		.create_party_invite()
// 		.send()
// 		.await
// 		.unwrap();
// 	let token = create_invite_res.invite().unwrap().token().unwrap().to_string();
// 	let decoded_token = rivet_claims::decode(&token).unwrap().unwrap();
// 	tracing::info!(?token, ?decoded_token, "created invite");

// 	// Invite identity
// 	let _ = user_a_ctx
// 		.http_client
// 		.send_invite_chat_message()
// 		.identity_id(user_b_ctx.user_id.to_string())
// 		.token(token)
// 		.send()
// 		.await
// 		.unwrap();
// }

#[tokio::test(flavor = "multi_thread")]
async fn revoke_invite() {
	let ctx = Ctx::init().await;
	let game = TestGame::init(&ctx.op_ctx).await;
	let user_a_ctx = UserCtx::init_game_user(&ctx, game.namespace_id).await;

	// Create party
	let party_alias_a = Uuid::new_v4().to_string();
	let _party_alias_b = Uuid::new_v4().to_string();
	let create_res = user_a_ctx
		.http_client
		.create_party()
		.party_size(4)
		.invites(
			model::CreatePartyInviteConfig::builder()
				.alias(&party_alias_a)
				.build(),
		)
		.invites(model::CreatePartyInviteConfig::builder().build())
		.send()
		.await
		.unwrap();
	let party_id = create_res.party_id().unwrap();

	// Get party
	let get_res = user_a_ctx
		.http_client
		.get_party_profile()
		.party_id(party_id)
		.send()
		.await
		.unwrap();

	user_a_ctx
		.http_client
		.revoke_party_invite()
		.invite_id(
			get_res
				.party
				.unwrap()
				.invites
				.unwrap()
				.first()
				.unwrap()
				.invite_id()
				.unwrap(),
		)
		.send()
		.await
		.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn game_user() {
	let ctx = Ctx::init().await;
	let game = TestGame::init(&ctx.op_ctx).await;
	let user_a_ctx = UserCtx::init_game_user(&ctx, game.namespace_id).await;
	let user_b_ctx = UserCtx::init_game_user(&ctx, game.namespace_id).await;

	let party_alias = Uuid::new_v4().to_string();

	// Create party
	let create_res = user_a_ctx
		.http_client
		.create_party()
		.party_size(4)
		.send()
		.await
		.unwrap();
	let _party_id = create_res.party_id().unwrap();

	// Create invite
	let _ = user_a_ctx
		.http_client
		.create_party_invite()
		.alias(&party_alias)
		.send()
		.await
		.unwrap();

	// Create new party member
	let _ = user_b_ctx
		.http_client
		.join_party()
		.invite(model::JoinPartyInvite::Alias(party_alias.clone()))
		.send()
		.await
		.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn create_party_with_invite() {
	let ctx = Ctx::init().await;
	let game = TestGame::init(&ctx.op_ctx).await;
	let user_a_ctx = UserCtx::init_game_user(&ctx, game.namespace_id).await;
	let _user_b_ctx = UserCtx::init_game_user(&ctx, game.namespace_id).await;

	// Create party
	let party_alias_a = Uuid::new_v4().to_string();
	let _party_alias_b = Uuid::new_v4().to_string();
	let create_res = user_a_ctx
		.http_client
		.create_party()
		.party_size(4)
		.invites(
			model::CreatePartyInviteConfig::builder()
				.alias(&party_alias_a)
				.build(),
		)
		.invites(model::CreatePartyInviteConfig::builder().build())
		.send()
		.await
		.unwrap();
	let party_id = create_res.party_id().unwrap();

	// Get party
	let get_res = user_a_ctx
		.http_client
		.get_party_profile()
		.party_id(party_id)
		.send()
		.await
		.unwrap();
	assert_eq!(2, get_res.party.unwrap().invites.unwrap().len());
}

#[tokio::test(flavor = "multi_thread")]
async fn matchmaker_join() {
	let ctx = Ctx::init().await;
	let game = TestGame::init(&ctx.op_ctx).await;
	let user_a_ctx = UserCtx::init_game_user(&ctx, game.namespace_id).await;
	let user_b_ctx = UserCtx::init_game_user(&ctx, game.namespace_id).await;

	let party_alias = Uuid::new_v4().to_string();

	// Create party
	let create_res = user_a_ctx
		.http_client
		.create_party()
		.party_size(4)
		.send()
		.await
		.unwrap();
	let _party_id = create_res.party_id().unwrap();

	// Create invite
	let _ = user_a_ctx
		.http_client
		.create_party_invite()
		.alias(&party_alias)
		.send()
		.await
		.unwrap();

	// Create new party member
	let _ = user_b_ctx
		.http_client
		.join_party()
		.invite(model::JoinPartyInvite::Alias(party_alias.clone()))
		.send()
		.await
		.unwrap();

	let lobby_res = op!([ctx] faker_mm_lobby {
		namespace_id: Some(game.namespace_id.into()),
		version_id: Some(game.version_id.into()),
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	// Join lobby
	let _ = user_a_ctx
		.http_client
		.join_matchmaker_lobby_for_party()
		.lobby_id(lobby_id.to_string())
		.send()
		.await
		.unwrap();

	// Set idle
	let _ = user_a_ctx
		.http_client
		.set_party_to_idle()
		.send()
		.await
		.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn matchmaker_find() {
	let ctx = Ctx::init().await;
	let game = TestGame::init(&ctx.op_ctx).await;
	let user_a_ctx = UserCtx::init_game_user(&ctx, game.namespace_id).await;
	let user_b_ctx = UserCtx::init_game_user(&ctx, game.namespace_id).await;

	let party_alias = Uuid::new_v4().to_string();

	// Create party
	let create_res = user_a_ctx
		.http_client
		.create_party()
		.party_size(4)
		.send()
		.await
		.unwrap();
	let _party_id = create_res.party_id().unwrap();

	// Create invite
	let _ = user_a_ctx
		.http_client
		.create_party_invite()
		.alias(&party_alias)
		.send()
		.await
		.unwrap();

	// Create new party member
	let _ = user_b_ctx
		.http_client
		.join_party()
		.invite(model::JoinPartyInvite::Alias(party_alias.clone()))
		.send()
		.await
		.unwrap();

	// Find lobby
	let _ = user_a_ctx
		.http_client
		.find_matchmaker_lobby_for_party()
		.game_modes(LOBBY_GROUP_NAME_ID)
		.send()
		.await
		.unwrap();

	// Set idle
	let _ = user_a_ctx
		.http_client
		.set_party_to_idle()
		.send()
		.await
		.unwrap();
}
