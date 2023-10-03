/// Creates a manager for each given worker and `tokio::select`'s them
#[macro_export]
macro_rules! worker_group {
    ($($worker:ident),* $(,)?) => {{
		use ::chirp_worker::prelude::chirp_types::message::Message;

        // Fetch env
        let service_name = std::env::var("CHIRP_SERVICE_NAME")
            .map_err(|_| ManagerError::MissingEnvVar("CHIRP_SERVICE_NAME".into()))?;
        let source_hash = std::env::var("RIVET_SOURCE_HASH")
            .map_err(|_| ManagerError::MissingEnvVar("RIVET_SOURCE_HASH".into()))?;
        let region = std::env::var("CHIRP_REGION")
            .map_err(|_| ManagerError::MissingEnvVar("CHIRP_REGION".into()))?;

        // Create connections
        let pools = rivet_pools::from_env(service_name.clone()).await?;
        let shared_client = chirp_client::SharedClient::new(
			pools.nats()?,
			pools.redis_chirp()?,
			pools.redis_cache()?,
			region,
		);
		let cache = rivet_cache::CacheInner::new(
			service_name,
			source_hash,
			pools.redis_cache()?,
		);

        // Start health checks
        tokio::task::Builder::new()
			.name("chirp_worker::rivet_health_checks_run")
			.spawn(rivet_health_checks::run_standalone(
				rivet_health_checks::Config {
					pools: Some(pools.clone()),
				},
			))
			.map_err(ManagerError::TokioSpawn)?;

        // Start metrics
		tokio::task::Builder::new()
			.name("rivet_metrics::run_standalone")
			.spawn(rivet_metrics::run_standalone())
			.map_err(ManagerError::TokioSpawn)?;

		// Create a manager for each worker
		$(
            let topic = <$worker::Worker as ::chirp_worker::Worker>::Request::NAME;
			let config = ::chirp_worker::config::Config::from_env(topic)?;
			let $worker =
				::chirp_worker::Manager::new(
                    config,
                    shared_client.clone(),
                    pools.clone(),
                    cache.clone(),
					$worker::Worker
				).await?;
        )*

        async {
			// Add select branch for each worker
            tokio::select! {
                $(  res = $worker.start() => {
					// TODO: Should the error just be logged instead?
					res?;
				} )*
            }

			Result::<_, ManagerError>::Ok(())
        }
    }}
}
