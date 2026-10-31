//! Coalesces log shard requests per destination worker.
//!
//! While fewer than `inflight_per_worker` requests to a worker are in flight, a request is
//! sent at once by its caller. Otherwise it waits in that worker's queue. When a slot frees,
//! the lane takes everything queued, up to a byte budget, and sends it as one batch from a task
//! the lanes own. Nothing waits on a timer, so a lightly loaded worker sees no added latency.
//!
//! A batch belongs to no caller, so a caller that times out or is cancelled only stops waiting
//! for its own answer; the other items in its batch are still sent and answered. Each item
//! keeps its own deadline, dispatch guard and response. An item whose caller left or whose
//! deadline passed while queued is never sent. A batch waits until the latest deadline among
//! its items, and an item answered after its own deadline answers a timeout. A batch that fails
//! fails each of its items with the batch's typed error, with the same unknown outcome a single
//! failed request has. Like any timed-out request, an item whose caller already got a timeout
//! may still reach the worker while its batch waits for its turn. Batch tasks are bounded by
//! the in-flight slots, end by their deadline, and are cancelled when the lanes are dropped.
//!
//! Requests reach a worker in the order the lane admitted them. Admission finishes without
//! yielding, so a caller that starts its requests in order admits them in order, and the lane
//! takes each UPS request's turn on the worker's ordered lane at the moment it decides to send
//! it, under the lane lock. The log shard therefore receives one volume client's appends in the
//! order the client issued them, even with several requests in flight.

use anyhow::Result;
use depot_types::keys::pubsub::VolumeLogSubject;
use depot_types::storage_worker::{batch_item, item_request};
use parking_lot::Mutex;
use rivet_data::generated::depot_storage_worker_protocol_v1::{
	StorageWorkerBatchItem as Item, StorageWorkerBatchRequest, StorageWorkerRequest as Request,
	StorageWorkerResponse as Response,
};
use rivet_error::RivetError;
use rivet_metrics::prometheus::{
	Histogram, IntGauge, register_histogram_with_registry, register_int_gauge_with_registry,
};
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use rivet_topology::worker::WorkerId;
use std::{
	collections::VecDeque,
	sync::{Arc, Weak},
};
use tokio::sync::oneshot;
use universalpubsub::{OrderTicket, PubSub, SubjectCapacities, SubjectReservation};

lazy_static::lazy_static! {
	static ref BATCH_REQUESTS: Histogram = register_histogram_with_registry!(
		"depot_volume_log_client_batch_requests",
		"Requests sent in one log shard request after waiting for an in-flight slot; one means a queued request went alone.",
		rivet_metrics::PAGE_COUNT_BUCKETS.to_vec(),
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	static ref QUEUED_REQUESTS: IntGauge = register_int_gauge_with_registry!(
		"depot_volume_log_client_queued_requests",
		"Log shard requests waiting for an in-flight slot to their destination worker, across workers.",
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	static ref QUEUE_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"depot_volume_log_client_queue_capacity",
		"Configured per-worker log shard request queue capacity, summed over live coalescers.",
		*rivet_metrics::REGISTRY
	)
	.unwrap();
}

/// An append slower than this, from send to reply, shows its worker cannot take new writes
/// within the append timeout; half of the volume client's 2 s append timeout.
const SLOW_APPEND_MS: u64 = 1_000;

/// Lanes are pruned only when this many exist, so a steady set of destinations never churns.
const PRUNE_LANES_AT: usize = 64;

/// The fixed part of a request's size hint, generously above any header and key.
const REQUEST_OVERHEAD_BYTES: usize = 4096;

/// Per-entry framing above its payload bytes.
const ENTRY_OVERHEAD_BYTES: usize = 128;

/// Shared by every client of one owner. Lanes hold only counts and waiting requests.
pub struct LogRequestLanes {
	/// Issues each send's turn on the destination worker's ordered UPS lane.
	pubsub: PubSub,
	inflight_per_worker: usize,
	max_queued_per_worker: usize,
	max_batch_bytes: usize,
	lanes: scc::HashMap<WorkerId, Arc<Mutex<Lane>>>,
	/// Until when each worker's log shard refuses this owner's appends, from the latest
	/// refusal's hint. Every client of the owner reads it before admitting a new write, so a
	/// shard's refusal of one volume's append sheds the other volumes' new writes too. One
	/// entry per storage worker this owner has written to. Like the lane map it is only ever
	/// accessed synchronously, from async callers too, and no access holds an entry across an
	/// await, so no async code ever waits on it.
	refusals: scc::HashMap<WorkerId, u64>,
	/// Until when each worker counts as too slow for new writes, and when that was learned: an
	/// append to it slower than `SLOW_APPEND_MS` holds new writes back for as long as it
	/// took, and only a quicker append sent after that clears the hold. Accessed like
	/// `refusals`.
	slow: scc::HashMap<WorkerId, SlowHold>,
	/// Owns the tasks that send batches. Each task holds a slot, so at most
	/// `inflight_per_worker` run per worker, and each ends by its batch's deadline. Tasks hold
	/// the lanes only weakly, so dropping the lanes cancels them.
	tasks: rivet_env::TaskScope,
	_capacity_metric: IntGaugeGuard,
}

/// A hold on new writes toward a worker whose append was slow.
#[derive(Clone, Copy)]
struct SlowHold {
	until_boot_ms: u64,
	learned_at_boot_ms: u64,
}

#[derive(Default)]
struct Lane {
	/// UPS requests to this worker in flight, single or batched. Each holds one `Slot`.
	in_flight: usize,
	queued: VecDeque<Queued>,
	/// Set when pruned out of the map, so a caller holding a stale handle looks up again.
	retired: bool,
	/// Set when the queue fills, cleared when it drains below half, so each overload warns once.
	full_warned: bool,
	/// The worker's log subject budgets and the capacities they were reserved with, kept while
	/// the lane exists so back-to-back requests do not rebuild them.
	reservation: Option<(SubjectCapacities, SubjectReservation)>,
}

/// One queued request and the facts that decide whether it is still sent.
struct Queued {
	/// The destination, for the turn its batch takes.
	worker: WorkerId,
	item: Item,
	deadline_boot_ms: u64,
	may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	size_hint: usize,
	reply: oneshot::Sender<ItemResult>,
	/// Keeps the subject's request budgets alive until this item is answered, even after its
	/// caller stopped waiting.
	reservation: SubjectReservation,
	/// Counts this request on `QUEUED_REQUESTS` while it waits in the lane's queue.
	queued_metric: Option<IntGaugeGuard>,
}

type ItemResult = std::result::Result<Response, ItemFailure>;

/// Queued requests a lane task sends as one UPS request.
struct Batch {
	worker: WorkerId,
	items: Vec<Queued>,
	/// Released when the batch completes, which hands the next queued requests on.
	slot: Slot,
	/// The batch's turn on the worker's ordered lane.
	ticket: OrderTicket,
}

/// Answers one sent item after its batch completes.
struct Answer {
	deadline_boot_ms: u64,
	reply: oneshot::Sender<ItemResult>,
	_reservation: SubjectReservation,
}

/// Why one item failed. Batch failures keep the batch error's structured type, so a timeout
/// or an overload reaches every item of the batch as that same error.
#[derive(Clone)]
enum ItemFailure {
	Timeout,
	DispatchRejected,
	Failed {
		typed: Option<Box<RivetError>>,
		detail: String,
	},
}

impl ItemFailure {
	fn from_error(error: &anyhow::Error) -> Self {
		Self::Failed {
			typed: error
				.chain()
				.find_map(|cause| cause.downcast_ref::<RivetError>())
				.cloned()
				.map(Box::new),
			detail: format!("{error:#}"),
		}
	}

	fn unavailable(detail: &str) -> Self {
		Self::Failed {
			typed: None,
			detail: detail.to_owned(),
		}
	}

	fn into_error(self) -> anyhow::Error {
		match self {
			Self::Timeout => universalpubsub::errors::Timeout.build(),
			Self::DispatchRejected => universalpubsub::errors::DispatchRejected.build(),
			Self::Failed {
				typed: Some(typed),
				detail,
			} => anyhow::Error::new(*typed).context(detail),
			Self::Failed {
				typed: None,
				detail,
			} => anyhow::Error::msg(detail),
		}
	}
}

/// One in-flight request to a lane's worker. Dropping it frees the slot and sends the next
/// queued requests as a batch.
pub(super) struct Slot {
	lane: Arc<Mutex<Lane>>,
	/// Weak so a running batch task does not keep its owner's lanes alive.
	lanes: Weak<LogRequestLanes>,
}

impl Drop for Slot {
	fn drop(&mut self) {
		// Lanes that were dropped have cancelled their tasks; nothing is left to hand off.
		let Some(lanes) = self.lanes.upgrade() else {
			return;
		};
		let batches = {
			let mut lane = self.lane.lock();
			lane.in_flight = lane.in_flight.saturating_sub(1);
			lanes.take_batches(&self.lane, &mut lane)
		};

		// Spawn outside the lane lock: a task dropped unpolled drops its slot, which locks
		// this lane again.
		for batch in batches {
			let _detached = lanes.tasks.spawn(send_batch(lanes.pubsub.clone(), batch));
		}
	}
}

/// How a request proceeds once its lane admitted it.
pub(super) enum Admission {
	/// Send the request alone now, on its turn, with the caller's reservation. A caller-built
	/// batch holds no slot and takes no turn.
	Send(Request, Option<(Slot, OrderTicket)>, SubjectReservation),
	/// The request waits for a lane task to send it.
	Queued(Waiting),
}

/// Yields the queued request's response.
pub(super) struct Waiting(oneshot::Receiver<ItemResult>);

impl LogRequestLanes {
	/// `max_queued_per_worker` bounds requests waiting for each worker, counted per request,
	/// not per batch. `max_batch_bytes` bounds a batch's estimated encoded size.
	pub fn new(
		pubsub: PubSub,
		inflight_per_worker: usize,
		max_queued_per_worker: usize,
		max_batch_bytes: usize,
	) -> Result<Arc<Self>> {
		anyhow::ensure!(
			inflight_per_worker > 0 && max_queued_per_worker > 0 && max_batch_bytes > 0,
			"log request lane bounds must be positive"
		);
		Ok(Arc::new(Self {
			_capacity_metric: QUEUE_CAPACITY.add_guard(max_queued_per_worker as i64),
			tasks: rivet_env::TaskScope::new(pubsub.env().clone()),
			pubsub,
			inflight_per_worker,
			max_queued_per_worker,
			max_batch_bytes,
			lanes: scc::HashMap::new(),
			refusals: scc::HashMap::new(),
			slow: scc::HashMap::new(),
		}))
	}

	/// Records that `worker`'s log shard refused an append from this owner until
	/// `until_boot_ms`. A later refusal never shortens an earlier one.
	pub(super) fn record_refusal(&self, worker: &WorkerId, until_boot_ms: u64) {
		self.refusals
			.entry_sync(worker.clone())
			.and_modify(|until| *until = (*until).max(until_boot_ms))
			.or_insert(until_boot_ms);
	}

	/// Records an append to `worker` sent at `sent_boot_ms` and answered, or failed, at
	/// `now_boot_ms`. A slow one extends the hold on new writes; a quick one that was
	/// `served` ends a hold learned before it was sent, so a slow reply that lands after it is
	/// not cleared by it.
	pub(super) fn record_append_latency(
		&self,
		worker: &WorkerId,
		sent_boot_ms: u64,
		now_boot_ms: u64,
		served: bool,
	) {
		let took_ms = now_boot_ms.saturating_sub(sent_boot_ms);
		if took_ms > SLOW_APPEND_MS {
			let hold = SlowHold {
				until_boot_ms: now_boot_ms.saturating_add(took_ms),
				learned_at_boot_ms: now_boot_ms,
			};
			self.slow
				.entry_sync(worker.clone())
				.and_modify(|held| {
					held.until_boot_ms = held.until_boot_ms.max(hold.until_boot_ms);
					held.learned_at_boot_ms = now_boot_ms;
				})
				.or_insert(hold);
		} else if served {
			self.slow
				.remove_if_sync(worker, |held| held.learned_at_boot_ms <= sent_boot_ms);
		}
	}

	/// Until when new writes toward `worker` are refused at `now_boot_ms`, if they are: the
	/// later of the latest refusal hint its log shard gave this owner and the hold its slow
	/// appends set.
	pub(super) fn refusing_until(&self, worker: &WorkerId, now_boot_ms: u64) -> Option<u64> {
		let refused = self.refusals.read_sync(worker, |_, until| *until);
		let slow = self.slow.read_sync(worker, |_, held| held.until_boot_ms);
		refused.max(slow).filter(|until| *until > now_boot_ms)
	}

	/// Sends at once while the worker has a free slot and nothing queued ahead, so order per
	/// destination stays first in, first out. Otherwise queues, or rejects as overloaded when
	/// the worker's queue is full. Admission never yields: the lane map is only ever accessed
	/// synchronously, and no async code waits on it.
	pub(super) fn admit(
		self: &Arc<Self>,
		worker: &WorkerId,
		subject: &VolumeLogSubject,
		request: Request,
		deadline_boot_ms: u64,
		may_dispatch: &Arc<dyn Fn() -> bool + Send + Sync>,
		reservation: SubjectReservation,
	) -> Result<Admission> {
		// A caller's own batch is never nested inside another.
		let item = match batch_item(request) {
			Ok(item) => item,
			Err(batch) => {
				return Ok(Admission::Send(
					Request::StorageWorkerBatchRequest(batch),
					None,
					reservation,
				));
			}
		};
		loop {
			let handle = self.lane(worker);
			let mut lane = handle.lock();
			if lane.retired {
				continue;
			}
			if lane.queued.is_empty() && lane.in_flight < self.inflight_per_worker {
				lane.in_flight += 1;
				let ticket = self.pubsub.order_ticket(worker, subject);
				drop(lane);
				return Ok(Admission::Send(
					item_request(item),
					Some((
						Slot {
							lane: handle,
							lanes: Arc::downgrade(self),
						},
						ticket,
					)),
					reservation,
				));
			}
			if lane.queued.len() >= self.max_queued_per_worker {
				if !lane.full_warned {
					lane.full_warned = true;
					tracing::warn!(
						%worker,
						capacity = self.max_queued_per_worker,
						"log request queue for a worker is full; the worker or the network is slower than the request rate"
					);
				}
				return Err(universalpubsub::errors::Overloaded.build());
			}
			let (reply, receiver) = oneshot::channel();
			lane.queued.push_back(Queued {
				worker: worker.clone(),
				size_hint: size_hint(&item),
				item,
				deadline_boot_ms,
				may_dispatch: may_dispatch.clone(),
				reply,
				reservation,
				queued_metric: Some(QUEUED_REQUESTS.inc_guard()),
			});
			return Ok(Admission::Queued(Waiting(receiver)));
		}
	}

	/// Reserves `worker`'s log subject for a request, reusing the reservation its lane keeps.
	/// A caller with other capacities reserves afresh, which fails while the kept one is alive.
	pub(super) fn reservation(
		&self,
		worker: &WorkerId,
		subject: &VolumeLogSubject,
		capacities: SubjectCapacities,
	) -> Result<SubjectReservation> {
		loop {
			let handle = self.lane(worker);
			let mut lane = handle.lock();
			if lane.retired {
				continue;
			}
			if let Some((kept, reservation)) = &lane.reservation
				&& *kept == capacities
			{
				return Ok(reservation.clone());
			}
			let reservation = self.pubsub.reserve_request_subject(
				subject,
				universalpubsub::RequestClass::DepotLog,
				capacities,
			)?;
			lane.reservation = Some((capacities, reservation.clone()));
			return Ok(reservation);
		}
	}

	/// Waits until the queued request is answered or its own deadline passes. Dropping this
	/// wait only stops waiting: a request still queued is skipped unsent, and one already sent
	/// completes for the rest of its batch.
	pub(super) async fn wait(
		clock: &dyn rivet_env::Clock,
		waiting: Waiting,
		deadline_boot_ms: u64,
	) -> Result<Response> {
		match rivet_env::timeout_at(clock, deadline_boot_ms, waiting.0).await {
			Err(_) => Err(universalpubsub::errors::Timeout.build()),
			Ok(Err(_)) => Err(universalpubsub::errors::Transport
				.build()
				.context("log request batch was cancelled before answering this request")),
			Ok(Ok(result)) => result.map_err(ItemFailure::into_error),
		}
	}

	/// Takes queued requests into batches while slots are free. The caller holds the lane and
	/// spawns the batches after releasing it.
	fn take_batches(self: &Arc<Self>, handle: &Arc<Mutex<Lane>>, lane: &mut Lane) -> Vec<Batch> {
		let mut batches = Vec::new();
		while lane.in_flight < self.inflight_per_worker {
			let Some(mut first) = lane.queued.pop_front() else {
				break;
			};
			first.queued_metric = None;
			let mut bytes = first.size_hint;
			let worker = first.worker.clone();
			let mut items = vec![first];
			while let Some(next) = lane.queued.front() {
				if bytes.saturating_add(next.size_hint) > self.max_batch_bytes {
					break;
				}
				bytes = bytes.saturating_add(next.size_hint);
				if let Some(mut queued) = lane.queued.pop_front() {
					queued.queued_metric = None;
					items.push(queued);
				}
			}
			lane.in_flight += 1;
			batches.push(Batch {
				ticket: self.ticket(&worker),
				worker,
				items,
				slot: Slot {
					lane: handle.clone(),
					lanes: Arc::downgrade(self),
				},
			});
		}
		if lane.queued.len() * 2 < self.max_queued_per_worker {
			lane.full_warned = false;
		}
		batches
	}

	/// The request for a batch: a single request when only one item remains.
	fn request(items: Vec<Item>) -> Request {
		if items.len() == 1 {
			let item = items.into_iter().next().expect("one item");
			return item_request(item);
		}
		Request::StorageWorkerBatchRequest(StorageWorkerBatchRequest { requests: items })
	}

	/// The next turn on `worker`'s ordered log lane. Called under the lane lock, in the order
	/// the lane decides to send.
	fn ticket(&self, worker: &WorkerId) -> OrderTicket {
		self.pubsub
			.order_ticket(worker, VolumeLogSubject::new(worker.clone()))
	}

	/// Synchronous, so admission completes in the poll that starts it; see `admit`.
	fn lane(&self, worker: &WorkerId) -> Arc<Mutex<Lane>> {
		if let Some(lane) = self.lanes.read_sync(worker, |_, lane| lane.clone()) {
			return lane;
		}
		if self.lanes.len() >= PRUNE_LANES_AT {
			self.lanes.retain_sync(|_, lane| {
				let mut lane = lane.lock();
				let idle = lane.in_flight == 0 && lane.queued.is_empty();
				if idle {
					lane.retired = true;
				}
				!idle
			});
		}
		self.lanes
			.entry_sync(worker.clone())
			.or_default()
			.get()
			.clone()
	}
}

/// Sends one batch and answers each of its items. Runs in a lane-owned task, so no caller's
/// cancellation can drop it.
async fn send_batch(pubsub: PubSub, batch: Batch) {
	let Batch {
		worker,
		items,
		slot,
		ticket,
	} = batch;
	let clock = pubsub.env().clock();
	let now = match clock.observe() {
		Ok(now) => now.boot_ms,
		Err(error) => {
			let failure = ItemFailure::from_error(&error.into());
			for item in items {
				let _ = item.reply.send(Err(failure.clone()));
			}
			return;
		}
	};

	// Answer items that may no longer be sent, and drop items whose caller left.
	let mut sending = Vec::with_capacity(items.len());
	for item in items {
		let failure = if item.deadline_boot_ms <= now {
			ItemFailure::Timeout
		} else if item.reply.is_closed() {
			continue;
		} else if !(item.may_dispatch)() {
			ItemFailure::DispatchRejected
		} else {
			sending.push(item);
			continue;
		};
		let _ = item.reply.send(Err(failure));
	}
	if sending.is_empty() {
		return;
	}
	BATCH_REQUESTS.observe(sending.len() as f64);

	// The batch waits for its most patient item and is dispatched only while every item
	// may still be sent.
	let deadline = sending
		.iter()
		.map(|item| item.deadline_boot_ms)
		.max()
		.unwrap_or(now);
	let guards = sending
		.iter()
		.map(|item| item.may_dispatch.clone())
		.collect::<Vec<_>>();
	let may_dispatch = move || guards.iter().all(|guard| guard());
	let mut requests = Vec::with_capacity(sending.len());
	let mut answers = Vec::with_capacity(sending.len());
	for item in sending {
		requests.push(item.item);
		answers.push(Answer {
			deadline_boot_ms: item.deadline_boot_ms,
			reply: item.reply,
			_reservation: item.reservation,
		});
	}
	let batched = requests.len() > 1;
	let request = LogRequestLanes::request(requests);
	let subject = VolumeLogSubject::new(worker.clone());
	let response = super::send(
		&pubsub,
		&worker,
		&subject,
		&request,
		deadline,
		&may_dispatch,
		Some(ticket),
	)
	.await;
	drop(slot);

	// Answer every item with its own response, in order. A failed or malformed batch fails
	// each item alike, with the outcome unknown.
	let mut responses: Vec<ItemResult> = match (batched, response) {
		(false, Ok(response)) => vec![Ok(response)],
		(true, Ok(Response::StorageWorkerBatchResponse(batch)))
			if batch.responses.len() == answers.len() =>
		{
			batch
				.responses
				.into_iter()
				.map(|response| Ok(depot_types::storage_worker::response_from_item(response)))
				.collect()
		}
		(true, Ok(_)) => vec![
			Err(ItemFailure::unavailable(
				"log shard answered a batch with a mismatched reply",
			));
			answers.len()
		],
		(_, Err(error)) => vec![Err(ItemFailure::from_error(&error)); answers.len()],
	};
	responses.truncate(answers.len());
	let answered_at = clock.observe().map(|now| now.boot_ms).ok();
	let mut responses = responses.into_iter();
	for answer in answers {
		let expired = answered_at.is_some_and(|now| now >= answer.deadline_boot_ms);
		let result = match responses.next() {
			_ if expired => Err(ItemFailure::Timeout),
			Some(result) => result,
			None => Err(ItemFailure::unavailable(
				"log shard batch reply ended early",
			)),
		};
		let _ = answer.reply.send(result);
	}
}

/// An upper bound on the item's encoded size: payload bytes plus generous fixed framing.
fn size_hint(item: &Item) -> usize {
	let entries = |entries: &[rivet_data::generated::depot_log_shard_v1::VolumeLogEntry]| {
		entries
			.iter()
			.map(|entry| {
				let payload = match &entry.body {
					rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::PayloadWrite(
						write,
					) => write.payload.len(),
					rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::ClaimEntry => 0,
				};
				payload + ENTRY_OVERHEAD_BYTES
			})
			.sum::<usize>()
	};
	REQUEST_OVERHEAD_BYTES
		+ match item {
			Item::AppendRequest(request) => entries(&request.entries),
			Item::LoadVolumeRequest(request) => entries(&request.initial_entries),
			Item::ReadVolumeLogStatusRequest(request) => {
				request.volume_log_keys.len() * REQUEST_OVERHEAD_BYTES
			}
			Item::ForgetVolumesRequest(request) => request.volumes.len() * REQUEST_OVERHEAD_BYTES,
			Item::TrimVolumesRequest(request) => request.volumes.len() * REQUEST_OVERHEAD_BYTES,
			Item::GetDiskIncarnationRequest
			| Item::ClaimRequest(_)
			| Item::AdvanceCommittedRequest(_)
			| Item::TruncateRequest(_)
			| Item::ReadVolumeLogEntriesRequest(_)
			| Item::MarkInSyncRequest(_)
			| Item::MarkVolumeDeletedRequest(_)
			| Item::TrimRequest(_)
			| Item::ForgetVolumeRequest(_)
			| Item::ListTabletReplicasRequest(_) => 0,
		}
}
