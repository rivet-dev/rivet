use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use anyhow::*;

#[derive(Clone)]
pub struct Service {
	pub name: &'static str,
	pub kind: ServiceKind,
	pub run: Arc<
		dyn Fn(
				rivet_config::Config,
				rivet_pools::Pools,
			) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
			+ Send
			+ Sync,
	>,
}

impl Service {
	pub fn new<F, Fut>(name: &'static str, kind: ServiceKind, run: F) -> Self
	where
		F: Fn(rivet_config::Config, rivet_pools::Pools) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<()>> + Send + 'static,
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
	ApiPublic,
	ApiPeer,
	Standalone,
	Singleton,
	Oneshot,
	Cron(CronConfig),
	/// Run no matter what.
	Core,
}

impl ServiceKind {
	fn behavior(&self) -> ServiceBehavior {
		use ServiceKind::*;

		match self {
			ApiPublic | ApiPeer | Standalone | Singleton | Core => ServiceBehavior::Service,
			Oneshot => ServiceBehavior::Oneshot,
			Cron(config) => ServiceBehavior::Cron(config.clone()),
		}
	}

	pub fn eq(&self, other: &Self) -> bool {
		use ServiceKind::*;

		match (self, other) {
			(ApiPublic, ApiPublic)
			| (ApiPeer, ApiPeer)
			| (Standalone, Standalone)
			| (Singleton, Singleton)
			| (Oneshot, Oneshot)
			| (Core, Core) => true,
			(Cron(_), Cron(_)) => true,
			_ => false,
		}
	}
}

/// Defines how a service should be ran.
#[derive(Debug, Clone, PartialEq)]
enum ServiceBehavior {
	/// Spawns a service that will run indefinitely.
	///
	/// If crashes or exits, will be restarted.
	Service,
	/// Runs a task that will exit upon completion.
	///
	/// If crashes, it will be retried indefinitely.
	Oneshot,
	/// Runs a task on a schedule.
	Cron(CronConfig),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CronConfig {
	pub run_immediately: bool,
	pub schedule: String,
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

/// Runs services & waits for completion.
///
/// Useful in order to allow for easily configuring an entrypoint where a custom set of services
/// run.
pub async fn start(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	services: Vec<Service>,
) -> Result<()> {
	// Spawn services
	tracing::info!(services = ?services.len(), "starting services");
	let mut join_set = tokio::task::JoinSet::new();
	let cron_schedule = tokio_cron_scheduler::JobScheduler::new().await?;
	let mut sleep_indefinitely = false;
	for service in services {
		tracing::debug!(name = %service.name, kind = ?service.kind, "server starting service");

		match service.kind.behavior() {
			ServiceBehavior::Service => {
				join_set
					.build_task()
					.name(&format!("rivet::service::{}", service.name))
					.spawn({
						let config = config.clone();
						let pools = pools.clone();
						async move {
							tracing::debug!(service = %service.name, "starting service");

							loop {
								match (service.run)(config.clone(), pools.clone()).await {
									Result::Ok(_) => {
										tracing::error!(service = %service.name, "service exited unexpectedly");
									}
									Err(err) => {
										tracing::error!(service = %service.name, ?err, "service crashed");
									}
								}

								tokio::time::sleep(Duration::from_secs(1)).await;

								tracing::info!(service = %service.name, "restarting service");
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
							tracing::debug!(oneoff = %service.name, "starting oneoff");

							loop {
								match (service.run)(config.clone(), pools.clone()).await {
									Result::Ok(_) => {
										tracing::debug!(oneoff = %service.name, "oneoff finished");
										break;
									}
									Err(err) => {
										tracing::error!(oneoff = %service.name, ?err, "oneoff crashed");

										tokio::time::sleep(Duration::from_secs(1)).await;

										tracing::info!(oneoff = %service.name, "restarting oneoff");
									}
								}
							}
						}
					})
					.context("failed to spawn oneoff")?;
			}
			ServiceBehavior::Cron(cron_config) => {
				sleep_indefinitely = true;

				// Spawn immediate task
				if cron_config.run_immediately {
					let service = service.clone();
					join_set
						.build_task()
						.name(&format!("rivet::cron_immediate::{}", service.name))
						.spawn({
							let config = config.clone();
							let pools = pools.clone();
							async move {
								tracing::debug!(cron = %service.name, "starting immediate cron");

								for attempt in 1..=8 {
									match (service.run)(config.clone(), pools.clone()).await {
										Result::Ok(_) => {
											tracing::debug!(cron = %service.name, ?attempt, "cron finished");
											break;
										}
										Err(err) => {
											tracing::error!(cron = %service.name, ?attempt, ?err, "cron crashed");

											tokio::time::sleep(Duration::from_secs(1)).await;

											tracing::info!(cron = %service.name, ?attempt, "restarting cron");
										}
									}
								}
							}
						})
						.context("failed to spawn cron")?;
				}

				// Spawn cron
				let config = config.clone();
				let pools = pools.clone();
				let service = service.clone();
				cron_schedule
					.add(tokio_cron_scheduler::Job::new_async_tz(
						&cron_config.schedule,
						chrono::Utc,
						move |notification, _| {
							let config = config.clone();
							let pools = pools.clone();
							let service = service.clone();
							Box::pin(async move {
								tracing::debug!(cron = %service.name, ?notification, "running cron");

								for attempt in 1..=8 {
									match (service.run)(config.clone(), pools.clone()).await {
										Result::Ok(_) => {
											tracing::debug!(cron = %service.name, ?attempt, "cron finished");
											return;
										}
										Err(err) => {
											tracing::error!(cron = %service.name, ?attempt, ?err, "cron crashed");

											tokio::time::sleep(Duration::from_secs(1)).await;

											tracing::info!(cron = %service.name, ?attempt, "restarting cron");
										}
									}
								}
							})
						},
					)?)
					.await?;
			}
		}
	}

	cron_schedule.start().await?;

	if sleep_indefinitely {
		std::future::pending().await
	} else {
		// Wait for services
		join_set.join_all().await;

		// Exit
		tracing::info!("all services finished");

		Ok(())
	}
}
