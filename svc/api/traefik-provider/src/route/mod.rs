use api_helper::define_router;
use hyper::{Body, Request, Response};

pub mod core;
pub mod game_guard;
pub mod tunnel;

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
		"config" / "core": {
			GET: core::config(
				query: core::ConfigQuery,
				internal_endpoint: true,
				opt_auth: true,
			),
		},
		"config" / "tunnel": {
			GET: tunnel::config(
				query: tunnel::ConfigQuery,
				internal_endpoint: true,
				opt_auth: true,
			),
		},
		"config" / "game-guard": {
			GET: game_guard::config(
				query: game_guard::ConfigQuery,
				internal_endpoint: true,
				opt_auth: true,
			),
		}
	}
}
