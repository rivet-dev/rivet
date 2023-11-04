use proto::backend::pkg::*;
use rivet_operation::prelude::*;

const CDN_AUTH_USER_MAX: i64 = 32;

#[operation(name = "cdn-namespace-auth-user-update")]
async fn handle(
	ctx: OperationContext<cdn::namespace_auth_user_update::Request>,
) -> GlobalResult<cdn::namespace_auth_user_update::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();
	ensure_with!(
		util::check::bcrypt(&ctx.password),
		CDN_INVALID_AUTH_USER_PASSWORD
	);

	let crdb = ctx.crdb().await?;
	let (auth_user_count,) = sql_fetch_one!(
		[ctx, (i64,)]
		"SELECT COUNT(*) FROM db_cdn.game_namespace_auth_users WHERE namespace_id = $1",
		namespace_id,
	)
	.await?;

	ensure_with!(auth_user_count < CDN_AUTH_USER_MAX, CDN_TOO_MANY_AUTH_USERS);

	sql_execute!(
		[ctx]
		"UPSERT INTO db_cdn.game_namespace_auth_users (namespace_id, user_name, password) VALUES ($1, $2, $3)",
		namespace_id,
		&ctx.user,
		&ctx.password,
	)
	.await?;

	msg!([ctx] cdn::msg::ns_config_update(namespace_id) {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	Ok(cdn::namespace_auth_user_update::Response {})
}
