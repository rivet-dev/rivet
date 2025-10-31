use anyhow::*;
use epoxy_protocol::protocol;
use futures_util::FutureExt;
use gas::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types;

pub mod reconfigure;
pub mod replica_status_change;

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct State {
	pub config: types::ClusterConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReplicaState {
	pub status: protocol::ReplicaStatus,
	pub api_peer_url: String,
	pub guard_url: String,
}

#[workflow]
pub async fn epoxy_coordinator(ctx: &mut WorkflowCtx, _input: &Input) -> Result<()> {
	ctx.activity(InitInput {}).await?;

	ctx.repeat(|ctx| {
		async move {
			match ctx.listen::<Main>().await? {
				Main::Reconfigure(_) => {
					reconfigure::reconfigure(ctx).await?;
				}
				Main::ReplicaStatusChange(sig) => {
					replica_status_change::replica_status_change(ctx, sig).await?;
				}
				Main::ReplicaReconfigure(_) => {
					replica_status_change::replica_reconfigure(ctx).await?;
				}
				Main::OverrideState(sig) => {
					ctx.activity(OverrideStateActivityInput { config: sig.config })
						.await?;

					reconfigure::reconfigure(ctx).await?;
				}
			}

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct InitInput {}

#[activity(Init)]
pub async fn check_config_changes(ctx: &ActivityCtx, _input: &InitInput) -> Result<()> {
	let mut state = ctx.state::<Option<State>>()?;
	*state = Some(State {
		config: types::ClusterConfig {
			coordinator_replica_id: ctx.config().epoxy_replica_id(),
			epoch: 0,
			replicas: Vec::new(),
		},
	});
	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct OverrideStateActivityInput {
	pub config: types::ClusterConfig,
}

#[activity(OverrideStateActivity)]
pub async fn override_state_activity(
	ctx: &ActivityCtx,
	input: &OverrideStateActivityInput,
) -> Result<()> {
	let mut state = ctx.state::<State>()?;
	state.config = input.config.clone();
	Ok(())
}

#[message("epoxy_coordinator_config_update")]
pub struct ConfigChangeMessage {
	pub config: types::ClusterConfig,
}

/// Idempotent signal to call any time there is a potential config change.
///
/// This gets called any time an engine node starts.
#[signal("epoxy_coordinator_reconfigure")]
pub struct Reconfigure {}

#[signal("epoxy_coordinator_replica_status_change")]
pub struct ReplicaStatusChange {
	pub replica_id: protocol::ReplicaId,
	pub status: types::ReplicaStatus,
}

#[signal("epoxy_coordinator_replica_reconfigure")]
pub struct ReplicaReconfigure {}

#[signal("epoxy_coordinator_override_state")]
pub struct OverrideState {
	pub config: types::ClusterConfig,
}

join_signal!(Main {
	Reconfigure,
	ReplicaStatusChange,
	ReplicaReconfigure,
	OverrideState,
});
