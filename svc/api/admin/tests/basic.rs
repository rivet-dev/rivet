use std::sync::Once;

use rivet_operation::prelude::*;

static GLOBAL_INIT: Once = Once::new();

const TOKEN: &str = "ZfGFOIALpu3vj1r2qO";

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ConsulService {
	address: String,
	service_port: u16,
}

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

		let pools = rivet_pools::from_env("api-admin-test").await.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-admin-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-admin-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-admin-test".to_string(),
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

#[tokio::test(flavor = "multi_thread")]
async fn convert_team() {
	let ctx = Ctx::init().await;

	todo!()
}
