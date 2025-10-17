use std::time::Duration;

use anyhow::*;
use clap::Parser;
use rivet_service_manager::{CronConfig, RunConfig};

// 7 day logs retention
const LOGS_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Parser)]
pub struct Opts {
	#[arg(long, value_enum)]
	services: Vec<ServiceKind>,

	/// Exclude the specified services instead of including them
	#[arg(long)]
	except_services: Vec<ServiceKind>,
}

#[derive(clap::ValueEnum, Clone, PartialEq)]
enum ServiceKind {
	ApiPublic,
	ApiPeer,
	Standalone,
	Singleton,
	Oneshot,
	Cron,
}

impl From<ServiceKind> for rivet_service_manager::ServiceKind {
	fn from(val: ServiceKind) -> Self {
		use ServiceKind::*;
		match val {
			ApiPublic => rivet_service_manager::ServiceKind::ApiPublic,
			ApiPeer => rivet_service_manager::ServiceKind::ApiPeer,
			Standalone => rivet_service_manager::ServiceKind::Standalone,
			Singleton => rivet_service_manager::ServiceKind::Singleton,
			Oneshot => rivet_service_manager::ServiceKind::Oneshot,
			Cron => rivet_service_manager::ServiceKind::Cron(CronConfig::default()),
		}
	}
}

impl Opts {
	pub async fn execute(
		&self,
		config: rivet_config::Config,
		run_config: &RunConfig,
	) -> Result<()> {
		// Redirect logs if enabled on the edge
		if let Some(logs_dir) = config.logs().redirect_logs_dir.as_ref() {
			rivet_logs::Logs::new(logs_dir.clone(), LOGS_RETENTION)
				.start()
				.await?;
		}

		// Select services to run
		let services = if self.services.is_empty() && self.except_services.is_empty() {
			// Run all services
			run_config.services.clone()
		} else if !self.except_services.is_empty() {
			// Exclude specified services
			let except_service_kinds = self
				.except_services
				.iter()
				.map(|x| x.clone().into())
				.collect::<Vec<rivet_service_manager::ServiceKind>>();

			run_config
				.services
				.iter()
				.filter(|x| !except_service_kinds.iter().any(|y| y.eq(&x.kind)))
				.cloned()
				.collect::<Vec<_>>()
		} else {
			// Include only specified services
			let service_kinds = self
				.services
				.iter()
				.map(|x| x.clone().into())
				.collect::<Vec<rivet_service_manager::ServiceKind>>();

			run_config
				.services
				.iter()
				.filter(|x| service_kinds.iter().any(|y| y.eq(&x.kind)))
				.cloned()
				.collect::<Vec<_>>()
		};

		// Start server
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		rivet_service_manager::start(config, pools, services).await?;

		Ok(())
	}
}
