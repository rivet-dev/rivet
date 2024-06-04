use std::fmt::{self, Debug};

use chirp_client::prelude::*;
use global_error::GlobalResult;
use rivet_pools::prelude::*;

#[derive(Clone)]
pub struct Connection {
	pub(crate) client: chirp_client::Client,
	pub(crate) pools: rivet_pools::Pools,
	pub(crate) cache: rivet_cache::Cache,
}

impl Connection {
	pub fn new(
		client: chirp_client::Client,
		pools: rivet_pools::Pools,
		cache: rivet_cache::Cache,
	) -> Self {
		Connection {
			client,
			pools,
			cache,
		}
	}

	/// Creates a new `Connection` with the appropriate context in the `Client` to make requests. Used when
	// calling another operation.
	pub fn wrap(
		&self,
		parent_req_id: Uuid,
		ray_id: Uuid,
		trace_entry: chirp_client::TraceEntry,
	) -> Connection {
		// Not the same as the operation ctx's ts because this cannot be overridden by debug start ts
		let ts = rivet_util::timestamp::now();

		Connection::new(
			(*self.client).clone().wrap(
				parent_req_id,
				ray_id,
				{
					let mut x = self.client.trace().to_vec();
					x.push(trace_entry);
					x
				},
			),
			self.pools.clone(),
			self.cache.clone(),
		)
	}

	pub fn chirp(&self) -> &chirp_client::Client {
		&self.client
	}

	pub fn cache(&self) -> rivet_cache::RequestConfig {
		self.cache.clone().request()
	}

	pub fn cache_handle(&self) -> rivet_cache::Cache {
		self.cache.clone()
	}

	pub async fn crdb(&self) -> Result<CrdbPool, rivet_pools::Error> {
		self.pools.crdb()
	}

	pub async fn redis_cache(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.pools.redis_cache()
	}

	pub async fn redis_cdn(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.pools.redis("persistent")
	}

	pub async fn redis_job(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.pools.redis("persistent")
	}

	pub async fn redis_mm(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.pools.redis("persistent")
	}

	pub async fn redis_user_presence(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.pools.redis("ephemeral")
	}

	pub fn perf(&self) -> &chirp_perf::PerfCtx {
		self.client.perf()
	}

	pub async fn clickhouse(&self) -> GlobalResult<ClickHousePool> {
		self.pools.clickhouse()
	}
}

impl std::ops::Deref for Connection {
	type Target = chirp_client::Client;

	fn deref(&self) -> &Self::Target {
		self.chirp()
	}
}

impl Debug for Connection {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Connection").finish()
	}
}
