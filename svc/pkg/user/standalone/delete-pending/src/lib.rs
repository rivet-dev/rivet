use futures_util::{StreamExt, TryStreamExt};
use indoc::indoc;
use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[tracing::instrument]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("user-delete-pending").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("user-delete-pending");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"user-delete-pending".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
	);

	let user_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT user_id
		FROM db_user.users
		WHERE delete_request_ts < $1
		",
		ts - util::duration::days(30),
	)
	.await?
	.into_iter()
	.map(|(user_id,)| user_id)
	.collect::<Vec<_>>();

	tracing::info!(count = user_ids.len(), "publishing deletes");

	futures_util::stream::iter(user_ids.into_iter())
		.map(|user_id| {
			msg!([ctx] user::msg::delete(user_id) -> user::msg::delete_complete {
				user_id: Some(user_id.into()),
			})
		})
		.buffer_unordered(32)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}
