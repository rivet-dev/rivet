use std::{
	hash::{DefaultHasher, Hash, Hasher},
	ops::Deref,
};

use serde::de::DeserializeOwned;
use strum::FromRepr;
use uuid::Uuid;

use super::location::Coordinate;
use crate::error::{WorkflowError, WorkflowResult};

/// An event that happened in the workflow run.
///
/// This is used to replay events.
#[derive(Debug)]
pub struct Event {
	/// Position within the root location.
	pub(crate) coordinate: Coordinate,
	pub(crate) version: usize,
	pub(crate) data: EventData,
}

impl Event {
	pub fn coordinate(&self) -> &Coordinate {
		&self.coordinate
	}
}

impl Deref for Event {
	type Target = EventData;

	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

#[derive(Debug)]
pub enum EventData {
	Activity(ActivityEvent),
	Signal(SignalEvent),
	SignalSend(SignalSendEvent),
	MessageSend(MessageSendEvent),
	SubWorkflow(SubWorkflowEvent),
	Loop(LoopEvent),
	Sleep(SleepEvent),
	Removed(RemovedEvent),
	VersionCheck,
	// Used as a placeholder for branching locations
	Branch,
}

impl std::fmt::Display for Event {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.data {
			EventData::Activity(activity) => write!(f, "activity {:?}", activity.event_id.name),
			EventData::Signal(signal) => write!(f, "signal {:?}", signal.name),
			EventData::SignalSend(signal_send) => write!(f, "signal send {:?}", signal_send.name),
			EventData::MessageSend(message_send) => {
				write!(f, "message send {:?}", message_send.name)
			}
			EventData::SubWorkflow(sub_workflow) => {
				write!(f, "sub workflow {:?}", sub_workflow.name)
			}
			EventData::Loop(_) => write!(f, "loop"),
			EventData::Sleep(_) => write!(f, "sleep"),
			EventData::Removed(removed) => {
				if let Some(name) = &removed.name {
					write!(f, "removed {} {name}", removed.event_type)
				} else {
					write!(f, "removed {}", removed.event_type)
				}
			}
			EventData::VersionCheck => write!(f, "version check"),
			EventData::Branch => write!(f, "branch"),
		}
	}
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum EventType {
	Activity = 0,
	Signal = 1,
	SignalSend = 2,
	MessageSend = 3,
	SubWorkflow = 4,
	Loop = 5,
	Sleep = 6,
	Branch = 7,
	Removed = 8,
	VersionCheck = 9,
}

impl std::fmt::Display for EventType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			EventType::Activity => write!(f, "activity"),
			EventType::Signal => write!(f, "signal"),
			EventType::SignalSend => write!(f, "signal send"),
			EventType::MessageSend => write!(f, "message send"),
			EventType::SubWorkflow => write!(f, "sub workflow"),
			EventType::Loop => write!(f, "loop"),
			EventType::Sleep => write!(f, "sleep"),
			EventType::Removed => write!(f, "removed event"),
			EventType::VersionCheck => write!(f, "version check"),
			EventType::Branch => write!(f, "branch"),
		}
	}
}

#[derive(Debug)]
pub struct ActivityEvent {
	pub event_id: EventId,
	pub create_ts: i64,

	/// If the activity succeeds, this will be some.
	pub(crate) output: Option<Box<serde_json::value::RawValue>>,
	pub error_count: usize,
}

impl ActivityEvent {
	pub fn parse_output<O: DeserializeOwned>(&self) -> WorkflowResult<Option<O>> {
		self.output
			.as_ref()
			.map(|x| serde_json::from_str(x.get()))
			.transpose()
			.map_err(WorkflowError::DeserializeActivityOutput)
	}
}

#[derive(Debug)]
pub struct SignalEvent {
	pub name: String,
	pub body: Box<serde_json::value::RawValue>,
}

#[derive(Debug)]
pub struct SignalSendEvent {
	pub signal_id: Uuid,
	pub name: String,
}

#[derive(Debug)]
pub struct MessageSendEvent {
	pub name: String,
}

#[derive(Debug)]
pub struct SubWorkflowEvent {
	pub sub_workflow_id: Uuid,
	pub name: String,
}

#[derive(Debug)]
pub struct LoopEvent {
	/// If the loop completes, this will be some.
	pub(crate) output: Option<Box<serde_json::value::RawValue>>,
	pub iteration: usize,
}

impl LoopEvent {
	pub fn parse_output<O: DeserializeOwned>(&self) -> WorkflowResult<Option<O>> {
		self.output
			.as_ref()
			.map(|x| serde_json::from_str(x.get()))
			.transpose()
			.map_err(WorkflowError::DeserializeLoopOutput)
	}
}

#[derive(Debug)]
pub struct SleepEvent {
	pub deadline_ts: i64,
	pub state: SleepState,
}

#[derive(Debug, Clone, Hash, Copy, PartialEq, Eq, FromRepr)]
pub enum SleepState {
	Normal,
	Uninterrupted,
	Interrupted,
}

#[derive(Debug)]
pub struct RemovedEvent {
	pub event_type: EventType,
	pub name: Option<String>,
}

/// Based on the name of the event and the hash of the input (if it has one).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EventId {
	pub name: String,
	pub input_hash: u64,
}

impl EventId {
	pub fn new<I: Hash>(name: &str, input: I) -> Self {
		let mut hasher = DefaultHasher::new();
		input.hash(&mut hasher);
		let input_hash = hasher.finish();

		Self {
			name: name.to_string(),
			input_hash,
		}
	}

	pub fn from_bytes(name: String, input_hash: Vec<u8>) -> WorkflowResult<Self> {
		Ok(EventId {
			name,
			input_hash: u64::from_le_bytes(
				input_hash
					.try_into()
					.map_err(|_| WorkflowError::IntegerConversion)?,
			),
		})
	}
}
