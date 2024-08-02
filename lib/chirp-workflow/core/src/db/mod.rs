use std::{collections::HashMap, sync::Arc};

use uuid::Uuid;

use crate::{
	activity::ActivityId,
	error::{WorkflowError, WorkflowResult},
	event::Event,
	util::Location,
	workflow::Workflow,
};

mod postgres;
pub use postgres::DatabasePostgres;

pub type DatabaseHandle = Arc<dyn Database + Sync>;

#[async_trait::async_trait]
pub trait Database: Send {
	async fn dispatch_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: serde_json::Value,
	) -> WorkflowResult<()>;
	async fn get_workflow(&self, id: Uuid) -> WorkflowResult<Option<WorkflowRow>>;
	async fn pull_workflows(
		&self,
		worker_instance_id: Uuid,
		filter: &[&str],
	) -> WorkflowResult<Vec<PulledWorkflow>>;

	// When a workflow is completed
	async fn commit_workflow(
		&self,
		workflow_id: Uuid,
		output: &serde_json::Value,
	) -> WorkflowResult<()>;
	// When a workflow fails
	async fn fail_workflow(
		&self,
		workflow_id: Uuid,
		wake_immediate: bool,
		wake_deadline_ts: Option<i64>,
		wake_signals: &[&str],
		wake_sub_workflow: Option<Uuid>,
		error: &str,
	) -> WorkflowResult<()>;
	async fn update_workflow_tags(
		&self,
		workflow_id: Uuid,
		tags: &serde_json::Value,
	) -> WorkflowResult<()>;

	async fn commit_workflow_activity_event(
		&self,
		workflow_id: Uuid,
		location: &[usize],
		activity_id: &ActivityId,
		create_ts: i64,
		input: serde_json::Value,
		output: Result<serde_json::Value, &str>,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()>;

	async fn pull_next_signal(
		&self,
		workflow_id: Uuid,
		filter: &[&str],
		location: &[usize],
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<Option<SignalRow>>;
	async fn publish_signal(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: serde_json::Value,
	) -> WorkflowResult<()>;
	async fn publish_tagged_signal(
		&self,
		ray_id: Uuid,
		tags: &serde_json::Value,
		signal_id: Uuid,
		signal_name: &str,
		body: serde_json::Value,
	) -> WorkflowResult<()>;
	async fn publish_signal_from_workflow(
		&self,
		from_workflow_id: Uuid,
		location: &[usize],
		ray_id: Uuid,
		workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: serde_json::Value,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()>;
	async fn publish_tagged_signal_from_workflow(
		&self,
		from_workflow_id: Uuid,
		location: &[usize],
		ray_id: Uuid,
		tags: &serde_json::Value,
		signal_id: Uuid,
		signal_name: &str,
		body: serde_json::Value,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()>;

	async fn dispatch_sub_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		location: &[usize],
		sub_workflow_id: Uuid,
		sub_workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: serde_json::Value,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()>;

	/// Fetches a workflow that has the given json as a subset of its input after the given ts.
	async fn poll_workflow(
		&self,
		name: &str,
		input: &serde_json::Value,
		after_ts: i64,
	) -> WorkflowResult<Option<(Uuid, i64)>>;

	async fn publish_message_from_workflow(
		&self,
		from_workflow_id: Uuid,
		location: &[usize],
		tags: &serde_json::Value,
		message_name: &str,
		body: serde_json::Value,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()>;

	async fn update_loop(
		&self,
		workflow_id: Uuid,
		location: &[usize],
		iteration: usize,
		output: Option<serde_json::Value>,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()>;
}

#[derive(sqlx::FromRow)]
pub struct WorkflowRow {
	pub workflow_id: Uuid,
	pub input: serde_json::Value,
	pub output: Option<serde_json::Value>,
}

impl WorkflowRow {
	pub fn parse_output<W: Workflow>(self) -> WorkflowResult<Option<W::Output>> {
		self.output
			.map(serde_json::from_value)
			.transpose()
			.map_err(WorkflowError::DeserializeWorkflowOutput)
	}
}

#[derive(sqlx::FromRow)]
pub struct PulledWorkflowRow {
	pub workflow_id: Uuid,
	pub workflow_name: String,
	pub create_ts: i64,
	pub ray_id: Uuid,
	pub input: serde_json::Value,
	pub wake_deadline_ts: Option<i64>,
}

#[derive(sqlx::FromRow)]
pub struct PulledWorkflow {
	pub workflow_id: Uuid,
	pub workflow_name: String,
	pub create_ts: i64,
	pub ray_id: Uuid,
	pub input: serde_json::Value,
	pub wake_deadline_ts: Option<i64>,

	pub events: HashMap<Location, Vec<Event>>,
}

#[derive(sqlx::FromRow)]
pub struct ActivityEventRow {
	pub workflow_id: Uuid,
	pub location: Vec<i64>,
	pub activity_name: String,
	pub input_hash: Vec<u8>,
	pub output: Option<serde_json::Value>,
	pub error_count: i64,
	pub create_ts: i64,
}

#[derive(sqlx::FromRow)]
pub struct SignalEventRow {
	pub workflow_id: Uuid,
	pub location: Vec<i64>,
	pub signal_name: String,
	pub body: serde_json::Value,
}

#[derive(sqlx::FromRow)]
pub struct SignalSendEventRow {
	pub workflow_id: Uuid,
	pub location: Vec<i64>,
	pub signal_id: Uuid,
	pub signal_name: String,
}

#[derive(sqlx::FromRow)]
pub struct MessageSendEventRow {
	pub workflow_id: Uuid,
	pub location: Vec<i64>,
	pub message_name: String,
}

#[derive(sqlx::FromRow)]
pub struct SubWorkflowEventRow {
	pub workflow_id: Uuid,
	pub location: Vec<i64>,
	pub sub_workflow_id: Uuid,
	pub sub_workflow_name: String,
}

#[derive(sqlx::FromRow)]
pub struct SignalRow {
	pub signal_id: Uuid,
	pub signal_name: String,
	pub body: serde_json::Value,
	pub create_ts: i64,
}

#[derive(sqlx::FromRow)]
pub struct LoopEventRow {
	pub workflow_id: Uuid,
	pub location: Vec<i64>,
	pub output: Option<serde_json::Value>,
	pub iteration: i64,
}
