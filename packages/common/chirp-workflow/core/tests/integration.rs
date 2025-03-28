use std::time::Duration;

use chirp_workflow::prelude::*;
// use serde_json::json;
// use uuid::Uuid;

mod common;
use common::*;

#[tokio::test(flavor = "multi_thread")]
async fn fdb_sqlite_nats_driver() {
	setup_tracing();

	let ctx = chirp_workflow::prelude::TestCtx::from_env::<db::DatabaseFdbSqliteNats>(
		"fdb_sqlite_nats_driver",
		true,
	)
	.await;
	let config = ctx.config().clone();
	let pools = ctx.pools().clone();

	let mut reg = Registry::new();
	reg.register_workflow::<def::Workflow>().unwrap();
	let reg = reg.handle();

	let db = db::DatabaseFdbSqliteNats::from_pools(pools.clone()).unwrap();

	// let workflow_id = Uuid::new_v4();
	// let input = serde_json::value::RawValue::from_string("null".to_string()).unwrap();

	// db.dispatch_workflow(
	// 	Uuid::new_v4(),
	// 	workflow_id,
	// 	"workflow_name",
	// 	Some(&json!({ "bald": "eagle" })),
	// 	&input,
	// 	false,
	// )
	// .await
	// .unwrap();

	// let res = db
	// 	.find_workflow(
	// 		"workflow_name",
	// 		&json!({
	// 			"bald": "eagle",
	// 			"fat": "man"
	// 		}),
	// 	)
	// 	.await
	// 	.unwrap();
	// tracing::info!(?res);

	// db.update_workflow_tags(
	// 	workflow_id,
	// 	"workflow_name",
	// 	&json!({
	// 		"bald": "eagle",
	// 		"fat": "man"
	// 	}),
	// )
	// .await
	// .unwrap();

	// let res = db
	// 	.find_workflow(
	// 		"workflow_name",
	// 		&json!({
	// 			"bald": "eagle",
	// 			"fat": "man"
	// 		}),
	// 	)
	// 	.await
	// 	.unwrap();
	// tracing::info!(?res);

	if std::env::var("SPAWN_WF").unwrap_or_default() == "1" {
		for _ in 0..1 {
			let ctx2 = ctx.clone();
			tokio::spawn(async move {
				ctx2.workflow(def::Input {})
					.tag("foo", "bar")
					.dispatch()
					.await
					.unwrap();
			});
		}
	}

	// let ctx2 = ctx.clone();
	// tokio::spawn(async move {
	// 	for _ in 0..10 {
	// 		tokio::time::sleep(Duration::from_secs(2)).await;
	// 		ctx2.signal(def::MySignal {
	// 			test: Uuid::new_v4(),
	// 		})
	// 		.to_workflow::<def::Workflow>()
	// 		.tag("foo", "bar")
	// 		.send()
	// 		.await
	// 		.unwrap();
	// 	}
	// });

	let worker = Worker::new(reg.clone(), db.clone());

	// Start worker
	tokio::select! {
		res = worker.start(config.clone(), pools.clone()) => res.unwrap(),
		res = tokio::signal::ctrl_c() => res.unwrap(),
	}
}

mod def {
	use chirp_workflow::prelude::*;
	use futures_util::FutureExt;
	use sqlx::Acquire;

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Input {}

	#[workflow]
	pub async fn test(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
		tracing::info!(w=?ctx.workflow_id(), "hello from workflow");

		ctx.activity(TestActivityInput {
			foo: "bar".to_string(),
		})
		.await?;

		// let workflow_id = ctx.workflow_id();
		// ctx.signal(MySignal {
		// 	test: Uuid::new_v4(),
		// })
		// .to_workflow_id(workflow_id)
		// .send()
		// .await?;

		ctx.repeat(|ctx| {
			async move {
				let sig = ctx.listen_with_timeout::<MySignal>(5 * 1000).await?;
				tracing::info!(?sig);

				let start = std::time::Instant::now();

				ctx.activity(TestActivityInput {
					foo: "bar".to_string(),
				})
				.await?;

				ctx.activity(TestActivityInput {
					foo: "bar".to_string(),
				})
				.await?;

				tracing::info!(dt=?start.elapsed(), "-------------");

				Ok(Loop::<()>::Continue)
			}
			.boxed()
		})
		.await?;

		Ok(())
	}

	#[derive(Debug, Serialize, Deserialize, Hash)]
	struct TestActivityInput {
		foo: String,
	}

	#[activity(TestActivity)]
	async fn test_activity(ctx: &ActivityCtx, input: &TestActivityInput) -> GlobalResult<()> {
		tracing::info!(?input.foo, "hello from activity");

		Ok(())
	}

	#[signal("my_signal")]
	#[derive(Debug)]
	pub struct MySignal {
		pub test: Uuid,
	}
}
