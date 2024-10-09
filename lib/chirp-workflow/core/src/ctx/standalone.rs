use global_error::{GlobalError, GlobalResult};
use rivet_pools::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::{
	builder::common as builder,
	ctx::{
		common,
		message::{SubscriptionHandle, TailAnchor, TailAnchorResponse},
		MessageCtx,
	},
	db::DatabaseHandle,
	error::WorkflowResult,
	listen::Listen,
	message::{Message, NatsMessage},
	operation::{Operation, OperationInput},
	signal::Signal,
	workflow::{Workflow, WorkflowInput},
};

#[derive(Clone)]
pub struct StandaloneCtx {
	ray_id: Uuid,
	name: &'static str,
	ts: i64,

	db: DatabaseHandle,

	conn: rivet_connection::Connection,
	msg_ctx: MessageCtx,

	// Backwards compatibility
	op_ctx: rivet_operation::OperationContext<()>,
}

impl StandaloneCtx {
	pub async fn new(
		db: DatabaseHandle,
		conn: rivet_connection::Connection,
		name: &'static str,
	) -> WorkflowResult<Self> {
		let req_id = Uuid::new_v4();
		let ray_id = Uuid::new_v4();
		let ts = rivet_util::timestamp::now();

		let op_ctx = rivet_operation::OperationContext::new(
			name.to_string(),
			std::time::Duration::from_secs(60),
			conn.clone(),
			req_id,
			ray_id,
			ts,
			ts,
			(),
		);

		let msg_ctx = MessageCtx::new(&conn, ray_id).await?;

		Ok(StandaloneCtx {
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

impl StandaloneCtx {
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
		builder::workflow::WorkflowBuilder::new(self.db.clone(), self.ray_id, input)
	}

	/// Creates a signal builder.
	pub fn signal<T: Signal + Serialize>(&self, body: T) -> builder::signal::SignalBuilder<T> {
		builder::signal::SignalBuilder::new(self.db.clone(), self.ray_id, body)
	}

	// /// Listens for a signal indefinitely.
	// pub async fn listen<T: Listen>(&mut self) -> GlobalResult<T> {
	// 	tracing::info!(name=%self.name, "listening for signal");

	// 	let ctx = ListenCtx::new(self);

	// 	T::listen(&ctx).await
	// }

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
			&self.conn,
			self.ray_id,
			self.op_ctx.req_ts(),
			false,
			input,
		)
		.await
	}

	pub async fn subscribe<M>(
		&self,
		tags: &serde_json::Value,
	) -> GlobalResult<SubscriptionHandle<M>>
	where
		M: Message,
	{
		self.msg_ctx
			.subscribe::<M>(tags)
			.await
			.map_err(GlobalError::raw)
	}

	pub async fn tail_read<M>(
		&self,
		tags: serde_json::Value,
	) -> GlobalResult<Option<NatsMessage<M>>>
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
		tags: serde_json::Value,
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

impl StandaloneCtx {
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
