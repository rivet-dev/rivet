use rivet_operation::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("monolith-main").await?;
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;

	let mut join_set = tokio::task::JoinSet::new();

	analytics_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	cdn_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	cf_custom_hostname_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	chat_message_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	chat_thread_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	chat_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	cloud_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	external_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	game_user_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	job_run_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	kv_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	mm_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	module_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	nomad_log_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	push_notification_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	team_dev_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	team_invite_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	team_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	upload_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	user_dev_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	user_follow_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	user_presence_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	user_report_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;
	user_worker::workers::spawn_workers(shared_client, pools, cache, &mut join_set)?;

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
