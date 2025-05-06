use std::time::{Duration, Instant};

use futures_util::StreamExt;
use global_error::{GlobalError, GlobalResult};
use rivet_pools::prelude::*;
use uuid::Uuid;

use crate::utils::tags::AsTags;

/// Time to delay a workflow from retrying after an error
pub const RETRY_TIMEOUT_MS: usize = 1000;
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
pub async fn wait_for_workflow_output<W: Workflow>(
	db: &DatabaseHandle,
	workflow_id: Uuid,
) -> GlobalResult<W::Output> {
	tracing::debug!("waiting for workflow");

	let mut wake_sub = db.wake_sub().await?;
	let mut interval = tokio::time::interval(db.sub_workflow_poll_interval());

	// Skip first tick, we wait after the db call instead of before
	interval.tick().await;

	tokio::time::timeout(WORKFLOW_TIMEOUT, async {
		loop {
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
) -> GlobalResult<Option<Uuid>> {
	db.find_workflow(W::NAME, &tags.as_tags()?)
		.await
		.map_err(GlobalError::raw)
}

pub async fn op<I>(
	db: &DatabaseHandle,
	config: &rivet_config::Config,
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
	tracing::debug!(?input, "operation call");

	// Record metrics
	crate::metrics::OPERATION_PENDING
		.with_label_values(&[I::Operation::NAME])
		.inc();
	crate::metrics::OPERATION_TOTAL
		.with_label_values(&[I::Operation::NAME])
		.inc();

	let start_instant = Instant::now();

	let ctx = OperationCtx::new(
		db.clone(),
		config,
		conn,
		ray_id,
		req_ts,
		from_workflow,
		I::Operation::NAME,
	)
	.await
	.map_err(GlobalError::raw)?;

	let res = tokio::time::timeout(I::Operation::TIMEOUT, I::Operation::run(&ctx, &input))
		.await
		.map_err(|_| WorkflowError::OperationTimeout(0))
		.map(|res| {
			res.map_err(WorkflowError::OperationFailure)
				.map_err(GlobalError::raw)
		});

	// Record metrics
	{
		let error_code_str = match &res {
			Ok(Err(GlobalError::Internal { ty, .. })) => {
				let err_code_str = "__UNKNOWN__".to_string();
				crate::metrics::OPERATION_ERRORS
					.with_label_values(&[I::Operation::NAME, &err_code_str, ty])
					.inc();

				err_code_str
			}
			Ok(Err(GlobalError::BadRequest { code, .. })) => {
				crate::metrics::OPERATION_ERRORS
					.with_label_values(&[I::Operation::NAME, code, "bad_request"])
					.inc();

				code.clone()
			}
			Ok(_) => String::new(),
			Err(_) => "timeout".to_string(),
		};

		// Other request metrics
		let dt = start_instant.elapsed().as_secs_f64();
		crate::metrics::OPERATION_PENDING
			.with_label_values(&[I::Operation::NAME])
			.dec();
		crate::metrics::OPERATION_DURATION
			.with_label_values(&[I::Operation::NAME, error_code_str.as_str()])
			.observe(dt);
	}

	let res = res?;

	tracing::debug!(?res, "operation response");

	res
}

pub async fn sqlite_for_workflow(
	db: &DatabaseHandle,
	conn: &rivet_connection::Connection,
	workflow_id: Uuid,
	read_only: bool,
) -> GlobalResult<SqlitePool> {
	// Validate workflow exists
	db.get_workflow(workflow_id)
		.await
		.map_err(GlobalError::raw)?
		.ok_or(WorkflowError::WorkflowNotFound)
		.map_err(GlobalError::raw)?;

	conn.sqlite(crate::db::sqlite_db_name_data(workflow_id), read_only)
		.await
		.map_err(Into::into)
}
