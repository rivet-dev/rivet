use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct PropertiesTestInput {}

#[workflow(PropertiesTestWorkflow)]
pub async fn properties_test_workflow(
	ctx: &mut WorkflowCtx,
	_input: &PropertiesTestInput,
) -> Result<()> {
	let workflow_id = ctx.workflow_id();
	let ray_id = ctx.ray_id();

	ctx.activity(PropertiesActivityInput {
		expected_workflow_id: workflow_id,
		expected_ray_id: ray_id,
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct PropertiesActivityInput {
	pub expected_workflow_id: Id,
	pub expected_ray_id: Id,
}

#[activity(PropertiesActivity)]
pub async fn properties_activity(ctx: &ActivityCtx, input: &PropertiesActivityInput) -> Result<()> {
	// Test name() method - the name comes in lowercase
	assert_eq!(ctx.name(), "properties_activity");

	// Test workflow_id() method
	assert_eq!(ctx.workflow_id(), input.expected_workflow_id);

	// Test ray_id() method
	assert_eq!(ctx.ray_id(), input.expected_ray_id);

	// Test ts() method - should be a valid timestamp
	assert!(ctx.ts() > 0);

	// Test pools() method - should return valid pools
	let _ = ctx.pools();

	// Test cache() method - should return valid cache
	let _ = ctx.cache();

	// Test config() method - should return valid config
	let _ = ctx.config();

	Ok(())
}
