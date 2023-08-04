use api_helper::define_router;
use hyper::{Body, Request, Response};
use uuid::Uuid;

pub mod example;

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
		"foo" / String / "bar": {
			POST: example::endpoint(
				body: serde_json::Value,
			),
		},
	},
}
