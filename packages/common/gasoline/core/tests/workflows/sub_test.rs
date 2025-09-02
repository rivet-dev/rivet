use super::basic::BasicWorkflowInput;
use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct SubWorkflowInput {
	pub parent_value: String,
}

#[workflow(SubTestWorkflow)]
pub async fn sub_test_workflow(ctx: &mut WorkflowCtx, input: &SubWorkflowInput) -> Result<String> {
	let sub_result = ctx
		.workflow(BasicWorkflowInput {
			value: format!("{}_sub", input.parent_value),
		})
		.output()
		.await?;

	Ok(sub_result)
}
