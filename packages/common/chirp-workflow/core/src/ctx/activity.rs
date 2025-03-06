use global_error::{GlobalError, GlobalResult};
use rivet_pools::prelude::*;
use uuid::Uuid;

use crate::{
	ctx::{
		common,
		message::{MessageCtx, SubscriptionHandle, TailAnchor, TailAnchorResponse},
	},
	db::DatabaseHandle,
	error::WorkflowResult,
	message::{Message, NatsMessage},
	operation::{Operation, OperationInput},
	utils::tags::AsTags,
};

#[derive(Clone)]
pub struct ActivityCtx {
	workflow_id: Uuid,
	workflow_name: String,
	ray_id: Uuid,
	name: &'static str,
	ts: i64,

	db: DatabaseHandle,

	config: rivet_config::Config,
	conn: rivet_connection::Connection,
	msg_ctx: MessageCtx,

	// Backwards compatibility
	op_ctx: rivet_operation::OperationContext<()>,
}

impl ActivityCtx {
	pub async fn new(
		workflow_id: Uuid,
		workflow_name: String,
		db: DatabaseHandle,
		config: &rivet_config::Config,
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
			config.clone(),
			conn.clone(),
			req_id,
			ray_id,
			ts,
			activity_create_ts,
			(),
		);
		op_ctx.from_workflow = true;

		let msg_ctx = MessageCtx::new(&conn, ray_id).await?;

		Ok(ActivityCtx {
			workflow_id,
			workflow_name,
			ray_id,
			name,
			ts,
			db,
			config: config.clone(),
			conn,
			msg_ctx,
			op_ctx,
		})
	}
}

impl ActivityCtx {
	#[tracing::instrument(err, skip_all, fields(operation = I::Operation::NAME))]
	pub async fn op<I>(
		&self,
		input: I,
	) -> GlobalResult<<<I as OperationInput>::Operation as Operation>::Output>
	where
		I: OperationInput,
		<I as OperationInput>::Operation: Operation<Input = I>,
	{
		common::op(
			&self.db,
			&self.config,
			&self.conn,
			self.ray_id,
			self.op_ctx.req_ts(),
			true,
			input,
		)
		.await
	}

	// TODO: Theres nothing preventing this from being able to be called from the workflow ctx also, but for
	// now its only in the activity ctx so it isn't called again during workflow retries
	pub async fn update_workflow_tags(&self, tags: &serde_json::Value) -> GlobalResult<()> {
		self.db
			.update_workflow_tags(self.workflow_id, &self.workflow_name, tags)
			.await
			.map_err(GlobalError::raw)
	}

	/// IMPORTANT: This is intended for ephemeral realtime events and should be used carefully. Use
	/// signals if you need this to be durable.
	pub async fn subscribe<M>(&self, tags: impl AsTags) -> GlobalResult<SubscriptionHandle<M>>
	where
		M: Message,
	{
		self.msg_ctx
			.subscribe::<M>(tags)
			.await
			.map_err(GlobalError::raw)
	}

	/// IMPORTANT: This is intended for ephemeral realtime events and should be used carefully. Use
	/// signals if you need this to be durable.
	pub async fn tail_read<M>(&self, tags: impl AsTags) -> GlobalResult<Option<NatsMessage<M>>>
	where
		M: Message,
	{
		self.msg_ctx
			.tail_read::<M>(tags)
			.await
			.map_err(GlobalError::raw)
	}

	/// IMPORTANT: This is intended for ephemeral realtime events and should be used carefully. Use
	/// signals if you need this to be durable.
	pub async fn tail_anchor<M>(
		&self,
		tags: impl AsTags,
		anchor: &TailAnchor,
	) -> GlobalResult<TailAnchorResponse<M>>
	where
		M: Message,
	{
		self.msg_ctx
			.tail_anchor::<M>(tags, anchor)
			.await
			.map_err(GlobalError::raw)
	}
}

impl ActivityCtx {
	pub fn name(&self) -> &str {
		self.name
	}

	pub fn workflow_id(&self) -> Uuid {
		self.workflow_id
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

	pub fn config(&self) -> &rivet_config::Config {
		&self.config
	}

	pub fn trace(&self) -> &[chirp_client::TraceEntry] {
		self.conn.trace()
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

	pub async fn clickhouse(&self) -> GlobalResult<ClickHousePool> {
		self.conn.clickhouse().await
	}

	pub async fn fdb(&self) -> Result<FdbPool, rivet_pools::Error> {
		self.conn.fdb().await
	}

	/// Access the SQLite database for this workflow. This cannot access any other database.
	pub async fn sqlite(&self) -> Result<SqlitePool, rivet_pools::Error> {
		self.conn
			.sqlite(
				crate::db::sqlite_db_name_data(self.workflow_id),
				false,
			)
			.await
	}

	pub async fn sqlite_for_workflow(&self, workflow_id: Uuid) -> GlobalResult<SqlitePool> {
		common::sqlite_for_workflow(&self.db, &self.conn, workflow_id, true).await
	}

	// Backwards compatibility
	pub fn op_ctx(&self) -> &rivet_operation::OperationContext<()> {
		&self.op_ctx
	}
}
