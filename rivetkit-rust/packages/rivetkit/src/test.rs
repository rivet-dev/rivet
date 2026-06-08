//! In-process end-to-end test helpers for Rust actors.
//!
//! Mirrors the TypeScript `rivetkit/test` `setupTest` helper: [`setup`] spawns
//! or reuses a local engine, serves the registry, and returns a client so a test
//! can call actions with no engine or HTTP plumbing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rivetkit_client::{Client, ClientConfig, GetOrCreateOptions};
use rivetkit_core::ServeConfig;
use serde_json::Value as JsonValue;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::{
	Action, Actor, Handles, IntoActorKey, TypedActorConnection, TypedActorHandle,
	registry::Registry, typed_client::encode_action_args,
};

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
/// Uses the same engine resolver as [`Registry::start`](crate::Registry::start):
/// explicit path, `RIVET_ENGINE_BINARY_PATH`, existing healthy engine, local
/// workspace build, cached binary, then opt-in verified download.
pub async fn setup(registry: Registry) -> Result<TestHandle> {
	// A unique pool name keeps concurrently-tested registries on the shared
	// engine from cross-routing each other's actors.
	let pool_name = format!("rivetkit-test-{}", POOL_SEQ.fetch_add(1, Ordering::Relaxed));
	let config = ServeConfig {
		endpoint: ENDPOINT.to_owned(),
		token: Some(TOKEN.to_owned()),
		namespace: NAMESPACE.to_owned(),
		pool_name: pool_name.clone(),
		..ServeConfig::default()
	};

	let shutdown = CancellationToken::new();
	let serve = {
		// Hold the lock until the engine port accepts connections so a second
		// concurrent `setup` reuses this engine instead of spawning its own.
		let _guard = ENGINE_LOCK.lock().await;
		let (ready_tx, ready_rx) = oneshot::channel();
		let serve = tokio::spawn({
			let shutdown = shutdown.clone();
			async move {
				registry
					.serve_with_config_and_handle_observer(config, shutdown, move |_| {
						let _ = ready_tx.send(());
					})
					.await
			}
		});
		wait_for_port().await;
		ready_rx.await.context("wait for registry envoy startup")?;
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
	pub fn actor<A: Actor>(&self, name: &str) -> TestActor<A> {
		self.actor_with_key(name, vec![format!("{name}-{}", unique_suffix())])
	}

	/// The actor of `name` with an explicit key.
	pub fn actor_with_key<A: Actor>(&self, name: &str, key: impl IntoActorKey) -> TestActor<A> {
		self.actor_with_options(name, key, GetOrCreateOptions::default())
	}

	/// The actor of `name` with an explicit key and creation/connection options.
	pub fn actor_with_options<A: Actor>(
		&self,
		name: &str,
		key: impl IntoActorKey,
		opts: GetOrCreateOptions,
	) -> TestActor<A> {
		TestActor {
			handle: TypedActorHandle::new(
				self.client
					.get_or_create(name, key.into_actor_key(), opts)
					.expect("build actor handle"),
			),
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
pub struct TestActor<A: Actor> {
	handle: TypedActorHandle<A>,
}

impl<A: Actor> TestActor<A> {
	pub async fn action(&self, name: &str, args: Vec<JsonValue>) -> Result<JsonValue> {
		let deadline = Instant::now() + READY_TIMEOUT;
		loop {
			match self.handle.inner().action(name, args.clone()).await {
				Ok(value) => return Ok(value),
				Err(error) if is_transient(&error) && Instant::now() < deadline => {
					tokio::time::sleep(Duration::from_millis(250)).await;
				}
				Err(error) => return Err(error),
			}
		}
	}

	pub async fn send<M>(&self, action: M) -> Result<M::Output>
	where
		A: Handles<M>,
		M: Action,
	{
		let args = encode_action_args(&action)?;
		let output = self.action(M::NAME, args).await?;
		serde_json::from_value(output)
			.map_err(anyhow::Error::from)
			.context("decode typed test action output")
	}

	/// The underlying typed client actor handle (for connections, events, etc.).
	pub fn handle(&self) -> &TypedActorHandle<A> {
		&self.handle
	}

	/// Opens a typed connection to this actor.
	pub fn connect(&self) -> TypedActorConnection<A> {
		self.handle.connect()
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

fn unique_suffix() -> u128 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_nanos())
		.unwrap_or_default()
}
