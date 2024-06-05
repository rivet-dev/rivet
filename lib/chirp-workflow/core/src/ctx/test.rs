use std::sync::Arc;

use serde::Serialize;
use tokio::time::Duration;
use uuid::Uuid;

use crate::{
	DatabaseHandle, DatabasePostgres, Signal, Workflow, WorkflowError, WorkflowInput,
	WorkflowResult,
};

pub type TestCtxHandle = Arc<TestCtx>;

pub struct TestCtx {
	name: String,
	pub db: DatabaseHandle,
}

impl TestCtx {
	pub fn new(db: DatabaseHandle) -> TestCtxHandle {
		Arc::new(TestCtx {
			name: "internal-test".to_string(),
			db,
		})
	}

	pub async fn from_env(test_name: &str) -> TestCtx {
		let service_name = format!(
			"{}-test--{}",
			std::env::var("CHIRP_SERVICE_NAME").unwrap(),
			test_name
		);
		let pools = rivet_pools::from_env(service_name.clone())
			.await
			.expect("failed to create pools");
		let db = DatabasePostgres::from_pool(pools.crdb().unwrap());

		TestCtx {
			name: service_name,
			db,
		}
	}
}

impl TestCtx {
	pub fn name(&self) -> &str {
		&self.name
	}
}

impl TestCtx {
	pub async fn dispatch_workflow<I>(&self, input: I) -> WorkflowResult<Uuid>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		let name = I::Workflow::name();

		tracing::debug!(%name, ?input, "dispatching workflow");

		let id = Uuid::new_v4();

		// Serialize input
		let input_str =
			serde_json::to_string(&input).map_err(WorkflowError::SerializeWorkflowOutput)?;

		self.db.dispatch_workflow(id, &name, &input_str).await?;

		tracing::info!(%name, ?id, "workflow dispatched");

		WorkflowResult::Ok(id)
	}

	pub async fn wait_for_workflow<W: Workflow>(
		&self,
		workflow_id: Uuid,
	) -> WorkflowResult<W::Output> {
		tracing::info!(name = W::name(), id = ?workflow_id, "waiting for workflow");

		let period = Duration::from_millis(50);
		let mut interval = tokio::time::interval(period);
		loop {
			interval.tick().await;

			// Check if state finished
			let workflow = self
				.db
				.get_workflow(workflow_id)
				.await?
				.ok_or(WorkflowError::WorkflowNotFound)?;
			if let Some(output) = workflow.parse_output::<W>()? {
				return WorkflowResult::Ok(output);
			}
		}
	}

	pub async fn workflow<I>(
		&self,
		input: I,
	) -> WorkflowResult<<<I as WorkflowInput>::Workflow as Workflow>::Output>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		let workflow_id = self.dispatch_workflow(input).await?;
		let output = self.wait_for_workflow::<I::Workflow>(workflow_id).await?;
		WorkflowResult::Ok(output)
	}

	pub async fn signal<I: Signal + Serialize>(
		&self,
		workflow_id: Uuid,
		input: I,
	) -> WorkflowResult<Uuid> {
		tracing::debug!(name=%I::name(), %workflow_id, "dispatching signal");

		let signal_id = Uuid::new_v4();

		// Serialize input
		let input_str =
			serde_json::to_string(&input).map_err(WorkflowError::SerializeSignalBody)?;

		self.db
			.publish_signal(workflow_id, signal_id, I::name(), &input_str)
			.await?;

		WorkflowResult::Ok(signal_id)
	}
}
