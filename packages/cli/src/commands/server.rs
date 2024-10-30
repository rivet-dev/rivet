use anyhow::*;
use clap::Parser;
use rivet_server::CronConfig;

use crate::run_config::RunConfig;

#[derive(Parser)]
pub struct Opts {
	#[arg(long)]
	skip_provision: bool,
	#[arg(long, value_enum)]
	services: Vec<ServiceKind>,
}

#[derive(clap::ValueEnum, Clone, PartialEq)]
enum ServiceKind {
	ApiPublic,
	ApiEdge,
	ApiPrivate,
	Standalone,
	Singleton,
	Oneshot,
	Cron,
}

impl Into<rivet_server::ServiceKind> for ServiceKind {
	fn into(self) -> rivet_server::ServiceKind {
		use ServiceKind::*;
		match self {
			ApiPublic => rivet_server::ServiceKind::ApiPublic,
			ApiEdge => rivet_server::ServiceKind::ApiEdge,
			ApiPrivate => rivet_server::ServiceKind::ApiPrivate,
			Standalone => rivet_server::ServiceKind::Standalone,
			Singleton => rivet_server::ServiceKind::Singleton,
			Oneshot => rivet_server::ServiceKind::Oneshot,
			Cron => rivet_server::ServiceKind::Cron(CronConfig::default()),
		}
	}
}

impl Opts {
	pub async fn execute(
		&self,
		config: rivet_config::Config,
		run_config: &RunConfig,
	) -> Result<()> {
		// Provision services before starting server
		if !self.skip_provision {
			s3_util::provision(config.clone(), &run_config.s3_buckets).await?;
			rivet_migrate::up(config.clone(), &run_config.sql_services).await?;
		}

		// Select services t orun
		let services = if self.services.is_empty() {
			// Run all services
			run_config.services.clone()
		} else {
			// Filter services
			let service_kinds = self
				.services
				.iter()
				.map(|x| x.clone().into())
				.collect::<Vec<rivet_server::ServiceKind>>();

			run_config
				.services
				.iter()
				.filter(|x| service_kinds.iter().any(|y| *y == x.kind))
				.cloned()
				.collect::<Vec<_>>()
		};

		// Start server
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		rivet_server::start(config, pools, services).await?;

		Ok(())
	}
}
