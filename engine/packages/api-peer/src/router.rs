use rivet_api_builder::{create_router, prelude::*};

use crate::{actors, internal, namespaces, runner_configs, runners};

#[tracing::instrument(skip_all)]
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
			// MARK: Runner configs
			.route("/runner-configs", get(runner_configs::list))
			.route("/runner-configs/{runner_name}", put(runner_configs::upsert))
			.route(
				"/runner-configs/{runner_name}",
				delete(runner_configs::delete),
			)
			// MARK: Actors
			.route("/actors", get(actors::list::list))
			.route("/actors", post(actors::create::create))
			.route("/actors", put(actors::get_or_create::get_or_create))
			.route("/actors/{actor_id}", delete(actors::delete::delete))
			.route("/actors/names", get(actors::list_names::list_names))
			.route(
				"/actors/{actor_id}/kv/keys/{key}",
				get(actors::kv_get::kv_get),
			)
			// MARK: Runners
			.route("/runners", get(runners::list))
			.route("/runners/names", get(runners::list_names))
			// MARK: Internal
			.route("/cache/purge", post(internal::cache_purge))
			.route(
				"/epoxy/coordinator/replica-reconfigure",
				post(internal::epoxy_replica_reconfigure),
			)
			.route("/epoxy/coordinator/state", get(internal::get_epoxy_state))
			.route("/epoxy/coordinator/state", post(internal::set_epoxy_state))
			.route("/debug/tracing/config", put(internal::set_tracing_config))
	})
	.await
}
