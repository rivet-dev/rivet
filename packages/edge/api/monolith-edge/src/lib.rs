use rivet_operation::prelude::*;

pub mod route;

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	api_helper::start(
		config.clone(),
		pools,
		"edge",
		config.server()?.rivet.api_edge.host(),
		config.server()?.rivet.api_edge.port(),
		route::handle,
	)
	.await
}
