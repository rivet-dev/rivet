use anyhow::*;
use reqwest::Method;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
	context::ProjectContext,
	dep::nomad::{self, NomadCtx},
	utils,
};

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
