use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

#[derive(Parser)]
pub enum SubCommand {
	/// Creates a new migration.
	Create {
		service: String,
		migration_name: String,
	},
	/// Checks that the migrations are valid.
	///
	/// Helpful for testing migrations before applying to a live database.
	Check { services: Vec<String> },
	/// Runs all pending migrations.
	Up { services: Vec<String> },
	/// Rolls back database to the given migration.
	Down { service: String, num: usize },
	/// Forces the migration state to a certain migration.
	///
	/// Helpful when dealing with bugged migrations.
	Force { service: String, num: usize },
	/// Lists migrations
	List,
	/// Drops the entire database.
	Drop { service: String },
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Create {
				service,
				migration_name,
			} => {
				tasks::migrate::create(
					&ctx,
					&ctx.service_with_name(&service).await,
					&migration_name,
				)
				.await?;
			}
			Self::Check { services: names } => {
				if names.is_empty() {
					tasks::migrate::check_all(&ctx).await?;
				} else {
					let services = ctx.services_with_patterns(&names).await;
					tasks::migrate::check(&ctx, &services[..]).await?;
				}
			}
			Self::Up { services: names } => {
				if names.is_empty() {
					tasks::migrate::up_all(&ctx).await?;
				} else {
					let services = ctx.services_with_patterns(&names).await;
					tasks::migrate::up(&ctx, &services[..]).await?;
				}
			}
			Self::Down { service, num } => {
				tasks::migrate::down(&ctx, &ctx.service_with_name(&service).await, num).await?;
			}
			Self::Force { service, num } => {
				tasks::migrate::force(&ctx, &ctx.service_with_name(&service).await, num).await?;
			}
			Self::List => {
				for svc in ctx.services_with_migrations().await {
					println!("{}", svc.name());
				}
			}
			Self::Drop { service } => {
				tasks::migrate::drop(&ctx, &ctx.service_with_name(&service).await).await?;
			}
		}

		utils::ringadingding();

		Ok(())
	}
}
