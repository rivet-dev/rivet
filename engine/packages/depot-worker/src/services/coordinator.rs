//! UPS command delivery to the tablet coordinator held by the current singleton tenure.

use crate::tablet_coordinator_machine::runner::Client;
use anyhow::{Result, ensure};
use depot_types::keys::pubsub::TabletCoordinatorSubject;
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use rivet_data::{
	generated::depot_tablet_coordinator_protocol_v1::{
		TabletCoordinatorRejectResponse, TabletCoordinatorRequest, TabletCoordinatorResponse,
	},
	versioned::DepotTabletCoordinatorResponseData,
};
use rivet_topology::Topology;
use std::sync::Arc;
use tokio::sync::watch;
use universalpubsub::{PubSub, RequestMessage, SubjectCapacities};
use vbare::Versioned;

type Reply = (RequestMessage, Option<Client>, TabletCoordinatorResponse);

/// A nonleader returns NotCoordinator. An admitted command retains its original client authority,
/// so a later tenure cannot authorize delivery of an old command's result. UPS drain closes
/// admission first; accepted commands finish before this service returns.
#[tracing::instrument(skip_all)]
pub async fn start(
	pubsub: PubSub,
	topology: Arc<Topology>,
	active: watch::Receiver<Option<Client>>,
	capacities: SubjectCapacities,
	mut stop: watch::Receiver<bool>,
) -> Result<()> {
	ensure!(
		topology
			.snapshot()?
			.worker_registration
			.as_ref()
			.is_some_and(|registration| {
				registration.identity.worker_id == *pubsub.worker_id()
					&& registration.identity.instance_id.as_bytes()
						== pubsub.instance_id().as_bytes()
			}),
		"coordinator service and UPS must share worker identity"
	);
	let subject = TabletCoordinatorSubject::new(pubsub.worker_id().clone());
	let _reservation = pubsub.reserve_request_subject(
		&subject,
		universalpubsub::RequestClass::DepotCoordinator,
		capacities,
	)?;
	let mut requests = pubsub.subscribe_request(&subject)?;
	let mut replies: FuturesUnordered<BoxFuture<'static, Reply>> = FuturesUnordered::new();
	let mut accepting = true;
	tracing::info!("tablet coordinator request service started");

	loop {
		if *stop.borrow() || (!accepting && replies.is_empty()) {
			break;
		}
		rivet_env::select_biased!("shutdown and completed coordinator commands precede admission",
			changed = stop.changed() => { if changed.is_err() { break; } },
			reply = rivet_env::tokio::when(!replies.is_empty(), replies.next()) => {
				if let Some((message, client, response)) = reply {
					let may_reply = || topology.may_admit().unwrap_or(false)
						&& client.as_ref().is_none_or(Client::may_serve);
					if may_reply() && !message.is_request_expired()? {
						let bytes = DepotTabletCoordinatorResponseData::serialize_with_embedded_version(&response, DepotTabletCoordinatorResponseData::LATEST)?;
						if may_reply() && !message.is_request_expired()? {
							let _ = message.reply(&bytes);
						}
					}
				}
			},
			message = rivet_env::tokio::when(accepting, requests.next()) => {
				let Some(message) = message? else { accepting = false; continue; };
				if !topology.may_admit()? || message.is_request_expired()? { continue; }
				let request = match rivet_data::versioned::DepotAlarmCoordinatorRequestData::deserialize_with_embedded_version(&message.payload) {
					Ok(request) => request,
					Err(_) => {
						if topology.may_admit()? {
							let _ = message.fail(&universalpubsub::errors::InvalidMessage.build()).await;
						}
						continue;
					}
				};
				// Bind cleanup and recovery reports to the authenticated UPS caller before
				// admission.
				use rivet_data::generated::depot_tablet_coordinator_protocol_v1::AlarmCoordinatorRequest;
				let reporter_mismatch = match &request {
					AlarmCoordinatorRequest::AlarmSplitPrepared(report) => report.maintainer != *message.source_worker_id.as_bytes(),
					AlarmCoordinatorRequest::AlarmIndexReady(report) => report.maintainer != *message.source_worker_id.as_bytes(),
					AlarmCoordinatorRequest::TabletCoordinatorRequest(request) => match request {
						TabletCoordinatorRequest::RetiredTabletDeleted(report) => {
							report.maintainer != *message.source_worker_id.as_bytes()
						}
						TabletCoordinatorRequest::RecoverTablets(report) => {
							report.maintainer != *message.source_worker_id.as_bytes()
						}
						TabletCoordinatorRequest::VersionMarksReport(report) => {
							report.maintainer != *message.source_worker_id.as_bytes()
						}
						TabletCoordinatorRequest::ParentLogImported(report) => {
							report.maintainer != *message.source_worker_id.as_bytes()
						}
						TabletCoordinatorRequest::RequestStop(_)
						| TabletCoordinatorRequest::CatchUpReport(_)
						| TabletCoordinatorRequest::CreateTablet(_)
						| TabletCoordinatorRequest::GrantTablet(_)
						| TabletCoordinatorRequest::MoveTablet(_)
						| TabletCoordinatorRequest::ReplaceReplica(_)
						| TabletCoordinatorRequest::AbortReconfiguration(_)
						| TabletCoordinatorRequest::Repair(_)
						| TabletCoordinatorRequest::SplitTablet(_)
						| TabletCoordinatorRequest::SplitPrepared(_)
						| TabletCoordinatorRequest::PauseTablet(_)
						| TabletCoordinatorRequest::ResumeTablet(_) => false,
					}
				};
				if reporter_mismatch {
					let response = TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
						TabletCoordinatorRejectResponse::InvalidCommand);
					replies.push(async move { (message, None, response) }.boxed());
					continue;
				}
				let client = active.borrow().clone();
				replies.push(async move {
					let response = match &client {
						Some(client) => client.request_envelope(request).await,
						None => TabletCoordinatorResponse::TabletCoordinatorRejectResponse(TabletCoordinatorRejectResponse::NotCoordinator),
					};
					(message, client, response)
				}.boxed());
			},
		);
	}
	tracing::info!("tablet coordinator request service stopped");
	Ok(())
}
