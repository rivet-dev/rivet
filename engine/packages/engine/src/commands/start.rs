use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use indoc::formatdoc;
use rivet_service_manager::{CronConfig, RunConfig};
use universaldb::utils::IsolationLevel::*;

use crate::keys;

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

		let pools = rivet_pools::Pools::new(config.clone()).await?;

		verify_engine_version(&config, &pools).await?;

		// Start server
		rivet_service_manager::start(config, pools, services).await?;

		Ok(())
	}
}

/// Verifies that no rollback has occurred (if allowing rollback is disabled).
async fn verify_engine_version(
	config: &rivet_config::Config,
	pools: &rivet_pools::Pools,
) -> Result<()> {
	if config.allow_version_rollback {
		return Ok(());
	}

	pools
		.udb()?
		.run(|tx| async move {
			let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
				.context("failed to parse cargo pkg version as semver")?;

			if let Some(existing_version) =
				tx.read_opt(&keys::EngineVersionKey {}, Serializable).await?
			{
				if current_version < existing_version {
					return Ok(Err(anyhow!("{}", formatdoc!(
						"
						Rivet Engine has been rolled back to a previous version:
						  - Last Used Version: {existing_version}
						  - Current Version:   {current_version}
						Cannot proceed without potential data corruption.
						
						(If you know what you're doing, this error can be disabled in the Rivet config via `allow_version_rollback: true`)
						"
					))));
				}
			}

			tx.write(&keys::EngineVersionKey {}, current_version)?;

			Ok(Ok(()))
		})
		.await?
}
