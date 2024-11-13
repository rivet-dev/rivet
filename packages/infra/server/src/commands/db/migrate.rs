use anyhow::*;
use clap::Parser;

use crate::run_config::RunConfig;

#[derive(Parser)]
pub enum SubCommand {
	/// Runs all pending migrations.
	Up { services: Vec<String> },
	/// Rolls back database to the given migration.
	Down { service: String, num: usize },
	/// Forces the migration state to a certain migration.
	///
	/// Helpful when dealing with bugged migrations.
	Force { service: String, num: usize },
	/// Drops the entire database.
	Drop { service: String },
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config, run_config: &RunConfig) -> Result<()> {
		match self {
			Self::Up { services: names } => {
				if names.is_empty() {
					rivet_migrate::up(config, &run_config.sql_services).await?;
				} else {
					let services = run_config
						.sql_services
						.iter()
						.filter(|x| names.iter().any(|y| *y == x.db_name))
						.cloned()
						.collect::<Vec<_>>();
					rivet_migrate::up(config, &services).await?;
				};
			}
			Self::Down { service, num } => {
				let service = run_config
					.sql_services
					.iter()
					.find(|x| x.db_name == service)
					.context("service not found")?;
				rivet_migrate::down(config, service, num).await?;
			}
			Self::Force { service, num } => {
				let service = run_config
					.sql_services
					.iter()
					.find(|x| x.db_name == service)
					.context("service not found")?;
				rivet_migrate::force(config, service, num).await?;
			}
			Self::Drop { service } => {
				let service = run_config
					.sql_services
					.iter()
					.find(|x| x.db_name == service)
					.context("service not found")?;
				rivet_migrate::drop(config, service).await?;
			}
		}

		Ok(())
	}
}
