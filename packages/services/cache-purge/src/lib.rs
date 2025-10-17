use anyhow::Result;
use gas::prelude::*;
use rivet_cache::{CachePurgeMessage, CACHE_PURGE_TOPIC};
use universalpubsub::NextOutput;

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	tracing::info!("starting cache purge subscriber service");

	// Subscribe to cache purge updates
	let ups = pools.ups()?;
	let mut sub = ups.subscribe(CACHE_PURGE_TOPIC).await?;

	tracing::info!(subject = ?CACHE_PURGE_TOPIC, "subscribed to cache purge updates");

	// Get cache instance
	let cache = rivet_cache::CacheInner::from_env(&config, pools)?;

	// Process incoming messages
	while let Ok(NextOutput::Message(msg)) = sub.next().await {
		match serde_json::from_slice::<CachePurgeMessage>(&msg.payload) {
			Ok(purge_msg) => {
				tracing::info!(
					base_key = ?purge_msg.base_key,
					keys_count = purge_msg.keys.len(),
					"received cache purge request"
				);

				// Purge the cache locally
				if let Err(err) = cache
					.clone()
					.request()
					.purge(&purge_msg.base_key, purge_msg.keys)
					.await
				{
					tracing::error!(?err, base_key = ?purge_msg.base_key, "failed to purge cache");
				}
			}
			Err(err) => {
				tracing::error!(?err, "failed to deserialize cache purge message");
			}
		}
	}

	tracing::warn!("cache purge subscriber service stopped");

	Ok(())
}
