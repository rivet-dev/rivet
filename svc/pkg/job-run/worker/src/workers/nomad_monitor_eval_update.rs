use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde::Deserialize;

use crate::NEW_NOMAD_CONFIG;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PlanResult {
	evaluation: nomad_client_new::models::Evaluation,
}

#[derive(Debug, Copy, Clone)]
enum EvalStatus {
	Failed,
	Complete,
}

#[derive(Debug, sqlx::FromRow)]
struct RunRow {
	run_id: Uuid,
	region_id: Uuid,
	eval_plan_ts: Option<i64>,
}

#[worker(name = "job-run-nomad-monitor-eval-update")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_eval_update::Message>,
) -> GlobalResult<()> {
	let _crdb = ctx.crdb().await?;

	let payload_value = serde_json::from_str::<serde_json::Value>(&ctx.payload_json)?;
	let PlanResult { evaluation: eval } = serde_json::from_str::<PlanResult>(&ctx.payload_json)?;

	let job_id = unwrap_ref!(eval.job_id, "eval has no job id");
	let eval_status_raw = unwrap_ref!(eval.status).as_str();

	// We can't decode this with serde, so manually deserialize the response
	let eval_value = unwrap!(payload_value.get("Evaluation"));

	if !util_job::is_nomad_job_run(job_id) {
		tracing::info!(%job_id, "disregarding event");
		return Ok(());
	}

	// HACK: Serde isn't deserializing this correctly for some reason so
	// we use raw JSON
	// Filter out data we need from the event. Ignore events we don't care about
	// before we touch the database.
	let failed_tg_allocs = eval_value.get("FailedTGAllocs").and_then(|x| x.as_object());
	let eval_status = match (eval_status_raw, &failed_tg_allocs) {
		("complete", Some(failed_tg_allocs)) if !failed_tg_allocs.is_empty() => {
			let failed_tg_allocs_str =
				serde_json::to_string(&failed_tg_allocs).unwrap_or("?".to_string());
			tracing::warn!(%job_id, failed_tg_allocs = %failed_tg_allocs_str, "eval failed");

			EvalStatus::Failed
		}
		("complete", _) => EvalStatus::Complete,
		_ => {
			tracing::info!(
				%job_id,
				?eval_status_raw,
				?failed_tg_allocs,
				"ignoring status"
			);
			return Ok(());
		}
	};

	// Fetch and update the run
	let run_row = sql_fetch_optional!(
		[ctx, RunRow]
		"
		WITH
			select_run AS (
				SELECT runs.run_id, runs.region_id, run_meta_nomad.eval_plan_ts
				FROM db_job_state.run_meta_nomad
				INNER JOIN db_job_state.runs ON runs.run_id = run_meta_nomad.run_id
				WHERE dispatched_job_id = $1
			),
			_update AS (
				UPDATE db_job_state.run_meta_nomad
				SET eval_plan_ts = $2
				FROM select_run
				WHERE
					run_meta_nomad.run_id = select_run.run_id AND
					run_meta_nomad.eval_plan_ts IS NULL
				RETURNING 1
			)
		SELECT * FROM select_run
		",
		job_id,
		ctx.ts(),
	)
	.await?;

	// Check if run found
	let Some(run_row) = run_row else {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_bail!("run not found, may be race condition with insertion");
		}
	};
	let run_id = run_row.run_id;

	if let Some(eval_plan_ts) = run_row.eval_plan_ts {
		tracing::info!(?eval_plan_ts, "eval already planned");
		return Ok(());
	}

	tracing::info!(%job_id, %run_id, ?eval_status, "updated run");

	match eval_status {
		EvalStatus::Failed => {
			tracing::info!(%run_id, "eval failed");

			let error_code = job_run::msg::fail::ErrorCode::NomadEvalPlanFailed;
			tracing::warn!(%run_id, ?error_code, "job run fail");
			msg!([ctx] job_run::msg::fail(run_id) {
				run_id: Some(run_id.into()),
				error_code: error_code as i32,
			})
			.await?;

			// Get the region
			let region_res = op!([ctx] region_get {
				region_ids: vec![run_row.region_id.into()],
			})
			.await?;
			let region = unwrap!(region_res.regions.first());

			// Stop the job from attempting to run on another node. This will
			// be called in job-run-stop too, but we want to catch this earlier.
			match nomad_client_new::apis::jobs_api::delete_job(
				&NEW_NOMAD_CONFIG,
				job_id,
				None,
				Some(&region.nomad_region),
				None,
				None,
				Some(false),
				None,
			)
			.await
			{
				Ok(_) => tracing::info!("job stopped"),
				Err(err) => {
					tracing::warn!(?err, "error thrown while stopping job, probably a 404, will continue as if stopped normally");
				}
			}

			// Cleanup the job
			msg!([ctx] job_run::msg::stop(run_id) {
				run_id: Some(run_id.into()),
				..Default::default()
			})
			.await?;
		}
		EvalStatus::Complete => {
			tracing::info!(%run_id, "eval complete");

			msg!([ctx] job_run::msg::eval_complete(run_id) {
				run_id: Some(run_id.into()),
			})
			.await?;
		}
	}

	Ok(())
}
