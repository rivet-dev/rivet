#![allow(dead_code)]

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use rivetkit_core::{CoreRegistry, ServeConfig};
use tempfile::TempDir;
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const TOKEN: &str = "dev";
const DEFAULT_NAMESPACE: &str = "default";
const DEFAULT_POOL: &str = "default";

pub struct IntegrationCtx {
	temp_dir: TempDir,
	endpoint: String,
	child: Child,
	client: reqwest::Client,
	stdout_path: PathBuf,
	stderr_path: PathBuf,
}

pub struct IntegrationCtxBuilder {
	snapshot_dir: Option<PathBuf>,
}

pub struct RegistryTask {
	shutdown: CancellationToken,
	task: JoinHandle<Result<()>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ActorList {
	pub actors: Vec<ApiActor>,
}

#[derive(Debug, serde::Deserialize)]
struct CreateActorResponse {
	actor: ApiActor,
}

#[derive(Debug, serde::Deserialize)]
struct EnvoyList {
	envoys: Vec<ApiEnvoy>,
}

#[derive(Debug, serde::Deserialize)]
struct ApiEnvoy {
	pool_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct RunnerConfigList {
	runner_configs: std::collections::HashMap<String, RunnerConfigDatacenters>,
}

#[derive(Debug, serde::Deserialize)]
struct RunnerConfigDatacenters {
	datacenters: std::collections::HashMap<String, RunnerConfigEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct RunnerConfigEntry {
	protocol_version: Option<u16>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ApiActor {
	pub actor_id: String,
}

impl IntegrationCtx {
	pub fn builder() -> IntegrationCtxBuilder {
		IntegrationCtxBuilder { snapshot_dir: None }
	}

	pub fn serve_registry(&self, registry: CoreRegistry) -> RegistryTask {
		let shutdown = CancellationToken::new();
		let config = ServeConfig {
			endpoint: self.endpoint.clone(),
			token: Some(TOKEN.to_owned()),
			namespace: DEFAULT_NAMESPACE.to_owned(),
			pool_name: DEFAULT_POOL.to_owned(),
			engine_binary_path: None,
			..ServeConfig::default()
		};
		let task = tokio::spawn({
			let shutdown = shutdown.clone();
			async move { registry.serve_with_config(config, shutdown).await }
		});

		RegistryTask { shutdown, task }
	}

	pub async fn wait_for_envoy_ready(&self) -> Result<()> {
		let deadline = Instant::now() + Duration::from_secs(30);
		let mut last_error = None;
		while Instant::now() < deadline {
			match self.envoy_ready().await {
				Ok(true) => return Ok(()),
				Ok(false) => {}
				Err(err) => last_error = Some(err),
			}

			tokio::time::sleep(Duration::from_millis(250)).await;
		}

		match last_error {
			Some(err) => Err(err).context("timed out waiting for envoy readiness"),
			None => bail!("timed out waiting for envoy readiness"),
		}
	}

	pub async fn actor_by_name(&self, name: &str) -> Result<ApiActor> {
		let response = self
			.client
			.get(format!("{}/actors", self.endpoint))
			.query(&[("namespace", DEFAULT_NAMESPACE), ("name", name)])
			.bearer_auth(TOKEN)
			.send()
			.await
			.context("list actors by name")?;
		let status = response.status();
		let body = response.text().await.context("read actor list response")?;
		if !status.is_success() {
			bail!("list actors by name failed with {status}: {body}");
		}

		let mut actors: ActorList =
			serde_json::from_str(&body).context("decode actor list response")?;
		actors
			.actors
			.pop()
			.with_context(|| format!("actor `{name}` should exist"))
	}

	pub async fn create_default_namespace(&self) -> Result<()> {
		let response = self
			.client
			.post(format!("{}/namespaces", self.endpoint))
			.bearer_auth(TOKEN)
			.json(&serde_json::json!({
				"name": DEFAULT_NAMESPACE,
				"display_name": "Default",
			}))
			.send()
			.await
			.context("create default namespace")?;
		let status = response.status();
		let body = response
			.text()
			.await
			.context("read create namespace response")?;
		if !status.is_success()
			&& !body.contains("\"code\":\"already_exists\"")
			&& !body.contains("\"code\":\"name_not_unique\"")
		{
			bail!("create default namespace failed with {status}: {body}");
		}

		Ok(())
	}

	pub async fn create_actor(&self, name: &str) -> Result<ApiActor> {
		let response = self
			.client
			.post(format!("{}/actors", self.endpoint))
			.query(&[("namespace", DEFAULT_NAMESPACE)])
			.bearer_auth(TOKEN)
			.json(&serde_json::json!({
				"name": name,
				"runner_name_selector": DEFAULT_POOL,
				"key": null,
				"input": null,
				"datacenter": null,
				"crash_policy": "destroy",
			}))
			.send()
			.await
			.context("create actor")?;
		let status = response.status();
		let body = response
			.text()
			.await
			.context("read create actor response")?;
		if !status.is_success() {
			bail!("create actor failed with {status}: {body}");
		}

		let response: CreateActorResponse =
			serde_json::from_str(&body).context("decode create actor response")?;
		Ok(response.actor)
	}

	async fn envoy_ready(&self) -> Result<bool> {
		let envoys_response = self
			.client
			.get(format!("{}/envoys", self.endpoint))
			.query(&[
				("namespace", DEFAULT_NAMESPACE),
				("name", DEFAULT_POOL),
				("limit", "1"),
			])
			.bearer_auth(TOKEN)
			.send()
			.await
			.context("list envoys")?;
		let envoys_status = envoys_response.status();
		let envoys_body = envoys_response
			.text()
			.await
			.context("read envoys response")?;
		if !envoys_status.is_success() {
			bail!("list envoys failed with {envoys_status}: {envoys_body}");
		}
		let envoys: EnvoyList =
			serde_json::from_str(&envoys_body).context("decode envoys response")?;
		let has_envoy = envoys
			.envoys
			.iter()
			.any(|envoy| envoy.pool_name == DEFAULT_POOL);
		if !has_envoy {
			return Ok(false);
		}

		let runner_configs_response = self
			.client
			.get(format!("{}/runner-configs", self.endpoint))
			.query(&[
				("namespace", DEFAULT_NAMESPACE),
				("runner_name", DEFAULT_POOL),
				("limit", "1"),
			])
			.bearer_auth(TOKEN)
			.send()
			.await
			.context("list runner configs")?;
		let runner_configs_status = runner_configs_response.status();
		let runner_configs_body = runner_configs_response
			.text()
			.await
			.context("read runner configs response")?;
		if !runner_configs_status.is_success() {
			bail!("list runner configs failed with {runner_configs_status}: {runner_configs_body}");
		}
		let runner_configs: RunnerConfigList =
			serde_json::from_str(&runner_configs_body).context("decode runner configs response")?;
		let protocol_ready = runner_configs
			.runner_configs
			.get(DEFAULT_POOL)
			.map(|runner_config| {
				runner_config
					.datacenters
					.values()
					.any(|dc| dc.protocol_version.is_some())
			})
			.unwrap_or(false);

		Ok(protocol_ready)
	}

	pub async fn send_json_action(&self, actor_id: &str, action: &str) -> Result<String> {
		let response = self
			.client
			.post(format!(
				"{}/gateway/{}/action/{}",
				self.endpoint, actor_id, action
			))
			.header("x-rivet-encoding", "json")
			.header("content-type", "application/json")
			.body(r#"{"args":[]}"#)
			.send()
			.await
			.context("send actor action")?;
		let status = response.status();
		let body = response
			.text()
			.await
			.context("read actor action response")?;
		if !status.is_success() {
			bail!(
				"actor action failed with {status}: {body}\n\nengine stdout:\n{}\n\nengine stderr:\n{}",
				tail_file(&self.stdout_path),
				tail_file(&self.stderr_path)
			);
		}
		Ok(body)
	}

	pub async fn wait_for_json_action(&self, actor_id: &str, action: &str) -> Result<String> {
		let deadline = Instant::now() + Duration::from_secs(30);
		let mut last_error = None;
		while Instant::now() < deadline {
			match self.send_json_action(actor_id, action).await {
				Ok(body) => return Ok(body),
				Err(err) => {
					let message = err.to_string();
					if !message.contains("actor_ready_timeout")
						&& !message.contains("Service Unavailable")
					{
						return Err(err).context("actor action returned non-readiness error");
					}
					last_error = Some(err);
					tokio::time::sleep(Duration::from_millis(250)).await;
				}
			}
		}

		match last_error {
			Some(err) => Err(err).context("timed out waiting for actor action"),
			None => bail!("timed out waiting for actor action"),
		}
	}

	pub fn endpoint(&self) -> &str {
		&self.endpoint
	}

	pub fn engine_stdout_tail(&self) -> String {
		tail_file(&self.stdout_path)
	}

	pub fn engine_stderr_tail(&self) -> String {
		tail_file(&self.stderr_path)
	}

	pub async fn shutdown(mut self) -> Result<()> {
		shutdown_child(&mut self.child).await;
		Ok(())
	}
}

impl IntegrationCtxBuilder {
	pub fn import_snapshot(mut self, snapshot_dir: impl Into<PathBuf>) -> Self {
		self.snapshot_dir = Some(snapshot_dir.into());
		self
	}

	pub async fn start(self) -> Result<IntegrationCtx> {
		let _ = tracing_subscriber::fmt()
			.with_env_filter("info")
			.with_ansi(false)
			.with_test_writer()
			.try_init();

		let temp_dir = tempfile::tempdir().context("create integration temp dir")?;
		let db_path = temp_dir.path().join("db");
		if let Some(snapshot_dir) = self.snapshot_dir.as_deref() {
			let snapshot_path = if snapshot_dir.is_absolute() {
				snapshot_dir.to_path_buf()
			} else {
				workspace_root().join(snapshot_dir)
			}
			.join("replica-1");
			copy_dir_recursive(&snapshot_path, &db_path)
				.with_context(|| format!("import snapshot `{}`", snapshot_path.display()))?;
		} else {
			std::fs::create_dir_all(&db_path).context("create empty engine db")?;
		}

		let guard_port = pick_port("guard")?;
		let api_peer_port = pick_port("api-peer")?;
		let metrics_port = pick_port("metrics")?;
		let endpoint = format!("http://127.0.0.1:{guard_port}");
		let binary_path = engine_binary_path()?;
		let stdout_path = temp_dir.path().join("engine.stdout.log");
		let stderr_path = temp_dir.path().join("engine.stderr.log");
		let stdout = File::create(&stdout_path).context("create engine stdout log")?;
		let stderr = File::create(&stderr_path).context("create engine stderr log")?;

		let mut child = Command::new(&binary_path)
			.arg("start")
			.env("RIVET__GUARD__HOST", "127.0.0.1")
			.env("RIVET__GUARD__PORT", guard_port.to_string())
			.env("RIVET__API_PEER__HOST", "127.0.0.1")
			.env("RIVET__API_PEER__PORT", api_peer_port.to_string())
			.env("RIVET__METRICS__HOST", "127.0.0.1")
			.env("RIVET__METRICS__PORT", metrics_port.to_string())
			.env("RIVET__FILE_SYSTEM__PATH", &db_path)
			.stdin(Stdio::null())
			.stdout(Stdio::from(stdout))
			.stderr(Stdio::from(stderr))
			.spawn()
			.with_context(|| format!("spawn engine binary `{}`", binary_path.display()))?;

		let client = reqwest::Client::new();
		wait_for_engine_health(&client, &endpoint, &mut child, &stderr_path).await?;

		Ok(IntegrationCtx {
			temp_dir,
			endpoint,
			child,
			client,
			stdout_path,
			stderr_path,
		})
	}
}

impl RegistryTask {
	pub async fn shutdown(self) -> Result<()> {
		self.shutdown.cancel();
		self.task
			.await
			.context("join registry task")?
			.context("registry task")
	}
}

impl Drop for IntegrationCtx {
	fn drop(&mut self) {
		let _ = self.child.start_kill();
		let _ = self.temp_dir.path();
	}
}

fn workspace_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.nth(3)
		.expect("rivetkit-core should live under the workspace root")
		.to_path_buf()
}

fn engine_binary_path() -> Result<PathBuf> {
	if let Some(path) = std::env::var_os("RIVET_ENGINE_BINARY_PATH").map(PathBuf::from) {
		return Ok(path);
	}

	let path = workspace_root().join("target/debug/rivet-engine");
	if path.exists() {
		return Ok(path);
	}

	bail!(
		"engine binary not found at {}; run `cargo build -p rivet-engine` or set RIVET_ENGINE_BINARY_PATH",
		path.display()
	)
}

fn pick_port(label: &str) -> Result<u16> {
	portpicker::pick_unused_port().with_context(|| format!("pick {label} port"))
}

async fn wait_for_engine_health(
	client: &reqwest::Client,
	endpoint: &str,
	child: &mut Child,
	stderr_path: &Path,
) -> Result<()> {
	let deadline = Instant::now() + Duration::from_secs(60);
	let health_url = format!("{endpoint}/health");
	while Instant::now() < deadline {
		if let Some(status) = child.try_wait().context("poll engine child")? {
			let stderr = std::fs::read_to_string(stderr_path).unwrap_or_default();
			bail!("engine exited before health check passed with {status}: {stderr}");
		}

		match client.get(&health_url).send().await {
			Ok(response) if response.status() == StatusCode::OK => return Ok(()),
			Ok(_) | Err(_) => {
				tokio::time::sleep(Duration::from_millis(250)).await;
			}
		}
	}

	let stderr = std::fs::read_to_string(stderr_path).unwrap_or_default();
	bail!("timed out waiting for engine health: {stderr}");
}

async fn shutdown_child(child: &mut Child) {
	if child.try_wait().ok().flatten().is_some() {
		return;
	}

	let _ = child.start_kill();
	let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
	std::fs::create_dir_all(dst)
		.with_context(|| format!("create destination directory `{}`", dst.display()))?;
	for entry in
		std::fs::read_dir(src).with_context(|| format!("read directory `{}`", src.display()))?
	{
		let entry = entry?;
		let file_type = entry.file_type()?;
		let dest_path = dst.join(entry.file_name());
		if file_type.is_dir() {
			copy_dir_recursive(&entry.path(), &dest_path)?;
		} else {
			std::fs::copy(entry.path(), &dest_path).with_context(|| {
				format!(
					"copy `{}` to `{}`",
					entry.path().display(),
					dest_path.display()
				)
			})?;
		}
	}
	Ok(())
}

fn tail_file(path: &Path) -> String {
	let Ok(contents) = std::fs::read_to_string(path) else {
		return format!("failed to read `{}`", path.display());
	};
	let mut lines = contents.lines().rev().take(120).collect::<Vec<_>>();
	lines.reverse();
	lines.join("\n")
}
