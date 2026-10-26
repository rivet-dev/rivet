use std::{future::Future, sync::Arc};

use anyhow::Result;
use rivet_error::RivetError;
use rivetkit_core::metrics::{RenderedMetrics, render_prometheus_metrics};
use rivetkit_core::registry::CoreEnvoyHandle;
use rivetkit_core::serverless::CoreServerlessRuntime;
use rivetkit_core::{
	ActorConfig, ActorFactory as CoreActorFactory, ActorStart, CoreRegistry, RuntimeMode,
	ServeConfig,
};
use tokio_util::sync::CancellationToken;

use crate::{
	actor::Actor,
	serverless_listener,
	start::{Start, run_actor, wrap_start},
};

pub struct Registry {
	inner: CoreRegistry,
}

impl Registry {
	pub fn new() -> Self {
		Self {
			inner: CoreRegistry::new(),
		}
	}

	#[deprecated(
		note = "use register_actor/register_actor_with and implement Actor + Handles instead"
	)]
	pub fn register<A, F, Fut>(&mut self, name: &str, entry: F) -> &mut Self
	where
		A: Actor,
		F: Fn(Start<A>) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<()>> + Send + 'static,
	{
		#[allow(deprecated)]
		self.register_with::<A, F, Fut>(name, ActorConfig::default(), entry)
	}

	pub fn register_actor<A>(&mut self, name: &str) -> &mut Self
	where
		A: Actor,
	{
		self.register_actor_with::<A>(name, ActorConfig::default())
	}

	pub fn register_actor_with<A>(&mut self, name: &str, config: ActorConfig) -> &mut Self
	where
		A: Actor,
	{
		#[allow(deprecated)]
		self.register_with::<A, _, _>(name, config, run_actor::<A>)
	}

	#[deprecated(
		note = "use register_actor/register_actor_with and implement Actor + Handles instead"
	)]
	pub fn register_with<A, F, Fut>(
		&mut self,
		name: &str,
		config: ActorConfig,
		entry: F,
	) -> &mut Self
	where
		A: Actor,
		F: Fn(Start<A>) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<()>> + Send + 'static,
	{
		self.inner
			.register(name, build_factory::<A, F, Fut>(config, entry));
		self
	}

	pub async fn serve(self, shutdown: CancellationToken) -> Result<()> {
		self.inner.serve(shutdown).await
	}

	pub async fn serve_with_config(
		self,
		config: ServeConfig,
		shutdown: CancellationToken,
	) -> Result<()> {
		self.inner.serve_with_config(config, shutdown).await
	}

	pub(crate) async fn serve_with_config_and_handle_observer(
		self,
		config: ServeConfig,
		shutdown: CancellationToken,
		on_handle: impl FnOnce(CoreEnvoyHandle) + Send + 'static,
	) -> Result<()> {
		self.inner
			.serve_with_config_and_handle_observer(config, shutdown, on_handle)
			.await
	}

	/// Converts the registry into a serverless runtime. The returned runtime
	/// lazily starts an envoy on first request and handles RivetKit serverless
	/// HTTP requests (start, metadata, health, metrics) via
	/// [`CoreServerlessRuntime::handle_request`]. Equivalent to the TypeScript
	/// `registry.handler` entry point for platform fetch handlers.
	pub async fn into_serverless_runtime(
		self,
		config: ServeConfig,
	) -> Result<CoreServerlessRuntime> {
		self.inner.into_serverless_runtime(config).await
	}

	/// Renders the process-global Prometheus metrics. Equivalent to the
	/// TypeScript `registry.routes.prometheusMetrics()`. Metrics are
	/// process-global, so this does not depend on registry instance state.
	pub fn prometheus_metrics(&self) -> Result<RenderedMetrics> {
		render_prometheus_metrics()
	}

	/// Serves actors until the process receives SIGINT or SIGTERM, then cancels
	/// and drains. This is the standalone-binary entry point and mirrors the
	/// TypeScript `registry.start()`. Connection settings (including the
	/// `RIVET_ENGINE_BINARY_PATH` used to spawn or reuse a local engine) are
	/// read from the environment.
	///
	/// Unlike TypeScript, this blocks until shutdown because Rust has no
	/// implicit runtime to keep the process alive. For programmatic lifecycle
	/// control (tests, embedding), drive [`serve`](Self::serve) with your own
	/// [`CancellationToken`] instead.
	///
	/// Flushes pending telemetry spans before returning.
	pub async fn start(self) -> Result<()> {
		self.start_with_config(ServeConfig::from_env()?).await
	}

	/// [`start`](Self::start) with an explicit [`ServeConfig`].
	///
	/// Selects the run mode from `RIVETKIT_RUNTIME_MODE` (see [`RuntimeMode`]),
	/// mirroring the TypeScript `registry.start()`. `Envoy` (default) holds one
	/// long-lived outbound envoy; `Serverless` runs an HTTP listener that lazily
	/// starts and caches an envoy on the first request.
	///
	/// Flushes pending telemetry spans before returning.
	pub async fn start_with_config(self, config: ServeConfig) -> Result<()> {
		let result = match RuntimeMode::from_env() {
			RuntimeMode::Envoy => self.start_envoy(config).await,
			RuntimeMode::Serverless => self.start_serverless(config).await,
		};
		crate::telemetry::shutdown().await;
		result
	}

	/// Persistent-envoy `start`: serves until SIGINT/SIGTERM, then drains.
	async fn start_envoy(self, config: ServeConfig) -> Result<()> {
		let shutdown = CancellationToken::new();
		let mut serve = tokio::spawn({
			let shutdown = shutdown.clone();
			async move { self.serve_with_config(config, shutdown).await }
		});

		tokio::select! {
			// Surface an early serve failure instead of waiting for a signal.
			result = &mut serve => return result?,
			_ = shutdown_signal() => {}
		}

		shutdown.cancel();
		serve.await?
	}

	/// Serverless `start`: runs the HTTP listener until SIGINT/SIGTERM, then
	/// drains the cached envoy.
	async fn start_serverless(self, config: ServeConfig) -> Result<()> {
		let runtime = self.into_serverless_runtime(config).await?;
		let shutdown = CancellationToken::new();
		let mut serve = tokio::spawn({
			let shutdown = shutdown.clone();
			async move { serverless_listener::serve(runtime, shutdown).await }
		});

		tokio::select! {
			// Surface an early listener failure instead of waiting for a signal.
			result = &mut serve => return result?,
			_ = shutdown_signal() => {}
		}

		shutdown.cancel();
		serve.await?
	}
}

/// Resolves when the process receives SIGINT or, on Unix, SIGTERM.
async fn shutdown_signal() {
	#[cfg(unix)]
	{
		use tokio::signal::unix::{SignalKind, signal};

		let mut terminate = match signal(SignalKind::terminate()) {
			Ok(terminate) => terminate,
			Err(error) => {
				tracing::warn!(?error, "failed to install SIGTERM handler");
				let _ = tokio::signal::ctrl_c().await;
				return;
			}
		};

		tokio::select! {
			_ = tokio::signal::ctrl_c() => {}
			_ = terminate.recv() => {}
		}
	}

	#[cfg(not(unix))]
	{
		let _ = tokio::signal::ctrl_c().await;
	}
}

impl Default for Registry {
	fn default() -> Self {
		Self::new()
	}
}

fn build_factory<A, F, Fut>(config: ActorConfig, entry: F) -> CoreActorFactory
where
	A: Actor,
	F: Fn(Start<A>) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<()>> + Send + 'static,
{
	let config = actor_config::<A>(config);
	let entry = Arc::new(entry);
	CoreActorFactory::new_with_manual_startup_ready(config, move |core_start: ActorStart| {
		let entry = Arc::clone(&entry);
		Box::pin(async move {
			let mut core_start = core_start;
			let startup_ready = core_start.startup_ready.take();
			match wrap_start::<A>(core_start) {
				Ok(mut start) => {
					start.startup_ready = startup_ready;
					entry(start).await
				}
				Err(error) => {
					if let Some(reply) = startup_ready {
						let startup_error = anyhow::Error::new(RivetError::extract(&error));
						let _ = reply.send(Err(startup_error));
					}
					Err(error)
				}
			}
		})
	})
}

fn actor_config<A: Actor>(mut config: ActorConfig) -> ActorConfig {
	config.has_database |= A::HAS_DATABASE;
	// Internal storage (state, KV, queue) always uses SQLite, so without local
	// SQLite compiled in every actor must route it through the engine.
	if !cfg!(feature = "sqlite-local") {
		config.remote_sqlite = true;
	}
	config
}


