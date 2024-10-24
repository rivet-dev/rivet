#[macro_export]
macro_rules! workers {
    ($($worker:ident),* $(,)?) => {
		use ::chirp_worker::prelude::*;
		use chirp_types::message::Message;

		pub fn spawn_workers(shared_client: chirp_client::SharedClientHandle, config: rivet_config::Config, pools: rivet_pools::Pools, cache: rivet_cache::Cache, join_set: &mut tokio::task::JoinSet<GlobalResult<()>>) -> GlobalResult<()> {
			// Spawn a manager for each worker
			$(
				{
					let topic = <$worker::Worker as ::chirp_worker::Worker>::Request::NAME;
					let worker_config = ::chirp_worker::config::Config::from_env(topic)?;
					let worker =
						::chirp_worker::Manager::new(
							worker_config,
							shared_client.clone(),
							config.clone(),
							pools.clone(),
							cache.clone(),
							$worker::Worker
						)?;

					join_set.spawn(async move {
						worker.start().await.map_err(Into::into)
					});
				}
			)*

			Ok(())
		}
    }
}
