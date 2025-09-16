use rivet_api_builder::{create_router, prelude::*};

use crate::{actors, internal, namespaces, runners};

pub async fn router(
	name: &'static str,
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
) -> anyhow::Result<axum::Router> {
	create_router(name, config, pools, |mut router| {
		router = epoxy::http_routes::mount_routes(router);
		router
			// MARK: Namespaces
			.route("/namespaces", get(namespaces::list))
			.route("/namespaces", post(namespaces::create))
			.route("/namespaces/{namespace_id}", get(namespaces::get))
			.route(
				"/namespaces/resolve/{name}",
				get(namespaces::resolve_for_name),
			)
			// MARK: Runner configs
			.route(
				"/namespaces/{namespace_id}/runner-configs",
				get(namespaces::runner_configs::list),
			)
			.route(
				"/namespaces/{namespace_id}/runner-configs/{runner_name}",
				put(namespaces::runner_configs::upsert),
			)
			.route(
				"/namespaces/{namespace_id}/runner-configs/{runner_name}",
				get(namespaces::runner_configs::get),
			)
			.route(
				"/namespaces/{namespace_id}/runner-configs/{runner_name}",
				delete(namespaces::runner_configs::delete),
			)
			// MARK: Actors
			.route("/actors", get(actors::list::list))
			.route("/actors", post(actors::create::create))
			.route("/actors/{actor_id}", get(actors::get::get))
			.route("/actors/{actor_id}", delete(actors::delete::delete))
			.route("/actors/names", get(actors::list_names::list_names))
			// MARK: Runners
			.route("/runners", get(runners::list))
			.route("/runners/{runner_id}", get(runners::get))
			.route("/runners/names", get(runners::list_names))
			// MARK: Internal
			.route("/cache/purge", post(internal::cache_purge))
			.route(
				"/bump-serverless-autoscaler",
				post(internal::bump_serverless_autoscaler),
			)
	})
	.await
}
