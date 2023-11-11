pub struct Service {
	workers: Vec<crate::config::Config>,
}

impl Service {
	async fn spawn(&self, join_set: &JoinSet) {
			for worker in service.workers {
				::chirp_worker::Manager::new(
                    config,
                    shared_client.clone(),
                    pools.clone(),
                    cache.clone(),
					$worker::Worker
				)?;
				join_set.spawn(worker.start());
			}
	}
}
