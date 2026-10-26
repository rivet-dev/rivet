//! Production HTTP, DNS policy, TLS and database adapters prepared at bootstrap.

pub mod local_object_store;
pub mod object_storage;
pub mod reqwest;

use crate::{EngineAdapters, ObjectStores};
use anyhow::Result;
use std::sync::Arc;

pub async fn adapters(
	config: &rivet_config::Config,
	env: Arc<dyn rivet_env::Env>,
	object_stores: ObjectStores,
) -> Result<EngineAdapters> {
	storage_adapters(config, env, object_stores, None).await
}

/// Like [`adapters`], with the cold-store disk cache a storage worker resolved at startup.
pub async fn storage_adapters(
	config: &rivet_config::Config,
	env: Arc<dyn rivet_env::Env>,
	object_stores: ObjectStores,
	disk_cache: Option<&depot_worker::disk_budget::DiskCacheBudget>,
) -> Result<EngineAdapters> {
	let provider = rustls::crypto::ring::default_provider();
	if provider.install_default().is_err() {
		tracing::debug!("crypto provider already installed in this process");
	}
	let http = reqwest::peer_client_with(&config.pubsub().peer_transport)?;
	// Validate configured destination policy at bootstrap; guarded clients are built by their owners.
	reqwest::outbound_policy(config).await?;
	let fs = env.fs();
	// One bounded cache serves every cold-store database this process opens.
	let cold_store = &config.depot().cold_store;
	let cache = rivet_env::backends::production::slatedb::SharedSlateDbCache::new(
		cold_store.block_cache_bytes(),
		cold_store.meta_cache_bytes(),
	)?;
	let mut factory = rivet_env::backends::production::slatedb::ProductionSlateDbFactory::new(
		env,
		object_stores.primary.clone(),
	)
	.with_shared_cache(cache);
	if let Some(disk_cache) = disk_cache {
		factory = factory
			.with_disk_cache(rivet_env::backends::production::slatedb::DiskCacheOptions {
				directory: disk_cache.directory.clone(),
				capacity_bytes: disk_cache.capacity_bytes,
				part_bytes: disk_cache.part_bytes,
				recorder: Some(depot_worker::disk_budget::cache_metrics_recorder()),
				cache_on_flush: cold_store.disk_cache_on_flush(),
				cache_on_compaction: cold_store.disk_cache_on_compaction(),
			})
			.await?;
	}
	let depot_databases = Arc::new(factory);
	Ok(EngineAdapters {
		fs,
		object_stores,
		http,
		http_server: Arc::new(
			rivet_env::backends::production::http::AxumServer::with_connection_capacity(
				config
					.pubsub()
					.peer_transport
					.server_connection_capacity
					.unwrap_or(8192),
			),
		),
		depot_databases,
	})
}
