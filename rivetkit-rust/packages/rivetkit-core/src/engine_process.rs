use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use reqwest::Url;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::error::EngineProcessError;

#[derive(Debug, Deserialize)]
struct EngineHealthResponse {
	status: Option<String>,
	runtime: Option<String>,
	version: Option<String>,
}

#[derive(Debug)]
pub(crate) struct EngineProcessManager {
	child: Child,
	stdout_task: Option<JoinHandle<()>>,
	stderr_task: Option<JoinHandle<()>>,
}

impl EngineProcessManager {
	pub(crate) async fn start(binary_path: &Path, endpoint: &str) -> Result<Self> {
		if !binary_path.exists() {
			return Err(EngineProcessError::BinaryNotFound {
				path: binary_path.display().to_string(),
			}
			.build());
		}

		let endpoint_url =
			Url::parse(endpoint).with_context(|| format!("parse engine endpoint `{endpoint}`"))?;
		let guard_host = endpoint_url
			.host_str()
			.ok_or_else(|| invalid_endpoint(endpoint, "missing host"))?
			.to_owned();
		let guard_port = endpoint_url
			.port_or_known_default()
			.ok_or_else(|| invalid_endpoint(endpoint, "missing port"))?;
		let api_peer_port = guard_port
			.checked_add(1)
			.ok_or_else(|| invalid_endpoint(endpoint, "port is too large"))?;
		let metrics_port = guard_port
			.checked_add(10)
			.ok_or_else(|| invalid_endpoint(endpoint, "port is too large"))?;
		let db_path = std::env::temp_dir()
			.join(format!("rivetkit-engine-{}", Uuid::new_v4()))
			.join("db");

		let mut command = Command::new(binary_path);
		command
			.arg("start")
			.env("RIVET__GUARD__HOST", &guard_host)
			.env("RIVET__GUARD__PORT", guard_port.to_string())
			.env("RIVET__API_PEER__HOST", &guard_host)
			.env("RIVET__API_PEER__PORT", api_peer_port.to_string())
			.env("RIVET__METRICS__HOST", &guard_host)
			.env("RIVET__METRICS__PORT", metrics_port.to_string())
			.env("RIVET__FILE_SYSTEM__PATH", &db_path)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped());

		let mut child = command
			.spawn()
			.with_context(|| format!("spawn engine binary `{}`", binary_path.display()))?;
		let pid = child
			.id()
			.ok_or_else(|| EngineProcessError::MissingPid.build())?;
		let stdout_task = spawn_engine_log_task(child.stdout.take(), "stdout");
		let stderr_task = spawn_engine_log_task(child.stderr.take(), "stderr");

		tracing::info!(
			pid,
			path = %binary_path.display(),
			endpoint = %endpoint,
			db_path = %db_path.display(),
			"spawned engine process"
		);

		let health_url = engine_health_url(endpoint);
		let health = match wait_for_engine_health(&health_url).await {
			Ok(health) => health,
			Err(error) => {
				let error = match child.try_wait() {
					Ok(Some(status)) => error.context(format!(
						"engine process exited before becoming healthy with status {status}"
					)),
					Ok(None) => error,
					Err(wait_error) => error.context(format!(
						"failed to inspect engine process status: {wait_error:#}"
					)),
				};
				let manager = Self {
					child,
					stdout_task,
					stderr_task,
				};
				if let Err(shutdown_error) = manager.shutdown().await {
					tracing::warn!(
						?shutdown_error,
						"failed to clean up unhealthy engine process"
					);
				}
				return Err(error);
			}
		};

		tracing::info!(
			pid,
			status = ?health.status,
			runtime = ?health.runtime,
			version = ?health.version,
			"engine process is healthy"
		);

		Ok(Self {
			child,
			stdout_task,
			stderr_task,
		})
	}

	pub(crate) async fn shutdown(mut self) -> Result<()> {
		terminate_engine_process(&mut self.child).await?;
		join_log_task(self.stdout_task.take()).await;
		join_log_task(self.stderr_task.take()).await;
		Ok(())
	}
}

fn engine_health_url(endpoint: &str) -> String {
	format!("{}/health", endpoint.trim_end_matches('/'))
}

fn spawn_engine_log_task<R>(reader: Option<R>, stream: &'static str) -> Option<JoinHandle<()>>
where
	R: AsyncRead + Unpin + Send + 'static,
{
	reader.map(|reader| {
		tokio::spawn(async move {
			let mut lines = BufReader::new(reader).lines();
			while let Ok(Some(line)) = lines.next_line().await {
				match stream {
					"stderr" => tracing::warn!(stream, line, "engine process output"),
					_ => tracing::info!(stream, line, "engine process output"),
				}
			}
		})
	})
}

async fn join_log_task(task: Option<JoinHandle<()>>) {
	let Some(task) = task else {
		return;
	};
	if let Err(error) = task.await {
		tracing::warn!(?error, "engine log task failed");
	}
}

async fn wait_for_engine_health(health_url: &str) -> Result<EngineHealthResponse> {
	const HEALTH_MAX_WAIT: Duration = Duration::from_secs(10);
	const HEALTH_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);
	const HEALTH_INITIAL_BACKOFF: Duration = Duration::from_millis(100);
	const HEALTH_MAX_BACKOFF: Duration = Duration::from_secs(1);

	let client = rivet_pools::reqwest::client()
		.await
		.context("build reqwest client for engine health check")?;
	let deadline = Instant::now() + HEALTH_MAX_WAIT;
	let mut attempt = 0u32;
	let mut backoff = HEALTH_INITIAL_BACKOFF;

	loop {
		attempt += 1;

		let last_error = match client
			.get(health_url)
			.timeout(HEALTH_REQUEST_TIMEOUT)
			.send()
			.await
		{
			Ok(response) if response.status().is_success() => {
				let health = response
					.json::<EngineHealthResponse>()
					.await
					.context("decode engine health response")?;
				return Ok(health);
			}
			Ok(response) => format!("unexpected status {}", response.status()),
			Err(error) => error.to_string(),
		};

		if Instant::now() >= deadline {
			return Err(EngineProcessError::HealthCheckFailed {
				attempts: attempt,
				reason: last_error,
			}
			.build());
		}

		tokio::time::sleep(backoff).await;
		backoff = std::cmp::min(backoff * 2, HEALTH_MAX_BACKOFF);
	}
}

async fn terminate_engine_process(child: &mut Child) -> Result<()> {
	const ENGINE_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

	let Some(pid) = child.id() else {
		return Ok(());
	};

	if let Some(status) = child.try_wait().context("check engine process status")? {
		tracing::info!(pid, ?status, "engine process already exited");
		return Ok(());
	}

	send_sigterm(child)?;
	tracing::info!(pid, "sent SIGTERM to engine process");

	match tokio::time::timeout(ENGINE_SHUTDOWN_TIMEOUT, child.wait()).await {
		Ok(wait_result) => {
			let status = wait_result.context("wait for engine process to exit")?;
			tracing::info!(pid, ?status, "engine process exited");
			Ok(())
		}
		Err(_) => {
			tracing::warn!(
				pid,
				"engine process did not exit after SIGTERM, forcing kill"
			);
			child
				.start_kill()
				.context("force kill engine process after SIGTERM timeout")?;
			let status = child
				.wait()
				.await
				.context("wait for forced engine process shutdown")?;
			tracing::warn!(pid, ?status, "engine process killed");
			Ok(())
		}
	}
}

fn send_sigterm(child: &mut Child) -> Result<()> {
	let pid = child
		.id()
		.ok_or_else(|| EngineProcessError::MissingPid.build())?;

	#[cfg(unix)]
	{
		signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
			.with_context(|| format!("send SIGTERM to engine process {pid}"))?;
	}

	#[cfg(not(unix))]
	{
		child
			.start_kill()
			.with_context(|| format!("terminate engine process {pid}"))?;
	}

	Ok(())
}

fn invalid_endpoint(endpoint: &str, reason: &str) -> anyhow::Error {
	EngineProcessError::InvalidEndpoint {
		endpoint: endpoint.to_owned(),
		reason: reason.to_owned(),
	}
	.build()
}
