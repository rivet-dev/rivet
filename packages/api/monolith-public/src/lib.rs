use rivet_operation::prelude::*;

pub mod route;

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	api_helper::start(
		config.clone(),
		pools,
		"public",
		config.server()?.rivet.api_public.host(),
		config.server()?.rivet.api_public.port(),
		route::handle,
	)
	.await
}
