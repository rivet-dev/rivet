use anyhow::{ensure, Context, Result};
use tokio::io::AsyncWriteExt;

pub async fn apply_specs(specs: Vec<serde_json::Value>) -> Result<()> {
	// Build YAML
	let mut full_yaml = String::new();
	for spec in &specs {
		full_yaml.push_str(&serde_yaml::to_string(spec)?);
		full_yaml.push_str("\n---\n");
	}

	// println!("{}", full_yaml);

	// Apply kubectl from stdin
	let mut cmd = tokio::process::Command::new("kubectl");
	cmd.stdin(std::process::Stdio::piped());
	cmd.stdout(std::process::Stdio::null());
	cmd.args(&["apply", "-f", "-"]);
	let mut child = cmd.spawn()?;

	{
		let mut stdin = child.stdin.take().context("missing stdin")?;
		stdin.write_all(full_yaml.as_bytes()).await?;
	}

	let status = child.wait().await?;
	ensure!(status.success(), "kubectl apply failed");

	Ok(())
}
