use chirp_workflow::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
	pub eval: nomad_client::models::Evaluation,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum EvalStatus {
	Failed,
	Complete,
	Unknown,
}

#[workflow]
pub async fn ds_server_nomad_eval_update(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> GlobalResult<EvalStatus> {
	let eval_status = unwrap_ref!(input.eval.status).as_str();

	// Filter out data we need from the event. Ignore events we don't care about
	// before we touch the database.
	let failed_tg_allocs = &input.eval.failed_tg_allocs;
	let eval_status = match (eval_status, &failed_tg_allocs) {
		("complete", Some(failed_tg_allocs)) if !failed_tg_allocs.is_empty() => {
			tracing::warn!(server_id=%input.server_id, ?failed_tg_allocs, "eval failed");

			EvalStatus::Failed
		}
		("complete", _) => EvalStatus::Complete,
		_ => {
			tracing::info!(
				server_id=%input.server_id,
				?eval_status,
				?failed_tg_allocs,
				"ignoring status"
			);
			return Ok(EvalStatus::Unknown);
		}
	};

	ctx.activity(UpdateDbInput {
		server_id: input.server_id,
	})
	.await?;

	Ok(eval_status)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	server_id: Uuid,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_ds.server_nomad
		SET nomad_eval_plan_ts = $2
		WHERE
			server_id = $1 AND
			nomad_eval_plan_ts IS NULL
		",
		input.server_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}
