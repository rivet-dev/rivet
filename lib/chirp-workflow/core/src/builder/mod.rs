pub mod common;
pub mod workflow;

#[derive(thiserror::Error, Debug)]
pub(crate) enum BuilderError {
	#[error("tags must be a JSON map")]
	TagsNotMap,
	#[error("cannot call `to_workflow` and set tags on the same signal")]
	WorkflowIdAndTags,
	#[error("must call `to_workflow` or set tags on signal")]
	NoWorkflowIdOrTags,
	#[error("cannot dispatch a workflow/signal from an operation within a workflow execution. trigger it from the workflow's body")]
	CannotDispatchFromOpInWorkflow,
}
