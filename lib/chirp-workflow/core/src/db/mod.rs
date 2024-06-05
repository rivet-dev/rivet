use std::sync::Arc;

use uuid::Uuid;

use crate::{schema::ActivityId, Workflow, WorkflowError, WorkflowResult};

mod postgres;
pub use postgres::DatabasePostgres;

pub type DatabaseHandle = Arc<dyn Database + Sync>;

#[async_trait::async_trait]
pub trait Database: Send {
	async fn dispatch_workflow(
		&self,
		ray_id: Uuid,
		id: Uuid,
		name: &str,
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

	async fn commit_workflow_activity_event(
		&self,
		workflow_id: Uuid,
		location: &[usize],
		activity_id: &ActivityId,
		input: serde_json::Value,
		output: Result<serde_json::Value, &str>,
	) -> WorkflowResult<()>;

	async fn publish_signal(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: serde_json::Value,
	) -> WorkflowResult<()>;
	async fn pull_latest_signal(
		&self,
		workflow_id: Uuid,
		filter: &[&str],
		location: &[usize],
	) -> WorkflowResult<Option<SignalRow>>;

	async fn dispatch_sub_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		location: &[usize],
		sub_workflow_id: Uuid,
		sub_workflow_name: &str,
		input: serde_json::Value,
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

	pub activity_events: Vec<ActivityEventRow>,
	pub signal_events: Vec<SignalEventRow>,
	pub sub_workflow_events: Vec<SubWorkflowEventRow>,
}

#[derive(sqlx::FromRow)]
pub struct ActivityEventRow {
	pub workflow_id: Uuid,
	pub location: Vec<i64>,
	pub activity_name: String,
	pub input_hash: Vec<u8>,
	pub output: Option<serde_json::Value>,
	pub error_count: i64,
}

#[derive(sqlx::FromRow)]
pub struct SignalEventRow {
	pub workflow_id: Uuid,
	pub location: Vec<i64>,
	pub signal_name: String,
	pub body: serde_json::Value,
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
}
