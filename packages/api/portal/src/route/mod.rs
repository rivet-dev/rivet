use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_portal_server::models;

mod games;

pub async fn handle(
	shared_client: chirp_client::SharedClientHandle,
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	cache: rivet_cache::Cache,
	ray_id: uuid::Uuid,
	request: Request<Body>,
) -> Result<Response<Body>, http::Error> {
	let response = Response::builder();

	// Handle route
	Router::handle(
		shared_client,
		config,
		pools,
		cache,
		ray_id,
		request,
		response,
	)
	.await
}

define_router! {
	cors: |config| CorsConfigBuilder::hub(config).build(),
	routes: {
		// Games
		"games": {
			GET: games::get_suggested_games(),
		},
		"games" / String / "profile": {
			GET: games::profile(),
		},
	},
}
