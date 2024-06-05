use global_error::{GlobalError, GlobalResult};
use serde::Serialize;
use tokio::time::Duration;
use uuid::Uuid;

use crate::{
	util, DatabaseHandle, DatabasePostgres, Operation, OperationCtx, OperationInput, Signal,
	Workflow, WorkflowError, WorkflowInput,
};

pub struct TestCtx {
	name: String,
	ray_id: Uuid,
	ts: i64,

	db: DatabaseHandle,

	conn: Option<rivet_connection::Connection>,
}

impl TestCtx {
	pub async fn from_env(test_name: &str) -> TestCtx {
		let service_name = format!(
			"{}-test--{}",
			std::env::var("CHIRP_SERVICE_NAME").unwrap(),
			test_name
		);

		let ray_id = Uuid::new_v4();
		let pools = rivet_pools::from_env(service_name.clone())
			.await
			.expect("failed to create pools");
		let shared_client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("failed to create chirp client");
		let cache =
			rivet_cache::CacheInner::from_env(pools.clone()).expect("failed to create cache");
		let conn = util::new_conn(
			&shared_client,
			&pools,
			&cache,
			ray_id,
			Uuid::new_v4(),
			&service_name,
		);

		let db = DatabasePostgres::from_pool(pools.crdb().unwrap());

		TestCtx {
			name: service_name,
			ray_id,
			ts: rivet_util::timestamp::now(),
			db,
			conn: Some(conn),
		}
	}
}

impl TestCtx {
	pub fn name(&self) -> &str {
		&self.name
	}
}

impl TestCtx {
	pub async fn dispatch_workflow<I>(&self, input: I) -> GlobalResult<Uuid>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		let name = I::Workflow::name();

		tracing::debug!(%name, ?input, "dispatching workflow");

		let id = Uuid::new_v4();

		// Serialize input
		let input_val = serde_json::to_value(input)
			.map_err(WorkflowError::SerializeWorkflowOutput)
			.map_err(GlobalError::raw)?;

		self.db
			.dispatch_workflow(self.ray_id, id, &name, input_val)
			.await
			.map_err(GlobalError::raw)?;

		tracing::info!(%name, ?id, "workflow dispatched");

		Ok(id)
	}

	pub async fn wait_for_workflow<W: Workflow>(
		&self,
		workflow_id: Uuid,
	) -> GlobalResult<W::Output> {
		tracing::info!(name=W::name(), id=?workflow_id, "waiting for workflow");

		let period = Duration::from_millis(50);
		let mut interval = tokio::time::interval(period);
		loop {
			interval.tick().await;

			// Check if state finished
			let workflow = self
				.db
				.get_workflow(workflow_id)
				.await
				.map_err(GlobalError::raw)?
				.ok_or(WorkflowError::WorkflowNotFound)
				.map_err(GlobalError::raw)?;
			if let Some(output) = workflow.parse_output::<W>().map_err(GlobalError::raw)? {
				return Ok(output);
			}
		}
	}

	pub async fn workflow<I>(
		&self,
		input: I,
	) -> GlobalResult<<<I as WorkflowInput>::Workflow as Workflow>::Output>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		let workflow_id = self.dispatch_workflow(input).await?;
		let output = self.wait_for_workflow::<I::Workflow>(workflow_id).await?;
		Ok(output)
	}

	pub async fn signal<I: Signal + Serialize>(
		&self,
		workflow_id: Uuid,
		input: I,
	) -> GlobalResult<Uuid> {
		tracing::debug!(name=%I::name(), %workflow_id, "dispatching signal");

		let signal_id = Uuid::new_v4();

		// Serialize input
		let input_val = serde_json::to_value(input)
			.map_err(WorkflowError::SerializeSignalBody)
			.map_err(GlobalError::raw)?;

		self.db
			.publish_signal(self.ray_id, workflow_id, signal_id, I::name(), input_val)
			.await
			.map_err(GlobalError::raw)?;

		Ok(signal_id)
	}

	pub async fn op<I>(
		&mut self,
		input: I,
	) -> GlobalResult<<<I as OperationInput>::Operation as Operation>::Output>
	where
		I: OperationInput,
		<I as OperationInput>::Operation: Operation<Input = I>,
	{
		let mut ctx = OperationCtx::new(
			self.db.clone(),
			self.conn
				.as_ref()
				.expect("ops cannot be triggered from an internal test"),
			self.ray_id,
			self.ts,
			false,
			I::Operation::name(),
		);

		I::Operation::run(&mut ctx, &input)
			.await
			.map_err(WorkflowError::OperationFailure)
			.map_err(GlobalError::raw)
	}
}
