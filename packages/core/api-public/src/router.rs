use axum::{
	extract::Request,
	middleware::{self, Next},
	response::{Redirect, Response},
};
use reqwest::header::{AUTHORIZATION, HeaderMap};
use rivet_api_builder::{create_router, extract::FailedExtraction};
use utoipa::OpenApi;

use crate::{actors, ctx, datacenters, namespaces, runner_configs, runners, ui};

#[derive(OpenApi)]
#[openapi(
	paths(
		actors::list::list,
		actors::create::create,
		actors::delete::delete,
		actors::list_names::list_names,
		actors::get_or_create::get_or_create,
		runners::list,
		runners::list_names,
		namespaces::list,
		namespaces::create,
		runner_configs::list,
		runner_configs::upsert,
		runner_configs::delete,
		datacenters::list,
	),
	components(
		schemas(namespace::keys::RunnerConfigVariant)
	),
	security( ("bearer_auth" = []) ),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

pub async fn router(
	name: &'static str,
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
) -> anyhow::Result<axum::Router> {
	create_router(name, config, pools, |router| {
		router
			// Root redirect
			.route(
				"/",
				axum::routing::get(|| async { Redirect::permanent("/ui/") }),
			)
			// MARK: Namespaces
			.route("/namespaces", axum::routing::get(namespaces::list))
			.route("/namespaces", axum::routing::post(namespaces::create))
			.route("/runner-configs", axum::routing::get(runner_configs::list))
			.route(
				"/runner-configs/{runner_name}",
				axum::routing::put(runner_configs::upsert),
			)
			.route(
				"/runner-configs/{runner_name}",
				axum::routing::delete(runner_configs::delete),
			)
			// MARK: Actors
			.route("/actors", axum::routing::get(actors::list::list))
			.route("/actors", axum::routing::post(actors::create::create))
			.route(
				"/actors",
				axum::routing::put(actors::get_or_create::get_or_create),
			)
			.route(
				"/actors/{actor_id}",
				axum::routing::delete(actors::delete::delete),
			)
			.route(
				"/actors/names",
				axum::routing::get(actors::list_names::list_names),
			)
			// MARK: Runners
			.route("/runners", axum::routing::get(runners::list))
			.route("/runners/names", axum::routing::get(runners::list_names))
			// MARK: Datacenters
			.route("/datacenters", axum::routing::get(datacenters::list))
			// MARK: UI
			.route("/ui", axum::routing::get(ui::serve_index))
			.route("/ui/", axum::routing::get(ui::serve_index))
			.route("/ui/{*path}", axum::routing::get(ui::serve_ui))
			// MARK: Middleware (must go after all routes)
			.layer(middleware::from_fn(auth_middleware))
	})
	.await
}

/// Middleware to wrap ApiCtx with auth handling capabilities and to throw an error if auth was not explicitly
// handled in an endpoint
async fn auth_middleware(
	headers: HeaderMap,
	mut req: Request,
	next: Next,
) -> std::result::Result<Response, String> {
	let ctx = req
		.extensions()
		.get::<rivet_api_builder::ApiCtx>()
		.ok_or_else(|| "ctx should exist".to_string())?;

	// Extract token
	let token = headers
		.get(AUTHORIZATION)
		.and_then(|x| x.to_str().ok().and_then(|x| x.strip_prefix("Bearer ")))
		.map(|x| x.to_string());

	// Insert the new ApiCtx into request extensions
	let ctx = ctx::ApiCtx::new(ctx.clone(), token);
	req.extensions_mut().insert(ctx.clone());

	let path = req.uri().path().to_string();

	// Run endpoint
	let res = next.run(req).await;

	// Verify auth was handled
	if res.extensions().get::<FailedExtraction>().is_none()
		&& path != "/"
		&& path != "/ui"
		&& !path.starts_with("/ui/")
		&& !ctx.is_auth_handled()
	{
		return Err(format!(
			"developer error: must explicitly handle auth in all endpoints (path: {path})"
		));
	}

	Ok(res)
}

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
	fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
		openapi.components.as_mut().unwrap().add_security_scheme(
			"bearer_auth",
			utoipa::openapi::security::SecurityScheme::Http(
				utoipa::openapi::security::HttpBuilder::new()
					.scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
					// .bearer_format("Rivet")
					.build(),
			),
		);
	}
}
