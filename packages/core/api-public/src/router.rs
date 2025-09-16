use axum::response::Redirect;
use rivet_api_builder::{
	create_router,
	wrappers::{get, post},
};
use utoipa::OpenApi;

use crate::{actors, datacenters, namespaces, runners, ui};

#[derive(OpenApi)]
#[openapi(paths(
	actors::list::list,
	actors::get::get,
	actors::create::create,
	actors::delete::delete,
	actors::list_names::list_names,
	actors::get_or_create::get_or_create,
	actors::get_by_id::get_by_id,
	actors::get_or_create_by_id::get_or_create_by_id,
	runners::list,
	runners::get,
	runners::list_names,
	namespaces::list,
	namespaces::get,
	namespaces::create,
	namespaces::runner_configs::list,
	namespaces::runner_configs::get,
	namespaces::runner_configs::upsert,
	namespaces::runner_configs::delete,
	datacenters::list,
))]
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
			.route(
				"/namespaces/{namespace_id}",
				axum::routing::get(namespaces::get),
			)
			.route(
				"/namespaces/{namespace_id}/runner-configs",
				axum::routing::get(namespaces::runner_configs::list),
			)
			.route(
				"/namespaces/{namespace_id}/runner-configs/{runner_name}",
				axum::routing::get(namespaces::runner_configs::get),
			)
			.route(
				"/namespaces/{namespace_id}/runner-configs/{runner_name}",
				axum::routing::put(namespaces::runner_configs::upsert),
			)
			.route(
				"/namespaces/{namespace_id}/runner-configs/{runner_name}",
				axum::routing::delete(namespaces::runner_configs::delete),
			)
			// MARK: Actors
			.route("/actors", axum::routing::get(actors::list::list))
			.route("/actors", post(actors::create::create))
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
			.route(
				"/actors/by-id",
				axum::routing::get(actors::get_by_id::get_by_id),
			)
			.route(
				"/actors/by-id",
				axum::routing::put(actors::get_or_create_by_id::get_or_create_by_id),
			)
			.route("/actors/{actor_id}", axum::routing::get(actors::get::get))
			// MARK: Runners
			.route("/runners", axum::routing::get(runners::list))
			.route("/runners/{runner_id}", axum::routing::get(runners::get))
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
