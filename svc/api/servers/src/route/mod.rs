use api_helper::{define_router, util::CorsConfigBuilder};
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
	cors: CorsConfigBuilder::hub().build(),
	routes: {
		"games" / Uuid / "environments" / Uuid / "servers": {
			GET: servers::list_servers(
				query: servers::ListQuery,
			),
			POST: servers::create(
				body: models::ServersCreateServerRequest,
			),
		},

		"games" / Uuid / "environments" / Uuid / "servers" / Uuid: {
			GET: servers::get(),
			DELETE: servers::destroy(
				query: servers::DeleteQuery,
			),
		},

		"games" / Uuid / "environments" / Uuid / "servers" / Uuid / "logs" : {
			GET: logs::get_logs(
				query: logs::GetServerLogsQuery,
			),
		},

		"games" / Uuid / "environments" / Uuid / "builds": {
			GET: builds::list(
				query: builds::GetQuery,
			),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid: {
			GET: builds::get(),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid / "tags": {
			PATCH: builds::patch_tags(body: models::ServersPatchBuildTagsRequest),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / "prepare": {
			POST: builds::create_build(body: models::ServersCreateBuildRequest),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid / "complete": {
			POST: builds::complete_build(body: serde_json::Value),
		},
	},
}
