use global_error::{GlobalError, GlobalResult};
use rivet_pools::prelude::*;
use serde::Serialize;
use tokio::time::Duration;
use uuid::Uuid;

use crate::{
	builder::common as builder,
	ctx::{
		common,
		message::{SubscriptionHandle, TailAnchor, TailAnchorResponse},
		MessageCtx,
	},
	db::{DatabaseHandle, DatabasePgNats},
	error::WorkflowError,
	message::{Message, NatsMessage},
	operation::{Operation, OperationInput},
	signal::Signal,
	utils,
	workflow::{Workflow, WorkflowInput},
};

pub struct TestCtx {
	name: String,
	ray_id: Uuid,
	ts: i64,

	db: DatabaseHandle,

	conn: rivet_connection::Connection,
	msg_ctx: MessageCtx,

	// Backwards compatibility
	op_ctx: rivet_operation::OperationContext<()>,
}

impl TestCtx {
	pub async fn from_env(test_name: &str) -> TestCtx {
		let service_name = format!(
			"{}-test--{}",
			std::env::var("CHIRP_SERVICE_NAME").unwrap(),
			test_name
		);

		let ray_id = Uuid::new_v4();
		let pools = rivet_pools::from_env(service_name.clone())
			.await
			.expect("failed to create pools");
		let shared_client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("failed to create chirp client");
		let cache =
			rivet_cache::CacheInner::from_env(pools.clone()).expect("failed to create cache");
		let conn = utils::new_conn(
			&shared_client,
			&pools,
			&cache,
			ray_id,
			Uuid::new_v4(),
			&service_name,
		);
		let ts = rivet_util::timestamp::now();
		let req_id = Uuid::new_v4();
		let op_ctx = rivet_operation::OperationContext::new(
			service_name.to_string(),
			std::time::Duration::from_secs(60),
			conn.clone(),
			req_id,
			ray_id,
			ts,
			ts,
			(),
		);

		let db =
			DatabasePgNats::from_pools(pools.crdb().unwrap(), pools.nats_option().clone().unwrap());
		let msg_ctx = MessageCtx::new(&conn, ray_id).await.unwrap();

		TestCtx {
			name: service_name,
			ray_id,
			ts: rivet_util::timestamp::now(),
			db,
			conn,
			op_ctx,
			msg_ctx,
		}
	}
}

impl TestCtx {
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

	#[tracing::instrument(err, skip_all, fields(operation = I::Operation::NAME))]
	pub async fn op<I>(
		&self,
		input: I,
	) -> GlobalResult<<<I as OperationInput>::Operation as Operation>::Output>
	where
		I: OperationInput,
		<I as OperationInput>::Operation: Operation<Input = I>,
	{
		common::op(&self.db, &self.conn, self.ray_id, self.ts, false, input).await
	}

	/// Waits for a workflow to be triggered with a superset of given input. Strictly for tests.
	pub fn observe<W: Workflow>(&self, input: serde_json::Value) -> GlobalResult<ObserveHandle> {
		// Serialize input
		let input_val = serde_json::to_value(input)
			.map_err(WorkflowError::SerializeWorkflowOutput)
			.map_err(GlobalError::raw)?;

		Ok(ObserveHandle {
			db: self.db.clone(),
			name: W::NAME,
			input: input_val,
			ts: rivet_util::timestamp::now(),
		})
	}

	pub async fn msg<M>(&self, body: M) -> builder::message::MessageBuilder<M>
	where
		M: Message,
	{
		builder::message::MessageBuilder::new(&self.msg_ctx, body)
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

impl TestCtx {
	pub fn name(&self) -> &str {
		&self.name
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

/// Like a subscription handle for messages but for workflows. Should only be used in tests
pub struct ObserveHandle {
	db: DatabaseHandle,
	name: &'static str,
	input: serde_json::Value,
	ts: i64,
}

impl ObserveHandle {
	pub async fn next(&mut self) -> GlobalResult<Uuid> {
		tracing::info!(name=%self.name, input=?self.input, "observing workflow");

		let (workflow_id, create_ts) = loop {
			if let Some((workflow_id, create_ts)) = self
				.db
				.poll_workflow(self.name, &self.input, self.ts)
				.await
				.map_err(GlobalError::raw)?
			{
				break (workflow_id, create_ts);
			}

			tokio::time::sleep(Duration::from_millis(200)).await;
		};

		tracing::info!(name=%self.name, id=?workflow_id, "workflow found");

		self.ts = create_ts + 1;

		Ok(workflow_id)
	}
}
