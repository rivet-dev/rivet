use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct AccessTokenRow {
	name: String,
	user_id: Uuid,
}

#[operation(name = "user-resolve-access_token")]
async fn handle(
	ctx: OperationContext<user::resolve_access_token::Request>,
) -> GlobalResult<user::resolve_access_token::Response> {
	let users = sql_fetch_all!(
		[ctx, AccessTokenRow]
		"
		SELECT name, user_id
		FROM db_user_identity.access_tokens
		WHERE name = ANY($1)
	",
		&ctx.names,
	)
	.await?
	.into_iter()
	.map(|row| user::resolve_access_token::response::User {
		name: row.name,
		user_id: Some(row.user_id.into()),
	})
	.collect::<Vec<_>>();

	Ok(user::resolve_access_token::Response { users })
}
