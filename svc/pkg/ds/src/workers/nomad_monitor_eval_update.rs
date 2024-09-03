use chirp_worker::prelude::*;
use rivet_operation::prelude::proto::backend::pkg::*;
use serde::Deserialize;

use crate::util::NEW_NOMAD_CONFIG;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PlanResult {}

#[derive(Debug, Copy, Clone)]
enum EvalStatus {
	Failed,
	Complete,
}

#[derive(Debug, sqlx::FromRow)]
struct ServerRow {
	server_id: Uuid,
	datacenter_id: Uuid,
	nomad_eval_plan_ts: Option<i64>,
}

#[worker(name = "ds-run-nomad-monitor-eval-update")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_eval_update::Message>,
) -> GlobalResult<()> {
	let payload_value = serde_json::from_str::<serde_json::Value>(&ctx.payload_json)?;

	// We can't decode this with serde, so manually deserialize the response
	let eval_value = unwrap!(payload_value.get("Evaluation"));

	let job_id = unwrap!(unwrap!(eval_value.get("JobID"), "eval has no job id").as_str());
	let eval_status_raw = unwrap!(unwrap!(eval_value.get("Status")).as_str());

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

	// TODO: Rewrite on workflows

	// // Fetch and update the run
	// let server_row = sql_fetch_optional!(
	// 	[ctx, ServerRow]
	// 	"
	// 	UPDATE db_ds.server_nomad
	// 	SET nomad_eval_plan_ts = $2
	// 	WHERE
	// 		nomad_dispatched_job_id = $1 AND
	// 		nomad_eval_plan_ts IS NULL
	// 	RETURNING server_id, datacenter_id, nomad_eval_plan_ts
	// 	",
	// 	job_id,
	// 	ctx.ts(),
	// )
	// .await?;

	// // Check if server found
	// let Some(server_row) = server_row else {
	// 	if ctx.req_dt() > util::duration::minutes(5) {
	// 		tracing::error!("discarding stale message");
	// 		return Ok(());
	// 	} else {
	// 		retry_bail!("server not found, may be race condition with insertion");
	// 	}
	// };
	// let server_id = server_row.server_id;

	// if let Some(eval_plan_ts) = server_row.nomad_eval_plan_ts {
	// 	tracing::info!(?eval_plan_ts, "eval already planned");
	// 	return Ok(());
	// }

	// tracing::info!(%job_id, %server_id, ?eval_status, "updated server");

	// match eval_status {
	// 	EvalStatus::Failed => {
	// 		tracing::info!(%server_id, "eval failed");

	// 		let error_code = job_run::msg::fail::ErrorCode::NomadEvalPlanFailed;
	// 		tracing::warn!(%server_id, ?error_code, "server fail");
	// 		msg!([ctx] job_run::msg::fail(server_id) {
	// 			run_id: Some(server_id.into()),
	// 			error_code: error_code as i32,
	// 		})
	// 		.await?;

	// 		// Get the region
	// 		let region_res = op!([ctx] region_get {
	// 			region_ids: vec![server_row.region_id.into()],
	// 		})
	// 		.await?;
	// 		let region = unwrap!(region_res.regions.first());

	// 		// Stop the job from attempting to server on another node. This will
	// 		// be called in job-run-stop too, but we want to catch this earlier.
	// 		match nomad_client::apis::jobs_api::delete_job(
	// 			&NEW_NOMAD_CONFIG,
	// 			job_id,
	// 			Some(&region.nomad_region),
	// 			None,
	// 			None,
	// 			None,
	// 			Some(false),
	// 			None,
	// 		)
	// 		.await
	// 		{
	// 			Ok(_) => tracing::info!("job stopped"),
	// 			Err(err) => {
	// 				tracing::warn!(?err, "error thrown while stopping job, probably a 404, will continue as if stopped normally");
	// 			}
	// 		}

	// 		// Cleanup the job
	// 		msg!([ctx] job_run::msg::stop(run_id) {
	// 			run_id: Some(run_id.into()),
	// 			..Default::default()
	// 		})
	// 		.await?;
	// 	}
	// 	EvalStatus::Complete => {
	// 		tracing::info!(%run_id, "eval complete");

	// 		msg!([ctx] job_run::msg::eval_complete(run_id) {
	// 			run_id: Some(run_id.into()),
	// 		})
	// 		.await?;
	// 	}
	// }

	Ok(())
}
