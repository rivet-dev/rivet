use chirp_workflow::prelude::*;

#[derive(Debug, Serialize, Deserialize, Hash, Copy, Clone)]
enum TaskState {
	Pending,
	Running,
	Dead,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
	pub alloc: nomad_client::models::Allocation,
}

#[workflow]
pub async fn ds_server_nomad_alloc_update(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> GlobalResult<bool> {
	let alloc_id = unwrap_ref!(input.alloc.ID);
	let eval_id = unwrap_ref!(input.alloc.eval_id, "alloc has no eval");
	let job_id = unwrap_ref!(input.alloc.job_id);
	let client_status = unwrap_ref!(input.alloc.client_status);
	let task_states = unwrap_ref!(input.alloc.task_states);

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
			return Ok(false);
		}
	};

	ctx.activity(UpdateDbInput {
		server_id: input.server_id,
		alloc_id: alloc_id.clone(),
		alloc_state_json: serde_json::to_string(&input.alloc)?,
		main_task_state,
	})
	.await?;

	let finished = matches!(main_task_state, TaskState::Dead);

	Ok(finished)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	server_id: Uuid,
	alloc_id: String,
	alloc_state_json: String,
	main_task_state: TaskState,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<()> {
	let (nomad_alloc_id,) = sql_fetch_one!(
		[ctx, (Option<String>,)]
		r#"
		WITH
			select_server AS (
				SELECT sn.nomad_alloc_id, sn.nomad_alloc_id = $2 AS same_alloc
				FROM db_ds.server_nomad AS sn
				INNER JOIN db_ds.servers AS s
				ON s.server_id = sn.server_id
				WHERE s.server_id = $1
			),
			update_server_nomad AS (
				UPDATE db_ds.server_nomad
				SET nomad_alloc_state = $3
				WHERE
					server_id = $1 AND
					(SELECT same_alloc FROM select_server)
				RETURNING 1
			),
			set_start_finish_ts AS (
				UPDATE db_ds.servers
				-- Have to do this ugly case statement because you cannot update the same table with two
				-- UPDATE statements in the same CTE
				SET
					start_ts = CASE
						WHEN $5 AND start_ts IS NULL
						THEN $4
						WHEN $6 AND finish_ts IS NULL
						THEN COALESCE(start_ts, $4)
						ELSE start_ts
					END,
					finish_ts = CASE
						WHEN $6 AND finish_ts IS NULL
						THEN $4
						ELSE finish_ts
					END
				WHERE
					server_id = $1 AND
					(SELECT same_alloc FROM select_server)
				RETURNING 1
			)
		SELECT nomad_alloc_id FROM select_server
		"#,
		input.server_id,
		&input.alloc_id,
		&input.alloc_state_json,
		util::timestamp::now(),
		matches!(input.main_task_state, TaskState::Running),
		matches!(input.main_task_state, TaskState::Dead),
	)
	.await?;

	if nomad_alloc_id
		.as_ref()
		.map(|id| id != &input.alloc_id)
		.unwrap_or_default()
	{
		tracing::warn!(server_id=%input.server_id, existing_alloc_id=?nomad_alloc_id, new_alloc_id=%input.alloc_id, "different allocation id given, not updating");
	}

	Ok(())
}
