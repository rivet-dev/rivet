use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "cdn-ns-auth-type-set")]
async fn handle(
	ctx: OperationContext<cdn::ns_auth_type_set::Request>,
) -> GlobalResult<cdn::ns_auth_type_set::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	internal_assert!(
		backend::cdn::namespace_config::AuthType::from_i32(ctx.auth_type).is_some(),
		"invalid auth type"
	);

	sqlx::query(indoc!(
		"
		UPDATE db_cdn.game_namespaces
		SET auth_type = $2
		WHERE namespace_id = $1
		"
	))
	.bind(namespace_id)
	.bind(ctx.auth_type)
	.execute(&ctx.crdb().await?)
	.await?;

	msg!([ctx] cdn::msg::ns_config_update(namespace_id) {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	Ok(cdn::ns_auth_type_set::Response {})
}
