use anyhow::*;
use reqwest::Method;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
	context::ProjectContext,
	dep::nomad::{self, job_schema, NomadCtx},
	utils,
};

pub async fn job_run(
	ctx: &ProjectContext,
	nomad_ctx: &NomadCtx,
	job: &job_schema::Job,
) -> Result<()> {
	let job_id = job.id.as_ref().context("missing job id")?;

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "PascalCase")]
	struct PlanRes {
		job_modify_index: u64,
		diff: PlanResDiff,
	}

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "PascalCase")]
	struct PlanResDiff {
		#[serde(rename = "Type")]
		ty: PlanDiffType,
	}

	#[derive(Debug, PartialEq, Deserialize)]
	#[serde(rename_all = "PascalCase")]
	enum PlanDiffType {
		Added,
		Deleted,
		Edited,
		None,
	}

	// Plan the job
	// println!("  * {}", serde_json::to_string(&job).unwrap());
	let plan_res = nomad_ctx
		.build_nomad_request(Method::POST, format!("/v1/job/{job_id}/plan"))
		.json(&serde_json::json!({
			"Job": job,
			"Diff": true,
		}))
		.send()
		.await?;
	ensure!(
		plan_res.status().is_success(),
		"failed to plan job ({}):\n{}",
		plan_res.status(),
		plan_res.text().await?,
	);
	let plan_res_body = plan_res.json::<PlanRes>().await?;
	// println!("  * {} @ plan: {:?}", job_id, &plan_res_body);

	if plan_res_body.diff.ty != PlanDiffType::None {
		job_run_with_modify_idx(ctx, nomad_ctx, &job, plan_res_body.job_modify_index).await?;
	} else {
		// println!("  * {} @ nothing to update", job_id);
	}

	Ok(())
}

pub async fn job_run_with_modify_idx(
	_ctx: &ProjectContext,
	nomad_ctx: &NomadCtx,
	job: &job_schema::Job,
	job_modify_index: u64,
) -> Result<()> {
	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "PascalCase")]
	struct RunRes {
		#[serde(rename = "EvalID")]
		eval_id: String,
	}

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "PascalCase")]
	struct EvalRes {
		status: String,
		#[serde(rename = "DeploymentID")]
		deployment_id: Option<String>,
	}

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "PascalCase")]
	struct DeploymentRes {
		status: String,
	}

	let job_id = job.id.as_ref().context("missing job id")?;

	// Run the job
	let run_res = nomad_ctx
		.build_nomad_request(Method::POST, format!("/v1/job/{job_id}", job_id = job_id))
		.json(&serde_json::json!({
			"Job": job,
			"JobModifyIndex": job_modify_index,
		}))
		.send()
		.await?;
	ensure!(
		run_res.status().is_success(),
		"failed to run job ({}):\n{}",
		run_res.status(),
		run_res.text().await?,
	);
	let run_res_body = run_res.json::<RunRes>().await?;
	// println!("  * {} @ run: {:?}", job_id, run_res_body);
	// println!("  * {} @ deploying", job_id);

	if job.job_type != Some("batch".to_owned()) {
		// Poll the evaluation until a deployment ID is associated with it
		let mut blocking_idx = Option::<String>::None;
		let deployment_id = loop {
			let mut eval_req = nomad_ctx.build_nomad_request(
				Method::GET,
				format!("/v1/evaluation/{eval_id}", eval_id = run_res_body.eval_id),
			);
			if let Some(blocking_idx) = &blocking_idx {
				eval_req = eval_req.query(&[("index", blocking_idx.as_str()), ("wait", "1m")]);
			}
			let eval_res = eval_req.send().await?;
			ensure!(
				eval_res.status().is_success(),
				"failed eval status ({}):\n{}",
				eval_res.status(),
				eval_res.text().await?,
			);
			blocking_idx = Some(
				eval_res
					.headers()
					.get("X-Nomad-Index")
					.context("missing index header")?
					.to_str()?
					.into(),
			);
			let eval_res_body = eval_res.json::<EvalRes>().await?;
			// println!("  * {} @ eval: {:?}", job_id, eval_res_body);

			// Return the deployment ID
			match eval_res_body.status.as_str() {
				"complete" => {
					if let Some(deployment_id) = eval_res_body.deployment_id {
						break deployment_id;
					} else {
						// No deployment for this evaluation
						return Ok(());
					}
				}
				"pending" => {}
				_ => bail!("unknown eval status {}", eval_res_body.status),
			}
		};

		// Poll the evaluation state to wait for the deploy to finish. This means that the health
		// checks need to pass once before finishing. See `crate::dep::nomad::gen::gen_svc`.
		let mut blocking_idx = Option::<String>::None;
		loop {
			let mut deployment_req = nomad_ctx
				.build_nomad_request(Method::GET, format!("/v1/deployment/{deployment_id}"));
			if let Some(blocking_idx) = &blocking_idx {
				deployment_req =
					deployment_req.query(&[("index", blocking_idx.as_str()), ("wait", "1m")]);
			}
			let deployment_res = deployment_req.send().await?;
			ensure!(
				deployment_res.status().is_success(),
				"failed deployment status ({}):\n{}",
				deployment_res.status(),
				deployment_res.text().await?,
			);
			blocking_idx = Some(
				deployment_res
					.headers()
					.get("X-Nomad-Index")
					.context("missing index header")?
					.to_str()?
					.into(),
			);
			let deployment_res_body = deployment_res.json::<DeploymentRes>().await?;
			// println!("  * {} @ deploy: {:?}", job_id, deployment_res_body);

			match deployment_res_body.status.as_str() {
				"successful" => {
					// println!("  * {} @ deploy finished", job_id);
					break;
				}
				"cancelled" | "failed" => {
					bail!("deployment of {} did not succeed", job_id);
				}
				"running" => {
					// Do nothing
				}
				status @ _ => {
					bail!("unknown status {}", status);
				}
			}
		}
	} else {
		// println!("  * found periodic job, nothing to poll");
	}

	Ok(())
}

pub async fn job_run_parallel(
	ctx: &ProjectContext,
	nomad_ctx: &NomadCtx,
	jobs: Vec<job_schema::Job>,
) -> Result<()> {
	// Run Nomad jobs in batches of workers
	let mut join_handles = Vec::new();
	let pb = utils::MultiProgress::new(jobs.len());
	let jobs = Arc::new(Mutex::new(jobs));
	let failed_runs = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
	for _ in 0..64 {
		let ctx = ctx.clone();
		let pb = pb.clone();
		let nomad_ctx = nomad_ctx.clone();
		let jobs = jobs.clone();
		let failed_runs = failed_runs.clone();

		let handle = tokio::spawn(async move {
			while let Some(job) = {
				let mut lock = jobs.lock().await;
				let val = lock.pop();
				drop(lock);
				val
			} {
				let job_id = job.id.clone().unwrap();

				// Run the job and handle error gracefully
				pb.insert(&job_id).await;
				match nomad::api::job_run(&ctx, &nomad_ctx, &job).await {
					Result::Ok(_) => {}
					Result::Err(err) => {
						failed_runs
							.lock()
							.await
							.push((job_id.clone(), format!("{}", err)));
					}
				}
				pb.remove(&job_id).await;
			}
		});

		join_handles.push(handle);
	}
	futures_util::future::try_join_all(join_handles).await?;
	pb.finish();

	// Check if all the services ran successfully
	let failed_runs = failed_runs.lock().await;
	if !failed_runs.is_empty() {
		let errs = failed_runs
			.iter()
			.map(|(job_id, err)| format!("  ! {}: {}", job_id, err))
			.collect::<Vec<String>>()
			.join("\n");
		println!("{}", errs);
		bail!("services failed to boot");
	}

	Ok(())
}

#[derive(Debug, Deserialize)]
pub struct JobSummary {
	#[serde(rename = "ID")]
	pub id: String,
}

pub async fn list_jobs_with_prefix(nomad_ctx: &NomadCtx, prefix: &str) -> Result<Vec<JobSummary>> {
	let jobs_res = nomad_ctx
		.build_nomad_request(Method::GET, format!("/v1/jobs?prefix={prefix}"))
		.send()
		.await?;
	ensure!(
		jobs_res.status().is_success(),
		"failed to fetch jobs ({}):\n{}",
		jobs_res.status(),
		jobs_res.text().await?,
	);
	let jobs = jobs_res.json::<Vec<JobSummary>>().await?;

	Ok(jobs)
}

/// Find jobs that are running that do not have a corresponding service
/// registered with Bolt. These should be stopped manually.
pub async fn list_dangling_jobs(
	ctx: &ProjectContext,
	nomad_ctx: &NomadCtx,
	nomad_region: &str,
) -> Result<Vec<String>> {
	// TODO: Check if the job's features are enabled in this region
	let jobs = list_jobs_with_prefix(nomad_ctx, "rivet-").await?;

	let mut dangling_svcs = Vec::new();
	for job in &jobs {
		// Skip dispatched jobs
		if job.id.contains("/dispatch-") || job.id.contains("/periodic-") {
			continue;
		}

		let is_dangling = if let Some((job_name, region)) = job.id.split_once(":") {
			if region != nomad_region {
				// Mismatching region
				true
			} else if let Some(svc_name) = job_name.strip_prefix("rivet-") {
				if ctx.service_with_name_opt(svc_name).await.is_some() {
					// Service exists
					false
				} else {
					// Service is orphaned
					true
				}
			} else {
				// Invalid service name
				true
			}
		} else {
			// Does not contain region slug
			true
		};

		if is_dangling {
			dangling_svcs.push(job.id.clone());
		}
	}

	Ok(dangling_svcs)
}

pub async fn stop_job(nomad_ctx: &NomadCtx, id: &str) -> Result<()> {
	let jobs_res = nomad_ctx
		.build_nomad_request(Method::DELETE, format!("/v1/job/{id}"))
		.send()
		.await?;
	ensure!(
		jobs_res.status().is_success(),
		"failed to stop job ({}):\n{}",
		jobs_res.status(),
		jobs_res.text().await?,
	);

	Ok(())
}
