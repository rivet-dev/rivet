use anyhow::Result;
use epoxy_protocol::protocol;
use futures_util::FutureExt;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
struct CatchUpState {
	last_versionstamp: Option<Vec<u8>>,
	applied_entries: usize,
}

#[tracing::instrument(skip_all)]
pub async fn setup_replica(ctx: &mut WorkflowCtx, _input: &super::Input) -> Result<()> {
	// Block until the coordinator sends BeginLearning. On fresh clusters with no
	// data to catch up the signal still arrives so the replica transitions to active.
	let sig = ctx.listen::<super::BeginLearning>().await?;
	begin_learning(ctx, &sig).await?;

	Ok(())
}

#[tracing::instrument(skip_all, fields(replica_id = %ctx.config().epoxy_replica_id()))]
pub async fn begin_learning(ctx: &mut WorkflowCtx, signal: &super::BeginLearning) -> Result<()> {
	ctx.activity(StoreConfigInput {
		config: signal.config.clone(),
	})
	.await?;

	ctx.removed::<Activity<CatchUpReplica>>().await?;

	ctx.v(2)
		.loope(CatchUpState::default(), |ctx, state| {
			let config = signal.config.clone();
			async move {
				let res = ctx
					.activity(CatchUpReplicaInput {
						config: config.clone(),
						after_versionstamp: state.last_versionstamp.clone(),
					})
					.await?;

				state.last_versionstamp = res.last_versionstamp;
				state.applied_entries += res.applied_entries;

				if state.last_versionstamp.is_none() {
					return Ok(Loop::Break(()));
				}

				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await?;

	ctx.activity(NotifyCoordinatorReplicaStatusInput {
		config: signal.config.clone(),
		status: crate::types::ReplicaStatus::Active,
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct StoreConfigInput {
	config: crate::types::ClusterConfig,
}

#[activity(StoreConfig)]
async fn store_config(ctx: &ActivityCtx, input: &StoreConfigInput) -> Result<()> {
	let replica_id = ctx.config().epoxy_replica_id();
	let update_req = protocol::UpdateConfigRequest {
		config: input.config.clone().into(),
	};

	ctx.udb()?
		.run(|tx| {
			let update_req = update_req.clone();
			async move { crate::replica::update_config::update_config(&*tx, replica_id, update_req) }
		})
		.custom_instrument(tracing::info_span!("store_replica_config_tx"))
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CatchUpReplicaInput {
	config: crate::types::ClusterConfig,
	after_versionstamp: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CatchUpReplicaOutput {
	last_versionstamp: Option<Vec<u8>>,
	applied_entries: usize,
}

#[activity(CatchUpReplica)]
async fn catch_up_replica(
	_ctx: &ActivityCtx,
	_input: &CatchUpReplicaInput,
) -> Result<CatchUpReplicaOutput> {
	Ok(CatchUpReplicaOutput {
		last_versionstamp: None,
		applied_entries: 0,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct NotifyCoordinatorReplicaStatusInput {
	config: crate::types::ClusterConfig,
	status: crate::types::ReplicaStatus,
}

#[activity(NotifyCoordinatorReplicaStatus)]
async fn notify_coordinator_replica_status(
	ctx: &ActivityCtx,
	input: &NotifyCoordinatorReplicaStatusInput,
) -> Result<()> {
	let config: protocol::ClusterConfig = input.config.clone().into();
	let replica_id = ctx.config().epoxy_replica_id();

	crate::http_client::send_message(
		&ApiCtx::new_from_activity(ctx)?,
		&config,
		protocol::Request {
			from_replica_id: replica_id,
			to_replica_id: config.coordinator_replica_id,
			kind: protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(
				protocol::CoordinatorUpdateReplicaStatusRequest {
					replica_id,
					status: input.status.clone().into(),
				},
			),
		},
	)
	.await?;

	Ok(())
}
