use rivet_util::Id;

use crate::workflow::WorkflowInput;

pub mod common;
pub mod workflow;

#[derive(thiserror::Error, Debug)]
pub(crate) enum BuilderError {
	#[error("tags must be a JSON map")]
	TagsNotMap,
	#[error("invalid signal send: {0}")]
	InvalidSignalSend(&'static str),
	#[error("invalid workflow dispatch: {0}")]
	InvalidWorkflowDispatch(&'static str),
	#[error(
		"cannot dispatch a workflow/signal from an operation within a workflow execution. trigger it from the workflow's body"
	)]
	CannotDispatchFromOpInWorkflow,
	#[error("using tags on a sub workflow ({0}) with `.output()` is not supported")]
	TagsOnSubWorkflowOutputNotSupported(&'static str),

	#[error("serde: {0}")]
	Serde(#[from] serde_json::Error),
}

pub trait WorkflowRepr<I: WorkflowInput> {
	#[allow(private_interfaces)]
	fn as_input(&self) -> Result<&I, BuilderError>;
	#[allow(private_interfaces)]
	fn as_workflow_id(&self) -> Result<Id, BuilderError>;
}

impl<I: WorkflowInput> WorkflowRepr<I> for I {
	#[allow(private_interfaces)]
	fn as_input(&self) -> Result<&I, BuilderError> {
		Ok(self)
	}

	#[allow(private_interfaces)]
	fn as_workflow_id(&self) -> Result<Id, BuilderError> {
		Err(BuilderError::InvalidWorkflowDispatch(
			"workflow inputs are not retrievable",
		))
	}
}

impl<I: WorkflowInput> WorkflowRepr<I> for Id {
	#[allow(private_interfaces)]
	fn as_input(&self) -> Result<&I, BuilderError> {
		Err(BuilderError::InvalidWorkflowDispatch(
			"id's are not instantiable",
		))
	}

	#[allow(private_interfaces)]
	fn as_workflow_id(&self) -> Result<Id, BuilderError> {
		Ok(*self)
	}
}
