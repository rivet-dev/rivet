use std::collections::HashSet;

use futures_util::StreamExt;
use rivet_operation::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("user-search-user-gc").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("user-search-user-gc");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"user-search-user-gc".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
	);

	let mut total_removed = 0;
	let start = std::time::Instant::now();

	// Filters out users who have been updated in the last 14 days
	let crdb = ctx.crdb().await?;
	let mut query = sql_fetch!(
		[ctx, (Uuid,), &crdb]
		"
		SELECT user_id
		FROM db_user.users AS OF SYSTEM TIME '-5s'
		WHERE
			is_searchable = TRUE AND
			update_ts < $1
		",
		ts - util::duration::days(14),
	);

	let mut batch_user_ids = Vec::with_capacity(1024);
	while let Some(row) = query.next().await {
		let (user_id,) = row?;

		batch_user_ids.push(user_id);

		if batch_user_ids.len() >= 1024 {
			total_removed += process_batch(&ctx, &batch_user_ids).await?;
			batch_user_ids.clear();
		}
	}

	if !batch_user_ids.is_empty() {
		total_removed += process_batch(&ctx, &batch_user_ids).await?;
	}

	tracing::info!(
		?total_removed,
		"finished in {:.2}s",
		start.elapsed().as_millis() as f64 / 1000.0
	);

	Ok(())
}

async fn process_batch(ctx: &OperationContext<()>, user_ids: &[Uuid]) -> GlobalResult<u64> {
	let user_ids_proto = user_ids.iter().cloned().map(Into::into).collect::<Vec<_>>();

	let (registered, has_followers) = tokio::try_join!(
		// TODO: When more identity methods are implemented, query those too
		async {
			op!([ctx] user_identity_get {
				user_ids: user_ids_proto.clone(),
			})
			.await?
			.users
			.iter()
			.map(|user| Ok(unwrap_ref!(user.user_id).as_uuid()))
			.collect::<GlobalResult<HashSet<_>>>()
		},
		async {
			op!([ctx] user_follow_count {
				user_ids: user_ids_proto.clone(),
			})
			.await?
			.follows
			.iter()
			.filter(|follows| follows.count != 0)
			.map(|follows| Ok(unwrap_ref!(follows.user_id).as_uuid()))
			.collect::<GlobalResult<HashSet<_>>>()
		},
	)?;

	// Users who are not registered and have no followers. Note that using `HashSet`'s `.difference`
	// method is not ideal here as it cannot be chained with another difference without reallocation.
	let deletions = user_ids
		.iter()
		.filter(|id| !registered.contains(id))
		.filter(|id| !has_followers.contains(id))
		.cloned()
		.collect::<Vec<_>>();

	let res = sql_execute!(
		[ctx]
		"
		UPDATE db_user.users
		SET is_searchable = FALSE
		WHERE user_id = ANY($1)
		",
		deletions,
	)
	.await?;

	Ok(res.rows_affected())
}
