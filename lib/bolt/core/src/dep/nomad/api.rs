use anyhow::*;
use reqwest::Method;
use serde::Deserialize;

use crate::dep::nomad::NomadCtx;

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
