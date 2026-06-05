//! In-process end-to-end test helpers for Rust actors.
//!
//! Mirrors the TypeScript `rivetkit/test` `setupTest` helper: [`setup`] spawns
//! or reuses a local engine, serves the registry, and returns a client so a test
//! can call actions with no engine or HTTP plumbing.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use rivetkit_client::handle::ActorHandle;
use rivetkit_client::{Client, ClientConfig, GetOrCreateOptions};
use rivetkit_core::ServeConfig;
use serde_json::Value as JsonValue;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::registry::Registry;

const ENDPOINT: &str = "http://127.0.0.1:6420";
const ENGINE_PORT: u16 = 6420;
const TOKEN: &str = "dev";
const NAMESPACE: &str = "default";
const READY_TIMEOUT: Duration = Duration::from_secs(30);

// Serializes engine bring-up so concurrent `setup` calls in one process reuse a
// single engine instead of racing to bind the same port.
static ENGINE_LOCK: Mutex<()> = Mutex::const_new(());
static POOL_SEQ: AtomicU64 = AtomicU64::new(0);

/// Spawns or reuses a local engine, serves `registry`, and returns a handle for
/// calling actions through the Rust client.
///
/// Requires `RIVET_ENGINE_BINARY_PATH`, or a `target/debug/rivet-engine` built
/// in the workspace (run `cargo build -p rivet-engine`).
pub async fn setup(registry: Registry) -> Result<TestHandle> {
	// A unique pool name keeps concurrently-tested registries on the shared
	// engine from cross-routing each other's actors.
	let pool_name = format!("rivetkit-test-{}", POOL_SEQ.fetch_add(1, Ordering::Relaxed));
	let config = ServeConfig {
		endpoint: ENDPOINT.to_owned(),
		token: Some(TOKEN.to_owned()),
		namespace: NAMESPACE.to_owned(),
		pool_name: pool_name.clone(),
		engine_binary_path: Some(engine_binary_path()?),
		..ServeConfig::default()
	};

	let shutdown = CancellationToken::new();
	let serve = {
		// Hold the lock until the engine port accepts connections so a second
		// concurrent `setup` reuses this engine instead of spawning its own.
		let _guard = ENGINE_LOCK.lock().await;
		let serve = tokio::spawn({
			let shutdown = shutdown.clone();
			async move { registry.serve_with_config(config, shutdown).await }
		});
		wait_for_port().await;
		serve
	};

	let client = Client::new(
		ClientConfig::new(ENDPOINT)
			.token(TOKEN)
			.namespace(NAMESPACE)
			.pool_name(pool_name),
	);

	Ok(TestHandle {
		client,
		shutdown,
		serve: Some(serve),
	})
}

/// A running test registry. Drop or [`shutdown`](TestHandle::shutdown) to stop
/// serving. The local engine is intentionally left running for reuse.
pub struct TestHandle {
	client: Client,
	shutdown: CancellationToken,
	serve: Option<JoinHandle<Result<()>>>,
}

impl TestHandle {
	/// A fresh actor of `name` with a unique key, so each run starts clean.
	pub fn actor(&self, name: &str) -> TestActor {
		self.actor_with_key(name, vec![format!("{name}-{}", unique_suffix())])
	}

	/// The actor of `name` with an explicit key.
	pub fn actor_with_key(&self, name: &str, key: Vec<String>) -> TestActor {
		TestActor {
			handle: self
				.client
				.get_or_create(name, key, GetOrCreateOptions::default())
				.expect("build actor handle"),
		}
	}

	/// The underlying Rust client.
	pub fn client(&self) -> &Client {
		&self.client
	}

	/// Cancels serving and waits for the registry task to drain.
	pub async fn shutdown(mut self) {
		self.shutdown.cancel();
		if let Some(serve) = self.serve.take() {
			let _ = serve.await;
		}
	}
}

impl Drop for TestHandle {
	fn drop(&mut self) {
		self.shutdown.cancel();
	}
}

/// A handle to an actor under test. Action calls retry while the actor is still
/// starting up, so the first call needs no manual readiness wait.
pub struct TestActor {
	handle: ActorHandle,
}

impl TestActor {
	pub async fn action(&self, name: &str, args: Vec<JsonValue>) -> Result<JsonValue> {
		let deadline = Instant::now() + READY_TIMEOUT;
		loop {
			match self.handle.action(name, args.clone()).await {
				Ok(value) => return Ok(value),
				Err(error) if is_transient(&error) && Instant::now() < deadline => {
					tokio::time::sleep(Duration::from_millis(250)).await;
				}
				Err(error) => return Err(error),
			}
		}
	}

	/// The underlying client actor handle (for connections, events, etc.).
	pub fn handle(&self) -> &ActorHandle {
		&self.handle
	}
}

// Transient gateway errors mean the request never reached a ready actor, so
// retrying cannot double-apply a mutation.
fn is_transient(error: &anyhow::Error) -> bool {
	let message = error.to_string();
	message.contains("actor_ready_timeout")
		|| message.contains("no_runner_config_configured")
		|| message.contains("Service Unavailable")
}

async fn wait_for_port() {
	let deadline = Instant::now() + Duration::from_secs(60);
	while Instant::now() < deadline {
		if TcpStream::connect(("127.0.0.1", ENGINE_PORT)).await.is_ok() {
			return;
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}
}

fn engine_binary_path() -> Result<PathBuf> {
	if let Some(path) = std::env::var_os("RIVET_ENGINE_BINARY_PATH") {
		return Ok(PathBuf::from(path));
	}
	for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
		for profile in ["debug", "release"] {
			let candidate = ancestor.join("target").join(profile).join("rivet-engine");
			if candidate.exists() {
				return Ok(candidate);
			}
		}
	}
	bail!(
		"rivet-engine binary not found; run `cargo build -p rivet-engine` or set RIVET_ENGINE_BINARY_PATH"
	)
}

fn unique_suffix() -> u128 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_nanos())
		.unwrap_or_default()
}
