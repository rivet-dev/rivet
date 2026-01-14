use anyhow::*;
use epoxy_protocol::protocol::{self};
use gas::prelude::*;
use rivet_api_builder::prelude::*;
use std::time::Instant;

use crate::{metrics, ops, replica};

#[tracing::instrument(skip_all)]
pub async fn message_request(
	ctx: &ApiCtx,
	request: protocol::Request,
) -> Result<protocol::Response> {
	let start = Instant::now();
	let request_type = match &request.kind {
		protocol::RequestKind::UpdateConfigRequest(_) => "update_config",
		protocol::RequestKind::PrepareRequest(_) => "prepare",
		protocol::RequestKind::PreAcceptRequest(_) => "pre_accept",
		protocol::RequestKind::AcceptRequest(_) => "accept",
		protocol::RequestKind::CommitRequest(_) => "commit",
		protocol::RequestKind::DownloadInstancesRequest(_) => "download_instances",
		protocol::RequestKind::HealthCheckRequest => "health_check",
		protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(_) => {
			"coordinator_update_replica_status"
		}
		protocol::RequestKind::BeginLearningRequest(_) => "begin_learning",
		protocol::RequestKind::KvGetRequest(_) => "kv_get",
		protocol::RequestKind::KvPurgeRequest(_) => "kv_purge",
	};
	let res = message_request_inner(ctx, request).await;

	metrics::REQUEST_DURATION
		.with_label_values(&[request_type])
		.observe(start.elapsed().as_secs_f64());
	metrics::REQUESTS_TOTAL
		.with_label_values(&[request_type, if res.is_ok() { "ok" } else { "err" }])
		.inc();

	res
}

#[tracing::instrument(skip_all)]
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

			// Store the configuration
			ctx.udb()?
				.run(|tx| {
					let req = req.clone();
					async move { replica::update_config::update_config(&*tx, current_replica_id, req) }
				})
				.custom_instrument(tracing::info_span!("update_config_tx"))
				.await?;

			protocol::ResponseKind::UpdateConfigResponse
		}
		protocol::RequestKind::PreAcceptRequest(req) => {
			let response = ctx
				.udb()?
				.run(|tx| {
					let req = req.clone();
					async move { replica::messages::pre_accept(&*tx, current_replica_id, req).await }
				})
				.custom_instrument(tracing::info_span!("pre_accept_tx"))
				.await?;
			protocol::ResponseKind::PreAcceptResponse(response)
		}
		protocol::RequestKind::AcceptRequest(req) => {
			let response = ctx
				.udb()?
				.run(|tx| {
					let req = req.clone();
					async move { replica::messages::accept(&*tx, current_replica_id, req).await }
				})
				.custom_instrument(tracing::info_span!("accept_tx"))
				.await?;
			protocol::ResponseKind::AcceptResponse(response)
		}
		protocol::RequestKind::CommitRequest(req) => {
			// Commit and update KV store
			ctx.udb()?
				.run(|tx| {
					let req = req.clone();
					async move {
						replica::messages::commit(&*tx, current_replica_id, req, true).await?;
						Result::Ok(())
					}
				})
				.custom_instrument(tracing::info_span!("commit_tx"))
				.await?;

			protocol::ResponseKind::CommitResponse
		}
		protocol::RequestKind::PrepareRequest(req) => {
			let response = ctx
				.udb()?
				.run(|tx| {
					let req = req.clone();
					async move { replica::messages::prepare(&*tx, current_replica_id, req).await }
				})
				.custom_instrument(tracing::info_span!("prepare_tx"))
				.await?;
			protocol::ResponseKind::PrepareResponse(response)
		}
		protocol::RequestKind::DownloadInstancesRequest(req) => {
			// Handle download instances request - read from UDB and return instances
			let instances = ctx
				.udb()?
				.run(|tx| {
					let req = req.clone();
					async move {
						replica::messages::download_instances(&*tx, current_replica_id, req).await
					}
				})
				.custom_instrument(tracing::info_span!("download_instances_tx"))
				.await?;

			protocol::ResponseKind::DownloadInstancesResponse(protocol::DownloadInstancesResponse {
				instances,
			})
		}
		protocol::RequestKind::HealthCheckRequest => {
			// Simple health check - just return success
			tracing::debug!("received health check request");
			protocol::ResponseKind::HealthCheckResponse
		}
		protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(req) => {
			// Send signal to coordinator workflow
			tracing::debug!(
				?current_replica_id,
				update_replica_id=?req.replica_id,
				update_status=?req.status,
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
			// Send signal to replica workflow
			tracing::debug!(?current_replica_id, "received begin learning request");

			ctx.signal(crate::workflows::replica::BeginLearning {
				config: req.config.clone().into(),
			})
			.bypass_signal_from_workflow_I_KNOW_WHAT_IM_DOING()
			.to_workflow::<crate::workflows::replica::Workflow>()
			.tag("replica", current_replica_id)
			.send()
			.await?;

			protocol::ResponseKind::BeginLearningResponse
		}
		protocol::RequestKind::KvGetRequest(req) => {
			// Handle KV get request
			let result = ctx
				.op(ops::kv::get_local::Input {
					replica_id: current_replica_id,
					key: req.key.clone(),
				})
				.await?;

			protocol::ResponseKind::KvGetResponse(protocol::KvGetResponse {
				value: result.value,
			})
		}
		protocol::RequestKind::KvPurgeRequest(req) => {
			// Handle KV purge request
			ctx.op(ops::kv::purge_local::Input {
				replica_id: current_replica_id,
				keys: req.keys.clone(),
			})
			.await?;

			protocol::ResponseKind::KvPurgeResponse
		}
	};

	Ok(protocol::Response { kind })
}
