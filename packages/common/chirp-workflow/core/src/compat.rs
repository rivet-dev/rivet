/// Forwards compatibility from old operation ctx to new workflows.
use std::fmt::Debug;

use global_error::prelude::*;
use serde::Serialize;
use tracing::Instrument;

use crate::{
	builder::{common as builder, WorkflowRepr},
	ctx::{
		common,
		message::{MessageCtx, SubscriptionHandle},
	},
	db::{Database, DatabaseCrdbNats, DatabaseHandle},
	message::Message,
	operation::{Operation, OperationInput},
	signal::Signal,
	utils::tags::AsTags,
	workflow::{Workflow, WorkflowInput},
};

/// Dispatch a new workflow and wait for it to complete. Has a 60s timeout.
#[tracing::instrument(skip_all)]
pub async fn workflow<T, I, B>(
	ctx: &rivet_operation::OperationContext<B>,
	input: T,
) -> GlobalResult<builder::workflow::WorkflowBuilder<T, I>>
where
	T: WorkflowRepr<I>,
	I: WorkflowInput,
	<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	B: Debug + Clone,
{
	let db = db_from_ctx(ctx).await?;

	Ok(builder::workflow::WorkflowBuilder::new(
		db,
		ctx.ray_id(),
		input,
		ctx.from_workflow,
	))
}

/// Creates a signal builder.
#[tracing::instrument(skip_all)]
pub async fn signal<T: Signal + Serialize, B: Debug + Clone>(
	ctx: &rivet_operation::OperationContext<B>,
	body: T,
) -> GlobalResult<builder::signal::SignalBuilder<T>> {
	let db = db_from_ctx(ctx).await?;

	Ok(builder::signal::SignalBuilder::new(
		db,
		ctx.ray_id(),
		body,
		ctx.from_workflow,
	))
}

/// Creates a message builder.
#[tracing::instrument(skip_all)]
pub async fn msg<M: Message, B: Debug + Clone>(
	ctx: &rivet_operation::OperationContext<B>,
	body: M,
) -> GlobalResult<builder::message::MessageBuilder<M>> {
	let msg_ctx = MessageCtx::new(ctx.conn(), ctx.ray_id()).await?;

	Ok(builder::message::MessageBuilder::new(msg_ctx, body))
}

#[tracing::instrument(skip_all, fields(operation_name=I::Operation::NAME))]
pub async fn op<I, B>(
	ctx: &rivet_operation::OperationContext<B>,
	input: I,
) -> GlobalResult<<<I as OperationInput>::Operation as Operation>::Output>
where
	I: OperationInput,
	<I as OperationInput>::Operation: Operation<Input = I>,
	B: Debug + Clone,
{
	let db = db_from_ctx(ctx).await?;
	common::op(
		&db,
		ctx.config(),
		ctx.conn(),
		ctx.ray_id(),
		ctx.req_ts(),
		ctx.from_workflow(),
		input,
	)
	.in_current_span()
	.await
}

#[tracing::instrument(skip_all)]
pub async fn subscribe<M, B>(
	ctx: &rivet_operation::OperationContext<B>,
	tags: impl AsTags,
) -> GlobalResult<SubscriptionHandle<M>>
where
	M: Message,
	B: Debug + Clone,
{
	let msg_ctx = MessageCtx::new(ctx.conn(), ctx.ray_id())
		.await
		.map_err(GlobalError::raw)?;

	msg_ctx.subscribe::<M>(tags).await.map_err(GlobalError::raw)
}

#[tracing::instrument(skip_all)]
async fn db_from_ctx<B: Debug + Clone>(
	ctx: &rivet_operation::OperationContext<B>,
) -> GlobalResult<DatabaseHandle> {
	DatabaseCrdbNats::from_pools(ctx.pools().clone())
		.map(|db| db as DatabaseHandle)
		.map_err(Into::into)
}
