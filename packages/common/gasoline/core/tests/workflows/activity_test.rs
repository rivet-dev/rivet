use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityTestInput {
	pub message: String,
}

#[workflow(ActivityTestWorkflow)]
pub async fn activity_test_workflow(
	ctx: &mut WorkflowCtx,
	input: &ActivityTestInput,
) -> Result<String> {
	let result = ctx
		.activity(TestActivityInput {
			message: input.message.clone(),
		})
		.await?;

	Ok(result)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct TestActivityInput {
	pub message: String,
}

#[activity(TestActivity)]
pub async fn test_activity(ctx: &ActivityCtx, input: &TestActivityInput) -> Result<String> {
	Ok(format!("Processed: {}", input.message))
}
