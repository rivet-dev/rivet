use proto::backend::pkg::*;
use rivet_operation::prelude::*;

struct Ctx {
	op_ctx: OperationContext<()>,
	http_client: rivet_cf_verification::ClientWrapper,
}

impl Ctx {
	async fn init() -> Ctx {
		let _ = tracing_subscriber::fmt()
			.pretty()
			.with_max_level(tracing::Level::INFO)
			.with_target(false)
			.try_init();

		let pools = rivet_pools::from_env("api-cf-verification-test")
			.await
			.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-cf-verification-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-cf-verification-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-cf-verification-test".to_string(),
			std::time::Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			util::timestamp::now(),
			util::timestamp::now(),
			(),
			Vec::new(),
		);

		let http_client = rivet_cf_verification::Config::builder()
			.set_uri(util::env::svc_router_url("api-cf-verification"))
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
async fn custom_hostname_verification() {
	let ctx = Ctx::init().await;

	// MARK: GET /.well-known/cf-custom-hostname-challenge/{}
	{
		tracing::info!("testing custom hostname verification");

		let game_res = op!([ctx] faker_game { }).await.unwrap();
		let namespace_id = game_res.namespace_ids.first().unwrap().as_uuid();

		let hostname = format!("{}.com", util::faker::ident());

		let res = msg!([ctx] cf_custom_hostname::msg::create(namespace_id, &hostname) -> Result<cf_custom_hostname::msg::create_complete, cf_custom_hostname::msg::create_fail> {
			namespace_id: Some(namespace_id.into()),
			hostname: hostname.clone(),
			bypass_pending_cap: false,
		}).await.unwrap().unwrap();
		let identifier = res.identifier.unwrap();

		let res = op!([ctx] cf_custom_hostname_get {
			identifiers: vec![identifier],
		})
		.await
		.unwrap();
		assert_eq!(1, res.custom_hostnames.len());

		let custom_hostname = res.custom_hostnames.first().unwrap();
		let challenge = custom_hostname.challenge.unwrap().as_uuid();

		let res = ctx
			.http_client
			.verify_custom_hostname()
			.identifier(identifier.as_uuid().to_string())
			.send()
			.await
			.unwrap();

		assert_eq!(format!("{challenge}\n"), res.body().unwrap());
	}
}
