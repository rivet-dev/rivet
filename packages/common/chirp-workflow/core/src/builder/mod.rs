pub mod common;
pub mod workflow;

#[derive(thiserror::Error, Debug)]
pub(crate) enum BuilderError {
	#[error("tags must be a JSON map")]
	TagsNotMap,
	#[error("invalid signal send: {0}")]
	InvalidSignalSend(&'static str),
	#[error("cannot dispatch a workflow/signal from an operation within a workflow execution. trigger it from the workflow's body")]
	CannotDispatchFromOpInWorkflow,
	#[error("using tags on a sub workflow ({0}) with `.output()` is not supported")]
	TagsOnSubWorkflowOutputNotSupported(&'static str),

	#[error("serde: {0}")]
	Serde(#[from] serde_json::Error),
}
