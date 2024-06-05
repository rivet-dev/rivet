use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::{
	Activity, ActivityEventRow, SignalEventRow, SubWorkflowEventRow, WorkflowError, WorkflowResult,
};

/// An event that happened in the workflow run.
///
/// This is used to replay events.
#[derive(Debug)]
pub enum Event {
	Activity(ActivityEvent),
	Signal(SignalEvent),
	SubWorkflow(SubWorkflowEvent),
}

#[derive(Debug)]
pub struct ActivityEvent {
	pub activity_id: ActivityId,

	/// If activity succeeds, this will be some.
	pub output: Option<serde_json::Value>,
}

impl ActivityEvent {
	pub fn get_output<O: DeserializeOwned>(&self) -> WorkflowResult<Option<O>> {
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
			output: value.output,
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

/// Unique identifier for a specific run of an activity. Used to check for equivalence of activity
/// runs performantly.
///
/// Based on the name of the activity and the hash of the inputs to the activity.
#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct ActivityId {
	pub name: String,
	pub input_hash: u64,
}

impl ActivityId {
	pub fn new<A: Activity>(input: &A::Input) -> Self {
		let mut hasher = DefaultHasher::new();
		input.hash(&mut hasher);
		let input_hash = hasher.finish();

		Self {
			name: A::name().to_string(),
			input_hash,
		}
	}

	pub fn from_bytes(name: String, input_hash: Vec<u8>) -> WorkflowResult<Self> {
		Ok(ActivityId {
			name,
			input_hash: u64::from_le_bytes(
				input_hash
					.try_into()
					.map_err(|_| WorkflowError::IntegerConversion)?,
			),
		})
	}
}
