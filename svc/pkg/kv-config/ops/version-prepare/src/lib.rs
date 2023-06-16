use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "kv-config-version-prepare")]
async fn handle(
	ctx: OperationContext<kv_config::version_prepare::Request>,
) -> GlobalResult<kv_config::version_prepare::Response> {
	let _game_id = internal_unwrap!(ctx.game_id).as_uuid();
	let _config = internal_unwrap!(ctx.config);

	Ok(kv_config::version_prepare::Response {
		config_ctx: Some(backend::kv::VersionConfigCtx {}),
	})
}
