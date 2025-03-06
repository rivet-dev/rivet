use global_error::{GlobalError, GlobalResult};
use rivet_pools::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::{
	builder::common as builder,
	ctx::{
		common,
		message::{MessageCtx, SubscriptionHandle, TailAnchor, TailAnchorResponse},
	},
	db::DatabaseHandle,
	error::WorkflowResult,
	message::{AsTags, Message, NatsMessage},
	operation::{Operation, OperationInput},
	signal::Signal,
	workflow::{Workflow, WorkflowInput},
};

// NOTE: Clonable because of inner arcs
#[derive(Clone)]
pub struct ApiCtx {
	ray_id: Uuid,
	name: String,
	ts: i64,

	db: DatabaseHandle,

	config: rivet_config::Config,
	conn: rivet_connection::Connection,
	msg_ctx: MessageCtx,

	// Backwards compatibility
	op_ctx: rivet_operation::OperationContext<()>,
}

impl ApiCtx {
	pub async fn new(
		db: DatabaseHandle,
		config: rivet_config::Config,
		conn: rivet_connection::Connection,
		req_id: Uuid,
		ray_id: Uuid,
		ts: i64,
		name: String,
	) -> WorkflowResult<Self> {
		let op_ctx = rivet_operation::OperationContext::new(
			name.clone(),
			std::time::Duration::from_secs(60),
			config.clone(),
			conn.clone(),
			req_id,
			ray_id,
			ts,
			ts,
			(),
		);

		let msg_ctx = MessageCtx::new(&conn, ray_id).await?;

		Ok(ApiCtx {
			ray_id,
			name,
			ts,
			db,
			config,
			conn,
			op_ctx,
			msg_ctx,
		})
	}
}

impl ApiCtx {
	/// Wait for a given workflow to complete.
	/// 60 second timeout.
	pub async fn wait_for_workflow<W: Workflow>(
		&self,
		workflow_id: Uuid,
	) -> GlobalResult<W::Output> {
		common::wait_for_workflow::<W>(&self.db, workflow_id).await
	}

	/// Creates a workflow builder.
	pub fn workflow<I>(&self, input: I) -> builder::workflow::WorkflowBuilder<I>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		builder::workflow::WorkflowBuilder::new(self.db.clone(), self.ray_id, input, false)
	}

	/// Creates a signal builder.
	pub fn signal<T: Signal + Serialize>(&self, body: T) -> builder::signal::SignalBuilder<T> {
		builder::signal::SignalBuilder::new(self.db.clone(), self.ray_id, body, false)
	}

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
			false,
			input,
		)
		.await
	}

	/// Creates a message builder.
	pub fn msg<M: Message>(&self, body: M) -> builder::message::MessageBuilder<M> {
		builder::message::MessageBuilder::new(self.msg_ctx.clone(), body)
	}

	pub async fn subscribe<M>(&self, tags: impl AsTags) -> GlobalResult<SubscriptionHandle<M>>
	where
		M: Message,
	{
		self.msg_ctx
			.subscribe::<M>(tags)
			.await
			.map_err(GlobalError::raw)
	}

	pub async fn tail_read<M>(&self, tags: impl AsTags) -> GlobalResult<Option<NatsMessage<M>>>
	where
		M: Message,
	{
		self.msg_ctx
			.tail_read::<M>(tags)
			.await
			.map_err(GlobalError::raw)
	}

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

impl ApiCtx {
	pub fn name(&self) -> &str {
		&self.name
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

	// Backwards compatibility
	pub fn op_ctx(&self) -> &rivet_operation::OperationContext<()> {
		&self.op_ctx
	}
}
