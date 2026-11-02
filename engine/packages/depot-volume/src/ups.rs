//! Versioned replica messages over UPS. The caller owns retries, operation IDs, and authority.

pub mod client;
mod lanes;
mod metrics;

pub use lanes::LogRequestLanes;

use anyhow::{Result, ensure};
use depot_types::keys::pubsub::VolumeLogSubject;
use lanes::Admission;
use rivet_data::{
	DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
	generated::depot_storage_worker_protocol_v1::{StorageWorkerRequest, StorageWorkerResponse},
	versioned::{DepotStorageWorkerRequestData, DepotStorageWorkerResponseData},
};
use rivet_topology::worker::WorkerId;
use std::{sync::Arc, time::Duration};
use universalpubsub::{PubSub, SubjectCapacities};
use vbare::Versioned;

/// Cloneable transport with one caller-owned dispatch fence. Clearing that fence prevents new
/// sends, including a request that was waiting on endpoint discovery or queued behind other
/// requests to its worker; it cannot undo a sent write.
#[derive(Clone)]
pub struct UpsLogClient {
	pubsub: PubSub,
	capacities: SubjectCapacities,
	timeout: Duration,
	may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	lanes: Arc<LogRequestLanes>,
}

impl UpsLogClient {
	/// This process as the holder of the volumes its clients claim: its topology worker and the
	/// instance that identifies this process's lifetime.
	pub fn holder(&self) -> rivet_data::generated::shared_depot_types_v1::VolumeHolder {
		rivet_data::generated::shared_depot_types_v1::VolumeHolder {
			worker_id: *self.pubsub.worker_id().as_bytes(),
			worker_incarnation: *self.pubsub.instance_id().as_bytes(),
		}
	}

	/// Constructs a transport without starting a service or resolving any worker. It coalesces
	/// only its own requests; owners of many clients share lanes with `with_lanes`.
	pub fn new(
		pubsub: PubSub,
		capacities: SubjectCapacities,
		timeout: Duration,
		may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Self> {
		ensure!(
			!timeout.is_zero(),
			"replica request timeout must be positive"
		);
		ensure!(
			capacities.incoming > 0 && capacities.outgoing > 0,
			"replica subject capacities must be positive"
		);
		let lanes = Self::lanes(&pubsub, capacities, DEFAULT_INFLIGHT_PER_WORKER)?;
		Ok(Self {
			pubsub,
			capacities,
			timeout,
			may_dispatch,
			lanes,
		})
	}

	/// Lanes sized for this pubsub and subject capacity. Queued requests per worker are bounded
	/// by the subject's outgoing capacity, counted per request. A batch is bounded by half the
	/// largest UPS message and by one default disk batch, so the shard can write every item of
	/// one request in one checksummed batch.
	pub fn lanes(
		pubsub: &PubSub,
		capacities: SubjectCapacities,
		inflight_per_worker: usize,
	) -> Result<Arc<LogRequestLanes>> {
		LogRequestLanes::new(
			pubsub.clone(),
			inflight_per_worker,
			capacities.outgoing,
			(pubsub.max_message_bytes() / 2).min(batch_payload_budget()),
		)
	}

	/// Shares coalescing with every other client using the same lanes.
	pub fn with_lanes(mut self, lanes: Arc<LogRequestLanes>) -> Self {
		self.lanes = lanes;
		self
	}

	/// Remembers a log shard's refusal of an append for every client sharing these lanes, for
	/// as long as its hint asks. Other replies are not refusals of the shard's capacity.
	pub(crate) fn record_append_refusal(
		&self,
		worker: &WorkerId,
		reason: &rivet_data::generated::depot_storage_worker_protocol_v1::RejectResponse,
	) {
		use rivet_data::generated::depot_storage_worker_protocol_v1::RejectResponse;
		let retry_after_ms = if let RejectResponse::Overloaded(refusal) = reason {
			refusal.retry_after_ms
		} else if let RejectResponse::DiskFull(refusal) = reason {
			refusal.retry_after_ms
		} else {
			return;
		};
		let Ok(now_boot_ms) = self.pubsub.env().clock().boot_ms() else {
			return;
		};
		self.lanes
			.record_refusal(worker, now_boot_ms.saturating_add(retry_after_ms));
	}

	/// Records that an append to `worker` sent at `sent_boot_ms` was answered, or failed in
	/// transport, now.
	pub(crate) fn record_append_latency(&self, worker: &WorkerId, sent_boot_ms: u64, served: bool) {
		let Ok(now_boot_ms) = self.pubsub.env().clock().boot_ms() else {
			return;
		};
		self.lanes
			.record_append_latency(worker, sent_boot_ms, now_boot_ms, served);
	}

	/// The current boot time on this owner's clock.
	pub(crate) fn now_boot_ms(&self) -> Option<u64> {
		self.pubsub.env().clock().boot_ms().ok()
	}

	/// Until when new writes toward `worker` are refused, if they are: its log shard refused
	/// this owner's appends, or this owner's requests to it queue too long.
	pub(crate) fn refusing_until(&self, worker: &WorkerId, now_boot_ms: u64) -> Option<u64> {
		self.lanes.refusing_until(worker, now_boot_ms)
	}

	/// Sends once at the exact worker, preserving every field the core selected. Endpoint lookup,
	/// queueing and receiving share the original deadline, and malformed replies fail decoding.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn request(
		&self,
		worker: &WorkerId,
		request: StorageWorkerRequest,
	) -> Result<StorageWorkerResponse> {
		let clock = self.pubsub.env().clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), self.timeout)?;
		let subject = VolumeLogSubject::new(worker.clone());
		let reservation = self.lanes.reservation(worker, &subject, self.capacities)?;
		match self.lanes.admit(
			worker,
			&subject,
			request,
			deadline,
			&self.may_dispatch,
			reservation,
		)? {
			Admission::Send(request, slot, _reservation) => {
				let (slot, ticket) = match slot {
					Some((slot, ticket)) => (Some(slot), Some(ticket)),
					None => (None, None),
				};
				let response = send(
					&self.pubsub,
					worker,
					&subject,
					&request,
					deadline,
					self.may_dispatch.as_ref(),
					ticket,
				)
				.await;
				drop(slot);
				response
			}
			Admission::Queued(waiting) => {
				LogRequestLanes::wait(clock.as_ref(), waiting, deadline).await
			}
		}
	}
}

/// Sends one encoded request at `worker` before `deadline`, on its ordered turn when it has one.
async fn send(
	pubsub: &PubSub,
	worker: &WorkerId,
	subject: &VolumeLogSubject,
	request: &StorageWorkerRequest,
	deadline: u64,
	may_dispatch: &(dyn Fn() -> bool + Send + Sync),
	ticket: Option<universalpubsub::OrderTicket>,
) -> Result<StorageWorkerResponse> {
	let clock = pubsub.env().clock();
	let bytes = DepotStorageWorkerRequestData::serialize_with_embedded_version(
		request,
		DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
	)?;
	let remaining = deadline.saturating_sub(clock.observe()?.boot_ms);
	if remaining == 0 {
		return Err(universalpubsub::errors::Timeout.build());
	}
	let timeout = Duration::from_millis(remaining);
	let reply = match ticket {
		Some(ticket) => {
			pubsub
				.request_ordered(
					universalpubsub::RequestClass::DepotLog,
					worker,
					subject,
					&bytes,
					timeout,
					may_dispatch,
					ticket,
				)
				.await?
		}
		None => {
			pubsub
				.request_with_timeout_guarded(
					universalpubsub::RequestClass::DepotLog,
					worker,
					subject,
					&bytes,
					timeout,
					may_dispatch,
				)
				.await?
		}
	};
	// Decoding shares the reply buffer: returned entries hold slices of it, and status and ack
	// responses allocate nothing.
	Ok(DepotStorageWorkerResponseData::decode(reply)?)
}

/// In-flight requests per destination worker before later ones queue for a batch.
pub const DEFAULT_INFLIGHT_PER_WORKER: usize = 2;

/// The request bytes a coalesced batch may carry so its frames fit one default disk batch. The
/// shard frames each item with its volume key and state, so a quarter of the batch is left for
/// that overhead.
pub fn batch_payload_budget() -> usize {
	let disk_batch = depot_types::log_shard::DEFAULT_MAX_BATCH_BYTES as usize;
	disk_batch - disk_batch / 4
}
