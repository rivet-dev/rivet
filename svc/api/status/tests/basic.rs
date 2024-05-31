// TODO: Rewrite with new api clients

// use std::sync::Once;

// use rivet_operation::prelude::*;

// static GLOBAL_INIT: Once = Once::new();

// struct Ctx {
// 	op_ctx: OperationContext<()>,
// 	http_client: rivet_status::ClientWrapper,
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

// 		let pools = rivet_pools::from_env("api-status-test").await.unwrap();
// 		let cache = rivet_cache::CacheInner::new(
// 			"api-status-test".to_string(),
// 			std::env::var("RIVET_SOURCE_HASH").unwrap(),
// 			pools.redis_cache().unwrap(),
// 		);
// 		let client = chirp_client::SharedClient::from_env(pools.clone())
// 			.expect("create client")
// 			.wrap_new("api-status-test");
// 		let conn = rivet_connection::Connection::new(client, pools, cache);
// 		let op_ctx = OperationContext::new(
// 			"api-status-test".to_string(),
// 			std::time::Duration::from_secs(60),
// 			conn,
// 			Uuid::new_v4(),
// 			Uuid::new_v4(),
// 			util::timestamp::now(),
// 			util::timestamp::now(),
// 			(),
// 			Vec::new(),
// 		);

// 		let http_client = rivet_status::Config::builder()
// 			.set_uri("http://traefik.traefik.svc.cluster.local:80/status")
// 			.set_bearer_token(
// 				util::env::read_secret(&["rivet", "api_status", "token"])
// 					.await
// 					.unwrap(),
// 			)
// 			.build_client();

// 		Ctx {
// 			op_ctx,
// 			http_client,
// 		}
// 	}

// 	fn chirp(&self) -> &chirp_client::Client {
// 		self.op_ctx.chirp()
// 	}

// 	fn op_ctx(&self) -> &OperationContext<()> {
// 		&self.op_ctx
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn matchmaker_find() {
// 	let ctx = Ctx::init().await;

// 	let region_res = op!([ctx] faker_region {}).await.unwrap();
// 	let region = region_res.region.unwrap();

// 	ctx.http_client
// 		.matchmaker()
// 		.region(&region.name_id)
// 		.send()
// 		.await
// 		.unwrap();
// }
