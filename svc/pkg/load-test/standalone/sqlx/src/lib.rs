use std::collections::HashSet;

use futures_util::StreamExt;
use rivet_operation::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("load-test-sqlx").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("load-test-sqlx");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"load-test-sqlx".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);
	
	let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
	loop {
		interval.tick().await;

		sql_fetch_one!(
			[ctx, (i64,)]
			"SELECT 1",
		).await?;
	}

	Ok(())
}
