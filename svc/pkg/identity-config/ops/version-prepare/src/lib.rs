use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "identity-config-version-prepare")]
async fn handle(
	ctx: OperationContext<identity_config::version_prepare::Request>,
) -> GlobalResult<identity_config::version_prepare::Response> {
	let _game_id = internal_unwrap!(ctx.game_id).as_uuid();
	let _config = internal_unwrap!(ctx.config);

	Ok(identity_config::version_prepare::Response {
		config_ctx: Some(backend::identity::VersionConfigCtx {}),
	})
}
