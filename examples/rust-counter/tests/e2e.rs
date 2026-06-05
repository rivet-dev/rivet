use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use rivetkit_core::ServeConfig;
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};
use tempfile::TempDir;
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const TOKEN: &str = "dev";
const NAMESPACE: &str = "default";
const POOL_NAME: &str = "rivetkit-rust-counter";

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_works_through_local_engine() -> Result<()> {
	let engine = TestEngine::start().await?;
	create_default_namespace(&engine).await?;

	let shutdown = CancellationToken::new();
	let registry_task = serve_registry(&engine, shutdown.clone());
	wait_for_envoy_ready(&engine).await?;

	let actor_id = create_counter_actor(&engine).await?;
	let first = wait_for_action(&engine, &actor_id, "increment").await?;
	let second = wait_for_action(&engine, &actor_id, "increment").await?;
	let current = wait_for_action(&engine, &actor_id, "get").await?;

	assert_eq!(action_output(&first)?, json!(1));
	assert_eq!(action_output(&second)?, json!(2));
	assert_eq!(action_output(&current)?, json!(2));

	shutdown.cancel();
	registry_task.await.context("join registry task")??;
	engine.shutdown().await;

	Ok(())
}

fn serve_registry(engine: &TestEngine, shutdown: CancellationToken) -> JoinHandle<Result<()>> {
	let config = ServeConfig {
		endpoint: engine.endpoint.clone(),
		token: Some(TOKEN.to_owned()),
		namespace: NAMESPACE.to_owned(),
		pool_name: POOL_NAME.to_owned(),
		engine_binary_path: None,
		..ServeConfig::default()
	};
	tokio::spawn(async move {
		rivetkit_rust_counter_example::registry()
			.serve_with_config(config, shutdown)
			.await
	})
}

struct TestEngine {
	_temp_dir: TempDir,
	endpoint: String,
	child: Child,
	client: reqwest::Client,
	stdout_path: PathBuf,
	stderr_path: PathBuf,
}

impl TestEngine {
	async fn start() -> Result<Self> {
		let _ = tracing_subscriber::fmt()
			.with_env_filter("info")
			.with_ansi(false)
			.with_test_writer()
			.try_init();

		let temp_dir = tempfile::tempdir().context("create temp dir")?;
		let db_path = temp_dir.path().join("db");
		std::fs::create_dir_all(&db_path).context("create engine db")?;
		let guard_port = pick_port("guard")?;
		let api_peer_port = pick_port("api-peer")?;
		let metrics_port = pick_port("metrics")?;
		let endpoint = format!("http://127.0.0.1:{guard_port}");
		let stdout_path = temp_dir.path().join("engine.stdout.log");
		let stderr_path = temp_dir.path().join("engine.stderr.log");
		let stdout = File::create(&stdout_path).context("create engine stdout log")?;
		let stderr = File::create(&stderr_path).context("create engine stderr log")?;
		let binary_path = engine_binary_path()?;

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
			.with_context(|| format!("spawn engine `{}`", binary_path.display()))?;

		let client = reqwest::Client::new();
		wait_for_engine_health(&client, &endpoint, &mut child, &stderr_path).await?;

		Ok(Self {
			_temp_dir: temp_dir,
			endpoint,
			child,
			client,
			stdout_path,
			stderr_path,
		})
	}

	async fn shutdown(mut self) {
		shutdown_child(&mut self.child).await;
	}

	fn stderr_tail(&self) -> String {
		tail_file(&self.stderr_path)
	}

	fn stdout_tail(&self) -> String {
		tail_file(&self.stdout_path)
	}
}

impl Drop for TestEngine {
	fn drop(&mut self) {
		let _ = self.child.start_kill();
	}
}

#[derive(Deserialize)]
struct EnvoyList {
	envoys: Vec<ApiEnvoy>,
}

#[derive(Deserialize)]
struct ApiEnvoy {
	pool_name: String,
}

#[derive(Deserialize)]
struct RunnerConfigList {
	runner_configs: std::collections::HashMap<String, RunnerConfigDatacenters>,
}

#[derive(Deserialize)]
struct RunnerConfigDatacenters {
	datacenters: std::collections::HashMap<String, RunnerConfigEntry>,
}

#[derive(Deserialize)]
struct RunnerConfigEntry {
	protocol_version: Option<u16>,
}

#[derive(Deserialize)]
struct CreateActorResponse {
	actor: ApiActor,
}

#[derive(Deserialize)]
struct ApiActor {
	actor_id: String,
}

async fn create_default_namespace(engine: &TestEngine) -> Result<()> {
	let response = engine
		.client
		.post(format!("{}/namespaces", engine.endpoint))
		.bearer_auth(TOKEN)
		.json(&json!({
			"name": NAMESPACE,
			"display_name": "Default",
		}))
		.send()
		.await
		.context("create default namespace")?;
	let status = response.status();
	let body = response.text().await.context("read namespace response")?;
	if !status.is_success()
		&& !body.contains("\"code\":\"already_exists\"")
		&& !body.contains("\"code\":\"name_not_unique\"")
	{
		bail!("create namespace failed with {status}: {body}");
	}
	Ok(())
}

async fn wait_for_envoy_ready(engine: &TestEngine) -> Result<()> {
	let deadline = Instant::now() + Duration::from_secs(30);
	let mut last_error = None;
	while Instant::now() < deadline {
		match envoy_ready(engine).await {
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

async fn envoy_ready(engine: &TestEngine) -> Result<bool> {
	let envoys_response = engine
		.client
		.get(format!("{}/envoys", engine.endpoint))
		.query(&[
			("namespace", NAMESPACE),
			("name", POOL_NAME),
			("limit", "1"),
		])
		.bearer_auth(TOKEN)
		.send()
		.await
		.context("list envoys")?;
	let status = envoys_response.status();
	let body = envoys_response.text().await.context("read envoys")?;
	if !status.is_success() {
		bail!("list envoys failed with {status}: {body}");
	}
	let envoys: EnvoyList = serde_json::from_str(&body).context("decode envoys")?;
	if !envoys
		.envoys
		.iter()
		.any(|envoy| envoy.pool_name == POOL_NAME)
	{
		return Ok(false);
	}

	let runner_configs_response = engine
		.client
		.get(format!("{}/runner-configs", engine.endpoint))
		.query(&[
			("namespace", NAMESPACE),
			("runner_name", POOL_NAME),
			("limit", "1"),
		])
		.bearer_auth(TOKEN)
		.send()
		.await
		.context("list runner configs")?;
	let status = runner_configs_response.status();
	let body = runner_configs_response
		.text()
		.await
		.context("read runner configs")?;
	if !status.is_success() {
		bail!("list runner configs failed with {status}: {body}");
	}
	let runner_configs: RunnerConfigList =
		serde_json::from_str(&body).context("decode runner configs")?;
	Ok(runner_configs
		.runner_configs
		.get(POOL_NAME)
		.map(|runner_config| {
			runner_config
				.datacenters
				.values()
				.any(|dc| dc.protocol_version.is_some())
		})
		.unwrap_or(false))
}

async fn create_counter_actor(engine: &TestEngine) -> Result<String> {
	let response = engine
		.client
		.post(format!("{}/actors", engine.endpoint))
		.query(&[("namespace", NAMESPACE)])
		.bearer_auth(TOKEN)
		.json(&json!({
			"name": rivetkit_rust_counter_example::ACTOR_NAME,
			"runner_name_selector": POOL_NAME,
			"key": null,
			"input": null,
			"datacenter": null,
			"crash_policy": "destroy",
		}))
		.send()
		.await
		.context("create counter actor")?;
	let status = response.status();
	let body = response.text().await.context("read actor response")?;
	if !status.is_success() {
		bail!("create actor failed with {status}: {body}");
	}
	let response: CreateActorResponse = serde_json::from_str(&body).context("decode actor")?;
	Ok(response.actor.actor_id)
}

async fn wait_for_action(engine: &TestEngine, actor_id: &str, action: &str) -> Result<String> {
	let deadline = Instant::now() + Duration::from_secs(30);
	let mut last_error = None;
	while Instant::now() < deadline {
		match send_action(engine, actor_id, action).await {
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
		Some(err) => Err(err).context("timed out waiting for action"),
		None => bail!("timed out waiting for action"),
	}
}

async fn send_action(engine: &TestEngine, actor_id: &str, action: &str) -> Result<String> {
	let response = engine
		.client
		.post(format!(
			"{}/gateway/{}/action/{}",
			engine.endpoint, actor_id, action
		))
		.header("x-rivet-encoding", "json")
		.header("content-type", "application/json")
		.body(r#"{"args":[]}"#)
		.send()
		.await
		.context("send action")?;
	let status = response.status();
	let body = response.text().await.context("read action response")?;
	if !status.is_success() {
		bail!(
			"action failed with {status}: {body}\n\nengine stdout:\n{}\n\nengine stderr:\n{}",
			engine.stdout_tail(),
			engine.stderr_tail()
		);
	}
	Ok(body)
}

fn action_output(body: &str) -> Result<JsonValue> {
	let value: JsonValue = serde_json::from_str(body).context("decode action response")?;
	Ok(value.get("output").cloned().unwrap_or(JsonValue::Null))
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

fn workspace_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.nth(2)
		.expect("example should live under examples/rust-counter")
		.to_path_buf()
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
			let stderr = tail_file(stderr_path);
			bail!("engine exited before health check passed with {status}: {stderr}");
		}

		match client.get(&health_url).send().await {
			Ok(response) if response.status() == StatusCode::OK => return Ok(()),
			Ok(_) | Err(_) => tokio::time::sleep(Duration::from_millis(250)).await,
		}
	}

	bail!(
		"timed out waiting for engine health: {}",
		tail_file(stderr_path)
	)
}

async fn shutdown_child(child: &mut Child) {
	if child.try_wait().ok().flatten().is_some() {
		return;
	}

	let _ = child.start_kill();
	let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

fn tail_file(path: &Path) -> String {
	let Ok(contents) = std::fs::read_to_string(path) else {
		return String::new();
	};
	let mut lines = contents.lines().rev().take(80).collect::<Vec<_>>();
	lines.reverse();
	lines.join("\n")
}
