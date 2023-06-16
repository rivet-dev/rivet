use futures_util::{StreamExt, TryStreamExt};
use indoc::indoc;
use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[tracing::instrument]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("user-delete-pending").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("user-delete-pending");
	let crdb_pool = pools.crdb("db-user")?;

	let user_ids = sqlx::query_as::<_, (Uuid,)>(indoc!(
		"
		SELECT user_id
		FROM users
		WHERE delete_request_ts < $1
		"
	))
	.bind(ts - util::duration::days(30))
	.fetch_all(&crdb_pool)
	.await?
	.into_iter()
	.map(|(user_id,)| user_id)
	.collect::<Vec<_>>();

	tracing::info!(count = user_ids.len(), "publishing deletes");

	futures_util::stream::iter(user_ids.into_iter())
		.map(|user_id| {
			msg!([client] user::msg::delete(user_id) -> user::msg::delete_complete {
				user_id: Some(user_id.into()),
			})
		})
		.buffer_unordered(32)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}
