use anyhow::*;
use clap::Parser;

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
	pub async fn execute(self) -> Result<()> {
		match self {
			Self::Up { services: names } => {
				if names.is_empty() {
					rivet_migrate::up_all().await?;
				} else {
					let services = rivet_migrate::registry::get_services(
						&names.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
					);
					rivet_migrate::up(&services).await?;
				};
			}
			Self::Down { service, num } => {
				let service =
					rivet_migrate::registry::get_service(&service).context("service not found")?;
				rivet_migrate::down(&service, num).await?;
			}
			Self::Force { service, num } => {
				let service =
					rivet_migrate::registry::get_service(&service).context("service not found")?;
				rivet_migrate::force(&service, num).await?;
			}
			Self::Drop { service } => {
				let service =
					rivet_migrate::registry::get_service(&service).context("service not found")?;
				rivet_migrate::drop(&service).await?;
			}
		}

		Ok(())
	}
}
