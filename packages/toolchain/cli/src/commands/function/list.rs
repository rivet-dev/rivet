use anyhow::*;
use clap::Parser;
use toolchain::rivet_api::apis;

/// List all functions for an environment
#[derive(Parser)]
pub struct Opts {
	/// Specify the environment to list function for (will prompt if not specified)
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

		for route in routes_response.routes {
			println!("- {}: {}{}", route.id, route.hostname, route.path);
		}

		Ok(())
	}
}
