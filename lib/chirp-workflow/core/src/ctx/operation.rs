use global_error::{GlobalError, GlobalResult};
use rivet_pools::prelude::*;
use uuid::Uuid;

use crate::{DatabaseHandle, Operation, OperationInput, WorkflowError};

pub struct OperationCtx {
	ray_id: Uuid,
	name: &'static str,
	ts: i64,

	db: DatabaseHandle,

	conn: rivet_connection::Connection,

	// Backwards compatibility
	op_ctx: rivet_operation::OperationContext<()>,
}

impl OperationCtx {
	pub fn new(
		db: DatabaseHandle,
		conn: &rivet_connection::Connection,
		ray_id: Uuid,
		req_ts: i64,
		from_workflow: bool,
		name: &'static str,
	) -> Self {
		let ts = rivet_util::timestamp::now();
		let req_id = Uuid::new_v4();
		let conn = conn.wrap(req_id, ray_id, name);
		let mut op_ctx = rivet_operation::OperationContext::new(
			name.to_string(),
			std::time::Duration::from_secs(60),
			conn.clone(),
			req_id,
			ray_id,
			ts,
			req_ts,
			(),
		);
		op_ctx.from_workflow = from_workflow;

		OperationCtx {
			ray_id,
			name,
			ts,
			db,
			conn,
			op_ctx,
		}
	}
}

impl OperationCtx {
	pub async fn op<I>(
		&self,
		input: I,
	) -> GlobalResult<<<I as OperationInput>::Operation as Operation>::Output>
	where
		I: OperationInput,
		<I as OperationInput>::Operation: Operation<Input = I>,
	{
		let ctx = OperationCtx::new(
			self.db.clone(),
			&self.conn,
			self.ray_id,
			self.op_ctx.req_ts(),
			self.op_ctx.from_workflow(),
			I::Operation::NAME,
		);

		I::Operation::run(&ctx, &input)
			.await
			.map_err(WorkflowError::OperationFailure)
			.map_err(GlobalError::raw)
	}
}

impl OperationCtx {
	pub fn name(&self) -> &str {
		self.name
	}

	// pub fn timeout(&self) -> Duration {
	// 	self.timeout
	// }

	pub fn req_id(&self) -> Uuid {
		self.op_ctx.req_id()
	}

	pub fn ray_id(&self) -> Uuid {
		self.ray_id
	}

	/// Timestamp at which the request started.
	pub fn ts(&self) -> i64 {
		self.ts
	}

	/// Timestamp at which the request was published.
	pub fn req_ts(&self) -> i64 {
		self.op_ctx.req_ts()
	}

	/// Time between when the timestamp was processed and when it was published.
	pub fn req_dt(&self) -> i64 {
		self.ts.saturating_sub(self.op_ctx.req_ts())
	}

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
