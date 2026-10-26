//! Lifecycle and cold-read UPS service owned together with the worker's tablet supervisor.

use crate::tablet_supervisor::{OperatorRequest, TabletSupervisor, VolumeReplyAuthority};
use anyhow::{Result, ensure};
use depot_types::keys::pubsub::VolumeCommandSubject;
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use rivet_data::{
	DEPOT_CLIENT_PROTOCOL_VERSION,
	generated::{
		depot_client_protocol_v1::{ClientRequest, ClientResponse},
		depot_kv_command_v1::UnavailableReason,
		depot_tablet_manifest_v1::TabletManifest,
	},
	versioned::{DepotClientRequestData, DepotClientResponseData},
};
use rivet_topology::Topology;
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
use universalpubsub::{PubSub, RequestMessage, SubjectCapacities};
use vbare::Versioned;

#[derive(Clone, Copy)]
enum RequestKind {
	Volume,
	Kv,
	Stop,
}

/// A volume request's message, the authority that admitted it, its response if the maintainer
/// answered, and whether it was a filesystem read.
type Reply = (
	RequestMessage,
	Option<VolumeReplyAuthority>,
	Option<ClientResponse>,
	bool,
);

/// Owns grant reconciliation, admission and reply delivery. Drain UPS before stopping this loop;
/// it joins all maintainers before returning, including on malformed manifest or transport failure.
#[tracing::instrument(skip_all)]
pub async fn start(
	pubsub: PubSub,
	topology: Arc<Topology>,
	supervisor: TabletSupervisor,
	manifests: watch::Receiver<Option<TabletManifest>>,
	capacities: SubjectCapacities,
	log_capacities: SubjectCapacities,
	stop: watch::Receiver<bool>,
) -> Result<()> {
	let registry = Arc::new(crate::command::CommandRegistry::with_core()?);
	start_with_commands(
		pubsub,
		topology,
		supervisor,
		manifests,
		capacities,
		log_capacities,
		stop,
		registry,
		crate::command::volume_host::Config::default(),
	)
	.await
}

/// Installs application handlers once, before any tablet grant starts serving.
#[allow(clippy::too_many_arguments)]
pub async fn start_with_commands(
	pubsub: PubSub,
	topology: Arc<Topology>,
	mut supervisor: TabletSupervisor,
	manifests: watch::Receiver<Option<TabletManifest>>,
	capacities: SubjectCapacities,
	log_capacities: SubjectCapacities,
	stop: watch::Receiver<bool>,
	registry: Arc<crate::command::CommandRegistry>,
	config: crate::command::volume_host::Config,
) -> Result<()> {
	let command_capacities = SubjectCapacities {
		incoming: config.request_capacity,
		outgoing: config.request_capacity,
	};
	let lease = topology.clone();
	// Every command volume on this worker keeps its alarms in one deadline queue. Its driver
	// runs below until the supervisor has closed every volume.
	let timers = rivet_state_machine_ephemeral::SharedTimers::new();
	let (timers_stop, timers_stopping) = watch::channel(false);
	let lifecycle = depot_client::volume_requests::VolumeRequests::new(
		pubsub.clone(),
		manifests.clone(),
		config
			.manifest_refresh
			.clone()
			.unwrap_or_else(|| watch::channel(()).0),
		capacities,
		Duration::from_secs(5),
		Arc::new(move || lease.may_admit().unwrap_or(false)),
	)?
	.with_command_capacity(config.request_capacity)?
	.with_shared_timers(timers.clone());
	let log_lanes = depot_volume::ups::UpsLogClient::lanes(
		&pubsub,
		log_capacities,
		config.log_inflight_per_worker,
	)?;
	if let Some(refresh) = &config.manifest_refresh {
		supervisor = supervisor.with_manifest_refresh(refresh.clone());
	}
	supervisor = supervisor.with_commands(crate::command::volume_host::Factory {
		pubsub: pubsub.clone(),
		lifecycle: lifecycle.clone(),
		log_capacities,
		log_lanes,
		registry,
		config,
		idle_changed: Default::default(),
	})?;
	let max_command_bytes = supervisor.command_limit_bytes();
	let work = async {
		let result = serve(
			&pubsub,
			&topology,
			&mut supervisor,
			manifests,
			capacities,
			command_capacities,
			stop,
			max_command_bytes,
		)
		.await;
		let closed = supervisor.close().await;
		// Every command volume has stopped, so no alarm is left to wake.
		timers_stop.send_replace(true);
		result.and(closed)
	};
	let (result, driven) =
		futures_util::join!(work, timers.run(pubsub.env().clone(), timers_stopping));
	result.and(driven.map_err(anyhow::Error::from))
}

/// Keeps the subscription inside the owner that joins maintainers on every exit path.
#[allow(clippy::too_many_arguments)]
async fn serve(
	pubsub: &PubSub,
	topology: &Topology,
	supervisor: &mut TabletSupervisor,
	mut manifests: watch::Receiver<Option<TabletManifest>>,
	capacities: SubjectCapacities,
	command_capacities: SubjectCapacities,
	mut stop: watch::Receiver<bool>,
	max_command_bytes: usize,
) -> Result<()> {
	ensure!(
		topology
			.snapshot()?
			.worker_registration
			.as_ref()
			.is_some_and(
				|registration| registration.identity.worker_id == *pubsub.worker_id()
					&& registration.identity.instance_id.as_bytes()
						== pubsub.instance_id().as_bytes()
			),
		"tablet service and UPS must share worker identity"
	);
	let subject = VolumeCommandSubject::new(pubsub.worker_id().clone());
	let _reservation = pubsub.reserve_request_subject(
		&subject,
		universalpubsub::RequestClass::DepotVolume,
		capacities,
	)?;
	let mut requests = pubsub.subscribe_request(&subject)?;
	let command_subject =
		depot_types::keys::pubsub::KvCommandSubject::new(pubsub.worker_id().clone());
	let _command_reservation = pubsub.reserve_request_subject(
		&command_subject,
		universalpubsub::RequestClass::DepotKvCommand,
		command_capacities,
	)?;
	let mut commands = pubsub.subscribe_request(&command_subject)?;
	let mut command_replies: FuturesUnordered<
		BoxFuture<
			'static,
			(
				RequestMessage,
				Option<VolumeReplyAuthority>,
				rivet_data::generated::depot_kv_command_v1::KvCommandResponse,
			),
		>,
	> = FuturesUnordered::new();
	let clock = pubsub.env().clock();
	let mut replies: FuturesUnordered<BoxFuture<'static, Reply>> = FuturesUnordered::new();
	let readiness_subject =
		depot_types::keys::pubsub::StopReadinessSubject::new(pubsub.worker_id().clone());
	let _readiness_reservation = pubsub.reserve_request_subject(
		&readiness_subject,
		universalpubsub::RequestClass::DepotStopReadiness,
		capacities,
	)?;
	let mut readiness_requests = pubsub.subscribe_request(&readiness_subject)?;
	let mut readiness_replies = FuturesUnordered::new();
	let mut accepting_readiness = true;
	let mut accepting = true;
	let mut accepting_commands = true;
	// Request and reply wakes cannot change what reconcile acts on: the manifest, leases,
	// closing maintainers and reserved inbox slots all wake the control arm, which sets this.
	let mut reconcile_due = true;

	loop {
		if *stop.borrow()
			|| (!accepting
				&& !accepting_commands
				&& !accepting_readiness
				&& readiness_replies.is_empty()
				&& replies.is_empty()
				&& command_replies.is_empty())
		{
			return Ok(());
		}

		// Command hosts whose idle queue head is due close or release their volumes now.
		// `wait_for_work` below wakes this loop at the next head.
		supervisor.sweep_due_commands(clock.observe()?.boot_ms);

		// Revocation precedes any reply after a manifest change. Reconcile hands revoked
		// maintainers to the supervisor's closing set instead of joining them here, so a
		// slow SlateDB close never stops this loop from delivering other tablets' replies.
		if reconcile_due || manifests.has_changed().unwrap_or(true) {
			reconcile_due = false;
			let manifest = manifests.borrow_and_update().clone();
			if let Some(manifest) = manifest {
				supervisor.reconcile(manifest).await?;
			} else {
				supervisor.close().await?;
			}
		}

		// Nothing here polls: the loop sleeps until a lease, recovery or command idle
		// deadline, and otherwise wakes only on the events listed below.
		let control = async {
			rivet_env::select_biased!("shutdown and manifest changes precede supervisor deadlines",
				changed = stop.changed() => Ok(if changed.is_err() || *stop.borrow() { None } else { Some("stop") }),
				changed = manifests.changed() => Ok(if changed.is_err() { None } else { Some("manifest") }),
				wake = supervisor.wait_for_work() => wake.map(|wake| Some(wake.as_str())),
			)
		};
		rivet_env::select_biased!("shutdown and grant changes precede replies and new admission",
			woken = control => {
				let Some(reason) = woken? else {
					return Ok(());
				};
				crate::metrics::record_tablet_loop_wake(reason);
				reconcile_due = true;
			},
			completion = async {
				rivet_env::select_biased!("completed KV and volume replies precede new admission",
					reply = rivet_env::tokio::when(!command_replies.is_empty(), command_replies.next()) => (reply, None, None),
					reply = rivet_env::tokio::when(!replies.is_empty(), replies.next()) => (None, reply, None),
					reply = rivet_env::tokio::when(!readiness_replies.is_empty(), readiness_replies.next()) => (None, None, reply),
				)
			} => {
				crate::metrics::record_tablet_loop_wake("reply");
				match completion {
					(Some((message, authority, response)), _, _) => {
						let response = if authority.as_ref().is_none_or(|guard| guard.may_reply()) { response } else { crate::kv_refusals::authority_moved() };
						if topology.may_admit()? && !message.is_request_expired()? {
							let bytes = rivet_data::versioned::DepotKvCommandResponseData::serialize_with_embedded_version(&response, rivet_data::DEPOT_KV_COMMAND_VERSION)?;
							let _ = message.reply(&bytes);
						}
					}
					(_, Some(reply), _) => deliver(topology, reply)?,
					(_, _, Some(reply)) => super::stop_readiness::deliver(topology, reply)?,
					(None, None, None) => {},
				}
			},
			incoming = async {
				rivet_env::select_biased!("volume lifecycle requests precede KV command admission",
					message = rivet_env::tokio::when(accepting, requests.next()) => (RequestKind::Volume, message),
					message = rivet_env::tokio::when(accepting_commands, commands.next()) => (RequestKind::Kv, message),
					message = rivet_env::tokio::when(accepting_readiness, readiness_requests.next()) => (RequestKind::Stop, message),
				)
			} => {
				crate::metrics::record_tablet_loop_wake("request");
				let (kind, message) = incoming;
				let Some(message) = message? else {
					match kind { RequestKind::Kv => accepting_commands = false, RequestKind::Volume => accepting = false, RequestKind::Stop => accepting_readiness = false }
					continue;
				};
				if !topology.may_admit()? || message.is_request_expired()? {
					continue;
				}

				if matches!(kind, RequestKind::Stop) {
					match super::stop_readiness::submit(topology, supervisor, message) {
						Ok(reply) => readiness_replies.push(reply),
						Err(error) => tracing::warn!(%error, "invalid stop readiness request"),
					}
					continue;
				}
				if matches!(kind, RequestKind::Kv) {
					if message.payload.len() > max_command_bytes {
						let bytes = rivet_data::versioned::DepotKvCommandResponseData::serialize_with_embedded_version(&rivet_data::generated::depot_kv_command_v1::KvCommandResponse::CommandTooLarge, rivet_data::DEPOT_KV_COMMAND_VERSION)?;
						let _ = message.reply(&bytes);
						continue;
					}
					let request = rivet_data::versioned::DepotKvCommandRequestData::deserialize_with_embedded_version(&message.payload);
					let submitted = match (request, message.deadline_boot_ms()) {
						(Ok(request), Some(deadline)) => supervisor.command(request, message.payload.len(), deadline),
						(Ok(_), None) => Err(crate::kv_refusals::unavailable(UnavailableReason::Internal)),
						(Err(_), _) => { let _ = message.fail(&universalpubsub::errors::InvalidMessage.build()).await; continue; }
					};
					command_replies.push(async move {
						let (authority, response) = match submitted { Ok((authority, response)) => (Some(authority), response.await.unwrap_or_else(|_| crate::kv_refusals::authority_moved())), Err(response) => (None, response) };
						(message, authority, response)
					}.boxed());
					continue;
				}

				// Malformed requests never enter a maintainer or reserve lifecycle state.
				let request = match DepotClientRequestData::deserialize_with_embedded_version(&message.payload) {
					Ok(request) => request,
					Err(_) => {
						if topology.may_admit()? {
							let _ = message.fail(&universalpubsub::errors::InvalidMessage.build()).await;
						}
						continue;
					}
				};
				// Operator requests span the worker's tablets and report what they started, so
				// their reply carries no single tablet's authority.
				let operator = match &request {
					ClientRequest::OperatorTierRequest(request) => Some(OperatorRequest::Tier { tablet: request.tablet }),
					ClientRequest::OperatorDropCachesRequest(request) => Some(OperatorRequest::DropCaches { tablet: request.tablet, quiet_ms: request.quiet_ms, limit: request.limit }),
					ClientRequest::FsReadRequest(_)
					| ClientRequest::VolumeCheckpointRequest(_)
					| ClientRequest::FsForkReferenceRequest(_)
					| ClientRequest::FsForkRequest(_)
					| ClientRequest::VolumeCreateRequest(_)
					| ClientRequest::VolumeDeleteRequest(_)
					| ClientRequest::VolumeOpenRequest(_)
					| ClientRequest::VolumeCloseRequest(_)
					| ClientRequest::ReplicaStatusNoticeRequest(_)
					| ClientRequest::RepairInspectRequest(_)
					| ClientRequest::RepairDiagnosticsRequest(_)
					| ClientRequest::RepairRequest(_)
					| ClientRequest::RepairAuditListRequest(_) => None,
				};
				if let Some(operator) = operator {
					let reply = supervisor.operator(operator);
					replies.push(async move { (message, None, Some(reply.await), false) }.boxed());
					continue;
				}
				// Each receipt retains the original authority until its network reply.
				let filesystem_read = matches!(request, ClientRequest::FsReadRequest(_));
				let submitted = supervisor.volume_request(message.source_worker_id.clone(), request, message.deadline_boot_ms().unwrap_or(0));
				replies.push(async move {
					match submitted {
						Ok((guard, response)) => (message, Some(guard), response.await.ok(), filesystem_read),
						Err(reason) => (message, None, Some(ClientResponse::VolumeRejectResponse(reason)), filesystem_read),
					}
				}.boxed());
			},
		);
	}
}

/// Checks the original grant both before encoding and immediately before handing bytes to UPS.
fn deliver(topology: &Topology, reply: Reply) -> Result<()> {
	let (message, authority, response, filesystem_read) = reply;
	let may_send =
		|| -> Result<bool> { Ok(topology.may_admit()? && !message.is_request_expired()?) };
	let authorized = || authority.as_ref().is_none_or(|guard| guard.may_reply());
	let encode = |response: &ClientResponse| {
		DepotClientResponseData::serialize_with_embedded_version(
			response,
			DEPOT_CLIENT_PROTOCOL_VERSION,
		)
	};
	if !may_send()? {
		return Ok(());
	}

	let Some(response) = deliverable(response, authorized(), filesystem_read) else {
		return Ok(());
	};
	if let ClientResponse::VolumeRejectResponse(reason) = &response {
		crate::signals::volume_request_refused(reason);
	}
	let bytes = encode(&response)?;
	if !may_send()? {
		return Ok(());
	}

	// The grant can be lost while the reply was encoded.
	if authorized() {
		let _ = message.reply(&bytes);
	} else if let Some(refusal) = deliverable(None, false, filesystem_read) {
		let _ = message.reply(&encode(&refusal)?);
	}
	Ok(())
}

/// What a volume request's caller receives. A response leaves only under the grant that
/// admitted its request. A filesystem read this maintainer instance can no longer answer,
/// because it lost the grant or closed, fenced or restarted with the read in flight, is
/// refused as `NotMaintainer`, so its
/// caller refreshes its route and retries on the tablet's next maintainer now instead of
/// waiting for its deadline. A read has no effects beyond the old maintainer's in-memory
/// session, so the refusal is true whatever the maintainer did. Any other request may have
/// executed, so it gets no reply and its caller treats the outcome as unknown.
fn deliverable(
	response: Option<ClientResponse>,
	authorized: bool,
	filesystem_read: bool,
) -> Option<ClientResponse> {
	match response {
		Some(response) if authorized => Some(response),
		Some(_) | None if filesystem_read => Some(ClientResponse::VolumeRejectResponse(
			depot_types::client::not_maintainer(),
		)),
		Some(_) | None => None,
	}
}


