use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "module-game-version-prepare")]
async fn handle(
	_ctx: OperationContext<module::game_version_prepare::Request>,
) -> GlobalResult<module::game_version_prepare::Response> {
	Ok(module::game_version_prepare::Response {
		config_ctx: Some(backend::module::GameVersionConfigCtx {}),
	})
}
