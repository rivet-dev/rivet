use api_helper::define_router;
use hyper::{Body, Request, Response};

pub mod traefik;

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
		"traefik" / "config" / "core": {
			GET: traefik::core::config(
				query: traefik::core::ConfigQuery,
				internal_endpoint: true,
				opt_auth: true,
			),
		},
		"traefik" / "config" / "game-guard": {
			GET: traefik::game_guard::config(
				query: traefik::game_guard::ConfigQuery,
				internal_endpoint: true,
				opt_auth: true,
			),
		}
	}
}
