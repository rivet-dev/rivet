use std::collections::HashSet;

use anyhow::{ensure, Context, Result};
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{
	context::ProjectContext,
	utils::{self, command_helper::CommandHelper},
};

pub async fn apply_specs(ctx: &ProjectContext, specs: Vec<serde_json::Value>) -> Result<()> {
	// Handle job redeployment
	let has_job = specs.iter().any(|spec| spec["kind"] == "Job");
	let specs = if has_job {
		// Get all job names
		let output = Command::new("kubectl")
			.args(&[
				"get",
				"job",
				"-n",
				"rivet-service",
				"-o",
				"jsonpath={.items[*].metadata.name}",
			])
			.env("KUBECONFIG", ctx.gen_kubeconfig_path())
			.output()
			.await?;
		let output_str = String::from_utf8_lossy(&output.stdout);
		let running_jobs = output_str.trim().split(' ').collect::<Vec<_>>();

		// Filter out all jobs
		let (filtered_specs, job_specs) = specs
			.into_iter()
			.filter(|x| !x["metadata"].is_null())
			.partition::<Vec<_>, _>(|spec| {
				!running_jobs
					.iter()
					.any(|j| &spec["metadata"]["name"].as_str().unwrap() == j)
			});

		if !job_specs.is_empty() {
			let filtered_jobs = job_specs
				.into_iter()
				.map(|spec| spec["metadata"]["name"].as_str().unwrap().to_string())
				.collect::<HashSet<_>>()
				.into_iter()
				.collect::<Vec<_>>()
				.join(", ");

			rivet_term::status::progress("Skipping existing jobs", filtered_jobs);
		}

		filtered_specs
	} else {
		specs
	};

	if specs.is_empty() {
		return Ok(());
	}

	// Build YAML
	let mut full_yaml = String::new();
	for spec in &specs {
		full_yaml.push_str(&serde_yaml::to_string(spec)?);
		full_yaml.push_str("\n---\n");
	}

	// println!("{}", full_yaml);

	// Apply kubectl from stdin
	let mut cmd = tokio::process::Command::new("kubectl");
	cmd.args(&["apply", "--wait", "-f", "-"]);
	cmd.env("KUBECONFIG", ctx.gen_kubeconfig_path());
	cmd.stdin(std::process::Stdio::piped());
	cmd.stdout(std::process::Stdio::null());
	let mut child = cmd.spawn()?;

	{
		let mut stdin = child.stdin.take().context("missing stdin")?;
		stdin.write_all(full_yaml.as_bytes()).await?;
	}

	let status = child.wait().await?;
	ensure!(status.success(), "kubectl apply failed");

	Ok(())
}
