use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "cdn-namespace-auth-user-remove")]
async fn handle(
	ctx: OperationContext<cdn::namespace_auth_user_remove::Request>,
) -> GlobalResult<cdn::namespace_auth_user_remove::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	sql_execute!(
		[ctx]
		"DELETE FROM db_cdn.game_namespace_auth_users WHERE namespace_id = $1 AND user_name = $2",
		namespace_id,
		&ctx.user,
	)
	.await?;

	msg!([ctx] cdn::msg::ns_config_update(namespace_id) {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	Ok(cdn::namespace_auth_user_remove::Response {})
}
