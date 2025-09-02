use anyhow::*;
use epoxy_protocol::protocol::{self, ReplicaId};
use gas::prelude::*;
use rivet_api_builder::prelude::*;

use crate::{ops, replica};

pub async fn message_request(
	ctx: &ApiCtx,
	replica_id: ReplicaId,
	request: protocol::Request,
) -> Result<protocol::Response> {
	let kind = match request.kind {
		protocol::RequestKind::UpdateConfigRequest(req) => {
			tracing::info!(
				epoch = ?req.config.epoch,
				replica_count = req.config.replicas.len(),
				"received configuration update request"
			);

			// Store the configuration
			ctx.udb()?
				.run(move |tx, _| {
					let req = req.clone();
					async move {
						replica::update_config::update_config(&*tx, replica_id, req)
							.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))
					}
				})
				.await?;

			protocol::ResponseKind::UpdateConfigResponse
		}
		protocol::RequestKind::PreAcceptRequest(req) => {
			let response = ctx
				.udb()?
				.run(move |tx, _| {
					let req = req.clone();
					async move {
						replica::messages::pre_accept(&*tx, replica_id, req)
							.await
							.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))
					}
				})
				.await?;
			protocol::ResponseKind::PreAcceptResponse(response)
		}
		protocol::RequestKind::AcceptRequest(req) => {
			let response = ctx
				.udb()?
				.run(move |tx, _| {
					let req = req.clone();
					async move {
						replica::messages::accept(&*tx, replica_id, req)
							.await
							.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))
					}
				})
				.await?;
			protocol::ResponseKind::AcceptResponse(response)
		}
		protocol::RequestKind::CommitRequest(req) => {
			// Commit and update KV store
			ctx.udb()?
				.run(move |tx, _| {
					let req = req.clone();
					async move {
						replica::messages::commit(&*tx, replica_id, req, true)
							.await
							.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))?;

						Result::Ok(())
					}
				})
				.await?;

			protocol::ResponseKind::CommitResponse
		}
		protocol::RequestKind::PrepareRequest(req) => {
			let response = ctx
				.udb()?
				.run(move |tx, _| {
					let req = req.clone();
					async move {
						replica::messages::prepare(&*tx, replica_id, req)
							.await
							.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))
					}
				})
				.await?;
			protocol::ResponseKind::PrepareResponse(response)
		}
		protocol::RequestKind::DownloadInstancesRequest(req) => {
			// Handle download instances request - read from FDB and return instances
			let instances = ctx
				.udb()?
				.run(move |tx, _| {
					let req = req.clone();
					async move {
						replica::messages::download_instances(&*tx, replica_id, req)
							.await
							.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))
					}
				})
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
			tracing::info!(
				?replica_id,
				update_replica_id = ?req.replica_id,
				update_status = ?req.status,
				"received coordinator update replica status request"
			);

			ctx.signal(crate::workflows::coordinator::ReplicaStatusChangeSignal {
				replica_id: req.replica_id,
				status: req.status.into(),
			})
			.to_workflow::<crate::workflows::coordinator::Workflow>()
			.tag("replica", replica_id)
			.send()
			.await?;

			protocol::ResponseKind::CoordinatorUpdateReplicaStatusResponse
		}
		protocol::RequestKind::BeginLearningRequest(req) => {
			// Send signal to replica workflow
			tracing::info!(
				replica_id = ?replica_id,
				"received begin learning request"
			);

			ctx.signal(crate::workflows::replica::BeginLearningSignal {
				config: req.config.clone().into(),
			})
			.to_workflow::<crate::workflows::replica::Workflow>()
			.tag("replica", replica_id)
			.to_workflow::<crate::workflows::replica::Workflow>()
			.send()
			.await?;

			protocol::ResponseKind::BeginLearningResponse
		}
		protocol::RequestKind::KvGetRequest(req) => {
			// Handle KV get request
			let result = ctx
				.op(ops::kv::get_local::Input {
					replica_id,
					key: req.key.clone(),
				})
				.await?;

			protocol::ResponseKind::KvGetResponse(protocol::KvGetResponse {
				value: result.value,
			})
		}
	};

	Ok(protocol::Response { kind })
}
