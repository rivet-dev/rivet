use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{
	activity::ActivityId, ActivityEventRow, SignalEventRow, SubWorkflowEventRow, WorkflowError,
	WorkflowResult,
};

/// An event that happened in the workflow run.
///
/// This is used to replay events.
#[derive(Debug)]
pub enum Event {
	Activity(ActivityEvent),
	Signal(SignalEvent),
	SubWorkflow(SubWorkflowEvent),
	// Used as a placeholder for branching locations
	Branch,
}

#[derive(Debug)]
pub struct ActivityEvent {
	pub activity_id: ActivityId,
	pub create_ts: i64,

	/// If activity succeeds, this will be some.
	pub(crate) output: Option<serde_json::Value>,
	pub error_count: u32,
}

impl ActivityEvent {
	pub fn parse_output<O: DeserializeOwned>(&self) -> WorkflowResult<Option<O>> {
		self.output
			.clone()
			.map(serde_json::from_value)
			.transpose()
			.map_err(WorkflowError::DeserializeActivityOutput)
	}
}

impl TryFrom<ActivityEventRow> for ActivityEvent {
	type Error = WorkflowError;

	fn try_from(value: ActivityEventRow) -> WorkflowResult<Self> {
		Ok(ActivityEvent {
			activity_id: ActivityId::from_bytes(value.activity_name, value.input_hash)?,
			create_ts: value.create_ts,
			output: value.output,
			error_count: value
				.error_count
				.try_into()
				.map_err(|_| WorkflowError::IntegerConversion)?,
		})
	}
}

#[derive(Debug)]
pub struct SignalEvent {
	pub name: String,
	pub body: serde_json::Value,
}

impl TryFrom<SignalEventRow> for SignalEvent {
	type Error = WorkflowError;

	fn try_from(value: SignalEventRow) -> WorkflowResult<Self> {
		Ok(SignalEvent {
			name: value.signal_name,
			body: value.body,
		})
	}
}

#[derive(Debug)]
pub struct SubWorkflowEvent {
	pub sub_workflow_id: Uuid,
	pub sub_workflow_name: String,
}

impl TryFrom<SubWorkflowEventRow> for SubWorkflowEvent {
	type Error = WorkflowError;

	fn try_from(value: SubWorkflowEventRow) -> WorkflowResult<Self> {
		Ok(SubWorkflowEvent {
			sub_workflow_id: value.sub_workflow_id,
			sub_workflow_name: value.sub_workflow_name,
		})
	}
}
