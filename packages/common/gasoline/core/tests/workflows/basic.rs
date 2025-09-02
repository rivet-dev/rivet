use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicWorkflowInput {
	pub value: String,
}

#[workflow(BasicWorkflow)]
pub async fn basic_workflow(ctx: &mut WorkflowCtx, input: &BasicWorkflowInput) -> Result<String> {
	Ok(input.value.clone())
}
