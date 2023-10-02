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
	let users = sqlx::query_as::<_, EmailRow>(indoc!(
		"
		SELECT email, user_id
		FROM db_user_identity.emails
		WHERE email = ANY($1)
	"
	))
	.bind(&ctx.emails)
	.fetch_all(&ctx.crdb().await?)
	.await?
	.into_iter()
	.map(|row| user::resolve_email::response::User {
		email: row.email,
		user_id: Some(row.user_id.into()),
	})
	.collect::<Vec<_>>();

	Ok(user::resolve_email::Response { users })
}
