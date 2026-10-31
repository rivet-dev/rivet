//! The calling side of peer channels: several channels per peer incarnation, each opened on
//! first use.

use std::sync::{
	Arc, Weak,
	atomic::{AtomicBool, AtomicUsize, Ordering},
};

use anyhow::Result;
use bytes::Bytes;
use rivet_data::generated::ups_channel_v1::{
	ChannelBody, ChannelCancel, ChannelOpen, ChannelResponse, Frame,
};
use rivet_topology::worker::Worker;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot};

use super::{
	PATH, frame,
	state::{ChannelState, Refusal},
};
use crate::{
	errors,
	pubsub::{PubSub, WorkerId, class::RequestClass, order::OrderTicket, stream::Reader},
};

/// A peer incarnation: its worker and the instance its endpoint names.
pub(crate) type PeerKey = (WorkerId, [u8; 16]);

/// One channel to a peer incarnation. Slots below `channels` carry small requests and the rest
/// carry large ones; slot `i` rides request connection `i % channels`.
pub(crate) type ChannelKey = (PeerKey, usize);

/// How a caller is answered: the reply, or the error that ended its request.
type Waiter = oneshot::Sender<Result<Bytes>>;

/// One open channel. The queue sender keeps its request body open; dropping the channel ends
/// the body once queued frames are written.
pub(crate) struct Connection {
	state: parking_lot::Mutex<ChannelState<Waiter>>,
	frames: mpsc::Sender<Outgoing>,
	/// Bytes of request frames waiting in `frames`.
	queued_bytes: Arc<frame::ByteBudget>,
}

/// A frame in a channel's write queue. A request frame whose caller stopped waiting before it
/// was written is dropped unwritten, so a stalled channel does not later spend its bandwidth
/// on requests nobody waits for.
pub(crate) struct Outgoing {
	frame: Bytes,
	abandoned: Option<Arc<AtomicBool>>,
	_reserved: Option<frame::ByteReservation>,
}

impl Outgoing {
	fn control(frame: Bytes) -> Self {
		Self {
			frame,
			abandoned: None,
			_reserved: None,
		}
	}
}

impl frame::Queued for Outgoing {
	fn into_frame(self) -> Option<Bytes> {
		match &self.abandoned {
			Some(abandoned) if abandoned.load(Ordering::Acquire) => {
				crate::metrics::CHANNEL_ABANDONED_FRAMES.inc();
				None
			}
			Some(_) | None => Some(self.frame),
		}
	}
}

/// Every channel this process has open.
pub(crate) struct Connections {
	open: scc::HashMap<ChannelKey, Arc<Connection>>,
	outgoing_slots: Arc<Semaphore>,
	incoming_slots: Arc<Semaphore>,
	outgoing_full: AtomicBool,
	incoming_full: AtomicBool,
	_outgoing_capacity: rivet_metrics::IntGaugeGuard,
	_incoming_capacity: rivet_metrics::IntGaugeGuard,
	/// Spreads requests over a peer's channels.
	next_slot: AtomicUsize,
	/// Set once shutdown begins; no channel opens after it.
	closed: AtomicBool,
	tasks: rivet_env::TaskScope,
	/// The serving side's channel readers and request handlers.
	pub(super) server_tasks: rivet_env::TaskScope,
	/// Reply bytes queued across every channel this process serves.
	pub(super) reply_bytes: Arc<frame::ByteBudget>,
	/// Set while `reply_bytes` is full, so its warning logs once per episode.
	pub(super) reply_bytes_full: AtomicBool,
}

impl Connections {
	pub(crate) fn new(
		env: &Arc<dyn rivet_env::Env>,
		options: &rivet_config::config::pubsub::ChannelOptions,
	) -> Self {
		crate::metrics::CHANNEL_REPLY_QUEUE_BYTES_CAPACITY
			.with_label_values(&["channel"])
			.set(options.reply_queue_bytes as i64);
		crate::metrics::CHANNEL_REPLY_QUEUE_BYTES_CAPACITY
			.with_label_values(&["process"])
			.set(options.total_reply_queue_bytes as i64);
		Self {
			open: scc::HashMap::new(),
			outgoing_slots: Arc::new(Semaphore::new(options.outgoing_capacity)),
			incoming_slots: Arc::new(Semaphore::new(options.incoming_capacity)),
			outgoing_full: AtomicBool::new(false),
			incoming_full: AtomicBool::new(false),
			_outgoing_capacity: rivet_metrics::GaugeGuardExt::add_guard(
				&crate::metrics::CHANNELS_CAPACITY.with_label_values(&["client"]),
				options.outgoing_capacity as i64,
			),
			_incoming_capacity: rivet_metrics::GaugeGuardExt::add_guard(
				&crate::metrics::CHANNELS_CAPACITY.with_label_values(&["server"]),
				options.incoming_capacity as i64,
			),
			next_slot: AtomicUsize::new(0),
			closed: AtomicBool::new(false),
			tasks: rivet_env::TaskScope::new(env.clone()),
			server_tasks: rivet_env::TaskScope::new(env.clone()),
			reply_bytes: frame::ByteBudget::new(
				options.total_reply_queue_bytes,
				Some(&*crate::metrics::CHANNEL_REPLY_QUEUE_BYTES),
			),
			reply_bytes_full: AtomicBool::new(false),
		}
	}

	fn reserve_outgoing(&self) -> Result<OwnedSemaphorePermit> {
		self.reserve(
			&self.outgoing_slots,
			&self.outgoing_full,
			"outgoing_channels",
		)
	}

	pub(super) fn reserve_incoming(&self) -> Result<OwnedSemaphorePermit> {
		self.reserve(
			&self.incoming_slots,
			&self.incoming_full,
			"incoming_channels",
		)
	}

	fn reserve(
		&self,
		slots: &Arc<Semaphore>,
		full: &AtomicBool,
		capacity: &'static str,
	) -> Result<OwnedSemaphorePermit> {
		match slots.clone().try_acquire_owned() {
			Ok(slot) => {
				full.store(false, Ordering::Release);
				Ok(slot)
			}
			Err(_) => {
				if !full.swap(true, Ordering::AcqRel) {
					tracing::warn!(capacity, "UPS peer channel capacity reached");
				}
				Err(crate::metrics::overloaded(capacity, None))
			}
		}
	}

	/// Closes every channel and joins its task. Requests still pending fail at once with
	/// `consumer_closed`; cancelling a channel's task would otherwise leave them waiting for their
	/// deadlines.
	pub(crate) async fn shutdown(&self) {
		self.closed.store(true, Ordering::Release);
		let mut waiters = Vec::new();
		self.open
			.iter_async(|_, connection| {
				waiters.extend(connection.state.lock().fail());
				true
			})
			.await;
		for waiter in waiters {
			let _ = waiter.send(Err(errors::ConsumerClosed.build()));
		}
		self.open.clear_async().await;
		self.tasks.shutdown().await;
		self.server_tasks.shutdown().await;
	}
}

/// Ends a request's registration when its caller stops waiting, and tells the peer to cancel
/// the handler if the request was still pending.
struct Pending {
	connection: Arc<Connection>,
	id: u64,
	abandoned: Arc<AtomicBool>,
}

impl Drop for Pending {
	fn drop(&mut self) {
		self.abandoned.store(true, Ordering::Release);
		let cancelled = self.connection.state.lock().cancel(self.id);
		if cancelled
			&& let Ok(cancel) = frame::encode(Frame::ChannelCancel(ChannelCancel { id: self.id }))
		{
			// A full queue drops the cancel; the late reply is then ignored.
			let _ = self.connection.frames.try_send(Outgoing::control(cancel));
		}
	}
}

impl PubSub {
	/// Sends one unary request over one of the peer's channels and waits for its one reply,
	/// until `deadline_boot_ms`. An ordered request rides its lane's channel and enters the
	/// write queue only on its turn.
	#[allow(clippy::too_many_arguments)]
	pub(in crate::pubsub) async fn channel_request(
		&self,
		class: RequestClass,
		worker: &WorkerId,
		endpoint: &Worker,
		subject: &[u8],
		payload: &[u8],
		deadline_boot_ms: u64,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
		mut ticket: Option<OrderTicket>,
	) -> Result<Bytes> {
		let channels = &self.inner.channels;
		let options = &self.inner.options.channel;
		let slot = match &ticket {
			// Every request on an ordered lane shares one channel, whatever its size, so they
			// leave in turn order on one stream.
			Some(ticket) => (ticket.route() % options.channels as u64) as usize,
			None => {
				// Small and large requests ride separate channels, each on the same request
				// connections but its own HTTP/2 stream, so a large frame never holds small ones
				// behind it in one stream's byte order.
				let mut slot =
					channels.next_slot.fetch_add(1, Ordering::Relaxed) % options.channels;
				if payload.len() > options.large_request_bytes {
					slot += options.channels;
				}
				slot
			}
		};
		let key = ((worker.clone(), *endpoint.instance_id.as_bytes()), slot);

		// A channel that ended after it was looked up either refuses the submit or has already
		// dropped its write queue. Either way nothing was sent, so the request takes the channel
		// that replaces it. Only a second refusal, which means this worker is shutting down,
		// fails the request.
		let clock = self.env().clock();
		if let Some(ticket) = &ticket {
			#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
			let waiting_since = std::time::Instant::now();
			let turn = ticket.turn(clock.as_ref(), deadline_boot_ms).await;
			crate::metrics::ORDERED_TURN_WAIT.observe(waiting_since.elapsed().as_secs_f64());
			turn?;
		}
		// The turn passes once this request is queued or has failed, never later.
		let result = self
			.submit_request(
				class,
				&key,
				endpoint,
				subject,
				payload,
				deadline_boot_ms,
				may_dispatch,
			)
			.await;
		if let Some(ticket) = &mut ticket {
			ticket.pass();
		}
		let (pending, replied, opened_before) = result?;
		let _pending_gauge =
			rivet_metrics::GaugeGuardExt::inc_guard(&*crate::metrics::CHANNEL_PENDING);

		let result = match rivet_env::timeout_at(clock.as_ref(), deadline_boot_ms, replied).await {
			Err(_) => Err(errors::Timeout.build()),
			Ok(Err(_)) => Err(errors::ConsumerClosed.build()),
			Ok(Ok(result)) => result,
		};
		drop(pending);
		if let Err(error) = &result {
			self.observe_channel_error(endpoint, error, opened_before)
				.await;
		}
		result
	}

	/// Registers the request on the channel to `key` and puts its frame in the write queue.
	#[allow(clippy::too_many_arguments)]
	async fn submit_request(
		&self,
		class: RequestClass,
		key: &ChannelKey,
		endpoint: &Worker,
		subject: &[u8],
		payload: &[u8],
		deadline_boot_ms: u64,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
	) -> Result<(Pending, oneshot::Receiver<Result<Bytes>>, bool)> {
		let channels = &self.inner.channels;
		let mut refused = false;
		loop {
			if channels.closed.load(Ordering::Acquire) {
				return Err(errors::ConsumerClosed.build());
			}
			let connection = self.channel_connection(key, endpoint).await?;
			let (reply, replied) = oneshot::channel();
			let submitted = connection.state.lock().submit(reply);
			let id = match submitted {
				Ok(id) => id,
				Err((Refusal::NotAccepting, _)) => {
					if refused {
						return Err(errors::ConsumerClosed.build());
					}
					refused = true;
					channels
						.open
						.remove_if_async(key, |current| Arc::ptr_eq(current, &connection))
						.await;
					continue;
				}
			};
			let abandoned = Arc::new(AtomicBool::new(false));
			let pending = Pending {
				connection: connection.clone(),
				id,
				abandoned: abandoned.clone(),
			};
			let opened_before = connection.state.lock().is_ready();
			let queued = self.queue_request(
				&connection,
				id,
				class,
				subject,
				payload,
				deadline_boot_ms,
				abandoned,
				may_dispatch,
			);
			match queued {
				Ok(()) => return Ok((pending, replied, opened_before)),
				Err(QueueRefusal::Closed) => {
					drop(pending);
					if refused {
						return Err(errors::ConsumerClosed.build());
					}
					refused = true;
					channels
						.open
						.remove_if_async(key, |current| Arc::ptr_eq(current, &connection))
						.await;
				}
				Err(QueueRefusal::Failed(error)) => {
					drop(pending);
					self.observe_channel_error(endpoint, &error, opened_before)
						.await;
					return Err(error);
				}
			}
		}
	}

	/// Frames one submitted request and puts it in the channel's write queue. A closed queue
	/// means the channel ended before the frame could be written, so nothing was sent.
	#[allow(clippy::too_many_arguments)]
	fn queue_request(
		&self,
		connection: &Connection,
		id: u64,
		class: RequestClass,
		subject: &[u8],
		payload: &[u8],
		deadline_boot_ms: u64,
		abandoned: Arc<AtomicBool>,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
	) -> Result<(), QueueRefusal> {
		let clock = self.env().clock();
		let now = clock
			.observe()
			.map_err(|error| QueueRefusal::Failed(error.into()))?;
		let remaining = deadline_boot_ms.saturating_sub(now.boot_ms);
		if remaining == 0 {
			return Err(QueueRefusal::Failed(errors::Timeout.build()));
		}
		let request = frame::encode_request(id, class.into(), subject, remaining, payload);
		let Some(reserved) = connection.queued_bytes.try_reserve(request.len()) else {
			crate::metrics::CHANNEL_QUEUE_FULL
				.with_label_values(&["bytes"])
				.inc();
			return Err(QueueRefusal::Failed(crate::metrics::overloaded(
				"channel_bytes",
				Some(class.as_str()),
			)));
		};
		// This is the last boundary before the request leaves this process.
		if !may_dispatch() {
			return Err(QueueRefusal::Failed(errors::DispatchRejected.build()));
		}
		let request = Outgoing {
			frame: request,
			abandoned: Some(abandoned),
			_reserved: Some(reserved),
		};
		match connection.frames.try_send(request) {
			Ok(()) => Ok(()),
			Err(mpsc::error::TrySendError::Full(_)) => {
				crate::metrics::CHANNEL_QUEUE_FULL
					.with_label_values(&["frames"])
					.inc();
				Err(QueueRefusal::Failed(crate::metrics::overloaded(
					"channel_queue",
					Some(class.as_str()),
				)))
			}
			Err(mpsc::error::TrySendError::Closed(_)) => Err(QueueRefusal::Closed),
		}
	}

	/// Keeps endpoint freshness exactly as a stream does: a transport failure or a changed
	/// incarnation drops the cached route, and a timeout before the channel opened reports a
	/// stateless route as possibly stale.
	async fn observe_channel_error(
		&self,
		endpoint: &Worker,
		error: &anyhow::Error,
		opened_before: bool,
	) {
		match rivet_error::RivetError::extract(error).code() {
			"transport" | "wrong_worker" => {
				if let Err(error) = self.inner.topology.invalidate_endpoint(endpoint).await {
					tracing::warn!(%error, "failed to invalidate worker endpoint");
				}
			}
			"timeout"
				if !opened_before
					&& endpoint.role == rivet_topology::worker::WorkerRole::Stateless =>
			{
				self.inner.topology.report_endpoint_timeout(endpoint);
			}
			_ => {}
		}
	}

	/// The open channel to `key`, opening one if none is open.
	async fn channel_connection(
		&self,
		key: &ChannelKey,
		endpoint: &Worker,
	) -> Result<Arc<Connection>> {
		let channels = &self.inner.channels;
		if let Some(connection) = channels.open.read_async(key, |_, c| c.clone()).await {
			return Ok(connection);
		}
		match channels.open.entry_async(key.clone()).await {
			scc::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
			scc::hash_map::Entry::Vacant(entry) => {
				let slot = channels.reserve_outgoing()?;
				let options = &self.inner.options.channel;
				crate::metrics::CHANNEL_QUEUE_BYTES_CAPACITY.set(options.queue_bytes as i64);
				let (frames, queued) = mpsc::channel(options.queue_frames);
				let connection = Arc::new(Connection {
					state: parking_lot::Mutex::new(ChannelState::new()),
					frames,
					queued_bytes: frame::ByteBudget::new(
						options.queue_bytes,
						Some(&*crate::metrics::CHANNEL_QUEUE_BYTES),
					),
				});
				entry.insert_entry(connection.clone());
				let open = frame::encode(Frame::ChannelOpen(ChannelOpen {
					source_worker_id: *self.worker_id().as_bytes(),
					worker_id: *key.0.0.as_bytes(),
					instance_id: key.0.1,
				}));
				if let Ok(open) = open {
					let _ = connection.frames.try_send(Outgoing::control(open));
				}
				let ups = self.clone();
				let weak = Arc::downgrade(&connection);
				let endpoint = endpoint.clone();
				let key = key.clone();
				let max_write_bytes = options.max_write_bytes;
				channels.tasks.spawn(async move {
					let _slot = slot;
					ups.drive_channel(key, endpoint, weak, queued, max_write_bytes)
						.await;
				});
				Ok(connection)
			}
		}
	}

	/// Opens the channel's HTTP stream and delivers replies until it ends, then fails every
	/// request still pending once.
	async fn drive_channel(
		&self,
		key: ChannelKey,
		endpoint: Worker,
		connection: Weak<Connection>,
		queued: mpsc::Receiver<Outgoing>,
		max_write_bytes: usize,
	) {
		let _open = ChannelsOpenGuard::new("client");
		let body = frame::batched(queued, max_write_bytes);
		let outcome = self.run_channel(&endpoint, key.1, &connection, body).await;
		let reason = match &outcome {
			Ok(ChannelEnd::Refused(_)) => "refused",
			Ok(ChannelEnd::Unreachable(_)) => "unreachable",
			Ok(ChannelEnd::Closed) => "closed",
			Err(_) => "protocol_error",
		};
		crate::metrics::CHANNEL_ENDS
			.with_label_values(&[reason])
			.inc();
		if let Some(open) = connection.upgrade() {
			self.inner
				.channels
				.open
				.remove_if_async(&key, |current| Arc::ptr_eq(current, &open))
				.await;
			let waiters = open.state.lock().fail();
			for waiter in waiters {
				let error = match &outcome {
					// The peer refused the channel before reading any request.
					Ok(ChannelEnd::Refused(code)) => frame::error_from_code(code),
					// Only an adapter-certified connect failure proves nothing was sent.
					// Other failures before response headers can still follow dispatch.
					Ok(ChannelEnd::Unreachable(error)) => {
						if error
							.downcast_ref::<rivet_env::capabilities::http::RequestNotSent>()
							.is_some()
						{
							errors::EndpointUnresolved
								.build()
								.context(format!("{error:#}"))
						} else {
							errors::Transport.build().context(format!("{error:#}"))
						}
					}
					// Requests may have run before the channel ended.
					Ok(ChannelEnd::Closed) | Err(_) => errors::ConsumerClosed.build(),
				};
				let _ = waiter.send(Err(error));
			}

			// A channel that failed suggests the peer moved or restarted. Drop its cached route
			// once here, so the next channel resolves it again even if no request was waiting.
			let stale = match &outcome {
				Ok(ChannelEnd::Refused(code)) => code == "wrong_worker" || code == "transport",
				Ok(ChannelEnd::Unreachable(_) | ChannelEnd::Closed) | Err(_) => true,
			};
			if stale && let Err(error) = self.inner.topology.invalidate_endpoint(&endpoint).await {
				tracing::warn!(%error, "failed to invalidate worker endpoint");
			}
		}
		if let Err(error) = outcome {
			tracing::debug!(worker = %key.0.0, ?error, "channel to a peer ended; the next request opens another");
		}
	}

	async fn run_channel(
		&self,
		endpoint: &Worker,
		slot: usize,
		connection: &Weak<Connection>,
		body: crate::pubsub::stream::HttpBody,
	) -> Result<ChannelEnd> {
		let request = rivet_env::capabilities::http::Request::builder()
			.method("POST")
			.version(axum::http::Version::HTTP_2)
			.uri(format!(
				"{}{PATH}",
				endpoint.api_peer_address.trim_end_matches('/')
			))
			.header("content-type", "application/octet-stream")
			// The slot picks the request connection; a small and a large channel share each one.
			.extension(rivet_env::capabilities::http::PeerRoute {
				lane: rivet_env::capabilities::http::PeerLane::Request,
				key: slot as u64,
			})
			.body(body)?;
		// A peer that never answers must not hold every request on this slot until its own
		// deadline, so opening shares the bound of one request.
		let clock = self.env().clock();
		let open_deadline =
			rivet_env::deadline_after(clock.as_ref(), self.inner.options.request_timeout)?;
		let opened = rivet_env::timeout_at(
			clock.as_ref(),
			open_deadline,
			self.inner.client.execute_stream(request),
		)
		.await
		.map_err(|error| anyhow::Error::from(error).context("peer channel did not open in time"))
		.and_then(|result| result);
		let response = match opened {
			Ok(response) => response,
			Err(error) => {
				crate::metrics::PEER_CONNECTION_ERRORS
					.with_label_values(&[
						rivet_env::capabilities::http::PeerLane::Request.as_str(),
						crate::pubsub::http::connection_error_reason(&error),
					])
					.inc();
				return Ok(ChannelEnd::Unreachable(error));
			}
		};
		if response.status() != axum::http::StatusCode::OK {
			let error = crate::pubsub::http::status_error(response.status());
			return Ok(ChannelEnd::Refused(
				rivet_error::RivetError::extract(&error).code().to_owned(),
			));
		}
		match connection.upgrade() {
			Some(open) => open.state.lock().opened(),
			None => return Ok(ChannelEnd::Closed),
		}
		let mut reader = Reader::new(response.into_body());
		let max_frame = self
			.inner
			.options
			.max_message_bytes
			.saturating_add(frame::ENVELOPE_BYTES);
		loop {
			let bytes = match reader.frame(max_frame).await {
				Ok(bytes) => bytes,
				Err(_) => return Ok(ChannelEnd::Closed),
			};
			let Frame::ChannelResponse(ChannelResponse { id, body }) = frame::decode(bytes)? else {
				anyhow::bail!("channel peer sent a frame other than a response");
			};
			let Some(open) = connection.upgrade() else {
				return Ok(ChannelEnd::Closed);
			};
			let waiter = open.state.lock().respond(id);
			if let Some(waiter) = waiter {
				let result = match body {
					ChannelBody::ChannelPayload(payload) => Ok(payload),
					ChannelBody::ChannelError(error) => Err(frame::error_from_code(&error.code)),
				};
				let _ = waiter.send(result);
			}
		}
	}
}

/// Why a request did not enter a channel's write queue.
enum QueueRefusal {
	/// The channel ended first; the request may take its replacement.
	Closed,
	/// The request cannot be sent, for the reason this error gives.
	Failed(anyhow::Error),
}

enum ChannelEnd {
	/// The stream ended or the channel was dropped.
	Closed,
	/// The channel's HTTP request failed before response headers; dispatch may be uncertain.
	Unreachable(anyhow::Error),
	/// The peer refused the channel, with this UPS error code, before reading any request.
	Refused(String),
}

/// Counts one open channel on `side` until it ends.
pub(super) struct ChannelsOpenGuard(
	#[allow(dead_code, reason = "held for its drop")] rivet_metrics::IntGaugeGuard,
);

impl ChannelsOpenGuard {
	pub(super) fn new(side: &'static str) -> Self {
		Self(rivet_metrics::GaugeGuardExt::inc_guard(
			&crate::metrics::CHANNELS_OPEN.with_label_values(&[side]),
		))
	}
}
