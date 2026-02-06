use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use indoc::formatdoc;
use rivet_service_manager::RunConfig;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

// 7 day logs retention
const LOGS_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Parser)]
pub struct Opts {
	#[arg(short = 's', long, conflicts_with = "except_services")]
	services: Vec<String>,

	/// Exclude the specified services instead of including them
	#[arg(long)]
	except_services: Vec<String>,
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
			let mut services = run_config.services.clone();

			for exclude_name in &self.except_services {
				if !run_config
					.services
					.iter()
					.any(|service| service.name == exclude_name)
				{
					bail!("service {exclude_name:?} not found");
				}

				services.retain(|service| service.name != exclude_name);
			}

			services
		} else {
			let mut services = Vec::new();

			for name in &self.services {
				let Some(service) = run_config
					.services
					.iter()
					.find(|service| service.name == name)
				else {
					bail!("service {name:?} not found");
				};

				services.push(service.clone());
			}

			services
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
	if config.runtime.allow_version_rollback() {
		return Ok(());
	}

	pools
		.udb()?
		.run(|tx| async move {
			let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION")).context("failed to parse cargo pkg version as semver")?;

			if let Some(existing_version) = tx.read_opt(&keys::EngineVersionKey {}, Serializable).await? {
				if current_version < existing_version {
					return Ok(Err(anyhow!(
						"{}",
						formatdoc!(
							"
						Rivet Engine has been rolled back to a previous version:
						  - Last Used Version: {existing_version}
						  - Current Version:   {current_version}
						Cannot proceed without potential data corruption.
						
						(If you know what you're doing, this error can be disabled in the Rivet config via `allow_version_rollback: true`)
						"
						)
					)));
				}
			}

			tx.write(&keys::EngineVersionKey {}, current_version)?;

			Ok(Ok(()))
		})
		.await?
}
