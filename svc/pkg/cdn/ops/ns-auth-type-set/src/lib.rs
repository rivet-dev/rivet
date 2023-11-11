use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "cdn-ns-auth-type-set")]
async fn handle(
	ctx: OperationContext<cdn::ns_auth_type_set::Request>,
) -> GlobalResult<cdn::ns_auth_type_set::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	ensure!(
		backend::cdn::namespace_config::AuthType::from_i32(ctx.auth_type).is_some(),
		"invalid auth type"
	);

	sql_execute!(
		[ctx]
		"
		UPDATE db_cdn.game_namespaces
		SET auth_type = $2
		WHERE namespace_id = $1
		",
		namespace_id,
		ctx.auth_type,
	)
	.await?;

	msg!([ctx] cdn::msg::ns_config_update(namespace_id) {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	Ok(cdn::ns_auth_type_set::Response {})
}
