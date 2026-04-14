use anyhow::{Context, Result};
use epoxy_protocol::protocol::{self, ReplicaId};
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use serde::{Deserialize, Serialize};

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
	ctx.activity(CatchUpReplicaInput {
		config: signal.config.clone(),
	})
	.await?;
	ctx.activity(NotifyCoordinatorReplicaStatusInput {
		config: signal.config.clone(),
		status: crate::types::ReplicaStatus::Active,
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct StoreConfigInput {
	pub config: crate::types::ClusterConfig,
}

#[activity(StoreConfig)]
pub async fn store_config(ctx: &ActivityCtx, input: &StoreConfigInput) -> Result<()> {
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

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct CatchUpReplicaInput {
	pub config: crate::types::ClusterConfig,
}

// TODO: Migrate to chunked catch up
#[activity(CatchUpReplica)]
#[timeout = 18000]
pub async fn catch_up_replica(ctx: &ActivityCtx, input: &CatchUpReplicaInput) -> Result<()> {
	let replica_id = ctx.config().epoxy_replica_id();
	let config: protocol::ClusterConfig = input.config.clone().into();
	let api_ctx = ApiCtx::new_from_activity(ctx)?;
	let source_replica_id = config
		.replicas
		.iter()
		.find(|replica| {
			replica.replica_id != replica_id
				&& matches!(replica.status, protocol::ReplicaStatus::Active)
		})
		.map(|replica| replica.replica_id);

	if source_replica_id.is_none() {
		tracing::info!(
			%replica_id,
			"skipping changelog catch-up because the cluster has no active source replica yet"
		);
		return Ok(());
	}
	let source_replica_id = source_replica_id.unwrap();

	// Pre-cutover committed values are readable via local dual-read fallback immediately. They only
	// become available to future learners after the background backfill populates the v2 changelog.
	let mut after_versionstamp = None;
	let mut applied_entries = 0usize;
	loop {
		let response = read_changelog_page(
			&api_ctx,
			&config,
			replica_id,
			source_replica_id,
			after_versionstamp.clone(),
		)
		.await?;

		if response.entries.is_empty() {
			break;
		}

		let page_entries = response.entries.len();
		let last_versionstamp = response.last_versionstamp.clone();
		for entry in response.entries {
			ctx.udb()?
				.run(|tx| {
					let entry = entry.clone();
					async move { crate::replica::changelog::apply_entry(&*tx, replica_id, entry).await }
				})
				.custom_instrument(tracing::info_span!("apply_changelog_entry_tx"))
				.await?;
		}

		applied_entries += page_entries;
		tracing::info!(
			%replica_id,
			%source_replica_id,
			applied_entries,
			page_entries,
			"applied changelog catch-up page"
		);

		after_versionstamp = Some(last_versionstamp);
	}

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct NotifyCoordinatorReplicaStatusInput {
	pub config: crate::types::ClusterConfig,
	pub status: crate::types::ReplicaStatus,
}

#[activity(NotifyCoordinatorReplicaStatus)]
pub async fn notify_coordinator_replica_status(
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

async fn read_changelog_page(
	api_ctx: &ApiCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	source_replica_id: ReplicaId,
	after_versionstamp: Option<Vec<u8>>,
) -> Result<protocol::ChangelogReadResponse> {
	crate::http_client::read_changelog(
		api_ctx,
		config,
		from_replica_id,
		source_replica_id,
		after_versionstamp,
		crate::consts::CHANGELOG_READ_COUNT,
	)
	.await
	.with_context(|| format!("failed reading changelog page from replica {source_replica_id}"))
}
