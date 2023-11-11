#[macro_export]
macro_rules! workers {
    ($($worker:ident),* $(,)?) => {
		use ::chirp_worker::prelude::*;
		use chirp_types::message::Message;

		$(
			pub mod $worker;
		)*

		pub fn spawn_workers(shared_client: chirp_client::SharedClientHandle, pools: rivet_pools::Pools, cache: rivet_cache::Cache, join_set: &mut tokio::task::JoinSet<GlobalResult<()>>) -> GlobalResult<()> {
			// Spawn a manager for each worker
			$(
				{
					let topic = <$worker::Worker as ::chirp_worker::Worker>::Request::NAME;
					let config = ::chirp_worker::config::Config::from_env(topic)?;
					let worker =
						::chirp_worker::Manager::new(
							config,
							shared_client.clone(),
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
