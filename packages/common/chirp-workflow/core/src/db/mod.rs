use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::stream::BoxStream;
use uuid::Uuid;

use crate::{
	error::{WorkflowError, WorkflowResult},
	history::{
		event::{Event, EventId, EventType, SleepState},
		location::Location,
	},
	workflow::Workflow,
};

mod crdb_nats;
pub mod debug;
pub use crdb_nats::DatabaseCrdbNats;
mod fdb_sqlite_nats;
pub use fdb_sqlite_nats::DatabaseFdbSqliteNats;

pub type DatabaseHandle = Arc<dyn Database + Sync>;

// TODO: Change tags type to &[(String, String)]. Requires a custom wrapper type for sqlx encoding so we
// dont have to clone strings a bunch
#[async_trait::async_trait]
pub trait Database: Send {
	/// Create a new DB instance.
	fn from_pools(pools: rivet_pools::Pools) -> Result<Arc<Self>, rivet_pools::Error>
	where
		Self: Sized;

	// ===== CONST FNS =====

	/// How often to pull workflows when polling. This runs alongside a wake sub.
	fn worker_poll_interval(&self) -> Duration {
		Duration::from_secs(120)
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

	// ===========

	/// This function returns a subscription which should resolve once the worker should fetch the database
	/// again.
	async fn wake_sub<'a, 'b>(&'a self) -> WorkflowResult<BoxStream<'b, ()>>;

	/// Updates the last ping ts for this worker.
	async fn update_worker_ping(&self, worker_instance_id: Uuid) -> WorkflowResult<()>;

	/// Releases workflows that were leased by workers that have since expired (their last ping has passed
	/// the expired threshold), making them eligible to be run again. Called periodically.
	async fn clear_expired_leases(&self, worker_instance_id: Uuid) -> WorkflowResult<()>;

	/// Function to publish metrics. Called periodically.
	async fn publish_metrics(&self, worker_instance_id: Uuid) -> WorkflowResult<()>;

	/// Writes a new workflow to the database. If unique is set, this should return the existing workflow ID
	/// (if one exists) instead of the given workflow ID.
	async fn dispatch_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
		unique: bool,
	) -> WorkflowResult<Uuid>;

	/// Retrieves a workflow with the given ID.
	async fn get_workflow(&self, workflow_id: Uuid) -> WorkflowResult<Option<WorkflowData>>;

	/// Retrieves the first incomplete workflow with the given name and tags.
	async fn find_workflow(
		&self,
		workflow_name: &str,
		tags: &serde_json::Value,
	) -> WorkflowResult<Option<Uuid>>;

	/// Pulls workflows for processing by the worker. Will only pull workflows with names matching the filter.
	/// Should also update the ping of this worker instance.
	async fn pull_workflows(
		&self,
		worker_instance_id: Uuid,
		filter: &[&str],
	) -> WorkflowResult<Vec<PulledWorkflow>>;

	/// Mark a workflow as completed.
	async fn complete_workflow(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		output: &serde_json::value::RawValue,
	) -> WorkflowResult<()>;

	/// Write a workflow sleep/failure to the database.
	async fn commit_workflow(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		wake_immediate: bool,
		wake_deadline_ts: Option<i64>,
		wake_signals: &[&str],
		wake_sub_workflow: Option<Uuid>,
		error: &str,
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

	/// Retrieves a workflow with the given ID. Can only be called from a workflow context.
	async fn get_sub_workflow(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		sub_workflow_id: Uuid,
	) -> WorkflowResult<Option<WorkflowData>>;

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
		unique: bool,
	) -> WorkflowResult<Uuid>;

	/// Updates workflow tags.
	async fn update_workflow_tags(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
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
	async fn upsert_workflow_loop_event(
		&self,
		workflow_id: Uuid,
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
		version: usize,
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

#[derive(Debug)]
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

/// Database name for the local SQLite database for a workflow.
pub fn sqlite_db_name_data(workflow_id: Uuid) -> (&'static str, Uuid, &'static str) {
	("workflow", workflow_id, "data")
}
