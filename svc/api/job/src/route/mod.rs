use api_helper::define_router;
use hyper::{Body, Request, Response};
use rivet_job_server::models;

pub mod run;

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
		"runs" / "cleanup": {
			POST: run::cleanup(
				body: models::CleanupRequest,
				internal_endpoint: true,
			),
		},
	},
}
