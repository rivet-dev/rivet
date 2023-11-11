use rivet_operation::prelude::*;
use std::sync::Once;

static GLOBAL_INIT: Once = Once::new();

#[allow(unused)]
struct Ctx {
	op_ctx: OperationContext<()>,
	http_client: rivet_chat::ClientWrapper,
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

		let pools = rivet_pools::from_env("api-chat-test").await.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-chat-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-chat-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-chat-test".to_string(),
			std::time::Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			util::timestamp::now(),
			util::timestamp::now(),
			(),
			Vec::new(),
		);

		let http_client = rivet_chat::Config::builder()
			.set_uri("http://traefik.traefik.svc.cluster.local:80/chat")
			.set_bearer_token("TODO".to_string())
			.build_client();

		Ctx {
			op_ctx,
			http_client,
		}
	}

	fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn generic() {
	let _ctx = Ctx::init().await;

	// TODO: Write tests
}
