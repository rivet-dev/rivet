use global_error::{GlobalError, GlobalResult};
use rivet_pools::prelude::*;
use uuid::Uuid;

use crate::{
	ctx::{MessageCtx, OperationCtx},
	error::{WorkflowError, WorkflowResult},
	message::Message,
	DatabaseHandle, Operation, OperationInput,
};

#[derive(Clone)]
pub struct ActivityCtx {
	workflow_id: Uuid,
	ray_id: Uuid,
	name: &'static str,
	ts: i64,

	db: DatabaseHandle,

	conn: rivet_connection::Connection,
	msg_ctx: MessageCtx,

	// Backwards compatibility
	op_ctx: rivet_operation::OperationContext<()>,
}

impl ActivityCtx {
	pub async fn new(
		workflow_id: Uuid,
		db: DatabaseHandle,
		conn: &rivet_connection::Connection,
		activity_create_ts: i64,
		ray_id: Uuid,
		name: &'static str,
	) -> WorkflowResult<Self> {
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
			activity_create_ts,
			(),
		);
		op_ctx.from_workflow = true;

		let msg_ctx = MessageCtx::new(&conn, req_id, ray_id).await?;

		Ok(ActivityCtx {
			workflow_id,
			ray_id,
			name,
			ts,
			db,
			conn,
			op_ctx,
			msg_ctx,
		})
	}
}

impl ActivityCtx {
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
			true,
			I::Operation::NAME,
		);

		tokio::time::timeout(I::Operation::TIMEOUT, I::Operation::run(&ctx, &input))
			.await
			.map_err(|_| WorkflowError::OperationTimeout)?
			.map_err(WorkflowError::OperationFailure)
			.map_err(GlobalError::raw)
	}

	pub async fn update_workflow_tags(&self, tags: &serde_json::Value) -> GlobalResult<()> {
		self.db
			.update_workflow_tags(self.workflow_id, tags)
			.await
			.map_err(GlobalError::raw)
	}

	pub async fn msg<M>(&self, tags: serde_json::Value, body: M) -> GlobalResult<()>
	where
		M: Message,
	{
		self.msg_ctx
			.message(tags, body)
			.await
			.map_err(GlobalError::raw)
	}

	pub async fn msg_wait<M>(&self, tags: serde_json::Value, body: M) -> GlobalResult<()>
	where
		M: Message,
	{
		self.msg_ctx
			.message_wait(tags, body)
			.await
			.map_err(GlobalError::raw)
	}
}

impl ActivityCtx {
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
