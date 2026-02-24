use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::{Value, json};
use std::{
	fmt::Write as _,
	path::PathBuf,
	process::{Child, Command, Stdio},
	sync::{Arc, OnceLock},
	time::{Duration, Instant},
};
use tempfile::TempDir;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_tungstenite::{
	connect_async,
	tungstenite::client::IntoClientRequest,
	WebSocketStream,
	MaybeTlsStream,
};
use urlencoding::encode;

pub struct EngineProcess {
	pub deps: rivet_test_deps::TestDeps,
	child: Child,
	_config_dir: TempDir,
}

impl EngineProcess {
	pub async fn start() -> Result<Self> {
		let deps = rivet_test_deps::TestDeps::new().await?;

		let config_dir = tempfile::tempdir().context("failed to create config dir")?;
		let config_path = config_dir.path().join("rivet.test.yaml");
		let mut root = (**deps.config()).clone();
		if let Some(rivet_config::config::Database::FileSystem(database)) = root.database.as_mut() {
			let db_path = config_dir.path().join("engine-db");
			std::fs::create_dir_all(&db_path).context("failed to create engine db dir")?;
			database.path = db_path;
		}

		let config_yaml = serde_yaml::to_string(&root)
			.context("failed to serialize config")?;
		std::fs::write(&config_path, config_yaml).context("failed to write config")?;

		let engine_bin = ensure_engine_binary()?;
		let mut cmd = Command::new(engine_bin);
		cmd.arg("--config")
			.arg(&config_path)
			.arg("start")
			.arg("-s")
			.arg("api_peer")
			.arg("-s")
			.arg("guard")
			.arg("-s")
			.arg("workflow_worker")
			.arg("-s")
			.arg("bootstrap")
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.stdin(Stdio::null());

		let child = cmd.spawn().context("failed to spawn rivet-engine")?;

		wait_for_port(deps.api_peer_port()).await?;
		wait_for_port(deps.guard_port()).await?;

		Ok(Self {
			deps,
			child,
			_config_dir: config_dir,
		})
	}

	pub fn guard_url(&self) -> String {
		format!("http://127.0.0.1:{}", self.deps.guard_port())
	}

	pub async fn create_actor(
		&self,
		namespace: &str,
		name: &str,
		runner_name_selector: &str,
		key: Option<&str>,
	) -> Result<String> {
		let client = reqwest::Client::new();
		let response = client
			.post(format!("{}/actors", self.guard_url()))
			.query(&[("namespace", namespace)])
			.json(&json!({
				"datacenter": null,
				"name": name,
				"key": key,
				"input": null,
				"runner_name_selector": runner_name_selector,
				"crash_policy": "sleep",
			}))
			.send()
			.await
			.context("failed to create actor")?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			bail!("create actor failed: {status} {body}");
		}

		let body: Value = response.json().await.context("failed to decode actor response")?;
		let actor_id = body
			.get("actor")
			.and_then(|x| x.get("actor_id"))
			.and_then(Value::as_str)
			.context("actor id missing from create actor response")?;
		Ok(actor_id.to_string())
	}

	#[allow(dead_code)]
	pub async fn actor_request_json(
		&self,
		method: Method,
		actor_id: &str,
		path: &str,
		body: Option<Value>,
	) -> Result<Value> {
		let response = self
			.actor_request_with_retry(method, actor_id, path, body)
			.await?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			bail!("actor request failed: {status} {body}");
		}

		response
			.json()
			.await
			.context("failed to decode actor response json")
	}

	#[allow(dead_code)]
	pub async fn get_actor(&self, namespace: &str, actor_id: &str) -> Result<Option<Value>> {
		let client = reqwest::Client::new();
		let response = client
			.get(format!("{}/actors", self.guard_url()))
			.query(&[
				("namespace", namespace),
				("actor_id", actor_id),
				("include_destroyed", "true"),
			])
			.send()
			.await
			.context("failed to fetch actors list")?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			bail!("actors list request failed: {status} {body}");
		}

		let body: Value = response
			.json()
			.await
			.context("failed to decode actors list response json")?;
		let actor = body
			.get("actors")
			.and_then(Value::as_array)
			.and_then(|actors| actors.first())
			.cloned();
		Ok(actor)
	}

	#[allow(dead_code)]
	pub async fn actor_request_with_retry(
		&self,
		method: Method,
		actor_id: &str,
		path: &str,
		body: Option<Value>,
	) -> Result<reqwest::Response> {
		let url = format!("{}{}", self.guard_url(), path);
		let client = reqwest::Client::new();

		let start = Instant::now();
		let timeout = Duration::from_secs(30);
		let mut last_error: Option<anyhow::Error> = None;

		loop {
			if start.elapsed() > timeout {
				if let Some(err) = last_error {
					return Err(err).context("timed out waiting for actor response");
				}
				bail!("timed out waiting for actor response");
			}

			let mut request = client
				.request(method.clone(), &url)
				.header("x-rivet-target", "actor")
				.header("x-rivet-token", "dev")
				.header("x-rivet-actor", actor_id);

			if let Some(json) = &body {
				request = request.json(json);
			}

			match request.send().await {
				Ok(response)
					if response.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE
						|| response.status() == reqwest::StatusCode::NOT_FOUND =>
				{
					tokio::time::sleep(Duration::from_millis(250)).await;
					continue;
				}
				Ok(response) if response.status() == reqwest::StatusCode::BAD_REQUEST => {
					tokio::time::sleep(Duration::from_millis(250)).await;
					drop(response);
					continue;
				}
				Ok(response) => return Ok(response),
				Err(err) => {
					last_error = Some(err.into());
					tokio::time::sleep(Duration::from_millis(250)).await;
				}
			}
		}
	}

	#[allow(dead_code)]
	pub async fn actor_websocket_connect(
		&self,
		actor_id: &str,
		path: &str,
	) -> Result<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>> {
		let start = Instant::now();
		let timeout = Duration::from_secs(30);
		let mut last_error: Option<anyhow::Error> = None;

		loop {
			if start.elapsed() > timeout {
				if let Some(err) = last_error {
					return Err(err).context("timed out connecting actor websocket");
				}
				bail!("timed out connecting actor websocket");
			}

			let mut ws_url = self.guard_url().replace("http://", "ws://");
			if path.starts_with('/') {
				ws_url.push_str(path);
			} else {
				ws_url.push('/');
				ws_url.push_str(path);
			}

			let mut request = ws_url
				.into_client_request()
				.context("failed to build websocket request")?;
			request
				.headers_mut()
				.insert("x-rivet-target", "actor".parse().context("invalid target header")?);
			request
				.headers_mut()
				.insert("x-rivet-token", "dev".parse().context("invalid token header")?);
			request
				.headers_mut()
				.insert("x-rivet-actor", actor_id.parse().context("invalid actor header")?);
			let actor_id_protocol = format!("rivet_actor.{}", encode(actor_id));
			let websocket_protocol = format!(
				"rivet_target.actor, {actor_id_protocol}, rivet_token.dev, rivet"
			);
			request.headers_mut().insert(
				"Sec-WebSocket-Protocol",
				websocket_protocol
					.parse()
					.context("invalid websocket protocol header")?,
			);

			match connect_async(request).await {
				Ok((ws, _response)) => return Ok(ws),
				Err(err) => {
					last_error = Some(err.into());
					tokio::time::sleep(Duration::from_millis(250)).await;
				}
			}
		}
	}
}

pub async fn acquire_test_lock() -> Result<OwnedSemaphorePermit> {
	static TEST_LOCK: OnceLock<Arc<Semaphore>> = OnceLock::new();
	let lock = TEST_LOCK
		.get_or_init(|| Arc::new(Semaphore::new(1)))
		.clone();
	lock.acquire_owned()
		.await
		.context("failed to acquire test lock")
}

pub fn random_name(prefix: &str) -> String {
	let mut name = String::with_capacity(prefix.len() + 17);
	let _ = write!(&mut name, "{}-{:016x}", prefix, rand::random::<u64>());
	name
}

impl Drop for EngineProcess {
	fn drop(&mut self) {
		let _ = self.child.kill();
		let _ = self.child.wait();
	}
}

fn ensure_engine_binary() -> Result<PathBuf> {
	static BUILD_RESULT: OnceLock<Result<PathBuf, String>> = OnceLock::new();

	let result = BUILD_RESULT.get_or_init(|| {
		let workspace = workspace_root();
		let status = Command::new("cargo")
			.arg("build")
			.arg("-p")
			.arg("rivet-engine")
			.current_dir(&workspace)
			.status();

		match status {
			Ok(status) if status.success() => {
				let bin = workspace.join("target").join("debug").join("rivet-engine");
				if bin.exists() {
					Ok(bin)
				} else {
					Err(format!("engine binary not found at {}", bin.display()))
				}
			}
			Ok(status) => Err(format!("cargo build -p rivet-engine failed with status {status}")),
			Err(err) => Err(format!("failed to execute cargo build: {err}")),
		}
	});

	result.clone().map_err(anyhow::Error::msg)
}

fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../../../")
		.canonicalize()
		.expect("workspace root")
}

async fn wait_for_port(port: u16) -> Result<()> {
	let addr = format!("127.0.0.1:{port}");
	let start = Instant::now();
	let timeout = Duration::from_secs(30);

	loop {
		match tokio::net::TcpStream::connect(&addr).await {
			Ok(_) => return Ok(()),
			Err(_) if start.elapsed() <= timeout => tokio::time::sleep(Duration::from_millis(100)).await,
			Err(err) => return Err(err).with_context(|| format!("timed out waiting for port {port}")),
		}
	}
}
