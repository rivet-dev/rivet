use api_helper::define_router;
use hyper::{Body, Request, Response};
use rivet_api::models;
use uuid::Uuid;

pub mod builds;
pub mod logs;
pub mod servers;

pub async fn handle(
	shared_client: chirp_client::SharedClientHandle,
	pools: rivet_pools::Pools,
	cache: rivet_cache::Cache,
	ray_id: uuid::Uuid,
	request: Request<Body>,
) -> Result<Response<Body>, http::Error> {
	let response = Response::builder();

	// Handle route
	Router::handle(shared_client, pools, cache, ray_id, request, response).await
}

define_router! {
	routes: {
		"" : {
			GET: servers::list_servers(
				query: servers::ListQuery,
			),
			POST: servers::create(
				body: models::ServersCreateServerRequest,
			),
		},

		Uuid : {
			GET: servers::get(),
			DELETE: servers::destroy(
				query: servers::DeleteQuery,
			),
		},

		Uuid / "logs" : {
			GET: logs::get_logs(
				query: logs::GetServerLogsQuery,
			),
		},

		"builds": {
			GET: builds::get_builds(
				query: builds::GetQuery,
			),
			POST: builds::create_build(body: models::ServersCreateBuildRequest),
		},

		"uploads" / Uuid / "complete": {
			POST: builds::complete_build(body: serde_json::Value),
		},
	},
}
