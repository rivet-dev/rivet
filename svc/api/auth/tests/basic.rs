use std::{net::SocketAddr, str::FromStr, sync::Once};

use proto::backend;
use rivet_auth::model;
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;

use ::api_auth::route;

static GLOBAL_INIT: Once = Once::new();

struct Ctx {
	op_ctx: OperationContext<()>,
	http_client: rivet_auth::ClientWrapper,
	user_id: Uuid,
}

impl Ctx {
	async fn init() -> Ctx {
		GLOBAL_INIT.call_once(|| {
			tracing_subscriber::fmt()
				.pretty()
				.with_max_level(tracing::Level::INFO)
				.with_target(false)
				.init();
		});

		let pools = rivet_pools::from_env("api-auth-test").await.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-auth-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-auth-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-auth-test".to_string(),
			std::time::Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			util::timestamp::now(),
			util::timestamp::now(),
			(),
			Vec::new(),
		);

		let (user_id, user_token) = Self::issue_user_token(&op_ctx).await;

		let http_client = rivet_auth::Config::builder()
			.set_uri(util::env::svc_router_url("api-auth"))
			.set_bearer_token(user_token)
			.build_client();

		Ctx {
			op_ctx,
			http_client,
			user_id,
		}
	}

	async fn issue_user_token(ctx: &OperationContext<()>) -> (Uuid, String) {
		let user_res = op!([ctx] faker_user {}).await.unwrap();
		let user_id = user_res.user_id.unwrap().as_uuid();

		let token_res = op!([ctx] user_token_create {
			user_id: user_res.user_id,
			client: Some(backend::net::ClientInfo {
				user_agent: Some(USER_AGENT.into()),
				remote_address: Some(socket_addr().to_string()),
			})
		})
		.await
		.unwrap();

		(user_id, token_res.token)
	}

	fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
	}
}

const USER_AGENT: &str = "test";

fn socket_addr() -> SocketAddr {
	"1.2.3.4:5678".parse().unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn register_then_refresh() {
	let ctx = Ctx::init().await;

	// MARK: POST /tokens/identity (create new token)
	let (identity_id, refresh_cookie) = {
		tracing::info!("register auth");

		let res = ctx
			.http_client
			.refresh_identity_token()
			.logout(false)
			.send()
			.await
			.unwrap();

		let refresh_cookie = res
			.set_cookie()
			.unwrap()
			.iter()
			.find_map(|cookie| {
				let cookie_auth = cookie.split(';').next().unwrap();
				cookie_auth.strip_prefix(&format!("{}=", route::tokens::USER_REFRESH_TOKEN_COOKIE))
			})
			.expect("no matching refresh token cookie")
			.to_owned();
		let refresh_claims = rivet_claims::decode(&refresh_cookie).unwrap().unwrap();
		assert_eq!(
			refresh_claims.iat + route::tokens::REFRESH_TOKEN_TTL,
			refresh_claims.exp.unwrap_or_default(),
			"bad expiration"
		);

		let claims = rivet_claims::decode(res.token().unwrap()).unwrap().unwrap();
		assert_eq!(
			claims.iat + route::tokens::TOKEN_TTL,
			claims.exp.unwrap_or_default(),
			"bad expiration"
		);
		let user = claims.as_user().unwrap();
		assert_eq!(
			res.identity_id().unwrap(),
			user.user_id.to_string().as_str(),
			"claims does not match returned user id"
		);

		(user.user_id, refresh_cookie)
	};

	// MARK: POST /tokens/identity (with refresh)
	{
		tracing::info!("refresh auth");

		let res = ctx
			.http_client
			.refresh_identity_token()
			.cookie(&format!(
				"{}={}",
				route::tokens::USER_REFRESH_TOKEN_COOKIE,
				refresh_cookie
			))
			.logout(false)
			.send()
			.await
			.unwrap();

		let claims = rivet_claims::decode(res.token().unwrap()).unwrap().unwrap();
		let user = claims.as_user().unwrap();
		assert_eq!(
			identity_id, user.user_id,
			"did not refresh original user id"
		);
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn register_then_logout() {
	let ctx = Ctx::init().await;

	// MARK: POST /tokens/identity (create new token)
	let (identity_id, refresh_cookie) = {
		tracing::info!("register auth");

		let res = ctx
			.http_client
			.refresh_identity_token()
			.logout(false)
			.send()
			.await
			.unwrap();

		let refresh_cookie = res
			.set_cookie()
			.unwrap()
			.iter()
			.find_map(|cookie| {
				let cookie_auth = cookie.split(';').next().unwrap();
				cookie_auth.strip_prefix(&format!("{}=", route::tokens::USER_REFRESH_TOKEN_COOKIE))
			})
			.expect("no matching refresh token cookie")
			.to_owned();
		let refresh_claims = rivet_claims::decode(&refresh_cookie).unwrap().unwrap();
		assert_eq!(
			refresh_claims.iat + route::tokens::REFRESH_TOKEN_TTL,
			refresh_claims.exp.unwrap_or_default(),
			"bad expiration"
		);

		let claims = rivet_claims::decode(res.token().unwrap()).unwrap().unwrap();
		assert_eq!(
			claims.iat + route::tokens::TOKEN_TTL,
			claims.exp.unwrap_or_default(),
			"bad expiration"
		);
		let user = claims.as_user().unwrap();
		assert_eq!(
			res.identity_id().unwrap(),
			user.user_id.to_string().as_str(),
			"claims does not match returned user id"
		);

		(user.user_id, refresh_cookie)
	};

	// MARK: POST /tokens/identity (with refresh and logout)
	{
		tracing::info!("logout auth");

		let res = ctx
			.http_client
			.refresh_identity_token()
			.cookie(&format!(
				"{}={}",
				route::tokens::USER_REFRESH_TOKEN_COOKIE,
				refresh_cookie
			))
			.logout(true)
			.send()
			.await
			.unwrap();

		let claims = rivet_claims::decode(res.token().unwrap()).unwrap().unwrap();
		let user = claims.as_user().unwrap();

		assert_ne!(
			identity_id, user.user_id,
			"logout did not return a new guest user token"
		);
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn register() {
	let ctx = Ctx::init().await;
	let email = util::faker::email();

	// MARK: /identity/email/start-verification
	let verification_id = {
		tracing::info!("start email identity verification");

		let res = ctx
			.http_client
			.start_email_verification()
			.email(email)
			.send()
			.await
			.unwrap();

		res.verification_id().unwrap().to_string()
	};

	let res = op!([ctx] debug_email_res {
		verification_id: Some(Uuid::from_str(verification_id.as_str()).unwrap().into())
	})
	.await
	.unwrap();

	// MARK: /identity/email/complete-verification
	{
		let complete_res = ctx
			.http_client
			.complete_email_verification()
			.verification_id(verification_id)
			.code(&res.code)
			.send()
			.await
			.unwrap();

		// Verify that identity has been created
		match complete_res.status().unwrap() {
			model::CompleteStatus::LinkedAccountAdded => {
				let identity_res = op!([ctx] user_identity_get {
					user_ids: vec![ctx.user_id.into()]
				})
				.await
				.unwrap();

				assert_eq!(
					1,
					identity_res.users[0].identities.len(),
					"email identity not found"
				);
			}
			// User email already registered
			model::CompleteStatus::SwitchIdentity => {
				panic!("user should not be switching");
			}
			_ => (),
		}
	}
}
