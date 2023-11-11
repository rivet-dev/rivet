use std::time::Duration;

use rivet_operation::OperationContext;
use rivet_pools::prelude::*;
use uuid::Uuid;

use crate::error::ManagerError;

#[derive(Clone)]
pub struct TestCtx {
	op_ctx: OperationContext<()>,
}

impl TestCtx {
	pub async fn from_env(test_name: &str) -> Result<TestCtx, ManagerError> {
		let service_name = format!(
			"{}-test--{}",
			std::env::var("CHIRP_SERVICE_NAME").unwrap(),
			test_name
		);
		let source_hash = std::env::var("RIVET_SOURCE_HASH").unwrap();
		let pools = rivet_pools::from_env(service_name.clone()).await?;
		let cache =
			rivet_cache::CacheInner::new(service_name.clone(), source_hash, pools.redis_cache()?);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.unwrap()
			.wrap_new(&service_name);
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			service_name,
			Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			rivet_util::timestamp::now(),
			rivet_util::timestamp::now(),
			(),
			Vec::new(),
		);

		Ok(TestCtx { op_ctx })
	}
}

impl TestCtx {
	pub fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	pub fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
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

	pub async fn redis_user_presence(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.op_ctx.redis_user_presence().await
	}
}
