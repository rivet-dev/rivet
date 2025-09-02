use std::time::Duration;

use gas::prelude::*;
use gas::test;

mod workflows;
use workflows::properties_test::*;
use workflows::state_test::*;
use workflows::tags_test::*;

#[tokio::test]
async fn test_activity_state() {
	let mut reg = Registry::new();
	reg.register_workflow::<StateTestWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	let workflow_id = test_ctx
		.workflow(StateTestInput { initial_value: 42 })
		.dispatch()
		.await
		.unwrap();

	// Wait for workflow to complete with timeout
	let res = tokio::time::timeout(
		Duration::from_secs(5),
		test_ctx.workflow::<StateTestInput>(workflow_id).output(),
	)
	.await
	.unwrap()
	.unwrap();
	assert_eq!(res, 42);
}

#[tokio::test]
async fn test_activity_update_tags() {
	let mut reg = Registry::new();
	reg.register_workflow::<TagsTestWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	let workflow_id = test_ctx
		.workflow(TagsTestInput {
			tag_key: "test_key".to_string(),
			tag_value: "test_value".to_string(),
		})
		.dispatch()
		.await
		.unwrap();

	// Wait for workflow to complete with timeout
	tokio::time::timeout(
		Duration::from_secs(5),
		test_ctx.workflow::<TagsTestInput>(workflow_id).output(),
	)
	.await
	.unwrap()
	.unwrap();
	// Tags were successfully updated if we got here
}

#[tokio::test]
async fn test_activity_ctx_properties() {
	let mut reg = Registry::new();
	reg.register_workflow::<PropertiesTestWorkflow>().unwrap();
	let test_ctx = test::setup(reg).await.unwrap();

	let workflow_id = test_ctx
		.workflow(PropertiesTestInput {})
		.dispatch()
		.await
		.unwrap();

	// Wait for workflow to complete with timeout
	tokio::time::timeout(
		Duration::from_secs(5),
		test_ctx
			.workflow::<PropertiesTestInput>(workflow_id)
			.output(),
	)
	.await
	.unwrap()
	.unwrap();
	// All property checks passed if we got here
}
