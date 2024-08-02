use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{
	activity::ActivityId,
	db::{
		ActivityEventRow, MessageSendEventRow, SignalEventRow, SignalSendEventRow,
		SubWorkflowEventRow,
	},
	error::{WorkflowError, WorkflowResult},
};

/// An event that happened in the workflow run.
///
/// This is used to replay events.
#[derive(Debug)]
pub enum Event {
	Activity(ActivityEvent),
	Signal(SignalEvent),
	SignalSend(SignalSendEvent),
	MessageSend(MessageSendEvent),
	SubWorkflow(SubWorkflowEvent),
	// Used as a placeholder for branching locations
	Branch,
}

impl std::fmt::Display for Event {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Event::Activity(activity) => write!(f, "activity {:?}", activity.activity_id.name),
			Event::Signal(signal) => write!(f, "signal {:?}", signal.name),
			Event::SignalSend(signal_send) => write!(f, "signal send {:?}", signal_send.name),
			Event::MessageSend(message_send) => write!(f, "message send {:?}", message_send.name),
			Event::SubWorkflow(sub_workflow) => write!(f, "sub workflow {:?}", sub_workflow.name),
			Event::Branch => write!(f, "branch"),
		}
	}
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
pub struct SignalSendEvent {
	pub signal_id: Uuid,
	pub name: String,
}

impl TryFrom<SignalSendEventRow> for SignalSendEvent {
	type Error = WorkflowError;

	fn try_from(value: SignalSendEventRow) -> WorkflowResult<Self> {
		Ok(SignalSendEvent {
			signal_id: value.signal_id,
			name: value.signal_name,
		})
	}
}

#[derive(Debug)]
pub struct MessageSendEvent {
	pub name: String,
}

impl TryFrom<MessageSendEventRow> for MessageSendEvent {
	type Error = WorkflowError;

	fn try_from(value: MessageSendEventRow) -> WorkflowResult<Self> {
		Ok(MessageSendEvent {
			name: value.message_name,
		})
	}
}

#[derive(Debug)]
pub struct SubWorkflowEvent {
	pub sub_workflow_id: Uuid,
	pub name: String,
}

impl TryFrom<SubWorkflowEventRow> for SubWorkflowEvent {
	type Error = WorkflowError;

	fn try_from(value: SubWorkflowEventRow) -> WorkflowResult<Self> {
		Ok(SubWorkflowEvent {
			sub_workflow_id: value.sub_workflow_id,
			name: value.sub_workflow_name,
		})
	}
}
