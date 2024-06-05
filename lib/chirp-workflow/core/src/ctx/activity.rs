use global_error::GlobalResult;
use rivet_pools::prelude::*;
use uuid::Uuid;

use crate::{
	ctx::OperationCtx, DatabaseHandle, Operation, OperationInput, WorkflowError, WorkflowResult,
};

pub struct ActivityCtx {
	db: DatabaseHandle,
	conn: rivet_connection::Connection,
	workflow_id: Uuid,
	name: &'static str,

	// Backwards compatibility
	op_ctx: rivet_operation::OperationContext<()>,
}

impl ActivityCtx {
	pub fn new(
		db: DatabaseHandle,
		conn: rivet_connection::Connection,
		workflow_id: Uuid,
		name: &'static str,
	) -> Self {
		let op_ctx = rivet_operation::OperationContext::new(
			name.to_string(),
			std::time::Duration::from_secs(60),
			conn.clone(),
			workflow_id,
			// TODO: ray_id
			Uuid::new_v4(),
			rivet_util::timestamp::now(),
			// TODO: req_ts
			rivet_util::timestamp::now(),
			(),
		);

		ActivityCtx {
			db,
			conn,
			workflow_id,
			name,
			op_ctx,
		}
	}
}

impl ActivityCtx {
	pub async fn op<I>(
		&mut self,
		input: I,
	) -> WorkflowResult<<<I as OperationInput>::Operation as Operation>::Output>
	where
		I: OperationInput,
		<I as OperationInput>::Operation: Operation<Input = I>,
	{
		let mut ctx = OperationCtx::new(self.db.clone(), self.workflow_id);

		I::Operation::run(&mut ctx, &input)
			.await
			.map_err(WorkflowError::OperationFailure)
	}

	pub fn name(&self) -> &str {
		self.name
	}

	// pub fn timeout(&self) -> Duration {
	// 	self.timeout
	// }

	// pub fn req_id(&self) -> Uuid {
	// 	self.req_id
	// }

	// pub fn ray_id(&self) -> Uuid {
	// 	self.ray_id
	// }

	// /// Timestamp at which the request started.
	// pub fn ts(&self) -> i64 {
	// 	self.ts
	// }

	// /// Timestamp at which the request was published.
	// pub fn req_ts(&self) -> i64 {
	// 	self.req_ts
	// }

	// /// Time between when the timestamp was processed and when it was published.
	// pub fn req_dt(&self) -> i64 {
	// 	self.ts.saturating_sub(self.req_ts)
	// }

	// pub fn perf(&self) -> &chirp_perf::PerfCtx {
	// 	self.conn.perf()
	// }

	pub fn trace(&self) -> &[chirp_client::TraceEntry] {
		self.conn.trace()
	}

	pub fn test(&self) -> bool {
		self.trace()
			.iter()
			.any(|x| x.run_context == chirp_client::RunContext::Test as i32)
	}

	pub fn chirp(&self) -> &chirp_client::Client {
		self.conn.chirp()
	}

	pub fn cache(&self) -> rivet_cache::RequestConfig {
		self.conn.cache()
	}

	pub fn cache_handle(&self) -> rivet_cache::Cache {
		self.conn.cache_handle()
	}

	pub async fn crdb(&self) -> Result<CrdbPool, rivet_pools::Error> {
		self.conn.crdb().await
	}

	pub async fn redis_cache(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_cache().await
	}

	pub async fn redis_cdn(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_cdn().await
	}

	pub async fn redis_job(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_job().await
	}

	pub async fn redis_mm(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_mm().await
	}

	pub async fn redis_user_presence(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_user_presence().await
	}

	pub async fn clickhouse(&self) -> GlobalResult<ClickHousePool> {
		self.conn.clickhouse().await
	}

	// Backwards compatibility
	pub fn op_ctx(&self) -> &rivet_operation::OperationContext<()> {
		&self.op_ctx
	}
}
