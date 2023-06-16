use api_helper::define_router;
use hyper::{Body, Request, Response};

pub mod matchmaker;

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
		"matchmaker": {
			GET: matchmaker::status(
				query: matchmaker::StatusQuery,
			),
		}
	},
}
