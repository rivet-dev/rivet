use proto::backend::pkg::*;
use rivet_operation::prelude::*;

const CDN_AUTH_USER_MAX: i64 = 32;

#[operation(name = "cdn-namespace-auth-user-update")]
async fn handle(
	ctx: OperationContext<cdn::namespace_auth_user_update::Request>,
) -> GlobalResult<cdn::namespace_auth_user_update::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();
	assert_with!(
		util::check::bcrypt(&ctx.password),
		CDN_INVALID_AUTH_USER_PASSWORD
	);

	let crdb = ctx.crdb().await?;
	let (auth_user_count,) = sqlx::query_as::<_, (i64,)>(
		"SELECT COUNT(*) FROM db_cdn.game_namespace_auth_users WHERE namespace_id = $1",
	)
	.bind(namespace_id)
	.fetch_one(&crdb)
	.await?;

	assert_with!(auth_user_count < CDN_AUTH_USER_MAX, CDN_TOO_MANY_AUTH_USERS);

	sqlx::query(
		"UPSERT INTO db_cdn.game_namespace_auth_users (namespace_id, user_name, password) VALUES ($1, $2, $3)",
	)
	.bind(namespace_id)
	.bind(&ctx.user)
	.bind(&ctx.password)
	.execute(&crdb)
	.await?;

	msg!([ctx] cdn::msg::ns_config_update(namespace_id) {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	Ok(cdn::namespace_auth_user_update::Response {})
}
