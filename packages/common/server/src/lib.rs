use anyhow::*;
use global_error::GlobalResult;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

#[derive(Clone)]
pub struct Service {
	pub name: &'static str,
	pub kind: ServiceKind,
	pub run: Arc<
		dyn Fn(
				rivet_config::Config,
				rivet_pools::Pools,
			) -> Pin<Box<dyn Future<Output = GlobalResult<()>> + Send>>
			+ Send
			+ Sync,
	>,
}

impl Service {
	pub fn new<F, Fut>(name: &'static str, kind: ServiceKind, run: F) -> Self
	where
		F: Fn(rivet_config::Config, rivet_pools::Pools) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = GlobalResult<()>> + Send + 'static,
	{
		Self {
			name,
			kind,
			run: Arc::new(move |config, pools| Box::pin(run(config, pools))),
		}
	}
}

/// Defines the type of the service. Used for filtering service types to run.
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceKind {
	Api,
	ApiInternal,
	Standalone,
	Singleton,
	Oneshot,
	Cron,
}

impl ServiceKind {
	fn behavior(&self) -> ServiceBehavior {
		use ServiceKind::*;

		match self {
			Api | ApiInternal | Standalone | Singleton => ServiceBehavior::Service,
			Oneshot | Cron => ServiceBehavior::Oneshot,
		}
	}
}

/// Defines how a service should be ran.
#[derive(Debug, Clone)]
enum ServiceBehavior {
	/// Spawns a service that will run indefinitely.
	///
	/// If crashes or exits, will be restarted.
	Service,
	/// Runs a task that will exit upon completion.
	///
	/// If crashes, it will be retried indefinitely.
	Oneshot,
}

/// Runs services & waits for completion.
///
/// Useful in order to allow for easily configuring an entrypoint where a custom set of services
/// run.
pub async fn start(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	services: Vec<Service>,
) -> Result<()> {
	tracing::info!(services = ?services.len(), "starting server");

	// Spawn services
	let mut join_set = tokio::task::JoinSet::new();
	for service in services {
		tracing::info!(name = %service.name, kind = ?service.kind, "server starting service");

		match service.kind.behavior() {
			ServiceBehavior::Service => {
				join_set
					.build_task()
					.name(&format!("rivet::service::{}", service.name))
					.spawn({
						let config = config.clone();
						let pools = pools.clone();
						async move {
							loop {
								tracing::info!(service = %service.name, "starting service");

								match (service.run)(config.clone(), pools.clone()).await {
									Result::Ok(_) => {
										tracing::error!(service = %service.name, "service exited unexpectedly");
									}
									Err(err) => {
										tracing::error!(service = %service.name, ?err, "service crashed");
									}
								}

								tokio::time::sleep(Duration::from_secs(1)).await;
							}
						}
					})
					.context("failed to spawn service")?;
			}
			ServiceBehavior::Oneshot => {
				join_set
					.build_task()
					.name(&format!("rivet::oneoff::{}", service.name))
					.spawn({
						let config = config.clone();
						let pools = pools.clone();
						async move {
							loop {
								tracing::info!(oneoff = %service.name, "starting oneoff");

								match (service.run)(config.clone(), pools.clone()).await {
									Result::Ok(_) => {
										tracing::error!(oneoff = %service.name, "oneoff finished");
										break;
									}
									Err(err) => {
										tracing::error!(oneoff = %service.name, ?err, "oneoff crashed");
									}
								}

								tokio::time::sleep(Duration::from_secs(1)).await;
							}
						}
					})
					.context("failed to spawn oneoff")?;
			}
		}
	}

	// Run health & metrics servers
	rivet_health_checks::spawn_standalone(rivet_health_checks::Config {
		config: config.clone(),
		pools: Some(pools.clone()),
	})
	.map_err(|err| anyhow!("failed to spawn health checks: {err}"))?;
	rivet_metrics::spawn_standalone(config.clone())
		.map_err(|err| anyhow!("failed to spawn metrics: {err}"))?;

	// Wait for services
	join_set.join_all().await;
	tracing::info!("all services finished");

	Ok(())
}
