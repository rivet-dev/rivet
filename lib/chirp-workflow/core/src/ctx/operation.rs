use global_error::{GlobalError, GlobalResult};
use rivet_pools::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::{
	db::DatabaseHandle,
	error::WorkflowError,
	operation::{Operation, OperationInput},
	signal::Signal,
};

#[derive(Clone)]
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
	#[tracing::instrument(err, skip_all, fields(operation = I::Operation::NAME))]
	pub async fn op<I>(
		&self,
		input: I,
	) -> GlobalResult<<<I as OperationInput>::Operation as Operation>::Output>
	where
		I: OperationInput,
		<I as OperationInput>::Operation: Operation<Input = I>,
	{
		tracing::info!(?input, "operation call");

		let ctx = OperationCtx::new(
			self.db.clone(),
			&self.conn,
			self.ray_id,
			self.op_ctx.req_ts(),
			self.op_ctx.from_workflow(),
			I::Operation::NAME,
		);

		let res = I::Operation::run(&ctx, &input)
			.await
			.map_err(WorkflowError::OperationFailure)
			.map_err(GlobalError::raw);

		tracing::info!(?res, "operation response");

		res
	}

	pub async fn signal<T: Signal + Serialize>(
		&self,
		workflow_id: Uuid,
		input: T,
	) -> GlobalResult<Uuid> {
		let signal_id = Uuid::new_v4();

		tracing::info!(name=%T::NAME, %workflow_id, %signal_id, "dispatching signal");

		// Serialize input
		let input_val = serde_json::to_value(input)
			.map_err(WorkflowError::SerializeSignalBody)
			.map_err(GlobalError::raw)?;

		self.db
			.publish_signal(self.ray_id, workflow_id, signal_id, T::NAME, input_val)
			.await
			.map_err(GlobalError::raw)?;

		Ok(signal_id)
	}

	pub async fn tagged_signal<T: Signal + Serialize>(
		&self,
		tags: &serde_json::Value,
		input: T,
	) -> GlobalResult<Uuid> {
		let signal_id = Uuid::new_v4();

		tracing::info!(name=%T::NAME, ?tags, %signal_id, "dispatching tagged signal");

		// Serialize input
		let input_val = serde_json::to_value(input)
			.map_err(WorkflowError::SerializeSignalBody)
			.map_err(GlobalError::raw)?;

		self.db
			.publish_tagged_signal(self.ray_id, tags, signal_id, T::NAME, input_val)
			.await
			.map_err(GlobalError::raw)?;

		Ok(signal_id)
	}
}

impl OperationCtx {
	pub fn name(&self) -> &str {
		self.name
	}

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
