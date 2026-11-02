//! Sends a log shard's replica status notices to tablet maintainer workers over UPS.

use depot_log::log_shard_machine::{runner::StatusNotifier, types::NoticeOutcome};
use rivet_data::{
	DEPOT_CLIENT_PROTOCOL_VERSION,
	generated::depot_client_protocol_v1::{
		ClientRequest, ClientResponse, ReplicaStatusNoticeRequest, VolumeRejectResponse,
	},
	versioned::{DepotClientRequestData, DepotClientResponseData},
};
use rivet_topology::worker::WorkerId;
use std::{future::Future, pin::Pin, time::Duration};
use universalpubsub::{PubSub, SubjectCapacities};
use vbare::Versioned;

/// Sends each notice as one `DepotVolume` request on the maintainer's volume subject.
/// It never retries; the log shard resends the replicas of a notice that did not end
/// `Sent` after a backoff.
pub struct UpsStatusNotifier {
	pub pubsub: PubSub,
	pub capacities: SubjectCapacities,
	pub timeout: Duration,
}

impl StatusNotifier for UpsStatusNotifier {
	fn notify(
		&self,
		worker: WorkerId,
		request: ReplicaStatusNoticeRequest,
	) -> Pin<Box<dyn Future<Output = NoticeOutcome> + Send>> {
		let pubsub = self.pubsub.clone();
		let capacities = self.capacities;
		let timeout = self.timeout;
		Box::pin(async move {
			let subject = depot_types::keys::pubsub::VolumeCommandSubject::new(worker.clone());
			let Ok(_reservation) = pubsub.reserve_request_subject(
				&subject,
				universalpubsub::RequestClass::DepotVolume,
				capacities,
			) else {
				return NoticeOutcome::Failed;
			};
			let Ok(payload) = DepotClientRequestData::serialize_with_embedded_version(
				&ClientRequest::ReplicaStatusNoticeRequest(request),
				DEPOT_CLIENT_PROTOCOL_VERSION,
			) else {
				return NoticeOutcome::Failed;
			};
			match pubsub
				.request_with_timeout(
					universalpubsub::RequestClass::DepotVolume,
					&worker,
					subject,
					&payload,
					timeout,
				)
				.await
			{
				Ok(reply) => {
					match DepotClientResponseData::deserialize_with_embedded_version(&reply) {
						Ok(ClientResponse::ReplicaStatusNoticeResponse(response)) => {
							NoticeOutcome::Sent {
								refused_tablets: response.refused_tablets,
								unknown_tablets: response.unknown_tablets,
							}
						}
						Ok(ClientResponse::VolumeRejectResponse(
							VolumeRejectResponse::NotMaintainer(_),
						)) => NoticeOutcome::NotMaintainer,
						Ok(_) | Err(_) => NoticeOutcome::Refused,
					}
				}
				Err(error)
					if error
						.downcast_ref::<rivet_error::RivetError>()
						.is_some_and(|error| {
							error.group() == "ups" && error.code() == "timeout"
						}) =>
				{
					NoticeOutcome::Timeout
				}
				Err(_) => NoticeOutcome::Failed,
			}
		})
	}
}
