use anyhow::*;
use clap::Parser;
use toolchain::{
	rivet_api::{apis, models},
	ToolchainCtx,
};

/// Get a RivetKit endpoint
#[derive(Parser, Clone)]
pub struct Opts {
	/// Specify the environment to get the kit from (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;
		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		// Get route information (kits are handled similarly to routes)
		let route = get_route(&ctx, &env, crate::util::rivetkit::SERVER_NAME).await?;

		match route {
			Some(route) => {
				println!("https://{}{}", route.hostname, route.path);

				Ok(())
			}
			None => Err(anyhow!(
				"Endpoint '{}' not found in environment '{}'",
				crate::util::rivetkit::SERVER_NAME,
				env
			)),
		}
	}
}

// Helper function to get route if it exists
async fn get_route(
	ctx: &ToolchainCtx,
	env: &str,
	route_id: &str,
) -> Result<Option<models::RoutesRoute>> {
	let routes_response = apis::routes_api::routes_list(
		&ctx.openapi_config_cloud,
		Some(&ctx.project.name_id.to_string()),
		Some(env),
	)
	.await?;

	// Find route that matches the ID
	let matching_route = routes_response
		.routes
		.iter()
		.find(|route| route.id == *route_id)
		.cloned();

	Ok(matching_route)
}
