use std::ops::Deref;

use anyhow::Result;
use rivet_util::Id;
use serde::Serialize;
use tracing::Instrument;

use crate::{
	builder::{WorkflowRepr, common as builder},
	ctx::{ActivityCtx, MessageCtx, OperationCtx, common, message::SubscriptionHandle},
	db::{DatabaseHandle, WorkflowData},
	error::WorkflowResult,
	message::Message,
	operation::{Operation, OperationInput},
	signal::Signal,
	utils::tags::AsTags,
	workflow::{Workflow, WorkflowInput},
};

#[derive(Clone)]
pub struct StandaloneCtx {
	ray_id: Id,
	req_id: Id,
	name: String,
	ts: i64,

	db: DatabaseHandle,

	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	cache: rivet_cache::Cache,
	msg_ctx: MessageCtx,
	from_workflow: bool,
}

impl StandaloneCtx {
	#[tracing::instrument(skip_all, fields(name, ray_id, req_id))]
	pub fn new(
		db: DatabaseHandle,
		config: rivet_config::Config,
		pools: rivet_pools::Pools,
		cache: rivet_cache::Cache,
		name: &str,
		ray_id: Id,
		req_id: Id,
	) -> WorkflowResult<Self> {
		let ts = rivet_util::timestamp::now();

		tracing::Span::current()
			.record("req_id", req_id.to_string())
			.record("ray_id", ray_id.to_string());

		let msg_ctx = MessageCtx::new(&config, &pools, &cache, ray_id)?;

		Ok(StandaloneCtx {
			ray_id,
			req_id,
			name: name.to_string(),
			ts,
			db,
			config,
			pools,
			cache,
			msg_ctx,
			from_workflow: false,
		})
	}

	#[tracing::instrument(skip_all)]
	pub fn new_from_activity(ctx: &ActivityCtx, req_id: Id) -> WorkflowResult<Self> {
		let mut ctx = StandaloneCtx::new(
			ctx.db().clone(),
			ctx.config().clone(),
			ctx.pools().clone(),
			ctx.cache().clone(),
			ctx.name(),
			ctx.ray_id(),
			req_id,
		)?;

		ctx.from_workflow = true;

		Ok(ctx)
	}

	#[tracing::instrument(skip_all)]
	pub fn new_from_operation(ctx: &OperationCtx, req_id: Id) -> WorkflowResult<Self> {
		let mut ctx = StandaloneCtx::new(
			ctx.db().clone(),
			ctx.config().clone(),
			ctx.pools().clone(),
			ctx.cache().clone(),
			ctx.name(),
			ctx.ray_id(),
			req_id,
		)?;

		ctx.from_workflow = ctx.from_workflow;

		Ok(ctx)
	}
}

impl StandaloneCtx {
	// TODO: Should be T: Workflow instead of I: WorkflowInput but rust can't figure it out
	/// Creates a workflow builder.
	pub fn workflow<I>(
		&self,
		input: impl WorkflowRepr<I>,
	) -> builder::workflow::WorkflowBuilder<impl WorkflowRepr<I>, I>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		builder::workflow::WorkflowBuilder::new(
			self.db.clone(),
			self.config.clone(),
			self.ray_id,
			input,
			self.from_workflow,
		)
	}

	/// Finds the first incomplete workflow with the given tags.
	#[tracing::instrument(skip_all, ret(Debug), fields(workflow_name=W::NAME))]
	pub async fn find_workflow<W: Workflow>(&self, tags: impl AsTags) -> Result<Option<Id>> {
		common::find_workflow::<W>(&self.db, tags)
			.in_current_span()
			.await
	}

	/// Finds the first incomplete workflow with the given tags.
	#[tracing::instrument(skip_all)]
	pub async fn get_workflows(&self, workflow_ids: Vec<Id>) -> Result<Vec<WorkflowData>> {
		common::get_workflows(&self.db, workflow_ids)
			.in_current_span()
			.await
	}

	/// Creates a signal builder.
	pub fn signal<T: Signal + Serialize>(&self, body: T) -> builder::signal::SignalBuilder<T> {
		builder::signal::SignalBuilder::new(
			self.db.clone(),
			self.config.clone(),
			self.ray_id,
			body,
			self.from_workflow,
		)
	}

	#[tracing::instrument(skip_all, fields(operation_name=I::Operation::NAME))]
	pub async fn op<I>(
		&self,
		input: I,
	) -> Result<<<I as OperationInput>::Operation as Operation>::Output>
	where
		I: OperationInput,
		<I as OperationInput>::Operation: Operation<Input = I>,
	{
		common::op(
			&self.db,
			&self.config,
			&self.pools,
			&self.cache,
			self.ray_id,
			self.from_workflow,
			input,
		)
		.in_current_span()
		.await
	}

	/// Creates a message builder.
	pub fn msg<M: Message>(&self, body: M) -> builder::message::MessageBuilder<M> {
		builder::message::MessageBuilder::new(self.msg_ctx.clone(), body)
	}

	#[tracing::instrument(skip_all, fields(message=M::NAME))]
	pub async fn subscribe<M>(&self, tags: impl AsTags) -> Result<SubscriptionHandle<M>>
	where
		M: Message,
	{
		self.msg_ctx
			.subscribe::<M>(tags)
			.in_current_span()
			.await
			.map_err(Into::into)
	}
}

impl StandaloneCtx {
	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn ray_id(&self) -> Id {
		self.ray_id
	}

	pub fn req_id(&self) -> Id {
		self.req_id
	}

	/// Timestamp at which the request started.
	pub fn ts(&self) -> i64 {
		self.ts
	}

	pub fn pools(&self) -> &rivet_pools::Pools {
		&self.pools
	}

	pub fn cache(&self) -> &rivet_cache::Cache {
		&self.cache
	}

	pub fn config(&self) -> &rivet_config::Config {
		&self.config
	}
}

impl Deref for StandaloneCtx {
	type Target = rivet_pools::Pools;

	fn deref(&self) -> &Self::Target {
		&self.pools
	}
}
