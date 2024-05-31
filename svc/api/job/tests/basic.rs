// TODO: Rewrite with new api clients

// use std::{sync::Once, time::Duration};

// use proto::backend::{self, pkg::*};
// use rivet_operation::prelude::*;

// static GLOBAL_INIT: Once = Once::new();

// struct Ctx {
// 	op_ctx: OperationContext<()>,
// 	nomad_config: nomad_client::apis::configuration::Configuration,
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

// 		let pools = rivet_pools::from_env("api-job-test").await.unwrap();
// 		let cache = rivet_cache::CacheInner::new(
// 			"api-job-test".to_string(),
// 			std::env::var("RIVET_SOURCE_HASH").unwrap(),
// 			pools.redis_cache().unwrap(),
// 		);
// 		let client = chirp_client::SharedClient::from_env(pools.clone())
// 			.expect("create client")
// 			.wrap_new("api-job-test");
// 		let conn = rivet_connection::Connection::new(client, pools, cache);
// 		let op_ctx = OperationContext::new(
// 			"api-job-test".to_string(),
// 			std::time::Duration::from_secs(60),
// 			conn,
// 			Uuid::new_v4(),
// 			Uuid::new_v4(),
// 			util::timestamp::now(),
// 			util::timestamp::now(),
// 			(),
// 			Vec::new(),
// 		);

// 		let nomad_config = nomad_util::config_from_env().unwrap();

// 		Ctx {
// 			op_ctx,
// 			nomad_config,
// 		}
// 	}

// 	fn http_client(&self, bearer_token: String) -> rivet_job::ClientWrapper {
// 		rivet_job::Config::builder()
// 			.set_uri("http://traefik.traefik.svc.cluster.local:80/job")
// 			.set_bearer_token(bearer_token)
// 			.build_client()
// 	}

// 	/// Issues a testing job run token. We use this since we can't access the job run token issued
// 	/// when the job is ran.
// 	async fn job_run_token(&self, run_id: Uuid) -> String {
// 		let token_res = op!([self] token_create {
// 			issuer: "test".into(),
// 			token_config: Some(token::create::request::TokenConfig {
// 				ttl: util::duration::days(365),
// 			}),
// 			refresh_token_config: None,
// 			client: None,
// 			kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
// 				entitlements: vec![
// 					proto::claims::Entitlement {
// 						kind: Some(
// 							proto::claims::entitlement::Kind::JobRun(proto::claims::entitlement::JobRun {
// 								run_id: Some(run_id.into()),
// 							})
// 						)
// 					}
// 				],
// 			})),
// 			label: Some("jr".into()),
// 			..Default::default()
// 		})
// 		.await
// 		.unwrap();

// 		token_res.token.as_ref().unwrap().token.clone()
// 	}

// 	fn chirp(&self) -> &chirp_client::Client {
// 		self.op_ctx.chirp()
// 	}

// 	fn op_ctx(&self) -> &OperationContext<()> {
// 		&self.op_ctx
// 	}
// }

// async fn run_job(ctx: &Ctx) -> (Uuid, backend::job::Run, backend::job::run_meta::Nomad) {
// 	let res = op!([ctx] faker_job_run {
// 		..Default::default()
// 	})
// 	.await
// 	.unwrap();
// 	let run_id = res.run_id.unwrap().as_uuid();

// 	// Check the run exists and that `job_run` is not broken
// 	let runs_res = op!([ctx] job_run_get {
// 		run_ids: vec![run_id.into()],
// 	})
// 	.await
// 	.unwrap();
// 	assert!(!runs_res.runs.is_empty(), "job was not created");
// 	let run = runs_res.runs.first().unwrap().clone();

// 	let run_meta = match run.run_meta.as_ref().unwrap().kind.as_ref().unwrap() {
// 		backend::job::run_meta::Kind::Nomad(x) => x.clone(),
// 	};

// 	(run_id, run, run_meta)
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn run_cleanup() {
// 	let ctx = Ctx::init().await;

// 	// MARK: POST /runs/cleanup
// 	{
// 		tracing::info!("run cleanup");

// 		// Create test job
// 		let (run_id, _, _run_meta) = run_job(&ctx).await;
// 		let run_token = ctx.job_run_token(run_id).await;
// 		let http_client = ctx.http_client(run_token);

// 		http_client.cleanup().send().await.unwrap();

// 		tokio::time::sleep(Duration::from_secs(2)).await;

// 		let run_res = op!([ctx] job_run_get {
// 			run_ids: vec![run_id.into()],
// 		})
// 		.await
// 		.unwrap();
// 		assert!(
// 			run_res.runs.first().unwrap().stop_ts.is_some(),
// 			"run was not cleaned up"
// 		);
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn run_cleanup_from_poststop() {
// 	let ctx = Ctx::init().await;

// 	// MARK: POST /runs/cleanup
// 	{
// 		tracing::info!("run cleanup");

// 		// Create test job
// 		let (run_id, _, _run_meta) = run_job(&ctx).await;
// 		let run_token = ctx.job_run_token(run_id).await;
// 		let http_client = ctx.http_client(run_token);

// 		let mut cleanup_sub = subscribe!([ctx] job_run::msg::cleanup_complete(run_id))
// 			.await
// 			.unwrap();

// 		http_client.cleanup().send().await.unwrap();

// 		cleanup_sub.next().await.unwrap();

// 		let run_res = op!([ctx] job_run_get {
// 			run_ids: vec![run_id.into()],
// 		})
// 		.await
// 		.unwrap();
// 		assert!(
// 			run_res.runs.first().unwrap().stop_ts.is_some(),
// 			"run was not cleaned up"
// 		);
// 	}

// 	// TODO: Test the logs
// }
