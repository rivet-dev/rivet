use std::{net::SocketAddr, sync::Once};

use proto::backend;
use rivet_operation::prelude::*;

static GLOBAL_INIT: Once = Once::new();

struct Ctx {
	op_ctx: OperationContext<()>,
	http_client: rivet_portal::ClientWrapper,
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

		let pools = rivet_pools::from_env().await.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-portal-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-portal-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-portal-test".to_string(),
			std::time::Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			util::timestamp::now(),
			util::timestamp::now(),
			(),
		);

		let (_user_id, user_token) = Self::issue_user_token(&op_ctx).await;

		let http_client = rivet_portal::Config::builder()
			.set_uri("http://traefik.traefik.svc.cluster.local:80/portal")
			.set_bearer_token(user_token)
			.build_client();

		Ctx {
			op_ctx,
			http_client,
		}
	}

	async fn issue_user_token(ctx: &OperationContext<()>) -> (Uuid, String) {
		let user_res = op!([ctx] faker_user {}).await.unwrap();
		let user_id = user_res.user_id.as_ref().unwrap().as_uuid();

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
async fn generic() {
	let _ctx = Ctx::init().await;

	// TODO: Write tests
}
