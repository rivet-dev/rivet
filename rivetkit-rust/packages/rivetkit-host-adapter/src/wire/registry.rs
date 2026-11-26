//! Registry handles: register actor types, then serve them.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use futures::FutureExt;
use parking_lot::Mutex;
use rivetkit_core::{ActorConfig, ActorConfigInput, CoreRegistry, EngineSpawnMode, ServeConfig};
use tokio_util::sync::CancellationToken;

use super::WireHost;
use super::messages::{Created, RegisterOptions, ServeOptions, encode};
use super::runtime::{Answer, Handle, Wire};
use crate::config::AdapterConfig;
use crate::event_loop::actor_factory;

pub(crate) struct RegistryHandle {
	/// Taken by `serve`, so a registry serves at most once.
	registry: Mutex<Option<CoreRegistry>>,
	shutdown: CancellationToken,
	done: Mutex<Done>,
}

#[derive(Default)]
struct Done {
	/// `Some` once serving stopped: `Err` holds the message.
	result: Option<std::result::Result<(), String>>,
	waiters: Vec<u64>,
}

impl RegistryHandle {
	pub fn stop(&self) {
		self.shutdown.cancel();
	}

	/// Called when the host releases the registry. Only the serve task settles `done`
	/// waiters, so a registry that never served settles them here instead of leaving
	/// them waiting forever.
	pub(crate) fn released(&self, wire: &Wire) {
		self.stop();
		if self.registry.lock().is_none() {
			return; // serving: its task settles the waiters as it stops
		}
		let message = "registry released before it served".to_owned();
		let waiters = {
			let mut done = self.done.lock();
			done.result = Some(Err(message.clone()));
			std::mem::take(&mut done.waiters)
		};
		for token in waiters {
			wire.complete(token, Err(anyhow!(message.clone())));
		}
	}
}

pub(crate) fn new(wire: &Wire) -> Answer {
	let id = wire.insert(
		0,
		Handle::Registry(Arc::new(RegistryHandle {
			registry: Mutex::new(Some(CoreRegistry::new())),
			shutdown: CancellationToken::new(),
			done: Mutex::new(Done::default()),
		})),
	);
	Answer::header(encode(&Created { handle: id }))
}

fn registry(wire: &Wire, id: u64) -> Result<Arc<RegistryHandle>> {
	wire.get(id, "registry", |handle| match handle {
		Handle::Registry(registry) => Some(registry.clone()),
		_ => None,
	})
}

pub(crate) fn register(wire: &Arc<Wire>, id: u64, options: RegisterOptions) -> Result<()> {
	if options.name.is_empty() {
		bail!("actor name must not be empty");
	}
	let mut config = ActorConfig::from_input(ActorConfigInput {
		action_timeout_ms: options.action_timeout_ms,
		sleep_timeout_ms: options.sleep_timeout_ms,
		no_sleep: options.no_sleep,
		..ActorConfigInput::default()
	});
	// SQLite is the durable storage API, like the Rust SDK.
	config.has_state = false;
	config.has_database = true;
	config.remote_sqlite = !cfg!(feature = "sqlite-local");

	let mut adapter = AdapterConfig::new(options.hooks, config.action_timeout);
	if let Some(ms) = options.hook_timeout_ms {
		let timeout = Duration::from_millis(u64::from(ms));
		adapter.on_migrate_timeout = timeout;
		adapter.create_vars_timeout = timeout;
		adapter.on_create_timeout = timeout;
		adapter.on_wake_timeout = timeout;
		adapter.on_before_connect_timeout = timeout;
		adapter.on_connect_timeout = timeout;
	}
	if let Some(ms) = options.request_timeout_ms {
		adapter.on_request_timeout = Duration::from_millis(u64::from(ms));
	}

	let factory = actor_factory(
		config,
		adapter,
		Arc::new(WireHost::new(wire, options.actor_type)),
	);
	let registry = registry(wire, id)?;
	let mut core = registry.registry.lock();
	let core = core
		.as_mut()
		.ok_or_else(|| anyhow!("registry {id} is already serving"))?;
	core.register(&options.name, factory);
	Ok(())
}

/// Starts serving. `token` completes once the first envoy connects, or with the
/// startup error.
pub(crate) fn serve(wire: &Arc<Wire>, id: u64, options: ServeOptions, token: u64) -> Result<()> {
	let config = serve_config(options)?;
	let handle = registry(wire, id)?;
	let core = handle
		.registry
		.lock()
		.take()
		.with_context(|| format!("registry {id} is already serving"))?;
	let shutdown = handle.shutdown.clone();
	let wire = wire.clone();
	wire.tokio.clone().spawn(async move {
		let ready = Arc::new(Mutex::new(Some(token)));
		let on_ready = {
			let ready = ready.clone();
			let wire = wire.clone();
			move |_| {
				if let Some(token) = ready.lock().take() {
					wire.complete(token, Ok(Answer::empty()));
				}
			}
		};
		// A panic in core still settles the ready and done waiters below.
		let result = std::panic::AssertUnwindSafe(
			core.serve_with_config_and_handle_observer(config, shutdown, on_ready),
		)
		.catch_unwind()
		.await
		.unwrap_or_else(|_| Err(anyhow!("serving panicked")));
		let message = result.as_ref().err().map(|error| format!("{error:#}"));
		// Serving ended before an envoy connected: the ready waiter gets the cause.
		if let Some(token) = ready.lock().take() {
			let message = message
				.clone()
				.unwrap_or_else(|| "registry stopped before envoy startup".to_owned());
			wire.complete(token, Err(anyhow!(message)));
		}
		let waiters = {
			let mut done = handle.done.lock();
			done.result = Some(message.clone().map_or(Ok(()), Err));
			std::mem::take(&mut done.waiters)
		};
		for token in waiters {
			wire.complete(token, done_answer(&message));
		}
	});
	Ok(())
}

fn done_answer(message: &Option<String>) -> Result<Answer> {
	match message {
		None => Ok(Answer::empty()),
		Some(message) => Err(anyhow!(message.clone())),
	}
}

/// Completes `token` when serving stops.
pub(crate) fn done(wire: &Wire, id: u64, token: u64) -> Result<()> {
	let handle = registry(wire, id)?;
	let finished = {
		let mut done = handle.done.lock();
		match &done.result {
			Some(result) => Some(result.clone().err()),
			None => {
				done.waiters.push(token);
				None
			}
		}
	};
	if let Some(message) = finished {
		wire.complete(token, done_answer(&message));
	}
	Ok(())
}

pub(crate) fn stop(wire: &Wire, id: u64) -> Result<()> {
	registry(wire, id)?.stop();
	Ok(())
}

/// Starts from the environment (RIVET_ENDPOINT, RIVET_TOKEN, RIVET_NAMESPACE,
/// RIVET_POOL, ...), like the Rust SDK, then applies the host's overrides.
fn serve_config(options: ServeOptions) -> Result<ServeConfig> {
	let mut config = ServeConfig::from_env()?;
	if let Some(endpoint) = options.endpoint {
		config.endpoint = endpoint;
	}
	if options.token.is_some() {
		config.token = options.token;
	}
	if let Some(namespace) = options.namespace {
		config.namespace = namespace;
	}
	if let Some(pool_name) = options.pool_name {
		config.pool_name = pool_name;
	}
	match options.engine_spawn.as_deref() {
		None => {}
		Some("auto") => config.engine_spawn = EngineSpawnMode::Auto,
		Some("always") => config.engine_spawn = EngineSpawnMode::Always,
		Some("never") => config.engine_spawn = EngineSpawnMode::Never,
		Some(other) => bail!("unknown engineSpawn `{other}`; expected auto, always or never"),
	}
	Ok(config)
}
