use anyhow::*;
use epoxy_protocol::protocol;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use serde::{Deserialize, Serialize};

use super::State;
use crate::types;

#[tracing::instrument(skip_all)]
pub async fn replica_status_change(
	ctx: &mut WorkflowCtx,
	signal: super::ReplicaStatusChange,
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

#[tracing::instrument(skip_all)]
pub async fn replica_reconfigure(ctx: &mut WorkflowCtx) -> Result<()> {
	ctx.activity(UpdateReplicaUrlsInput {}).await?;

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

	tracing::debug!(
		replica_id=?input.replica_id,
		new_status=?input.new_status,
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

	tracing::debug!(new_epoch = state.config.epoch, "incremented epoch");

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UpdateReplicaUrlsInput {}

#[activity(UpdateReplicaUrls)]
pub async fn update_replica_urls(ctx: &ActivityCtx, _input: &UpdateReplicaUrlsInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	// Update URLs for all replicas based on topology
	for replica in state.config.replicas.iter_mut() {
		let Some(dc) = ctx.config().dc_for_label(replica.replica_id as u16) else {
			tracing::warn!(
				replica_id=?replica.replica_id,
				"datacenter not found for replica, skipping url update"
			);
			continue;
		};

		replica.api_peer_url = dc.peer_url.to_string();
		replica.guard_url = dc.public_url.to_string();

		tracing::debug!(
			replica_id=?replica.replica_id,
			api_peer_url=?dc.peer_url,
			guard_url=?dc.public_url,
			"updated replica urls"
		);
	}

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

	tracing::debug!(
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

			crate::http_client::send_message(&ApiCtx::new_from_activity(&ctx)?, &config, request)
				.await
				.with_context(|| format!("failed to update config for replica {}", replica_id))?;

			tracing::debug!(?replica_id, "config update sent");
			Ok(())
		}
	});

	futures_util::future::try_join_all(update_futures).await?;

	Ok(NotifyAllReplicasOutput {
		config: state.config.clone(),
	})
}
