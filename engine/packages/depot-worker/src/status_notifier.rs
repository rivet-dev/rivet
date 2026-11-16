//! Sends a log shard's replica status notices to tablet maintainer workers over UPS.

use depot_log::log_shard_machine::{runner::StatusNotifier, types::NoticeOutcome};
use rivet_data::{
	generated::depot_client_protocol_v2::{
		ClientRequest, ClientResponse, ReplicaStatusNoticeRequest, VolumeRejectResponse,
	},
	versioned::DepotClientResponseData,
};
use rivet_topology::{versions::PeerVersions, worker::WorkerId};
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use universalpubsub::{PubSub, SubjectCapacities};
use vbare::Versioned;

/// Sends each notice as one `DepotVolume` request on the maintainer's volume subject.
/// It never retries; the log shard resends the replicas of a notice that did not end
/// `Sent` after a backoff.
pub struct UpsStatusNotifier {
	pub pubsub: PubSub,
	pub capacities: SubjectCapacities,
	pub timeout: Duration,
	/// Topology's view of the version each maintainer reads, so a notice to a worker of an
	/// older release is sealed at a version it decodes.
	pub versions: Arc<dyn PeerVersions>,
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
		let version =
			depot_client::client_protocol::request_version(self.versions.as_ref(), &worker);
		let retry_after_ms = self.versions.refresh_hint_ms();
		Box::pin(async move {
			let subject = depot_types::keys::pubsub::VolumeCommandSubject::new(worker.clone());
			let Ok(_reservation) = pubsub.reserve_request_subject(
				&subject,
				universalpubsub::RequestClass::DepotVolume,
				capacities,
			) else {
				return NoticeOutcome::Failed;
			};
			let Ok(payload) = depot_client::client_protocol::seal_request(
				&ClientRequest::ReplicaStatusNoticeRequest(request),
				version,
				retry_after_ms,
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
