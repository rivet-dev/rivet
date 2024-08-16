use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde::Deserialize;
use sqlx;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AllocationUpdated {
	allocation: nomad_client::models::Allocation,
}

#[derive(Debug, Copy, Clone)]
enum TaskState {
	Pending,
	Running,
	Dead,
}

#[worker(name = "ds-run-nomad-monitor-alloc-update")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_alloc_update::Message>,
) -> GlobalResult<()> {
	let _crdb = ctx.crdb().await?;

	let AllocationUpdated { allocation: alloc } = serde_json::from_str(&ctx.payload_json)?;
	let alloc_state_json = serde_json::to_value(&alloc)?;

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

	match main_task_state {
		TaskState::Pending => {
			let run_row = sql_fetch_optional!(
				[ctx, (Uuid,)]
				"
				UPDATE
					db_ds.server_nomad
				SET
					nomad_alloc_state = $2
				WHERE
					nomad_dispatched_job_id = $1 RETURNING server_id
				",
				job_id,
				&alloc_state_json,
			)
			.await?;

			if run_row.is_none() {
				if ctx.req_dt() > util::duration::minutes(5) {
					tracing::error!("discarding stale message");
					return Ok(());
				} else {
					retry_bail!("run not found, may be race condition with insertion");
				}
			};

			crate::workers::webhook_call(ctx, alloc_id.to_string()).await?;

			Ok(())
		}
		TaskState::Running => {
			let run_row = sql_fetch_optional!(
				[ctx, (Uuid, Option<i64>)]
				"
				WITH select_server AS (
					SELECT
						servers.server_id,
						servers.start_ts
					FROM
						db_ds.server_nomad
						INNER JOIN db_ds.servers ON servers.server_id = server_nomad.server_id
					WHERE
						nomad_dispatched_job_id = $1
				),
				_update_servers AS (
					UPDATE
						db_ds.servers
					SET
						start_ts = $2
					FROM
						select_server
					WHERE
						servers.server_id = select_server.server_id
						AND servers.start_ts IS NULL RETURNING 1
				),
				_update_server_nomad AS (
					UPDATE
						db_ds.server_nomad
					SET
						nomad_alloc_state = $3
					FROM
						select_server
					WHERE
						server_nomad.server_id = select_server.server_id RETURNING 1
				)
				SELECT
					*
				FROM
					select_server
				",
				job_id,
				ctx.ts(),
				&alloc_state_json,
			)
			.await?;

			let Some((run_id, start_ts)) = run_row else {
				if ctx.req_dt() > util::duration::minutes(5) {
					tracing::error!("discarding stale message");
					return Ok(());
				} else {
					retry_bail!("run not found, may be race condition with insertion");
				}
			};

			crate::workers::webhook_call(ctx, alloc_id.to_string()).await?;

			if start_ts.is_none() {
				tracing::info!("run started");

				msg!([ctx] job_run::msg::started(run_id) {
					run_id: Some(run_id.into()),
				})
				.await?;

				Ok(())
			} else {
				tracing::info!("run already started");

				Ok(())
			}
		}
		TaskState::Dead => {
			let run_row = sql_fetch_optional!(
				[ctx, (Uuid, Option<i64>)]
				r#"
				WITH select_server AS (
					SELECT
						servers.server_id,
						servers.finish_ts
					FROM
						db_ds.server_nomad
						INNER JOIN db_ds.servers ON servers.server_id = server_nomad.server_id
					WHERE
						nomad_dispatched_job_id = $1
				),
				_update_servers AS (
					UPDATE
						db_ds.servers
					SET
						-- If the job stops immediately, the task state will never be "running" so we need to
						-- make sure start_ts is set here as well
						start_ts = COALESCE(start_ts, $2),
						finish_ts = $2
					FROM
						select_server
					WHERE
						servers.server_id = select_server.server_id
						AND servers.finish_ts IS NULL RETURNING 1
				),
				_update_server_nomad AS (
					UPDATE
						db_ds.server_nomad
					SET
						nomad_alloc_state = $3
					FROM
						select_server
					WHERE
						server_nomad.server_id = select_server.server_id RETURNING 1
				)
				SELECT
					*
				FROM
					select_server
				"#,
				job_id,
				ctx.ts(),
				&alloc_state_json,
			)
			.await?;

			let Some((run_id, finish_ts)) = run_row else {
				if ctx.req_dt() > util::duration::minutes(5) {
					tracing::error!("discarding stale message");
					return Ok(());
				} else {
					retry_bail!("run not found, may be race condition with insertion");
				}
			};

			crate::workers::webhook_call(ctx, alloc_id.to_string()).await?;

			if finish_ts.is_none() {
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

				Ok(())
			} else {
				tracing::info!("run already finished");
				Ok(())
			}
		}
	}
}
