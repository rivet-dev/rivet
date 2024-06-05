use std::sync::Arc;

use uuid::Uuid;

use crate::{schema::ActivityId, Workflow, WorkflowError, WorkflowResult};

mod postgres;
pub use postgres::DatabasePostgres;

pub type DatabaseHandle = Arc<dyn Database + Sync>;

#[async_trait::async_trait]
pub trait Database: Send {
	async fn dispatch_workflow(&self, id: Uuid, name: &str, input: &str) -> WorkflowResult<()>;
	async fn get_workflow(&self, id: Uuid) -> WorkflowResult<Option<WorkflowRow>>;
	async fn pull_workflows(&self, filter: &[&str]) -> WorkflowResult<Vec<PulledWorkflow>>;

	// When a workflow is completed
	async fn commit_workflow(&self, workflow_id: Uuid, output: &str) -> WorkflowResult<()>;
	// When a workflow fails
	async fn fail_workflow(
		&self,
		workflow_id: Uuid,
		wake_immediate: bool,
		wake_deadline_ts: Option<i64>,
		wake_signals: &[&str],
		wake_sub_workflow: Option<Uuid>,
	) -> WorkflowResult<()>;

	async fn commit_workflow_activity_event(
		&self,
		workflow_id: Uuid,
		location: &[usize],
		activity_id: &ActivityId,
		input: &str,
		output: Option<&str>,
	) -> WorkflowResult<()>;

	async fn publish_signal(
		&self,
		workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: &str,
	) -> WorkflowResult<()>;
	async fn pull_latest_signal(
		&self,
		workflow_id: Uuid,
		filter: &[&str],
		location: &[usize],
	) -> WorkflowResult<Option<SignalRow>>;

	async fn dispatch_sub_workflow(
		&self,
		workflow_id: Uuid,
		location: &[usize],
		sub_workflow_id: Uuid,
		sub_workflow_name: &str,
		input: &str,
	) -> WorkflowResult<()>;
}

#[derive(sqlx::FromRow)]
pub struct WorkflowRow {
	pub workflow_id: Uuid,
	pub input: String,
	pub output: Option<String>,
}

impl WorkflowRow {
	pub fn parse_output<W: Workflow>(&self) -> WorkflowResult<Option<W::Output>> {
		self.output
			.as_deref()
			.map(serde_json::from_str)
			.transpose()
			.map_err(WorkflowError::DeserializeWorkflowOutput)
	}
}

#[derive(sqlx::FromRow)]
pub struct PulledWorkflowRow {
	pub workflow_id: Uuid,
	pub workflow_name: String,
	pub input: String,
	pub wake_deadline_ts: Option<i64>,
}

#[derive(sqlx::FromRow)]
pub struct PulledWorkflow {
	pub workflow_id: Uuid,
	pub workflow_name: String,
	pub input: String,
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
	pub output: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct SignalEventRow {
	pub workflow_id: Uuid,
	pub location: Vec<i64>,
	pub signal_name: String,
	pub body: String,
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
	pub body: String,
}
