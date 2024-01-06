use rivet_operation::prelude::*;

macro_rules! spawn_workers {
	([$shared_client:ident, $pools:ident, $cache:ident, $join_set:ident] $($worker:ident),* $(,)?) => {
		$(
			$worker::workers::spawn_workers(
				$shared_client.clone(),
				$pools.clone(),
				$cache.clone(),
				&mut $join_set,
			)?;
		)*
	};
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;

	// Start workers
	let mut join_set = tokio::task::JoinSet::new();
	spawn_workers![
		[shared_client, pools, cache, join_set]
		analytics_worker,
		cdn_worker,
		cf_custom_hostname_worker,
		cloud_worker,
		external_worker,
		game_user_worker,
		job_log_worker,
		job_run_worker,
		kv_worker,
		mm_worker,
		module_worker,
		push_notification_worker,
		team_invite_worker,
		team_worker,
		upload_worker,
		user_dev_worker,
		user_follow_worker,
		user_presence_worker,
		user_report_worker,
		user_worker,
	];

	// Wait for task to exit
	if let Some(res) = join_set.join_next().await {
		match res? {
			Ok(_) => {
				bail!("worker exited unexpectedly")
			}
			Err(err) => {
				return Err(err);
			}
		}
	}

	bail!("no workers running")
}
