use anyhow::*;
use clap::Parser;

use crate::run_config::RunConfig;

#[derive(Parser)]
pub struct Opts {
	#[arg(long, value_enum)]
	services: Vec<ServiceKind>,
}

#[derive(clap::ValueEnum, Clone, PartialEq)]
enum ServiceKind {
	Api,
	ApiInternal,
	Standalone,
	Singleton,
	Oneshot,
	Cron,
}

impl Into<rivet_server::ServiceKind> for ServiceKind {
	fn into(self) -> rivet_server::ServiceKind {
		use ServiceKind::*;
		match self {
			Api => rivet_server::ServiceKind::Api,
			ApiInternal => rivet_server::ServiceKind::ApiInternal,
			Standalone => rivet_server::ServiceKind::Standalone,
			Singleton => rivet_server::ServiceKind::Singleton,
			Oneshot => rivet_server::ServiceKind::Oneshot,
			Cron => rivet_server::ServiceKind::Cron,
		}
	}
}

impl Opts {
	pub async fn execute(&self, run_config: &RunConfig) -> Result<()> {
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

		rivet_server::start(services).await?;

		Ok(())
	}
}
