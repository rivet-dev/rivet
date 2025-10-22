use anyhow::*;
use rivet_util::Id;

use super::Database;
use crate::history::{
	event::{RemovedEvent, SleepEvent},
	location::Location,
};

#[async_trait::async_trait]
pub trait DatabaseDebug: Database {
	async fn get_workflows(&self, workflow_ids: Vec<Id>) -> Result<Vec<WorkflowData>>;

	async fn find_workflows(
		&self,
		tags: &[(String, String)],
		name: Option<&str>,
		state: Option<WorkflowState>,
	) -> Result<Vec<WorkflowData>>;

	async fn silence_workflows(&self, workflow_ids: Vec<Id>) -> Result<()>;

	async fn wake_workflows(&self, workflow_ids: Vec<Id>) -> Result<()>;

	async fn get_workflow_history(
		&self,
		workflow_id: Id,
		include_forgotten: bool,
	) -> Result<Option<HistoryData>>;

	async fn get_signals(&self, signal_ids: Vec<Id>) -> Result<Vec<SignalData>>;

	async fn find_signals(
		&self,
		tags: &[(String, String)],
		workflow_id: Option<Id>,
		name: Option<&str>,
		state: Option<SignalState>,
	) -> Result<Vec<SignalData>>;

	async fn silence_signals(&self, signal_ids: Vec<Id>) -> Result<()>;
}

#[derive(Debug)]
pub struct WorkflowData {
	pub workflow_id: Id,
	pub workflow_name: String,
	pub tags: serde_json::Value,
	pub create_ts: i64,
	pub input: serde_json::Value,
	// Internally same as state, renamed to data to avoid confusion
	pub data: serde_json::Value,
	pub output: Option<serde_json::Value>,
	pub error: Option<String>,
	pub state: WorkflowState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkflowState {
	Complete,
	Running,
	Sleeping,
	Dead,
	Silenced,
}

#[derive(Debug)]
pub struct HistoryData {
	pub wf: WorkflowData,
	pub events: Vec<Event>,
}

#[derive(Debug)]
pub struct Event {
	pub location: Location,
	pub version: usize,
	pub create_ts: i64,
	pub forgotten: bool,
	pub data: EventData,
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
	Branch,

	/// NOTE: Strictly used as a placeholder for backfilling. When using this, the coordinate of the `Event`
	/// must still be valid.
	Empty,
}

impl std::fmt::Display for EventData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self {
			EventData::Activity(activity) => write!(f, "activity {}", activity.name),
			EventData::Signal(signal) => write!(f, "signal receive {}", signal.name),
			EventData::SignalSend(signal_send) => write!(f, "signal send {}", signal_send.name),
			EventData::MessageSend(message_send) => {
				write!(f, "message send {}", message_send.name)
			}
			EventData::SubWorkflow(sub_workflow) => {
				write!(f, "sub workflow {}", sub_workflow.name)
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
			EventData::Empty => write!(f, "empty"),
		}
	}
}

#[derive(Debug)]
pub struct ActivityEvent {
	pub name: String,
	pub input: serde_json::Value,
	pub output: Option<serde_json::Value>,
	pub errors: Vec<ActivityError>,
}

#[derive(Debug)]
pub struct SignalEvent {
	pub signal_id: Id,
	pub name: String,
	pub body: serde_json::Value,
}

#[derive(Debug)]
pub struct SignalSendEvent {
	pub signal_id: Id,
	pub name: String,
	pub workflow_id: Option<Id>,
	pub tags: Option<serde_json::Value>,
	pub body: serde_json::Value,
}

#[derive(Debug)]
pub struct MessageSendEvent {
	pub name: String,
	pub tags: serde_json::Value,
	pub body: serde_json::Value,
}

#[derive(Debug)]
pub struct SubWorkflowEvent {
	pub sub_workflow_id: Id,
	pub name: String,
	pub tags: serde_json::Value,
	pub input: serde_json::Value,
}

#[derive(Debug)]
pub struct LoopEvent {
	pub state: serde_json::Value,
	/// If the loop completes, this will be some.
	pub output: Option<serde_json::Value>,
	pub iteration: usize,
}

#[derive(Debug, Clone)]
pub struct ActivityError {
	pub error: String,
	pub count: usize,
	pub latest_ts: i64,
}

#[derive(Debug)]
pub struct SignalData {
	pub signal_id: Id,
	pub signal_name: String,
	pub tags: Option<serde_json::Value>,
	pub workflow_id: Option<Id>,
	pub create_ts: i64,
	pub ack_ts: Option<i64>,
	pub body: serde_json::Value,
	pub state: SignalState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignalState {
	Acked,
	Pending,
	Silenced,
}
