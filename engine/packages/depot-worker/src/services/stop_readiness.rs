//! Maintainer readiness replies retain their original grant through network delivery.

use crate::tablet_supervisor::{HandoffReplyAuthority, TabletSupervisor, VolumeReplyAuthority};
use anyhow::Result;
use futures_util::{FutureExt, future::BoxFuture};
use rivet_data::{
	DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
	generated::depot_storage_worker_protocol_v1::{
		self as wire, ReadinessPurpose, StopReadiness, StopReadinessResponse,
	},
	versioned::{
		DepotHandoffReadinessResponseData, DepotStopReadinessData, DepotStopReadinessResponseData,
	},
};
use rivet_topology::Topology;
use universalpubsub::RequestMessage;
use vbare::Versioned;

pub(super) enum ReplyAuthority {
	Stop(VolumeReplyAuthority),
	Handoff(HandoffReplyAuthority),
}
impl ReplyAuthority {
	fn may_reply(&self) -> bool {
		match self {
			Self::Stop(guard) => guard.may_reply(),
			Self::Handoff(guard) => guard.may_reply(),
		}
	}
}
pub(super) enum Response {
	Stop(StopReadinessResponse),
	Handoff {
		request: wire::StopReadiness,
		response: StopReadinessResponse,
	},
}
impl Response {
	fn refuse(&mut self) {
		match self {
			Self::Stop(response) | Self::Handoff { response, .. } => {
				*response = StopReadinessResponse::NotServing
			}
		}
	}
	fn encode(&self) -> Result<Vec<u8>> {
		match self {
			Self::Stop(response) => Ok(
				DepotStopReadinessResponseData::serialize_with_embedded_version(
					response,
					DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
				)?,
			),
			Self::Handoff { request, response } => {
				let wire::ReadinessPurpose::Handoff(purpose) = &request.purpose else {
					anyhow::bail!("not an enhanced handoff");
				};
				let result = match response {
					StopReadinessResponse::Ready => wire::HandoffReadinessResult::HandoffReady,
					StopReadinessResponse::NotReady(_) | StopReadinessResponse::Overloaded(_) => {
						wire::HandoffReadinessResult::HandoffNotReady
					}
					StopReadinessResponse::StaleTabletGrantEpoch(_)
					| StopReadinessResponse::StaleTabletReplicaSet(_)
					| StopReadinessResponse::NotServing => wire::HandoffReadinessResult::HandoffNotServing,
				};
				Ok(
					DepotHandoffReadinessResponseData::serialize_with_embedded_version(
						&wire::HandoffReadinessResponse {
							tablet_id: request.tablet_id,
							tablet_grant_epoch: request.tablet_grant_epoch,
							tablet_replica_set_version: request.tablet_replica_set_version,
							worker_id: request.worker_id,
							worker_incarnation: request.worker_incarnation,
							tablet_manifest_version: purpose.tablet_manifest_version,
							result,
						},
						DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
					)?,
				)
			}
		}
	}
}
pub(super) type Reply = (RequestMessage, Option<ReplyAuthority>, Response);

pub(super) fn submit(
	topology: &Topology,
	supervisor: &TabletSupervisor,
	message: RequestMessage,
) -> Result<BoxFuture<'static, Reply>> {
	let request = DepotStopReadinessData::deserialize_with_embedded_version(&message.payload)?;
	if let wire::ReadinessPurpose::GrantWritten(written) = &request.purpose {
		supervisor.grant_written(written.tablet_manifest_version);
		let response = Response::Stop(StopReadinessResponse::Ready);
		return Ok(async move { (message, None, response) }.boxed());
	}
	let registration = topology.snapshot()?.worker_registration.clone();
	if matches!(request.purpose, wire::ReadinessPurpose::Handoff(_)) {
		let submitted = if registration.as_ref().is_some_and(|registration| {
			request.worker_id == *registration.identity.worker_id.as_bytes()
				&& request.worker_incarnation == *registration.identity.instance_id.as_bytes()
		}) {
			supervisor.handoff_readiness(request.clone())
		} else {
			Err(StopReadinessResponse::NotServing)
		};
		return Ok(async move {
			let (guard, response) = match submitted {
				Ok((guard, response)) => (
					Some(ReplyAuthority::Handoff(guard)),
					response.await.unwrap_or(StopReadinessResponse::NotServing),
				),
				Err(response) => (None, response),
			};
			(message, guard, Response::Handoff { request, response })
		}
		.boxed());
	}
	anyhow::ensure!(
		matches!(request.purpose, ReadinessPurpose::Stop),
		"readiness purpose is neither a stop nor a handoff"
	);
	let submitted = if registration
		.as_ref()
		.is_some_and(|registration| matches_receiver_instance(&request, &registration.identity))
	{
		supervisor.stop_readiness(request)
	} else {
		Err(StopReadinessResponse::NotServing)
	};
	Ok(async move {
		match submitted {
			Ok((guard, response)) => (
				message,
				Some(ReplyAuthority::Stop(guard)),
				Response::Stop(response.await.unwrap_or(StopReadinessResponse::NotServing)),
			),
			Err(response) => (message, None, Response::Stop(response)),
		}
	}
	.boxed())
}

/// A prior process's stop cannot quiesce the replacement that received its request.
fn matches_receiver_instance(
	request: &StopReadiness,
	receiver: &rivet_topology::worker::WorkerIdentity,
) -> bool {
	request.worker_id != *receiver.worker_id.as_bytes()
		|| request.worker_incarnation == *receiver.instance_id.as_bytes()
}



pub(super) fn deliver(topology: &Topology, reply: Reply) -> Result<()> {
	let (message, authority, mut response) = reply;
	if authority.as_ref().is_some_and(|guard| !guard.may_reply()) {
		response.refuse();
	}
	if topology.may_admit()? && !message.is_request_expired()? {
		let bytes = response.encode()?;
		if topology.may_admit()?
			&& authority.as_ref().is_none_or(|guard| guard.may_reply())
			&& !message.is_request_expired()?
		{
			let _ = message.reply(&bytes);
		}
	}
	Ok(())
}
