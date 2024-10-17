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

		#[cfg(feature = "api")]
		if all_services || self.services.contains(&ServiceType::Api) {
			spawn_service("api_monolith", || api_monolith::start());
		}

		#[cfg(feature = "api-internal")]
		if all_services || self.services.contains(&ServiceType::ApiInternal) {
			spawn_service("api_internal_monolith", || api_internal_monolith::start());
		}

		#[cfg(feature = "standalone")]
		if all_services || self.services.contains(&ServiceType::Standalone) {
			spawn_service("pegboard_ws", || pegboard_ws::start());
			spawn_service("monolith_worker", || monolith_worker::start());
			spawn_service("monolith_workflow_worker", || {
				monolith_workflow_worker::start()
			});
			spawn_service("pegboard_gc", || pegboard_gc::start());
			spawn_service("nomad_monitor", || nomad_monitor::start());
			spawn_service("load_test_mm_sustain", || load_test_mm_sustain::start());
			spawn_service("cluster_metrics_publish", || {
				cluster_metrics_publish::start()
			});
			spawn_service("load_test_mm", || load_test_mm::start());
			spawn_service("cluster_gc", || cluster_gc::start());
			spawn_service("load_test_sqlx", || load_test_sqlx::start());
			spawn_service("cluster_datacenter_tls_renew", || {
				cluster_datacenter_tls_renew::start()
			});
			spawn_service("load_test_watch_requests", || {
				load_test_watch_requests::start()
			});
			spawn_service("load_test_api_cloud", || load_test_api_cloud::start());
			spawn_service("linode_gc", || linode_gc::start());
			spawn_service("workflow_metrics_publish", || {
				workflow_metrics_publish::start()
			});
			spawn_service("workflow_gc", || workflow_gc::start());
			spawn_service("mm_gc", || mm_gc::start());
			spawn_service("job_gc", || job_gc::start());
			spawn_service("user_delete_pending", || user_delete_pending::start());
		}

		#[cfg(feature = "oneshot")]
		if all_services || self.services.contains(&ServiceType::Oneshot) {
			spawn_service("build_default_create", || build_default_create::start());
			spawn_service("pegboard_dc_init", || pegboard_dc_init::start());
			spawn_service("cluster_default_update", || {
				cluster_default_update::start(false)
			});
			spawn_service("cluster_workflow_backfill", || {
				cluster_workflow_backfill::start()
			});
		}

		#[cfg(feature = "cron")]
		if all_services || self.services.contains(&ServiceType::Cron) {
			spawn_service("telemetry_beacon", || telemetry_beacon::start());
			spawn_service("user_delete_pending", || user_delete_pending::start());
		}

		Result::Ok(())
	}
}

fn spawn_service<F, Fut>(service: &'static str, cb: F)
where
	F: Fn() -> Fut + Send + 'static,
	Fut: Future<Output = global_error::GlobalResult<()>> + Send + 'static,
{
	tokio::task::Builder::new()
		.name(&format!("rivet::spawn_service::{service}"))
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
		.expect("failed to spawn service");
}
