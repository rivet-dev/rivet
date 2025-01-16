use std::time::Duration;

use global_error::GlobalResult;
use rivet_operation::OperationContext;
use rivet_pools::prelude::*;
use uuid::Uuid;

use crate::error::ManagerError;

#[derive(Clone)]
pub struct TestCtx {
	name: String,
	op_ctx: OperationContext<()>,
}

impl TestCtx {
	pub async fn from_env(test_name: &str) -> Result<TestCtx, ManagerError> {
		let service_name = format!("{}-test--{}", rivet_env::service_name(), test_name);
		let config = rivet_config::Config::load::<String>(&[])
			.await
			.map_err(ManagerError::Global)?;
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let cache = rivet_cache::CacheInner::new(
			service_name.clone(),
			pools.redis_cache()?,
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.unwrap()
			.wrap_new(&service_name);
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			service_name.clone(),
			Duration::from_secs(60),
			config,
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			rivet_util::timestamp::now(),
			rivet_util::timestamp::now(),
			(),
		);

		Ok(TestCtx {
			name: service_name,
			op_ctx,
		})
	}
}

impl TestCtx {
	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	pub fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
	}

	pub fn config(&self) -> &rivet_config::Config {
		self.op_ctx.config()
	}

	pub async fn crdb(&self) -> Result<CrdbPool, rivet_pools::Error> {
		self.op_ctx.crdb().await
	}

	pub async fn redis_cache(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.op_ctx.redis_cache().await
	}

	pub async fn redis_cdn(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.op_ctx.redis_cdn().await
	}

	pub async fn redis_job(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.op_ctx.redis_job().await
	}

	pub async fn redis_mm(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.op_ctx.redis_mm().await
	}

	pub async fn clickhouse(&self) -> GlobalResult<ClickHousePool> {
		self.op_ctx.clickhouse().await
	}
}
