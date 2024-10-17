use rivet_operation::prelude::*;

pub mod route;

pub async fn start() -> GlobalResult<()> {
	api_helper::start(route::handle).await;
	Ok(())
}
