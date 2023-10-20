use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "cdn-ns-enable-domain-public-auth-set")]
async fn handle(
	ctx: OperationContext<cdn::ns_enable_domain_public_auth_set::Request>,
) -> GlobalResult<cdn::ns_enable_domain_public_auth_set::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	sqlx::query(indoc!(
		"
		UPDATE db_cdn.game_namespaces
		SET enable_domain_public_auth = $2
		WHERE namespace_id = $1
		"
	))
	.bind(namespace_id)
	.bind(ctx.enable_domain_public_auth)
	.execute(&ctx.crdb().await?)
	.await?;

	Ok(cdn::ns_enable_domain_public_auth_set::Response {})
}
