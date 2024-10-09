use std::{collections::HashMap, sync::Arc};

use uuid::Uuid;

use crate::{
	error::{WorkflowError, WorkflowResult},
	history::{
		event::{Event, EventId, EventType, SleepState},
		location::Location,
	},
	workflow::Workflow,
};

mod pg_nats;
pub use pg_nats::DatabasePgNats;

pub type DatabaseHandle = Arc<dyn Database + Sync>;

// TODO: Make this use generics for input types instead of using serde_json values. Ser/de should be handled
// manually in the driver.
#[async_trait::async_trait]
pub trait Database: Send {
	/// When using a wake worker instead of a polling worker, this function will return once the worker
	/// should fetch the database again.
	async fn wake(&self) -> WorkflowResult<()> {
		unimplemented!(
			"{} does not implement Database::wake",
			std::any::type_name::<Self>(),
		);
	}

	/// Writes a new workflow to the database.
	async fn dispatch_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
	) -> WorkflowResult<()>;
	async fn get_workflow(&self, id: Uuid) -> WorkflowResult<Option<WorkflowData>>;

	/// Pulls workflows for processing by the worker. Will only pull workflows with names matching the filter.
	async fn pull_workflows(
		&self,
		worker_instance_id: Uuid,
		filter: &[&str],
	) -> WorkflowResult<Vec<PulledWorkflow>>;

	/// Mark a workflow as completed.
	async fn commit_workflow(
		&self,
		workflow_id: Uuid,
		output: &serde_json::value::RawValue,
	) -> WorkflowResult<()>;

	/// Write a workflow failure to the database.
	async fn fail_workflow(
		&self,
		workflow_id: Uuid,
		wake_immediate: bool,
		wake_deadline_ts: Option<i64>,
		wake_signals: &[&str],
		wake_sub_workflow: Option<Uuid>,
		error: &str,
	) -> WorkflowResult<()>;

	/// Updates workflow tags.
	async fn update_workflow_tags(
		&self,
		workflow_id: Uuid,
		tags: &serde_json::Value,
	) -> WorkflowResult<()>;

	/// Write a workflow activity event to history.
	async fn commit_workflow_activity_event(
		&self,
		workflow_id: Uuid,
		location: &Location,
		version: usize,
		event_id: &EventId,
		create_ts: i64,
		input: &serde_json::value::RawValue,
		output: Result<&serde_json::value::RawValue, &str>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Pulls the oldest signal with the given filter. Pulls from regular and tagged signals.
	async fn pull_next_signal(
		&self,
		workflow_id: Uuid,
		filter: &[&str],
		location: &Location,
		version: usize,
		loop_location: Option<&Location>,
	) -> WorkflowResult<Option<SignalData>>;

	/// Write a new signal to the database.
	async fn publish_signal(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
	) -> WorkflowResult<()>;

	/// Write a new tagged signal to the database.
	async fn publish_tagged_signal(
		&self,
		ray_id: Uuid,
		tags: &serde_json::Value,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
	) -> WorkflowResult<()>;

	/// Write a new signal to the database. Contains extra info used to populate the history.
	async fn publish_signal_from_workflow(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		ray_id: Uuid,
		workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Write a new tagged signal to the database. Contains extra info used to populate the history.
	async fn publish_tagged_signal_from_workflow(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		ray_id: Uuid,
		tags: &serde_json::Value,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Publish a new workflow from an existing workflow.
	async fn dispatch_sub_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		location: &Location,
		version: usize,
		sub_workflow_id: Uuid,
		sub_workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Fetches a workflow that has the given json as a subset of its input after the given ts. Used primarily
	/// in tests.
	async fn poll_workflow(
		&self,
		name: &str,
		input: &serde_json::value::RawValue,
		after_ts: i64,
	) -> WorkflowResult<Option<(Uuid, i64)>>;

	/// Writes a message send event to history.
	async fn commit_workflow_message_send_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		tags: &serde_json::Value,
		message_name: &str,
		body: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Updates a loop event in history and forgets all history items in the previous iteration.
	async fn upsert_loop(
		&self,
		workflow_id: Uuid,
		location: &Location,
		version: usize,
		iteration: usize,
		output: Option<&serde_json::value::RawValue>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Writes a workflow sleep event to history.
	async fn commit_workflow_sleep_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		deadline_ts: i64,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Updates a workflow sleep event's state.
	async fn update_workflow_sleep_event_state(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		state: SleepState,
	) -> WorkflowResult<()>;

	/// Writes a workflow branch event to history.
	async fn commit_workflow_branch_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Writes a workflow branch event to history.
	async fn commit_workflow_removed_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		event_type: EventType,
		event_name: Option<&str>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Writes a workflow version check event to history.
	async fn commit_workflow_version_check_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;
}

pub struct WorkflowData {
	pub workflow_id: Uuid,
	pub input: Box<serde_json::value::RawValue>,
	pub output: Option<Box<serde_json::value::RawValue>>,
}

impl WorkflowData {
	pub fn parse_output<W: Workflow>(self) -> WorkflowResult<Option<W::Output>> {
		self.output
			.map(|x| serde_json::from_str(x.get()))
			.transpose()
			.map_err(WorkflowError::DeserializeWorkflowOutput)
	}
}

pub struct PulledWorkflow {
	pub workflow_id: Uuid,
	pub workflow_name: String,
	pub create_ts: i64,
	pub ray_id: Uuid,
	pub input: Box<serde_json::value::RawValue>,
	pub wake_deadline_ts: Option<i64>,

	pub events: HashMap<Location, Vec<Event>>,
}

pub struct SignalData {
	pub signal_id: Uuid,
	pub signal_name: String,
	pub body: Box<serde_json::value::RawValue>,
	pub create_ts: i64,
}
