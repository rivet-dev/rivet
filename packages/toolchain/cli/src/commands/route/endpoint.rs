use anyhow::*;
use clap::Parser;
use std::collections::HashMap;
use toolchain::{
	rivet_api::{apis, models},
	ToolchainCtx,
};

/// Create or update a route endpoint
#[derive(Parser)]
pub struct Opts {
	/// Name/ID of the route
	name: String,

	/// Specify the environment to deploy to (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,

	/// Hostname for the route
	#[clap(long)]
	hostname: Option<String>,

	/// Path for the route
	#[clap(long)]
	path: Option<String>,

	/// Route subpaths to the function (true/false)
	#[clap(long)]
	route_subpaths: Option<bool>,

	/// Strip prefix from the route (true/false)
	#[clap(long)]
	strip_prefix: Option<bool>,

	/// Selector tags in key=value comma-separated format (e.g. type=function,function=my-function)
	#[clap(long)]
	selector_tags: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;
		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		// Get existing route if it exists
		let route = get_route(&ctx, &env, &self.name).await?;

		// Parse selector tags
		let selector_tags = self
			.selector_tags
			.as_ref()
			.map(|tags| kv_str::from_str::<HashMap<String, String>>(tags))
			.transpose()
			.context("Failed to parse selector tags")?;

		// Build route update body
		let mut update_route_body = models::RoutesUpdateRouteBody {
			hostname: route
				.as_ref()
				.map(|r| r.hostname.clone())
				.unwrap_or_else(|| {
					// Default hostname is project-env.domain
					format!(
						"{}-{}.{}",
						ctx.project.name_id,
						env,
						ctx.bootstrap
							.domains
							.job
							.as_ref()
							.expect("bootstrap.domains.job")
					)
				}),
			path: route
				.as_ref()
				.map(|r| r.path.clone())
				.unwrap_or_else(|| "/".to_string()),
			route_subpaths: route.as_ref().map(|r| r.route_subpaths).unwrap_or(true),
			strip_prefix: route.as_ref().map(|r| r.strip_prefix).unwrap_or(true),
			target: Box::new(models::RoutesRouteTarget {
				actors: Some(Box::new(models::RoutesRouteTargetActors {
					selector_tags: route
						.as_ref()
						.and_then(|r| r.target.actors.as_ref().map(|a| a.selector_tags.clone()))
						.unwrap_or_else(|| {
							// Default selector tags for functions
							let mut tags = HashMap::new();
							tags.insert("type".to_string(), "function".to_string());
							tags.insert("function".to_string(), self.name.clone());
							tags
						}),
				})),
			}),
		};

		// Override with any provided options
		if let Some(hostname) = &self.hostname {
			update_route_body.hostname = hostname.clone();
		}

		if let Some(path) = &self.path {
			update_route_body.path = path.clone();
		}

		if let Some(route_subpaths) = self.route_subpaths {
			update_route_body.route_subpaths = route_subpaths;
		}

		if let Some(strip_prefix) = self.strip_prefix {
			update_route_body.strip_prefix = strip_prefix;
		}

		if let Some(tags) = selector_tags {
			if let Some(actors) = &mut update_route_body.target.actors {
				actors.selector_tags = tags;
			}
		}

		// Create/update route
		let result = apis::routes_api::routes_update(
			&ctx.openapi_config_cloud,
			&self.name,
			update_route_body.clone(),
			Some(&ctx.project.name_id.to_string()),
			Some(&env),
		)
		.await;

		match result {
			Result::Ok(_) => {
				println!(
					"Successfully {} route: {}{}",
					if route.is_some() {
						"updated"
					} else {
						"created"
					},
					update_route_body.hostname,
					update_route_body.path
				);
				Ok(())
			}
			Err(err) => {
				eprintln!(
					"Failed to {}: {}",
					if route.is_some() {
						"update route"
					} else {
						"create route"
					},
					err
				);
				Err(err.into())
			}
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
