use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-search-update")]
async fn worker(ctx: &OperationContext<user::msg::search_update::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb().await?;
	let user_id = internal_unwrap_owned!(ctx.user_id).as_uuid();

	sqlx::query(indoc!(
		"
		UPDATE db_user.users
		SET
			is_searchable = TRUE,
			update_ts = $1
		WHERE user_id = $2
		"
	))
	.bind(ctx.ts())
	.bind(user_id)
	.execute(&crdb)
	.await?;

	ctx.cache().purge("user", [user_id]).await?;

	Ok(())
}
