use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AllocationUpdated {
	allocation: nomad_client_new::models::Allocation,
}

#[derive(Debug, Copy, Clone)]
enum TaskState {
	Pending,
	Running,
	Dead,
}

#[derive(Clone)]
struct RunData {
	job_id: String,
	alloc_id: String,
	alloc_state_json: String,
	main_task_state: TaskState,
}

#[derive(Debug, sqlx::FromRow)]
struct RunRow {
	run_id: Uuid,
	alloc_id: Option<String>,
	start_ts: Option<i64>,
	finish_ts: Option<i64>,
}

#[worker(name = "job-run-nomad-monitor-alloc-update")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_alloc_update::Message>,
) -> GlobalResult<()> {
	let AllocationUpdated { allocation: alloc } = serde_json::from_str(&ctx.payload_json)?;
	let alloc_state_json = serde_json::to_string(&alloc)?;

	let alloc_id = unwrap_ref!(alloc.ID);
	let eval_id = unwrap_ref!(alloc.eval_id, "alloc has no eval");
	let job_id = unwrap_ref!(alloc.job_id);
	let client_status = unwrap_ref!(alloc.client_status);
	let task_states = unwrap_ref!(alloc.task_states);

	if !util_job::is_nomad_job_run(job_id) {
		tracing::info!(%job_id, "disregarding event");
		return Ok(());
	}

	// Get the main task by finding the task that is not the run cleanup task
	let main_task = task_states
		.iter()
		.filter(|(k, _)| k.as_str() == util_job::RUN_MAIN_TASK_NAME)
		.map(|(_, v)| v)
		.next();
	let main_task = unwrap!(main_task, "could not find main task");
	let main_task_state_raw = unwrap_ref!(main_task.state);

	tracing::info!(
		?client_status,
		?alloc_id,
		?eval_id,
		?job_id,
		?main_task_state_raw,
		main_task_events = ?main_task.events,
		"alloc updated"
	);

	let main_task_state = match (main_task_state_raw.as_str(), client_status.as_str()) {
		("pending", _) => TaskState::Pending,
		("running", _) => TaskState::Running,
		("dead", _) | (_, "failed" | "lost") => TaskState::Dead,
		_ => {
			tracing::error!(?main_task_state_raw, ?client_status, "unknown task state");
			return Ok(());
		}
	};

	let run_data = RunData {
		job_id: job_id.clone(),
		alloc_id: alloc_id.clone(),
		alloc_state_json,
		main_task_state,
	};

	let run_found = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let run_data = run_data.clone();
		Box::pin(update_db(ctx, tx, run_data))
	})
	.await?;

	// Check if run found
	if !run_found {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_bail!("run not found, may be race condition with insertion");
		}
	};

	Ok(())
}

/// Returns false if the run could not be found.
#[tracing::instrument(skip_all)]
async fn update_db(
	ctx: OperationContext<nomad::msg::monitor_alloc_update::Message>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	run_data: RunData,
) -> GlobalResult<bool> {
	let run_row = sql_fetch_optional!(
		[ctx, RunRow, @tx tx]
		"
		SELECT r.run_id, rn.alloc_id, r.start_ts, r.finish_ts
		FROM db_job_state.run_meta_nomad AS rn
		INNER JOIN db_job_state.runs AS r
		ON r.run_id = rn.run_id
		WHERE dispatched_job_id = $1
		FOR UPDATE OF r, rn
		",
		&run_data.job_id,
	)
	.await?;

	// Check if run found
	let run_row = if let Some(run_row) = run_row {
		run_row
	} else {
		tracing::info!("caught race condition");
		return Ok(false);
	};

	if run_row
		.alloc_id
		.as_ref()
		.map(|id| id != &run_data.alloc_id)
		.unwrap_or_default()
	{
		tracing::warn!(existing_alloc_id=?run_row.alloc_id, new_alloc_id=%run_data.alloc_id, "alloc id does not match existing alloc id for job");

		return Ok(true);
	}

	let run_id = run_row.run_id;

	match run_data.main_task_state {
		TaskState::Pending => {
			tracing::info!("run pending");

			sql_execute!(
				[ctx, @tx tx]
				"
				UPDATE db_job_state.run_meta_nomad
				SET alloc_state = $2
				WHERE run_id = $1
				",
				run_id,
				&run_data.alloc_state_json,
			)
			.await?;
		}
		TaskState::Running => {
			if run_row.start_ts.is_none() {
				sql_execute!(
					[ctx, @tx tx]
					"
					WITH
						update_runs AS (
							UPDATE db_job_state.runs
							SET start_ts = $2
							WHERE run_id = $1
							RETURNING 1
						),
						update_run_meta_nomad AS (
							UPDATE db_job_state.run_meta_nomad
							SET alloc_state = $3
							WHERE run_id = $1
							RETURNING 1
						)
					SELECT 1
					",
					run_id,
					ctx.ts(),
					&run_data.alloc_state_json,
				)
				.await?;

				tracing::info!("run started");

				msg!([ctx] job_run::msg::started(run_id) {
					run_id: Some(run_id.into()),
				})
				.await?;
			} else {
				tracing::info!("run already started");
			}
		}
		TaskState::Dead => {
			if run_row.finish_ts.is_none() {
				sql_execute!(
					[ctx, @tx tx]
					r#"
					WITH
						update_runs AS (
							UPDATE db_job_state.runs
							SET
								-- If the job stops immediately, the task state will never be "running" so we need to
								-- make sure start_ts is set here as well
								start_ts = COALESCE(start_ts, $2),
								finish_ts = $2
							WHERE run_id = $1
							RETURNING 1
						),
						update_run_meta_nomad AS (
							UPDATE db_job_state.run_meta_nomad
							SET alloc_state = $3
							WHERE run_id = $1
							RETURNING 1
						)
					SELECT 1
					"#,
					run_id,
					ctx.ts(),
					&run_data.alloc_state_json,
				)
				.await?;

				tracing::info!("run finished");

				// Publish message
				//
				// It's fine if this is called multiple times. The operation is
				// idempotent and it's better to ensure the job gets cleaned up
				// rather than forgotten.
				msg!([ctx] job_run::msg::cleanup(run_id) {
					run_id: Some(run_id.into()),
					..Default::default()
				})
				.await?;
				msg!([ctx] job_run::msg::finished(run_id) {
					run_id: Some(run_id.into()),
				})
				.await?;
			} else {
				tracing::info!("run already finished");
			}
		}
	}

	Ok(true)
}
