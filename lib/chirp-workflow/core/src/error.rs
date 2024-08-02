use global_error::GlobalError;
use uuid::Uuid;

pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// Throwing this will eject from the workflow scope back in to the engine.
///
/// This error should not be touched by the user and is only intended to be handled by the workflow
/// engine.
#[derive(thiserror::Error, Debug)]
pub enum WorkflowError {
	#[error("workflow failure: {0:?}")]
	WorkflowFailure(GlobalError),

	#[error("activity failure: {0:?}")]
	ActivityFailure(GlobalError),

	#[error("activity failure, max retries reached: {0:?}")]
	ActivityMaxFailuresReached(GlobalError),

	#[error("operation failure: {0:?}")]
	OperationFailure(GlobalError),

	#[error("workflow missing from registry: {0}")]
	WorkflowMissingFromRegistry(String),

	#[error("workflow not found")]
	WorkflowNotFound,

	#[error("history diverged")]
	HistoryDiverged,

	#[error("serialize workflow input: {0}")]
	SerializeWorkflowInput(serde_json::Error),

	#[error("deserialize workflow input: {0}")]
	DeserializeWorkflowInput(serde_json::Error),

	#[error("serialize activity output: {0}")]
	SerializeWorkflowOutput(serde_json::Error),

	#[error("deserialize workflow input: {0}")]
	DeserializeWorkflowOutput(serde_json::Error),

	#[error("serialize workflow tags: {0}")]
	SerializeWorkflowTags(serde_json::Error),

	#[error("deserialize workflow tags: {0}")]
	DeserializeWorkflowTags(serde_json::Error),

	#[error("serialize activity input: {0}")]
	SerializeActivityInput(serde_json::Error),

	#[error("serialize activity output: {0}")]
	SerializeActivityOutput(serde_json::Error),

	#[error("deserialize activity output: {0}")]
	DeserializeActivityOutput(serde_json::Error),

	#[error("serialize signal body: {0}")]
	SerializeSignalBody(serde_json::Error),

	#[error("deserialize signal body: {0}")]
	DeserializeSignalBody(serde_json::Error),

	#[error("no signal found: {0:?}")]
	NoSignalFound(Box<[&'static str]>),

	#[error("sub workflow incomplete: {0:?}")]
	SubWorkflowIncomplete(Uuid),

	#[error("integer conversion failed")]
	IntegerConversion,

	#[error("build sql pool: {0}")]
	BuildSqlx(sqlx::Error),

	#[error("sql: {0}")]
	Sqlx(sqlx::Error),

	#[error("activity timed out")]
	ActivityTimeout,

	#[error("operation timed out")]
	OperationTimeout,
}

impl WorkflowError {
	pub fn is_recoverable(&self) -> bool {
		match self {
			WorkflowError::ActivityFailure(_) => true,
			WorkflowError::ActivityTimeout => true,
			WorkflowError::OperationTimeout => true,
			_ => false,
		}
	}

	pub(crate) fn signals(&self) -> &[&'static str] {
		if let WorkflowError::NoSignalFound(signals) = self {
			signals
		} else {
			&[]
		}
	}

	pub(crate) fn sub_workflow(&self) -> Option<Uuid> {
		if let WorkflowError::SubWorkflowIncomplete(sub_workflow_id) = self {
			Some(*sub_workflow_id)
		} else {
			None
		}
	}
}
