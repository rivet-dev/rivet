use anyhow::*;
use gas::prelude::*;

/// Global API context that contains shared resources
#[derive(Clone)]
pub struct GlobalApiCtx {
	pub db: gas::prelude::db::DatabaseHandle,
	pub config: rivet_config::Config,
	pub pools: rivet_pools::Pools,
	pub cache: rivet_cache::Cache,
	pub name: &'static str,
}

impl GlobalApiCtx {
	pub async fn new(
		config: rivet_config::Config,
		pools: rivet_pools::Pools,
		name: &'static str,
	) -> Result<Self> {
		let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
		let db = gas::prelude::db::DatabaseKv::from_pools(pools.clone()).await?;

		Ok(Self {
			db,
			config,
			pools,
			cache,
			name,
		})
	}
}
