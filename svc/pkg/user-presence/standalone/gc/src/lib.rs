use futures_util::{StreamExt, TryStreamExt};
use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("user-presence-gc");
	let mut redis = pools.redis("redis-user-presence")?;

	let expire_ts = ts - util_user_presence::USER_PRESENCE_TTL;
	let (user_ids,) = redis::pipe()
		.zrangebyscore(
			util_user_presence::key::user_presence_touch(),
			0,
			expire_ts as isize,
		)
		.zrembyscore(
			util_user_presence::key::user_presence_touch(),
			0,
			expire_ts as isize,
		)
		.ignore()
		.query_async::<_, (Vec<String>,)>(&mut redis)
		.await?;

	let user_ids = user_ids
		.into_iter()
		.filter_map(|x| util::uuid::parse(&x).ok())
		.collect::<Vec<_>>();
	tracing::info!(count = user_ids.len(), "removing user presences");

	futures_util::stream::iter(user_ids.into_iter())
		.map(|user_id| {
			msg!([client] @wait user_presence::msg::leave(user_id) {
				user_id: Some(user_id.into()),
			})
		})
		.buffer_unordered(32)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}
