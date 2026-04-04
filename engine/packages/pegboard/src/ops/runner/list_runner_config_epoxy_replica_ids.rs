use std::collections::BTreeSet;

use anyhow::{Result, bail};
use epoxy_protocol::protocol::{ReplicaId, ReplicaStatus};
use gas::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub replicas: Vec<ReplicaId>,
}

#[operation]
pub async fn list_runner_config_epoxy_replica_ids(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Output> {
	let (enabled_dcs, cluster_config) = tokio::try_join!(
		ctx.op(crate::ops::runner::list_runner_config_enabled_dcs::Input {
			namespace_id: input.namespace_id,
			runner_name: input.runner_name.clone(),
		}),
		ctx.op(epoxy::ops::read_cluster_config::Input {}),
	)?;

	let active_replicas = cluster_config
		.config
		.replicas
		.into_iter()
		.filter(|replica| matches!(replica.status, ReplicaStatus::Active))
		.map(|replica| replica.replica_id)
		.collect::<BTreeSet<_>>();

	let replicas = enabled_dcs
		.dc_labels
		.into_iter()
		.map(|dc_label| dc_label as ReplicaId)
		.filter(|replica_id| active_replicas.contains(replica_id))
		.collect::<Vec<_>>();

	if replicas.is_empty() {
		bail!("resolved runner config epoxy replica set is empty");
	}

	Ok(Output { replicas })
}
