use anyhow::*;
use epoxy_protocol::protocol;
use gas::prelude::*;
use serde::{Deserialize, Serialize};

use super::State;
use crate::types;

pub async fn replica_status_change(
	ctx: &mut WorkflowCtx,
	signal: super::ReplicaStatusChangeSignal,
) -> Result<()> {
	// Update replica status
	let should_increment_epoch = ctx
		.activity(UpdateReplicaStatusInput {
			replica_id: signal.replica_id,
			new_status: signal.status.into(),
		})
		.await?;

	if should_increment_epoch {
		ctx.activity(IncrementEpochInput {}).await?;
	}

	let notify_out = ctx.activity(NotifyAllReplicasInput {}).await?;

	let replica_id = ctx.config().epoxy_replica_id();
	ctx.msg(super::ConfigChangeMessage {
		config: notify_out.config,
	})
	.tag("replica", replica_id)
	.send()
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UpdateReplicaStatusInput {
	pub replica_id: protocol::ReplicaId,
	pub new_status: protocol::ReplicaStatus,
}

#[activity(UpdateReplicaStatus)]
pub async fn update_replica_status(
	ctx: &ActivityCtx,
	input: &UpdateReplicaStatusInput,
) -> Result<bool> {
	let mut state = ctx.state::<State>()?;

	// Check if replica exists
	let replica_state = state
		.config
		.replicas
		.iter_mut()
		.find(|r| r.replica_id == input.replica_id)
		.with_context(|| format!("replica {} not found", input.replica_id))?;

	let was_active = matches!(replica_state.status, types::ReplicaStatus::Active);
	let now_active = matches!(
		input.new_status.clone().into(),
		types::ReplicaStatus::Active
	);
	let should_increment_epoch = !was_active && now_active;

	// Update status
	replica_state.status = input.new_status.clone().into();

	tracing::info!(
		replica_id = ?input.replica_id,
		new_status = ?input.new_status,
		"updated replica status"
	);

	Ok(should_increment_epoch)
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct IncrementEpochInput {}

#[activity(IncrementEpoch)]
pub async fn increment_epoch(ctx: &ActivityCtx, _input: &IncrementEpochInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	state.config.epoch += 1;

	tracing::info!(new_epoch = state.config.epoch, "incremented epoch");

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct NotifyAllReplicasInput {}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct NotifyAllReplicasOutput {
	config: types::ClusterConfig,
}

#[activity(NotifyAllReplicas)]
pub async fn notify_all_replicas(
	ctx: &ActivityCtx,
	_input: &NotifyAllReplicasInput,
) -> Result<NotifyAllReplicasOutput> {
	let state = ctx.state::<State>()?;

	let config: protocol::ClusterConfig = state.config.clone().into();

	tracing::info!(
		epoch = config.epoch,
		replica_count = config.replicas.len(),
		"notifying all replicas of config change"
	);

	// Send update config to all replicas
	let update_futures = config.replicas.iter().map(|replica| {
		let replica_id = replica.replica_id;
		let config = config.clone();

		async move {
			let request = protocol::Request {
				from_replica_id: config.coordinator_replica_id,
				to_replica_id: replica_id,
				kind: protocol::RequestKind::UpdateConfigRequest(protocol::UpdateConfigRequest {
					config: config.clone(),
				}),
			};

			crate::http_client::send_message(&config, replica_id, request)
				.await
				.with_context(|| format!("failed to update config for replica {}", replica_id))?;

			tracing::info!(?replica_id, "config update sent");
			Ok(())
		}
	});

	futures_util::future::try_join_all(update_futures).await?;

	Ok(NotifyAllReplicasOutput {
		config: state.config.clone(),
	})
}
