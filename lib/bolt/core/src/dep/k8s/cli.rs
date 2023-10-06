use anyhow::{ensure, Context, Result};
use tokio::io::AsyncWriteExt;

use crate::{context::ProjectContext, utils};

pub async fn apply_specs(ctx: &ProjectContext, specs: Vec<serde_json::Value>) -> Result<()> {
	let jobs = specs
		.iter()
		.filter_map(|spec| {
			(spec["kind"] == "Job").then(|| {
				(
					spec["metadata"]["name"].as_str().unwrap().to_string(),
					spec["metadata"]["namespace"].as_str().unwrap().to_string(),
				)
			})
		})
		.collect::<Vec<_>>();

	// Delete previous jobs (must delete because pod specs are immutable)
	if !jobs.is_empty() {
		eprintln!();
		rivet_term::status::progress("Deleting jobs", "");

		let pb = utils::progress_bar(jobs.len());
		for (name, namespace) in jobs {
			pb.set_message(name.clone());

			let mut cmd = tokio::process::Command::new("kubectl");
			cmd.arg("delete")
				.arg("-n")
				.arg(namespace)
				.arg("--ignore-not-found=true")
				.arg(format!("Job/{name}"));
			cmd.env("KUBECONFIG", ctx.gen_kubeconfig_path());
			cmd.stdout(std::process::Stdio::null());

			let status = cmd.status().await?;
			ensure!(status.success(), "failed to delete job {name}");

			pb.inc(1);
		}

		pb.finish();

		eprintln!();
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
