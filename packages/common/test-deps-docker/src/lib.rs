use anyhow::*;
use tokio::process::Command;

mod database;
mod pubsub;

pub use database::*;
pub use pubsub::*;

#[derive(Debug, Clone)]
pub struct DockerRunConfig {
	pub image: String,
	pub container_name: String,
	pub port_mapping: (u16, u16), // (host_port, container_port)
	pub env_vars: Vec<(String, String)>,
	container_id: Option<String>,
}

impl DockerRunConfig {
	pub async fn start(&mut self) -> Result<bool> {
		// If we already have a container ID from config, it's already running
		if self.container_id.is_some() {
			tracing::debug!(
				container_name = %self.container_name,
				container_id = ?self.container_id,
				"using existing container from config"
			);
			return Ok(false);
		}

		tracing::debug!(
			container_name = %self.container_name,
			image = %self.image,
			port_mapping = ?self.port_mapping,
			"starting new docker container"
		);

		let mut cmd = Command::new("docker");
		cmd.arg("run")
			.arg("-d")
			.arg("-p")
			.arg(format!("{}:{}", self.port_mapping.0, self.port_mapping.1))
			.arg("--name")
			.arg(&self.container_name);

		// Add environment variables
		for (key, value) in &self.env_vars {
			cmd.arg("-e").arg(format!("{}={}", key, value));
		}

		cmd.arg(&self.image);

		let output = cmd.output().await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			anyhow::bail!(
				"Failed to start container {}: {}",
				self.container_name,
				stderr
			);
		}

		let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
		self.container_id = Some(container_id.clone());

		tracing::debug!(
			container_name = %self.container_name,
			container_id = %container_id,
			"new container started successfully"
		);

		Ok(true)
	}

	pub fn container_id(&self) -> Option<&str> {
		self.container_id.as_deref()
	}
}

/// Check if a container with the given name exists and is running
pub async fn check_container_exists(container_name: &str) -> Result<Option<String>> {
	let output = Command::new("docker")
		.arg("ps")
		.arg("-a")
		.arg("--filter")
		.arg(format!("name=^{}$", container_name))
		.arg("--format")
		.arg("{{.ID}}\t{{.State}}")
		.output()
		.await?;

	if !output.status.success() {
		return Ok(None);
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let lines: Vec<&str> = stdout.trim().lines().collect();

	if lines.is_empty() {
		return Ok(None);
	}

	// Parse the first line (should only be one container with exact name match)
	if let Some(line) = lines.first() {
		let parts: Vec<&str> = line.split('\t').collect();
		if parts.len() >= 2 {
			let container_id = parts[0];
			let state = parts[1];

			if state == "running" {
				return Ok(Some(container_id.to_string()));
			} else {
				// Container exists but is not running, try to start it
				tracing::debug!(
					container_name = %container_name,
					container_id = %container_id,
					state = %state,
					"found stopped container, attempting to start"
				);

				let status = Command::new("docker")
					.arg("start")
					.arg(container_id)
					.status()
					.await?;

				if status.success() {
					return Ok(Some(container_id.to_string()));
				} else {
					// Failed to start, remove the old container
					tracing::debug!(
						container_name = %container_name,
						"failed to start existing container, removing it"
					);
					let _ = Command::new("docker")
						.arg("rm")
						.arg("-f")
						.arg(container_id)
						.status()
						.await;
				}
			}
		}
	}

	Ok(None)
}

/// Get the port mapping from an existing container
pub async fn get_container_port(container_name: &str) -> Result<Option<u16>> {
	let output = Command::new("docker")
		.arg("inspect")
		.arg(container_name)
		.arg("--format")
		.arg("{{range $p, $conf := .NetworkSettings.Ports}}{{(index $conf 0).HostPort}}{{end}}")
		.output()
		.await?;

	if !output.status.success() {
		return Ok(None);
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let port_str = stdout.trim();

	if port_str.is_empty() {
		return Ok(None);
	}

	port_str.parse::<u16>().map(Some).or(Ok(None))
}
