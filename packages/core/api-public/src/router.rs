use axum::response::Redirect;
use rivet_api_builder::{create_router, wrappers::get};
use utoipa::OpenApi;

use crate::{actors, datacenters, namespaces, runner_configs, runners, ui};

#[derive(OpenApi)]
#[openapi(paths(
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
))]
#[openapi(components(schemas(namespace::keys::RunnerConfigVariant)))]
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
			.route("/datacenters", get(datacenters::list))
			// MARK: UI
			.route("/ui", axum::routing::get(ui::serve_index))
			.route("/ui/", axum::routing::get(ui::serve_index))
			.route("/ui/{*path}", axum::routing::get(ui::serve_ui))
	})
	.await
}
