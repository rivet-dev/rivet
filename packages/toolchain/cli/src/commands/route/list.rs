use anyhow::*;
use clap::Parser;
use toolchain::rivet_api::apis;

/// List all routes for an environment
#[derive(Parser)]
pub struct Opts {
	/// Specify the environment to list routes for (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;
		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		// Get routes
		let routes_response = apis::routes_api::routes_list(
			&ctx.openapi_config_cloud,
			Some(&ctx.project.name_id.to_string()),
			Some(&env),
		)
		.await?;

		if routes_response.routes.is_empty() {
			println!("No routes found for environment '{}'", env);
			return Ok(());
		}

		println!("Routes for environment '{}':", env);
		println!();
		println!("{:<20} {:<40} {:<40}", "ID", "HOSTNAME", "PATH");
		println!("{:<20} {:<40} {:<40}", "----", "--------", "----");

		for route in routes_response.routes {
			println!("{:<20} {:<40} {:<40}", route.id, route.hostname, route.path);
		}

		Ok(())
	}
}