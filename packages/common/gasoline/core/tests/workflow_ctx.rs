use std::time::Duration;

use gas::prelude::*;
use gas::test;

mod workflows;
use workflows::activity_test::*;
use workflows::basic::*;
use workflows::eviction_test::*;
use workflows::listen_timeout::*;
use workflows::loop_test::*;
use workflows::signal_test::*;
use workflows::sleep_test::*;
use workflows::sub_test::*;

#[tokio::test]
async fn test_workflow_basic() {
	let mut reg = Registry::new();
	reg.register_workflow::<BasicWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	// Test basic workflow execution
	let workflow_id = test_ctx
		.workflow(BasicWorkflowInput {
			value: "test_value".to_string(),
		})
		.dispatch()
		.await
		.unwrap();

	// Wait for workflow to complete with timeout
	let res = tokio::time::timeout(
		Duration::from_secs(5),
		test_ctx
			.workflow::<BasicWorkflowInput>(workflow_id)
			.output(),
	)
	.await
	.unwrap()
	.unwrap();
	assert_eq!(res, "test_value");
}

#[tokio::test]
async fn test_workflow_activity() {
	let mut reg = Registry::new();
	reg.register_workflow::<ActivityTestWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	let workflow_id = test_ctx
		.workflow(ActivityTestInput {
			message: "hello".to_string(),
		})
		.dispatch()
		.await
		.unwrap();

	// Wait for workflow to complete with timeout
	let res = tokio::time::timeout(
		Duration::from_secs(5),
		test_ctx.workflow::<ActivityTestInput>(workflow_id).output(),
	)
	.await
	.unwrap()
	.unwrap();
	assert_eq!(res, "Processed: hello");
}

#[tokio::test]
async fn test_workflow_sub_workflow() {
	let mut reg = Registry::new();
	reg.register_workflow::<BasicWorkflow>().unwrap();
	reg.register_workflow::<SubTestWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	let workflow_id = test_ctx
		.workflow(SubWorkflowInput {
			parent_value: "parent".to_string(),
		})
		.dispatch()
		.await
		.unwrap();

	// Wait for workflow to complete with timeout
	let res = tokio::time::timeout(
		Duration::from_secs(5),
		test_ctx.workflow::<SubWorkflowInput>(workflow_id).output(),
	)
	.await
	.unwrap()
	.unwrap();
	assert_eq!(res, "parent_sub");
}

#[tokio::test]
async fn test_workflow_sleep() {
	let mut reg = Registry::new();
	reg.register_workflow::<SleepTestWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	let start_time = std::time::Instant::now();
	let workflow_id = test_ctx
		.workflow(SleepTestInput { duration_ms: 100 })
		.dispatch()
		.await
		.unwrap();

	// Wait for workflow to complete with timeout
	tokio::time::timeout(
		Duration::from_secs(5),
		test_ctx.workflow::<SleepTestInput>(workflow_id).output(),
	)
	.await
	.unwrap()
	.unwrap();
	let elapsed = start_time.elapsed();
	// Verify sleep duration was at least the requested time
	assert!(elapsed >= Duration::from_millis(100));
}

#[tokio::test]
async fn test_workflow_signal() {
	let mut reg = Registry::new();
	reg.register_workflow::<SignalTestWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	let workflow_id = test_ctx
		.workflow(SignalTestInput {})
		.dispatch()
		.await
		.unwrap();

	// Give workflow time to start listening
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Send signal
	test_ctx
		.signal(TestSignal {
			value: "signal_value".to_string(),
		})
		.to_workflow_id(workflow_id)
		.send()
		.await
		.unwrap();

	// Wait for workflow to complete
	let res = test_ctx
		.workflow::<SignalTestInput>(workflow_id)
		.output()
		.await
		.unwrap();
	assert_eq!(res, "signal_value");
}

#[tokio::test]
async fn test_workflow_loop() {
	let mut reg = Registry::new();
	reg.register_workflow::<LoopTestWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	let workflow_id = test_ctx
		.workflow(LoopWorkflowInput { iterations: 3 })
		.dispatch()
		.await
		.unwrap();

	// Wait for workflow to complete with timeout
	let res = tokio::time::timeout(
		Duration::from_secs(5),
		test_ctx.workflow::<LoopWorkflowInput>(workflow_id).output(),
	)
	.await
	.unwrap()
	.unwrap();
	assert_eq!(res, 3);
}

#[tokio::test]
async fn test_workflow_listen_with_timeout() {
	let mut reg = Registry::new();
	reg.register_workflow::<ListenTimeoutWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	let workflow_id = test_ctx
		.workflow(ListenTimeoutInput { timeout_ms: 100 })
		.dispatch()
		.await
		.unwrap();

	// Wait for workflow to complete with timeout
	let res = tokio::time::timeout(
		Duration::from_secs(5),
		test_ctx
			.workflow::<ListenTimeoutInput>(workflow_id)
			.output(),
	)
	.await
	.unwrap()
	.unwrap();
	assert!(res); // Should have timed out since we didn't send a signal
}

#[tokio::test]
async fn test_workflow_eviction() {
	fn build_reg() -> Registry {
		let mut reg = Registry::new();
		reg.register_workflow::<EvictionTestWorkflow>().unwrap();
		reg
	}

	let test_id = Uuid::new_v4();

	// Run workflow
	let (workflow_id, db_path) = {
		let test_deps = rivet_test_deps::TestDeps::new_with_test_id(test_id)
			.await
			.unwrap();
		let mut test_ctx = test::setup_with_deps(build_reg(), test_deps).await.unwrap();

		let mut sub = test_ctx
			.subscribe::<RunningMessage>(("test", test_id))
			.await
			.unwrap();

		let workflow_id = test_ctx
			.workflow(EvictionTestInput { test_id })
			.dispatch()
			.await
			.unwrap();

		sub.next().await.unwrap();

		// Shut down worker
		test_ctx.shutdown().await.unwrap();

		//// Check workflow is sleeping
		let res =
			gas::db::debug::DatabaseDebug::get_workflows(test_ctx.debug_db(), vec![workflow_id])
				.await
				.unwrap()
				.into_iter()
				.next()
				.unwrap();
		assert_eq!(res.state, gas::db::debug::WorkflowState::Sleeping,);

		(
			workflow_id,
			if let rivet_config::config::Database::FileSystem(fs) =
				test_ctx.config().database.as_ref().unwrap()
			{
				fs.path.clone()
			} else {
				panic!()
			},
		)
	};

	// HACK: Lock on rocksdb file doesn't drop without this
	tokio::task::yield_now().await;

	// Wake the workflow again
	{
		let test_deps = rivet_test_deps::TestDeps::new_with_test_id(test_id)
			.await
			.unwrap();
		let test_ctx = test::setup_with_deps(build_reg(), test_deps).await.unwrap();

		let mut sub = test_ctx
			.subscribe::<RunningMessage>(("test", test_id))
			.await
			.unwrap();

		test_ctx
			.signal(TestSignal2 {
				value: "foo".into(),
			})
			.to_workflow_id(workflow_id)
			.send()
			.await
			.unwrap();

		sub.next().await.unwrap();
	}
}
