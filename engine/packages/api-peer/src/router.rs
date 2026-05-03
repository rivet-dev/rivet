use rivet_api_builder::{create_router, prelude::*};

use crate::{actors, depot_inspect, envoys, internal, namespaces, runner_configs, runners};

#[tracing::instrument(skip_all)]
pub async fn router(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
) -> anyhow::Result<axum::Router> {
	create_router("api-peer", config, pools, |mut router| {
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
			.route("/actors/{actor_id}/sleep", post(actors::sleep::sleep))
			.route(
				"/actors/{actor_id}/reschedule",
				post(actors::reschedule::reschedule),
			)
			// MARK: Runners
			.route("/runners", get(runners::list))
			.route("/runners/names", get(runners::list_names))
			// MARK: Envoys
			.route("/envoys", get(envoys::list))
			// MARK: Depot inspect
			.route("/depot/inspect/summary", get(depot_inspect::summary))
			.route("/depot/inspect/catalog", get(depot_inspect::catalog))
			.route(
				"/depot/inspect/buckets/{bucket_id}",
				get(depot_inspect::bucket),
			)
			.route(
				"/depot/inspect/buckets/{bucket_id}/databases/{database_id}",
				get(depot_inspect::database),
			)
			.route(
				"/depot/inspect/branches/{branch_id}",
				get(depot_inspect::branch),
			)
			.route(
				"/depot/inspect/branches/{branch_id}/pages/{pgno}/trace",
				get(depot_inspect::page_trace),
			)
			.route(
				"/depot/inspect/branches/{branch_id}/rows/{family}",
				get(depot_inspect::branch_rows),
			)
			.route("/depot/inspect/raw/key/{key}", get(depot_inspect::raw_key))
			.route("/depot/inspect/raw/scan", get(depot_inspect::raw_scan))
			.route(
				"/depot/inspect/raw/decode-key/{key}",
				get(depot_inspect::decode_key),
			)
			// MARK: Internal
			.route("/cache/purge", post(internal::cache_purge))
			.route(
				"/epoxy/coordinator/replica-reconfigure",
				post(internal::epoxy_replica_reconfigure),
			)
			.route("/epoxy/coordinator/state", get(internal::get_epoxy_state))
			.route("/epoxy/coordinator/state", post(internal::set_epoxy_state))
			.route(
				"/epoxy/replica/debug",
				get(internal::get_epoxy_replica_debug),
			)
			.route(
				"/epoxy/replica/key/{key}",
				get(internal::get_epoxy_key_debug),
			)
			.route(
				"/epoxy/replica/key/{key}/fanout",
				get(internal::get_epoxy_key_debug_fanout),
			)
			.route(
				"/epoxy/replica/kv/{key}/local",
				get(internal::get_epoxy_kv_local),
			)
			.route(
				"/epoxy/replica/kv/{key}/optimistic",
				get(internal::get_epoxy_kv_optimistic),
			)
			.route("/epoxy/replica/kv/{key}", put(internal::set_epoxy_kv))
			.route("/debug/tracing/config", put(internal::set_tracing_config))
	})
	.await
}
