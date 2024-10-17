use anyhow::*;
use clap::Parser;
use std::{future::Future, time::Duration};

#[derive(Parser)]
pub struct Opts {
	#[arg(long, value_enum)]
	services: Vec<ServiceType>,
}

#[derive(clap::ValueEnum, Clone, PartialEq)]
enum ServiceType {
	#[cfg(feature = "api")]
	Api,
	#[cfg(feature = "api-internal")]
	ApiInternal,
	#[cfg(feature = "standalone")]
	Standalone,
	#[cfg(feature = "oneshot")]
	Oneshot,
	#[cfg(feature = "cron")]
	Cron,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let all_services = self.services.is_empty();

		start_health_and_metrics_servers().await?;

		let mut runner = ServiceRunner::new();

		#[cfg(feature = "api")]
		if all_services || self.services.contains(&ServiceType::Api) {
			runner.run_service("api_monolith", || api_monolith::start())?;
		}

		#[cfg(feature = "api-internal")]
		if all_services || self.services.contains(&ServiceType::ApiInternal) {
			runner.run_service("api_internal_monolith", || api_internal_monolith::start())?;
		}

		#[cfg(feature = "standalone")]
		if all_services || self.services.contains(&ServiceType::Standalone) {
			runner.run_service("pegboard_ws", || pegboard_ws::start())?;
			runner.run_service("monolith_worker", || monolith_worker::start())?;
			runner.run_service("monolith_workflow_worker", || {
				monolith_workflow_worker::start()
			})?;
			runner.run_service("pegboard_gc", || pegboard_gc::start())?;
			runner.run_service("nomad_monitor", || nomad_monitor::start())?;
			runner.run_service("cluster_metrics_publish", || {
				cluster_metrics_publish::start()
			})?;
			runner.run_service("cluster_gc", || cluster_gc::start())?;
			runner.run_service("cluster_datacenter_tls_renew", || {
				cluster_datacenter_tls_renew::start()
			})?;
			runner.run_service("linode_gc", || linode_gc::start())?;
			runner.run_service("workflow_metrics_publish", || {
				workflow_metrics_publish::start()
			})?;
			runner.run_service("workflow_gc", || workflow_gc::start())?;
			runner.run_service("mm_gc", || mm_gc::start())?;
			runner.run_service("job_gc", || job_gc::start())?;
			runner.run_service("user_delete_pending", || user_delete_pending::start())?;

			std::future::pending::<()>().await;
		}

		#[cfg(feature = "oneshot")]
		if all_services || self.services.contains(&ServiceType::Oneshot) {
			runner.run_oneoff("build_default_create", || build_default_create::start())?;
			runner.run_oneoff("pegboard_dc_init", || pegboard_dc_init::start())?;
			runner.run_oneoff("cluster_default_update", || {
				cluster_default_update::start(false)
			})?;
			runner.run_oneoff("cluster_workflow_backfill", || {
				cluster_workflow_backfill::start()
			})?;
		}

		#[cfg(feature = "cron")]
		if all_services || self.services.contains(&ServiceType::Cron) {
			runner.run_oneoff("telemetry_beacon", || telemetry_beacon::start())?;
			runner.run_oneoff("user_delete_pending", || user_delete_pending::start())?;
		}

		// runner.run_service("load_test_mm_sustain", || load_test_mm_sustain::start());
		// runner.run_service("load_test_mm", || load_test_mm::start());
		// runner.run_service("load_test_sqlx", || load_test_sqlx::start());
		// runner.run_service("load_test_watch_requests", || {
		// 	load_test_watch_requests::start()
		// });
		// runner.run_service("load_test_api_cloud", || load_test_api_cloud::start());

		runner.run().await;

		Ok(())
	}
}

/// Runs services & waits for completion.
///
/// Useful in order to allow for easily configuring an entrypoint where a custom set of services
/// run.
struct ServiceRunner {
	join_set: tokio::task::JoinSet<()>,
}

impl ServiceRunner {
	fn new() -> Self {
		Self {
			join_set: tokio::task::JoinSet::new(),
		}
	}

	/// Spawns a service that will run indefinitely.
	///
	/// If crashes or exits, will be restarted.
	fn run_service<F, Fut>(&mut self, service: &'static str, cb: F) -> Result<()>
	where
		F: Fn() -> Fut + Send + 'static,
		Fut: Future<Output = global_error::GlobalResult<()>> + Send + 'static,
	{
		self.join_set
			.build_task()
			.name(&format!("rivet::service::{service}"))
			.spawn(async move {
				loop {
					tracing::info!(%service, "starting service");

					match cb().await {
						Result::Ok(_) => {
							tracing::error!(%service, "service exited unexpectedly");
						}
						Err(err) => {
							tracing::error!(%service, ?err, "service crashed");
						}
					}

					tokio::time::sleep(Duration::from_secs(1)).await;
				}
			})
			.context("failed to spawn service")?;
		Ok(())
	}

	/// Runs a task that will exit upon completion.
	///
	/// If crashes, it will be retried indefinitely.
	fn run_oneoff<F, Fut>(&mut self, oneoff: &'static str, cb: F) -> Result<()>
	where
		F: Fn() -> Fut + Send + 'static,
		Fut: Future<Output = global_error::GlobalResult<()>> + Send + 'static,
	{
		self.join_set
			.build_task()
			.name(&format!("rivet::oneoff::{oneoff}"))
			.spawn(async move {
				loop {
					tracing::info!(%oneoff, "starting oneoff");

					match cb().await {
						Result::Ok(_) => {
							tracing::error!(%oneoff, "oneoff finished");
							break;
						}
						Err(err) => {
							tracing::error!(%oneoff, ?err, "oneoff crashed");
						}
					}

					tokio::time::sleep(Duration::from_secs(1)).await;
				}
			})
			.context("failed to spawn oneoff")?;
		Ok(())
	}

	async fn run(self) {
		self.join_set.join_all().await;
		tracing::info!("all services finished");
		tokio::time::sleep(Duration::from_secs(200)).await;
	}
}

async fn start_health_and_metrics_servers() -> Result<()> {
	let pools = rivet_pools::from_env().await?;

	tokio::task::Builder::new()
		.name("rivet::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("rivet::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	Ok(())
}
