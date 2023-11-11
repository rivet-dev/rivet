use rivet_operation::prelude::*;

#[tracing::instrument]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("team-dev-halt-collect").await?;
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
	let client = shared_client.wrap_new("team-dev-halt-collect");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"team-dev-halt-collect".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		ts,
		ts,
		(),
		Vec::new(),
	);
	let crdb_pool = ctx.crdb().await?;

	let cutoff_ts = ts - util::billing::CUTOFF_DURATION;
	let team_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT team_id
		FROM db_team_dev.dev_teams
		WHERE payment_failed_ts < $1
		",
		cutoff_ts,
	)
	.await?
	.into_iter()
	.map(|(team_id,)| team_id.into())
	.collect::<Vec<_>>();

	op!([ctx] team_dev_halt {
		team_ids: team_ids,
	})
	.await?;

	Ok(())
}
