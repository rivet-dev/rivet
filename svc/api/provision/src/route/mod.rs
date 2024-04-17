use std::net::Ipv4Addr;

use api_helper::define_router;
use hyper::{Body, Request, Response};
use uuid::Uuid;

pub mod datacenters;
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
		"datacenters" / Uuid / "tls": {
			GET: datacenters::tls(
				internal_endpoint: true,
			),
		},

		"servers" / Ipv4Addr / "info": {
			GET: servers::info(
				internal_endpoint: true,
			),
		},
	},
}
