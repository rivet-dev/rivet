use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-search-update")]
async fn worker(ctx: &OperationContext<user::msg::search_update::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb().await?;
	let user_id = unwrap!(ctx.user_id).as_uuid();

	sql_query!(
		[ctx]
		"
		UPDATE db_user.users
		SET
			is_searchable = TRUE,
			update_ts = $1
		WHERE user_id = $2
		",
		ctx.ts(),
		user_id,
	)
	.await?;

	ctx.cache().purge("user", [user_id]).await?;

	Ok(())
}
