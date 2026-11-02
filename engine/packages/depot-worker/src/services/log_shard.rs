//! UPS admission and reply delivery for the worker's single disk-backed log shard.

use anyhow::{Result, ensure};
use depot_log::log_shard_machine::{runner::Shard, snapshot::Resource, types::LogShardCommand};
use depot_types::keys::pubsub::VolumeLogSubject;
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use rivet_data::{
	DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
	generated::depot_storage_worker_protocol_v1::{
		RejectResponse, StorageWorkerBatchItemResponse, StorageWorkerBatchRequest,
		StorageWorkerBatchResponse, StorageWorkerResponse,
	},
	versioned::{DepotStorageWorkerRequestData, DepotStorageWorkerResponseData},
};
use rivet_topology::Topology;
use std::sync::Arc;
use tokio::sync::watch;
use universalpubsub::{PubSub, RequestMessage, SubjectCapacities};
use vbare::Versioned;

/// Serves until shutdown or UPS drain closes its subscription. The caller drains UPS before
/// stopping this loop, then joins the shard and maintainers. Outstanding disk work belongs to
/// the shard; cancelling a reply never cancels or undoes an admitted mutation.
#[tracing::instrument(skip_all)]
pub async fn start(
	pubsub: PubSub,
	topology: Arc<Topology>,
	shard: Arc<Shard>,
	capacities: SubjectCapacities,
	mut stop: watch::Receiver<bool>,
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
		"log service and UPS must share worker identity"
	);
	// Leave room for the core to return its typed overload instead of exhausting UPS first.
	ensure!(
		shard
			.snapshot
			.borrow()
			.resources
			.iter()
			.any(|usage| usage.resource == Resource::PendingRequests
				&& usage.capacity < capacities.incoming as u64),
		"log subject incoming capacity must exceed shard request capacity"
	);
	let subject = VolumeLogSubject::new(pubsub.worker_id().clone());
	let _reservation = pubsub.reserve_request_subject(
		&subject,
		universalpubsub::RequestClass::DepotLog,
		capacities,
	)?;
	let mut requests = pubsub.subscribe_request(&subject)?;
	let mut replies: FuturesUnordered<
		BoxFuture<'static, (RequestMessage, Option<StorageWorkerResponse>)>,
	> = FuturesUnordered::new();
	let mut accepting = true;
	tracing::info!(worker_id = %pubsub.worker_id(), "log shard service started");

	loop {
		if *stop.borrow() || (!accepting && replies.is_empty()) {
			break;
		}
		rivet_env::select_biased!("shutdown and ready disk replies precede new requests",
			changed = stop.changed() => { if changed.is_err() { break; } },
			reply = rivet_env::tokio::when(!replies.is_empty(), replies.next()) => {
				if let Some((message, Some(response))) = reply {
					// Re-read authority after disk I/O. An old completion cannot restore a lease.
					if topology.may_admit()? && !message.is_request_expired()? {
						let bytes = DepotStorageWorkerResponseData::serialize_with_embedded_version(&response, DEPOT_STORAGE_WORKER_PROTOCOL_VERSION)?;
						if topology.may_admit()? && !message.is_request_expired()? {
							let _ = message.reply(&bytes);
						}
					}
				}
			},
			message = rivet_env::tokio::when(accepting, requests.next()) => {
				let Some(message) = message? else { accepting = false; continue; };
				if !topology.may_admit()? || message.is_request_expired()? { continue; }
				// Admission runs before decoding, so a refused request decodes nothing. An admitted
				// request shares the UPS frame: appended payloads enter the tail without a copy.
				let request = match DepotStorageWorkerRequestData::decode(message.payload.clone()) {
					Ok(request) => request,
					Err(_) => {
						if topology.may_admit()? {
							let _ = message.fail(&universalpubsub::errors::InvalidMessage.build()).await;
						}
						continue;
					}
				};
				let label = crate::tablet_maintainer_machine::runner::replica_request_label(&request);
				let deadline_boot_ms = message.deadline_boot_ms().unwrap_or(u64::MAX);
				// A disk probe admits this worker as a new tablet replica or destination, so a
				// worker whose lease renewal is late refuses it while it keeps serving the rest.
				let refuse_probes = !topology.may_take_ownership()?;
				let reply = match LogShardCommand::from_wire(request) {
					Ok(LogShardCommand::GetDiskIncarnation) if refuse_probes => {
						crate::metrics::LEASE_DEGRADED_REFUSALS.with_label_values(&["disk_probe"]).inc();
						futures_util::future::ready(Some(StorageWorkerResponse::RejectResponse(lease_degraded()))).boxed()
					}
					Ok(command) => single_reply(&shard, label, command, deadline_boot_ms),
					Err(batch) => batch_reply(&shard, batch, refuse_probes, deadline_boot_ms),
				};
				replies.push(reply.map(move |response| (message, response)).boxed());
			},
		);
	}
	tracing::info!(worker_id = %pubsub.worker_id(), "log shard service stopped");
	Ok(())
}

/// Submits one request. `None` means the shard dropped it without an answer, so the caller
/// sees a timeout: the outcome is unknown.
fn single_reply(
	shard: &Shard,
	label: &'static str,
	command: LogShardCommand,
	deadline_boot_ms: u64,
) -> BoxFuture<'static, Option<StorageWorkerResponse>> {
	let timer = crate::metrics::LOG_SHARD_REQUEST_DURATION
		.with_label_values(&[label])
		.start_timer();
	let response = shard.submit_until(command, deadline_boot_ms);
	async move {
		let response = match response {
			Ok(response) => response.await.ok(),
			Err(reason) => Some(StorageWorkerResponse::RejectResponse(reason)),
		};
		timer.observe_duration();
		response
	}
	.boxed()
}

/// Submits a coalesced batch as one shard event and answers once every item has its own
/// answer, in request order. If the shard drops any item without an answer, the batch gets no
/// reply, so every item's outcome is unknown, exactly as it would be for that item alone.
/// With `refuse_probes`, disk probes are refused here exactly as a single probe would be, and
/// only the other items reach the shard.
fn batch_reply(
	shard: &Shard,
	batch: StorageWorkerBatchRequest,
	refuse_probes: bool,
	deadline_boot_ms: u64,
) -> BoxFuture<'static, Option<StorageWorkerResponse>> {
	crate::metrics::LOG_SHARD_BATCH_REQUESTS.observe(batch.requests.len() as f64);
	let count = batch.requests.len();
	let mut timers = Vec::with_capacity(count);
	let mut commands = Vec::with_capacity(count);
	let mut refused = vec![false; count];
	for (index, item) in batch.requests.into_iter().enumerate() {
		let label = depot_types::storage_worker::item_label(&item);
		let command = LogShardCommand::from_item(item);
		if refuse_probes && matches!(command, LogShardCommand::GetDiskIncarnation) {
			crate::metrics::LEASE_DEGRADED_REFUSALS
				.with_label_values(&["disk_probe"])
				.inc();
			refused[index] = true;
			continue;
		}
		timers.push(
			crate::metrics::LOG_SHARD_REQUEST_DURATION
				.with_label_values(&[label])
				.start_timer(),
		);
		commands.push(command);
	}
	let receivers = shard.submit_many_until(commands, deadline_boot_ms);
	async move {
		let receivers = match receivers {
			Ok(receivers) => receivers,
			Err(reason) => {
				let responses = refused
					.iter()
					.map(|refused| {
						let reason = match refused {
							true => lease_degraded(),
							false => reason.clone(),
						};
						StorageWorkerBatchItemResponse::RejectResponse(reason)
					})
					.collect();
				return Some(StorageWorkerResponse::StorageWorkerBatchResponse(
					StorageWorkerBatchResponse { responses },
				));
			}
		};
		let answers = futures_util::future::join_all(receivers.into_iter().zip(timers).map(
			|(receiver, timer)| async move {
				let response = receiver.await.ok();
				timer.observe_duration();
				response
			},
		))
		.await;
		let mut answers = answers.into_iter();
		let mut responses = Vec::with_capacity(count);
		for refused in refused {
			if refused {
				responses.push(StorageWorkerBatchItemResponse::RejectResponse(
					lease_degraded(),
				));
				continue;
			}
			// The shard answers each item with a single response, never a batch.
			let response = depot_types::storage_worker::item_response(answers.next()??).ok()?;
			responses.push(response);
		}
		Some(StorageWorkerResponse::StorageWorkerBatchResponse(
			StorageWorkerBatchResponse { responses },
		))
	}
	.boxed()
}

/// The refusal of new ownership while this worker's lease renewal is late. The worker keeps
/// serving; the caller retries later or places elsewhere.
fn lease_degraded() -> RejectResponse {
	RejectResponse::NotServing
}
