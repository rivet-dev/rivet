use std::{
	future::Future,
	pin::Pin,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

use anyhow::{Result, ensure};
use futures_util::{StreamExt, stream::FuturesUnordered};
use rivet_env::TaskHandle;

/// Keep force-shutdown policy active across startup, service execution, and cleanup.
pub async fn with_shutdown<F: Future>(
	env: Arc<dyn rivet_env::Env>,
	mut shutdown: rivet_env::capabilities::shutdown::Shutdown,
	force_after: Duration,
	work: F,
) -> F::Output {
	let watchdog = async {
		use rivet_env::capabilities::shutdown::ShutdownEvent;
		let result = async {
			if shutdown.recv().await? == ShutdownEvent::Force {
				return Ok::<(), rivet_env::ClockError>(());
			}
			let deadline = shutdown.deadline(force_after)?;
			let clock = env.clock();
			loop {
				rivet_env::select_biased!("force requests and the original deadline bound shutdown",
					event = shutdown.recv() => {
						if event? == ShutdownEvent::Force { return Ok(()); }
					},
					result = clock.sleep_until(deadline) => return result,
				);
			}
		}
		.await;
		if let Err(error) = result {
			tracing::error!(%error, "shutdown clock failed");
		}
		tracing::warn!("forcing process shutdown");
		env.exit(1).await
	};
	rivet_env::select_biased!("force shutdown remains active even while startup or cleanup is blocked",
		never = watchdog => match never {},
		output = work => output,
	)
}

#[derive(Clone)]
pub struct Service {
	pub name: &'static str,
	pub class: ServiceClass,
	pub run: Arc<
		dyn Fn(
				rivet_config::Config,
				rivet_engine_context::EngineContext,
			) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
			+ Send
			+ Sync,
	>,
	pub requires_graceful_shutdown: bool,
}

impl Service {
	pub fn new<F, Fut>(
		name: &'static str,
		class: ServiceClass,
		run: F,
		requires_graceful_shutdown: bool,
	) -> Self
	where
		F: Fn(rivet_config::Config, rivet_engine_context::EngineContext) -> Fut
			+ Send
			+ Sync
			+ 'static,
		Fut: Future<Output = Result<()>> + Send + 'static,
	{
		Self {
			name,
			class,
			run: Arc::new(move |config, context| Box::pin(run(config, context))),
			requires_graceful_shutdown,
		}
	}
}

/// Controls service selection and execution lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceClass {
	/// Selected for stateless workloads.
	Stateless,
	/// Selected for storage workloads.
	Storage,
	/// Runs regardless of the selected workload.
	Always,
	/// Runs only inside an exclusive scope granted by a cluster singleton host.
	ClusterSingleton,
	/// Retries until successful, then stops for this process lifetime.
	Once,
}

pub type RunConfig = Arc<RunConfigData>;

pub struct RunConfigData {
	pub services: Vec<Service>,
}

impl RunConfigData {
	/// Replaces an existing service. Throws an error if cannot find service.
	pub fn replace_service(&mut self, service: Service) -> Result<()> {
		let old_len = self.services.len();
		self.services.retain(|x| x.name != service.name);
		ensure!(
			self.services.len() < old_len,
			"could not find instance of service {} to replace",
			service.name
		);
		self.services.push(service);
		Ok(())
	}
}

struct ServiceTask {
	name: String,
	handle: Option<TaskHandle>,
	requires_graceful_shutdown: bool,
}

/// Run services within the caller's ownership lifetime. Dropping this future
/// drops every service future; no detached tasks survive loss of that ownership.
pub async fn run_scoped(
	config: rivet_config::Config,
	context: rivet_engine_context::EngineContext,
	services: Vec<Service>,
) -> Result<()> {
	let shutting_down = Arc::new(AtomicBool::new(false));
	let mut running = services
		.into_iter()
		.map(|service| {
			supervise(
				service,
				config.clone(),
				context.clone(),
				shutting_down.clone(),
			)
		})
		.collect::<FuturesUnordered<_>>();
	while running.next().await.is_some() {}
	std::future::pending().await
}

async fn supervise(
	service: Service,
	config: rivet_config::Config,
	context: rivet_engine_context::EngineContext,
	shutting_down: Arc<AtomicBool>,
) {
	use futures_util::FutureExt;
	loop {
		// Catch panics as well as returned errors without detaching the future.
		let result = std::panic::AssertUnwindSafe(async {
			(service.run)(config.clone(), context.clone()).await
		})
		.catch_unwind()
		.await;
		if shutting_down.load(Ordering::SeqCst) {
			return;
		}
		match &result {
			Ok(Ok(())) => tracing::warn!(
				service = service.name,
				"service returned before shutdown; restarting"
			),
			Ok(Err(error)) => {
				tracing::error!(service = service.name, ?error, "service failed; restarting")
			}
			Err(_) => tracing::error!(service = service.name, "service panicked; restarting"),
		}
		restart_delay(&context).await;
	}
}

/// Runs services & waits for completion.
///
/// Useful in order to allow for easily configuring an entrypoint where a custom set of services
/// run.
pub async fn start(
	config: rivet_config::Config,
	context: rivet_engine_context::EngineContext,
	mut services: Vec<Service>,
	registration: Option<rivet_topology::WorkerRegistration>,
) -> Result<()> {
	ensure!(
		services
			.iter()
			.all(|s| s.class != ServiceClass::ClusterSingleton),
		"singleton services must be scheduled through a singleton host"
	);
	services.push(Service::new(
		"metrics",
		ServiceClass::Always,
		|config, _context| rivet_metrics_server::run_standalone(config),
		false,
	));

	let mut startup_shutdown = context.shutdown();
	let background = rivet_env::select_biased!("termination cancels registration before any service starts",
		signal = startup_shutdown.recv() => { signal?; return Ok(()); },
		result = context.start_background(registration) => result?,
	);

	// Spawn services
	tracing::info!(services=?services.len(), "starting services");
	let mut running_services = Vec::new();
	let env = context.env().clone();
	let scope = env.task_scope();

	let mut term_signal = context.shutdown();
	let shutting_down = Arc::new(AtomicBool::new(false));

	for service in services {
		match service.class {
			ServiceClass::Stateless
			| ServiceClass::Storage
			| ServiceClass::Always
			| ServiceClass::ClusterSingleton => {
				let config = config.clone();
				let context = context.clone();
				let shutting_down = shutting_down.clone();
				let task_name = format!("rivet::service::{}", service.name);
				let requires_graceful_shutdown = service.requires_graceful_shutdown;

				let join_handle = scope.spawn(async move {
					supervise(service, config, context, shutting_down).await;
				});

				running_services.push(ServiceTask {
					name: task_name,
					handle: Some(join_handle),
					requires_graceful_shutdown,
				});
			}
			ServiceClass::Once => {
				let config = config.clone();
				let context = context.clone();
				let shutting_down = shutting_down.clone();
				let task_name = format!("rivet::oneoff::{}", service.name);

				let join_handle = scope.spawn(async move {
					tracing::debug!(oneoff=%service.name, "starting oneoff");

					loop {
						match (service.run)(config.clone(), context.clone()).await {
							Result::Ok(_) => {
								tracing::debug!(oneoff=%service.name, "oneoff finished");
								break;
							}
							Err(err) => {
								tracing::error!(oneoff=%service.name, ?err, "oneoff crashed");

								if shutting_down.load(Ordering::SeqCst) {
									break;
								} else {
									restart_delay(&context).await;

									tracing::info!(oneoff=%service.name, "restarting oneoff");
								}
							}
						}
					}
				});

				running_services.push(ServiceTask {
					name: task_name,
					handle: Some(join_handle),
					requires_graceful_shutdown: service.requires_graceful_shutdown,
				});
			}
		}
	}

	loop {
		// Waits for all service tasks to complete
		let join_fut = async {
			let mut handle_futs = running_services
				.iter_mut()
				.filter(|task| task.handle.is_some())
				.map(|task| async move {
					let result = task.handle.as_mut().expect("unjoined task").await;
					task.handle.take();
					if let Err(error) = result {
						if !error.is_cancelled() {
							tracing::error!(name = %task.name, %error, "service task failed");
						}
					}
				})
				.collect::<FuturesUnordered<_>>();

			while let Some(_) = handle_futs.next().await {}
		};

		rivet_env::select! { env.rng("services.shutdown.select");
			_ = join_fut => {
				tracing::info!("all services finished");
				break;
			},
			event = term_signal.recv() => {
				event?;

				shutting_down.store(true, Ordering::SeqCst);

				// Abort services that don't require graceful shutdown
				for task in &running_services {
					if !task.requires_graceful_shutdown {
						tracing::debug!(name=%task.name, "aborting service");
						if let Some(handle) = &task.handle { handle.abort(); }
					}

				}

			}
		}
	}

	scope.shutdown().await;
	for task in &mut running_services {
		if let Some(handle) = task.handle.take() {
			if let Err(error) = handle.await {
				if !error.is_cancelled() {
					tracing::error!(name = %task.name, %error, "service task failed during shutdown");
				}
			}
		}
	}

	background.shutdown().await?;
	Ok(())
}

async fn restart_delay(context: &rivet_engine_context::EngineContext) {
	if let Err(error) =
		rivet_env::sleep(context.env().clock().as_ref(), Duration::from_secs(1)).await
	{
		tracing::error!(%error, "service restart clock failed");
		context.env().exit(1).await;
	}
}
