use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models;
use uuid::Uuid;

pub mod envs;

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
		"games" / Uuid / "environments" / Uuid / "tokens" / "service": {
			POST: envs::tokens::create_service(
				body: serde_json::Value
			),
		},
	},
}
