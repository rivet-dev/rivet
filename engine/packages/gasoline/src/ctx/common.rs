use std::time::{Duration, Instant};

use anyhow::Result;
use futures_util::StreamExt;
use rivet_metrics::KeyValue;
use rivet_util::Id;

/// Time to delay a workflow from retrying after an error
pub const RETRY_TIMEOUT_MS: usize = 1000;
pub const WORKFLOW_TIMEOUT: Duration = Duration::from_secs(60);

use crate::{
	ctx::OperationCtx,
	db::{DatabaseHandle, WorkflowData},
	error::WorkflowError,
	operation::{Operation, OperationInput},
	utils::tags::AsTags,
	workflow::Workflow,
};

/// Polls the database for the workflow.
/// 60 second timeout.
pub async fn wait_for_workflow_output<W: Workflow>(
	db: &DatabaseHandle,
	workflow_id: Id,
) -> Result<W::Output> {
	tracing::debug!(?workflow_id, "waiting for workflow");

	let mut wake_sub = db.wake_sub().await?;
	let mut interval = tokio::time::interval(db.sub_workflow_poll_interval());

	// Skip first tick, we wait after the db call instead of before
	interval.tick().await;

	tokio::time::timeout(WORKFLOW_TIMEOUT, async {
		loop {
			// Check if state finished
			let workflow = db
				.get_workflows(vec![workflow_id])
				.await?
				.into_iter()
				.next()
				.ok_or(WorkflowError::WorkflowNotFound)?;
			if let Some(output) = workflow.parse_output::<W>()? {
				return Ok(output);
			}

			// Poll and wait for a wake at the same time
			tokio::select! {
				_ = wake_sub.next() => {},
				_ = interval.tick() => {},
			}
		}
	})
	.await?
}

/// Finds the first incomplete workflow with the given tags.
pub async fn find_workflow<W: Workflow>(
	db: &DatabaseHandle,
	tags: impl AsTags,
) -> Result<Option<Id>> {
	db.find_workflow(W::NAME, &tags.as_tags()?)
		.await
		.map_err(Into::into)
}

/// Finds the first incomplete workflow with the given tags.
pub async fn get_workflows(
	db: &DatabaseHandle,
	workflow_ids: Vec<Id>,
) -> Result<Vec<WorkflowData>> {
	db.get_workflows(workflow_ids).await.map_err(Into::into)
}

pub async fn op<I>(
	db: &DatabaseHandle,
	config: &rivet_config::Config,
	pools: &rivet_pools::Pools,
	cache: &rivet_cache::Cache,
	ray_id: Id,
	from_workflow: bool,
	input: I,
) -> Result<<<I as OperationInput>::Operation as Operation>::Output>
where
	I: OperationInput,
	<I as OperationInput>::Operation: Operation<Input = I>,
{
	tracing::debug!(?input, "operation call");

	// Record metrics
	crate::metrics::OPERATION_PENDING
		.add(1, &[KeyValue::new("operation_name", I::Operation::NAME)]);
	crate::metrics::OPERATION_TOTAL.add(1, &[KeyValue::new("operation_name", I::Operation::NAME)]);

	let start_instant = Instant::now();

	let ctx = OperationCtx::new(
		db.clone(),
		config,
		pools,
		cache,
		ray_id,
		from_workflow,
		I::Operation::NAME,
	)?;

	let res = tokio::time::timeout(I::Operation::TIMEOUT, I::Operation::run(&ctx, &input))
		.await
		.map_err(|_| WorkflowError::OperationTimeout(0))
		.map(|res| res.map_err(WorkflowError::OperationFailure));

	// Record metrics
	{
		let error_code_str = match &res {
			Ok(Err(err)) => {
				let error_code_str = err.to_string();

				crate::metrics::OPERATION_ERRORS.add(
					1,
					&[
						KeyValue::new("operation_name", I::Operation::NAME),
						KeyValue::new("error", error_code_str.clone()),
					],
				);

				error_code_str
			}
			Ok(_) => String::new(),
			Err(_) => "timeout".to_string(),
		};

		// Other request metrics
		let dt = start_instant.elapsed().as_secs_f64();
		crate::metrics::OPERATION_PENDING
			.add(-1, &[KeyValue::new("operation_name", I::Operation::NAME)]);
		crate::metrics::OPERATION_DURATION.record(
			dt,
			&[
				KeyValue::new("operation_name", I::Operation::NAME),
				KeyValue::new("error", error_code_str.clone()),
			],
		);
	}

	let res = res?;

	tracing::debug!(?res, "operation response");

	res.map_err(Into::into)
}
