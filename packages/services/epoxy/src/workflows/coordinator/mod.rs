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
pub async fn coordinator(ctx: &mut WorkflowCtx, _input: &Input) -> Result<()> {
	ctx.activity(InitInput {}).await?;

	ctx.repeat(|ctx| {
		async move {
			match ctx.listen::<Main>().await? {
				Main::ReconfigureSignal(_) => {
					reconfigure::reconfigure(ctx).await?;
				}
				Main::ReplicaStatusChangeSignal(sig) => {
					replica_status_change::replica_status_change(ctx, sig).await?;
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

#[message("epoxy_coordinator_config_update")]
pub struct ConfigChangeMessage {
	pub config: types::ClusterConfig,
}

/// Idempotent signal to call any time there is a potential config change.
///
/// This gets called any time an engine node starts.
#[signal("epoxy_coordinator_reconfigure")]
pub struct ReconfigureSignal {}

#[signal("epoxy_coordinator_replica_status_change")]
pub struct ReplicaStatusChangeSignal {
	pub replica_id: protocol::ReplicaId,
	pub status: types::ReplicaStatus,
}

join_signal!(Main {
	ReconfigureSignal,
	ReplicaStatusChangeSignal,
});
