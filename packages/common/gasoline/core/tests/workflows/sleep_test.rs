use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct SleepTestInput {
	pub duration_ms: u64,
}

#[workflow(SleepTestWorkflow)]
pub async fn sleep_test_workflow(ctx: &mut WorkflowCtx, input: &SleepTestInput) -> Result<()> {
	ctx.sleep(input.duration_ms).await?;

	Ok(())
}
