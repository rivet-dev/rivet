use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "user-identity-delete")]
async fn handle(
	ctx: OperationContext<user_identity::delete::Request>,
) -> GlobalResult<user_identity::delete::Response> {
	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	sqlx::query(indoc!(
		"
		DELETE FROM db_user_identity.emails
		WHERE user_id = ANY($1)
		"
	))
	.bind(&user_ids)
	.execute(&ctx.crdb().await?)
	.await?;

	Ok(user_identity::delete::Response {})
}
