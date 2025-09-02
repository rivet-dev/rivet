use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use futures_util::stream::BoxStream;
use rivet_util::Id;
use serde::de::DeserializeOwned;

use crate::{
	error::{WorkflowError, WorkflowResult},
	history::{
		event::{Event, EventId, EventType, SleepState},
		location::Location,
	},
	workflow::Workflow,
};

pub mod debug;
mod kv;
pub use kv::DatabaseKv;

pub type DatabaseHandle = Arc<dyn Database + Sync>;

// TODO: Change tags type to &[(String, String)]
#[async_trait::async_trait]
pub trait Database: Send {
	/// Create a new DB instance.
	async fn from_pools(pools: rivet_pools::Pools) -> Result<Arc<Self>>
	where
		Self: Sized;

	// MARK: Const fns

	/// How often to pull workflows when polling. This runs alongside a wake sub.
	fn worker_poll_interval(&self) -> Duration {
		Duration::from_secs(90)
	}

	/// Poll interval when polling for signals in-process. This runs alongside a wake sub.
	fn signal_poll_interval(&self) -> Duration {
		Duration::from_millis(500)
	}

	/// Most in-process signal poll tries.
	fn max_signal_poll_retries(&self) -> usize {
		4
	}

	/// Poll interval when polling for a sub workflow in-process. This runs alongside a wake sub.
	fn sub_workflow_poll_interval(&self) -> Duration {
		Duration::from_millis(500)
	}

	/// Most in-process sub workflow poll tries.
	fn max_sub_workflow_poll_retries(&self) -> usize {
		4
	}

	// MARK: Worker fns

	/// This function returns a subscription which should resolve once the worker should fetch the database
	/// again.
	async fn wake_sub<'a, 'b>(&'a self) -> WorkflowResult<BoxStream<'b, ()>>;

	/// Updates the last ping ts for this worker.
	async fn update_worker_ping(&self, worker_instance_id: Id) -> WorkflowResult<()>;

	/// Releases workflows that were leased by workers that have since expired (their last ping has passed
	/// the expired threshold), making them eligible to be run again. Called periodically.
	async fn clear_expired_leases(&self, worker_instance_id: Id) -> WorkflowResult<()>;

	/// Function to publish metrics. Called periodically.
	async fn publish_metrics(&self, worker_instance_id: Id) -> WorkflowResult<()>;

	// MARK: Workflows/signals

	/// Writes a new workflow to the database. If unique is set, this should return the existing workflow ID
	/// (if one exists) instead of the given workflow ID.
	async fn dispatch_workflow(
		&self,
		ray_id: Id,
		workflow_id: Id,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
		unique: bool,
	) -> WorkflowResult<Id>;

	/// Retrieves workflows with the given IDs.
	async fn get_workflows(&self, workflow_ids: Vec<Id>) -> WorkflowResult<Vec<WorkflowData>>;

	/// Retrieves the first incomplete workflow with the given name and tags.
	async fn find_workflow(
		&self,
		workflow_name: &str,
		tags: &serde_json::Value,
	) -> WorkflowResult<Option<Id>>;

	/// Pulls workflows for processing by the worker. Will only pull workflows with names matching the filter.
	/// Should also update the ping of this worker instance.
	async fn pull_workflows(
		&self,
		worker_instance_id: Id,
		filter: &[&str],
	) -> WorkflowResult<Vec<PulledWorkflowData>>;

	/// Mark a workflow as completed.
	async fn complete_workflow(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		output: &serde_json::value::RawValue,
	) -> WorkflowResult<()>;

	/// Write a workflow sleep/failure to the database.
	async fn commit_workflow(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		wake_immediate: bool,
		wake_deadline_ts: Option<i64>,
		wake_signals: &[&str],
		wake_sub_workflow_id: Option<Id>,
		error: &str,
	) -> WorkflowResult<()>;

	/// Pulls the oldest signal with the given filter.
	async fn pull_next_signal(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		filter: &[&str],
		location: &Location,
		version: usize,
		loop_location: Option<&Location>,
		last_try: bool,
	) -> WorkflowResult<Option<SignalData>>;

	/// Retrieves a workflow with the given ID. Can only be called from a workflow context.
	async fn get_sub_workflow(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		sub_workflow_id: Id,
	) -> WorkflowResult<Option<WorkflowData>>;

	/// Write a new signal to the database.
	async fn publish_signal(
		&self,
		ray_id: Id,
		workflow_id: Id,
		signal_id: Id,
		signal_name: &str,
		body: &serde_json::value::RawValue,
	) -> WorkflowResult<()>;

	/// Write a new signal to the database. Contains extra info used to populate the history.
	async fn publish_signal_from_workflow(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		ray_id: Id,
		workflow_id: Id,
		signal_id: Id,
		signal_name: &str,
		body: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Publish a new workflow from an existing workflow.
	async fn dispatch_sub_workflow(
		&self,
		ray_id: Id,
		workflow_id: Id,
		location: &Location,
		version: usize,
		sub_workflow_id: Id,
		sub_workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
		unique: bool,
	) -> WorkflowResult<Id>;

	/// Updates workflow tags.
	async fn update_workflow_tags(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		tags: &serde_json::Value,
	) -> WorkflowResult<()>;

	/// Updates workflow state.
	async fn update_workflow_state(
		&self,
		workflow_id: Id,
		state: &serde_json::value::RawValue,
	) -> WorkflowResult<()>;

	// MARK: History

	/// Write a workflow activity event to history.
	async fn commit_workflow_activity_event(
		&self,
		workflow_id: Id,
		location: &Location,
		version: usize,
		event_id: &EventId,
		create_ts: i64,
		input: &serde_json::value::RawValue,
		output: Result<&serde_json::value::RawValue, &str>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Writes a message send event to history.
	async fn commit_workflow_message_send_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		tags: &serde_json::Value,
		message_name: &str,
		body: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Updates a loop event in history and forgets all history items in the previous iteration.
	async fn upsert_workflow_loop_event(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		location: &Location,
		version: usize,
		iteration: usize,
		state: &serde_json::value::RawValue,
		output: Option<&serde_json::value::RawValue>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Writes a workflow sleep event to history.
	async fn commit_workflow_sleep_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		deadline_ts: i64,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Updates a workflow sleep event's state.
	async fn update_workflow_sleep_event_state(
		&self,
		from_workflow_id: Id,
		location: &Location,
		state: SleepState,
	) -> WorkflowResult<()>;

	/// Writes a workflow branch event to history.
	async fn commit_workflow_branch_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Writes a workflow branch event to history.
	async fn commit_workflow_removed_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		event_type: EventType,
		event_name: Option<&str>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;

	/// Writes a workflow version check event to history.
	async fn commit_workflow_version_check_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()>;
}

#[derive(Debug)]
pub struct WorkflowData {
	pub workflow_id: Id,
	input: Box<serde_json::value::RawValue>,
	state: Box<serde_json::value::RawValue>,
	output: Option<Box<serde_json::value::RawValue>>,
	pub has_wake_condition: bool,
}

impl WorkflowData {
	pub fn parse_input<W: Workflow>(&self) -> WorkflowResult<W::Input> {
		serde_json::from_str(self.input.get()).map_err(WorkflowError::DeserializeWorkflowInput)
	}

	pub fn parse_state<T: DeserializeOwned>(&self) -> WorkflowResult<T> {
		serde_json::from_str(self.state.get()).map_err(WorkflowError::DeserializeWorkflowState)
	}

	pub fn parse_output<W: Workflow>(&self) -> WorkflowResult<Option<W::Output>> {
		self.output
			.as_ref()
			.map(|x| serde_json::from_str(x.get()))
			.transpose()
			.map_err(WorkflowError::DeserializeWorkflowOutput)
	}
}

#[derive(Debug)]
pub struct PulledWorkflowData {
	pub workflow_id: Id,
	pub workflow_name: String,
	pub create_ts: i64,
	pub ray_id: Id,
	pub input: Box<serde_json::value::RawValue>,
	pub state: Box<serde_json::value::RawValue>,
	pub wake_deadline_ts: Option<i64>,

	pub events: HashMap<Location, Vec<Event>>,
}

pub struct SignalData {
	pub signal_id: Id,
	pub signal_name: String,
	pub body: Box<serde_json::value::RawValue>,
	pub create_ts: i64,
}
