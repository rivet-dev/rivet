use anyhow::*;
use epoxy_protocol::protocol;
use gas::prelude::*;
use rivet_api_builder::prelude::*;

use crate::{ops, replica};

#[tracing::instrument(skip_all)]
pub async fn message_request(
	ctx: &ApiCtx,
	request: protocol::Request,
) -> Result<protocol::Response> {
	message_request_inner(ctx, request).await
}

#[tracing::instrument(level = "debug", skip_all)]
async fn message_request_inner(
	ctx: &ApiCtx,
	request: protocol::Request,
) -> Result<protocol::Response> {
	let current_replica_id = ctx.config().epoxy_replica_id();

	let kind = match request.kind {
		protocol::RequestKind::UpdateConfigRequest(req) => {
			tracing::debug!(
				epoch = ?req.config.epoch,
				replica_count = req.config.replicas.len(),
				"received configuration update request"
			);

			ctx.udb()?
				.txn("epoxy_replica_update_config", |tx| {
					let req = req.clone();
					async move { replica::update_config::update_config(&*tx, current_replica_id, req) }
				})
				.custom_instrument(tracing::debug_span!("update_config_tx"))
				.await?;

			protocol::ResponseKind::UpdateConfigResponse
		}
		protocol::RequestKind::PrepareRequest(req) => {
			let response = ctx
				.udb()?
				.txn("epoxy_replica_prepare", |tx| {
					let req = req.clone();
					async move { replica::messages::prepare(&*tx, current_replica_id, req).await }
				})
				.custom_instrument(tracing::debug_span!("prepare_tx"))
				.await?;
			protocol::ResponseKind::PrepareResponse(response)
		}
		protocol::RequestKind::PreAcceptRequest(_) => {
			bail!("unreachable: pre accept is not sent over the wire")
		}
		protocol::RequestKind::AcceptRequest(req) => {
			let response = ctx
				.udb()?
				.txn("epoxy_replica_accept", |tx| {
					let req = req.clone();
					async move { replica::messages::accept(&*tx, current_replica_id, req).await }
				})
				.custom_instrument(tracing::debug_span!("accept_tx"))
				.await?;
			protocol::ResponseKind::AcceptResponse(response)
		}
		protocol::RequestKind::CommitRequest(req) => {
			let response = ctx
				.udb()?
				.txn("epoxy_replica_commit", |tx| {
					let req = req.clone();
					async move { replica::messages::commit(&*tx, current_replica_id, req).await }
				})
				.custom_instrument(tracing::debug_span!("commit_tx"))
				.await?;
			protocol::ResponseKind::CommitResponse(response)
		}
		protocol::RequestKind::ChangelogReadRequest(req) => {
			let response = ctx
				.udb()?
				.txn("epoxy_replica_changelog_read", |tx| {
					let req = req.clone();
					async move { replica::changelog::read(&*tx, current_replica_id, req).await }
				})
				.custom_instrument(tracing::debug_span!("changelog_read_tx"))
				.await?;
			protocol::ResponseKind::ChangelogReadResponse(response)
		}
		protocol::RequestKind::HealthCheckRequest => {
			tracing::debug!("received health check request");
			protocol::ResponseKind::HealthCheckResponse
		}
		protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(req) => {
			tracing::debug!(
				?current_replica_id,
				update_replica_id = ?req.replica_id,
				update_status = ?req.status,
				"received coordinator update replica status request"
			);

			ctx.signal(crate::workflows::coordinator::ReplicaStatusChange {
				replica_id: req.replica_id,
				status: req.status.into(),
			})
			.bypass_signal_from_workflow_I_KNOW_WHAT_IM_DOING()
			.to_workflow::<crate::workflows::coordinator::Workflow>()
			.tag("replica", current_replica_id)
			.send()
			.await?;

			protocol::ResponseKind::CoordinatorUpdateReplicaStatusResponse
		}
		protocol::RequestKind::BeginLearningRequest(req) => {
			tracing::debug!(?current_replica_id, "received begin learning request");

			ctx.signal(crate::workflows::replica::BeginLearning {
				config: req.config.into(),
			})
			.bypass_signal_from_workflow_I_KNOW_WHAT_IM_DOING()
			.to_workflow::<crate::workflows::replica::Workflow>()
			.tag("replica", current_replica_id)
			.send()
			.await?;

			protocol::ResponseKind::BeginLearningResponse
		}
		protocol::RequestKind::KvReadStateRequest(req) => {
			// This also gates direct/local dispatch. An old writer in this datacenter may
			// still omit coordinator acceptance, so its local state cannot certify freshness.
			ensure!(
				ctx.config().protocols().epoxy.version() >= 4,
				"Epoxy read-state requires a completed v4 rollout in this datacenter"
			);
			let response = ctx
				.udb()?
				.txn("epoxy_read_state", |tx| {
					let req = req.clone();
					async move {
						replica::messages::read_state::read_state(&tx, current_replica_id, req)
							.await
					}
				})
				.await?;
			protocol::ResponseKind::KvReadStateResponse(response)
		}
		protocol::RequestKind::KvGetRequest(req) => {
			let result = ctx
				.op(ops::kv::get_local::Input {
					replica_id: current_replica_id,
					key: req.key,
				})
				.await?;

			protocol::ResponseKind::KvGetResponse(protocol::KvGetResponse {
				value: result.map(|value| protocol::CommittedValue {
					value: value.value,
					version: value.version,
					mutable: value.mutable,
				}),
			})
		}
		protocol::RequestKind::KvPurgeCacheRequest(req) => {
			ctx.op(ops::kv::purge_local::Input {
				replica_id: current_replica_id,
				entries: req.entries,
			})
			.await?;

			protocol::ResponseKind::KvPurgeCacheResponse
		}
	};

	Ok(protocol::Response { kind })
}
