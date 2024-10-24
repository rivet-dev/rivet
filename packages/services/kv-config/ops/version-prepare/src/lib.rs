use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "kv-config-version-prepare")]
async fn handle(
	ctx: OperationContext<kv_config::version_prepare::Request>,
) -> GlobalResult<kv_config::version_prepare::Response> {
	let _game_id = unwrap_ref!(ctx.game_id).as_uuid();
	let _config = unwrap_ref!(ctx.config);

	Ok(kv_config::version_prepare::Response {
		config_ctx: Some(backend::kv::VersionConfigCtx {}),
	})
}
