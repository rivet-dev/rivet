use std::time::Duration;

use global_error::{GlobalError, GlobalResult};
use uuid::Uuid;

/// Poll interval when polling for a sub workflow in-process
pub const SUB_WORKFLOW_RETRY: Duration = Duration::from_millis(150);
/// Time to delay a workflow from retrying after an error
pub const RETRY_TIMEOUT_MS: usize = 2000;
pub const WORKFLOW_TIMEOUT: Duration = Duration::from_secs(60);

use crate::{
	ctx::OperationCtx,
	db::DatabaseHandle,
	error::WorkflowError,
	operation::{Operation, OperationInput},
	workflow::Workflow,
};

/// Polls the database for the workflow.
/// 60 second timeout.
pub async fn wait_for_workflow<W: Workflow>(
	db: &DatabaseHandle,
	workflow_id: Uuid,
) -> GlobalResult<W::Output> {
	tracing::info!(workflow_name=%W::NAME, %workflow_id, "waiting for workflow");

	let mut interval = tokio::time::interval(SUB_WORKFLOW_RETRY);

	tokio::time::timeout(WORKFLOW_TIMEOUT, async {
		loop {
			interval.tick().await;

			// Check if state finished
			let workflow = db
				.get_workflow(workflow_id)
				.await
				.map_err(GlobalError::raw)?
				.ok_or(WorkflowError::WorkflowNotFound)
				.map_err(GlobalError::raw)?;
			if let Some(output) = workflow.parse_output::<W>().map_err(GlobalError::raw)? {
				return Ok(output);
			}
		}
	})
	.await?
}

pub async fn op<I>(
	db: &DatabaseHandle,
	conn: &rivet_connection::Connection,
	ray_id: Uuid,
	req_ts: i64,
	from_workflow: bool,
	input: I,
) -> GlobalResult<<<I as OperationInput>::Operation as Operation>::Output>
where
	I: OperationInput,
	<I as OperationInput>::Operation: Operation<Input = I>,
{
	tracing::info!(?input, "operation call");

	let ctx = OperationCtx::new(
		db.clone(),
		conn,
		ray_id,
		req_ts,
		from_workflow,
		I::Operation::NAME,
	);

	let res = tokio::time::timeout(I::Operation::TIMEOUT, I::Operation::run(&ctx, &input))
		.await
		.map_err(|_| WorkflowError::OperationTimeout(0))?
		.map_err(WorkflowError::OperationFailure)
		.map_err(GlobalError::raw);

	tracing::info!(?res, "operation response");

	res
}
