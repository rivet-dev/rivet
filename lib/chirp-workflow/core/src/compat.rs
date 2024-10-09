/// Forwards compatibility from old operation ctx to new workflows.
use std::fmt::Debug;

use global_error::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::{
	builder::common as builder,
	builder::BuilderError,
	ctx::{
		common,
		message::{MessageCtx, SubscriptionHandle},
	},
	db::{DatabaseHandle, DatabasePgNats},
	message::Message,
	operation::{Operation, OperationInput},
	signal::Signal,
	workflow::{Workflow, WorkflowInput},
};

/// Wait for a given workflow to complete.
/// 60 second timeout.
pub async fn wait_for_workflow<W: Workflow, B: Debug + Clone>(
	ctx: &rivet_operation::OperationContext<B>,
	workflow_id: Uuid,
) -> GlobalResult<W::Output> {
	let db = db_from_ctx(ctx).await?;

	common::wait_for_workflow::<W>(&db, workflow_id).await
}

/// Dispatch a new workflow and wait for it to complete. Has a 60s timeout.
pub async fn workflow<I, B>(
	ctx: &rivet_operation::OperationContext<B>,
	input: I,
) -> GlobalResult<builder::workflow::WorkflowBuilder<I>>
where
	I: WorkflowInput,
	<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	B: Debug + Clone,
{
	if ctx.from_workflow {
		return Err(BuilderError::CannotDispatchFromOpInWorkflow.into());
	}

	let db = db_from_ctx(ctx).await?;

	Ok(builder::workflow::WorkflowBuilder::new(
		db,
		ctx.ray_id(),
		input,
	))
}

/// Creates a signal builder.
pub async fn signal<T: Signal + Serialize, B: Debug + Clone>(
	ctx: &rivet_operation::OperationContext<B>,
	body: T,
) -> GlobalResult<builder::signal::SignalBuilder<T>> {
	if ctx.from_workflow {
		return Err(BuilderError::CannotDispatchFromOpInWorkflow.into());
	}

	let db = db_from_ctx(ctx).await?;

	Ok(builder::signal::SignalBuilder::new(db, ctx.ray_id(), body))
}

#[tracing::instrument(err, skip_all, fields(operation = I::Operation::NAME))]
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
		ctx.conn(),
		ctx.ray_id(),
		ctx.req_ts(),
		ctx.from_workflow(),
		input,
	)
	.await
}

pub async fn subscribe<M, B>(
	ctx: &rivet_operation::OperationContext<B>,
	tags: &serde_json::Value,
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

// Get pool as a trait object
async fn db_from_ctx<B: Debug + Clone>(
	ctx: &rivet_operation::OperationContext<B>,
) -> GlobalResult<DatabaseHandle> {
	let crdb = ctx.crdb().await?;
	let nats = ctx.conn().nats().await?;

	Ok(DatabasePgNats::from_pools(crdb, nats))
}

// Get crdb pool as a trait object
pub async fn db_from_pools(pools: &rivet_pools::Pools) -> GlobalResult<DatabaseHandle> {
	let crdb = pools.crdb()?;
	let nats = pools.nats()?;

	Ok(DatabasePgNats::from_pools(crdb, nats))
}
