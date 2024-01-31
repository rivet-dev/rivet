use api_helper::define_router;
use hyper::{Body, Request, Response};
use rivet_api::models;

pub mod login;

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
		"login": {
			POST: login::login(
				body: models::AdminLoginRequest,
			),
		},
	},
}
