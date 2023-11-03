use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct EmailRow {
	email: String,
	user_id: Uuid,
}

#[operation(name = "user-resolve-email")]
async fn handle(
	ctx: OperationContext<user::resolve_email::Request>,
) -> GlobalResult<user::resolve_email::Response> {
	let users = sql_fetch_all!(
		[ctx, EmailRow]
		"
		SELECT email, user_id
		FROM db_user_identity.emails
		WHERE email = ANY($1)
	",
		&ctx.emails,
	)
	.await?
	.into_iter()
	.map(|row| user::resolve_email::response::User {
		email: row.email,
		user_id: Some(row.user_id.into()),
	})
	.collect::<Vec<_>>();

	Ok(user::resolve_email::Response { users })
}
