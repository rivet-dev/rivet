use chirp_workflow::prelude::*;
use serde_json::json;
use uuid::Uuid;

mod common;
use common::*;

#[tokio::test(flavor = "multi_thread")]
async fn fdb_sqlite_nats_driver() {
	setup_tracing();
	setup_dependencies(false).await;

	let ctx = chirp_workflow::prelude::TestCtx::from_env::<db::DatabaseFdbSqliteNats>(
		"fdb_sqlite_nats_driver",
		true,
	)
	.await;
	let config = ctx.config().clone();
	let pools = ctx.pools().clone();

	// // CLEAR DB
	// pools
	// 	.fdb()
	// 	.unwrap()
	// 	.run(|tx, _mc| async move {
	// 		tx.clear_range(&[0], &[255]);
	// 		Ok(())
	// 	})
	// 	.await
	// 	.unwrap();
	// tokio::time::sleep(std::time::Duration::from_millis(250)).await;

	let mut reg = Registry::new();
	reg.register_workflow::<def::Workflow>().unwrap();
	let reg = reg.handle();

	let db = db::DatabaseFdbSqliteNats::from_pools(pools.clone()).unwrap();

	let workflow_id = Uuid::new_v4();
	let input = serde_json::value::RawValue::from_string("null".to_string()).unwrap();

	db.dispatch_workflow(
		Uuid::new_v4(),
		workflow_id,
		"workflow_name",
		Some(&json!({ "bald": "eagle" })),
		&input,
		false,
	)
	.await
	.unwrap();

	let res = db
		.find_workflow(
			"workflow_name",
			&json!({
				"bald": "eagle",
				"fat": "man"
			}),
		)
		.await
		.unwrap();
	tracing::info!(?res);

	db.update_workflow_tags(
		workflow_id,
		"workflow_name",
		&json!({
			"bald": "eagle",
			"fat": "man"
		}),
	)
	.await
	.unwrap();

	let res = db
		.find_workflow(
			"workflow_name",
			&json!({
				"bald": "eagle",
				"fat": "man"
			}),
		)
		.await
		.unwrap();
	tracing::info!(?res);

	tokio::spawn(async move {
		ctx.workflow(def::Input {})
			.tag("foo", "bar")
			.dispatch()
			.await
			.unwrap();
	});

	// let db2 = db.clone();
	// tokio::spawn(async move {
	// 	use chirp_workflow::db::debug::DatabaseDebug;

	// 	loop {
	// 		let wf = db2
	// 			.find_workflows(
	// 				&[],
	// 				None,
	// 				None,
	// 			)
	// 			.await.unwrap().into_iter().next();

	// 		if let Some(wf) = wf {
	// 			match db2
	// 				.get_workflow_history(
	// 					wf.workflow_id,
	// 					true,
	// 				)
	// 				.await
	// 			{
	// 				Ok(res) => tracing::info!(?res, "-------------"),
	// 				Err(err) => {
	// 					tracing::error!(?err);
	// 					break;
	// 				}
	// 			}
	// 		};

	// 		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
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

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Input {}

	#[workflow]
	pub async fn test(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
		tracing::info!(w=?ctx.workflow_id(), "hello from workflow");

		ctx.activity(TestActivityInput {
			foo: "bar".to_string(),
		})
		.await?;

		let workflow_id = ctx.workflow_id();
		ctx.signal(MySignal {
			test: Uuid::new_v4(),
		})
		.to_workflow_id(workflow_id)
		.send()
		.await?;

		ctx.repeat(|ctx| {
			async move {
				tracing::info!("eepy");
				ctx.sleep(3000).await?;
				tracing::info!("eeped");

				let sig = ctx.listen::<MySignal>().await?;
				tracing::info!(?sig);

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
	struct MySignal {
		test: Uuid,
	}
}
