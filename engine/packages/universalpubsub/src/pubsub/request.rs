use std::{sync::Arc, time::Duration};

use anyhow::Result;
use bytes::Bytes;
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::{
	PubSub, WorkerId,
	class::RequestClass,
	order::OrderTicket,
	stream::{PacketSender, StreamOptions, StreamSubscriber},
};
use crate::{
	errors,
	metrics::{Buffer, BufferUsage},
};

impl PubSub {
	pub fn subscribe_request(&self, subject: impl AsRef<[u8]>) -> Result<RequestSubscriber> {
		Ok(RequestSubscriber {
			streams: self.subscribe_stream(subject.as_ref())?,
			input_timeout: self.inner.options.request_timeout,
			clock: self.env().clock(),
			pending: FuturesUnordered::new(),
			input_capacity: self.subject_queue_capacity(subject.as_ref()),
			closed: false,
		})
	}

	pub async fn request(
		&self,
		class: RequestClass,
		worker: &WorkerId,
		subject: impl AsRef<[u8]>,
		payload: &[u8],
	) -> Result<Bytes> {
		self.request_with_timeout(
			class,
			worker,
			subject,
			payload,
			self.inner.options.request_timeout,
		)
		.await
	}

	/// Requires exactly one response packet and a clean end. The budget includes
	/// resolution, opening, queues, and reply reads. Requests are never replayed.
	pub async fn request_with_timeout(
		&self,
		class: RequestClass,
		worker: &WorkerId,
		subject: impl AsRef<[u8]>,
		payload: &[u8],
		timeout: Duration,
	) -> Result<Bytes> {
		self.request_with_timeout_guarded(class, worker, subject, payload, timeout, &|| true)
			.await
	}

	/// Evaluates caller authority again after endpoint resolution and immediately before local/HTTP
	/// dispatch. Rejection proves this request was not sent; it cannot fence a previously sent mutation.
	pub async fn request_with_timeout_guarded(
		&self,
		class: RequestClass,
		worker: &WorkerId,
		subject: impl AsRef<[u8]>,
		payload: &[u8],
		timeout: Duration,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
	) -> Result<Bytes> {
		self.guarded_request(
			class,
			worker,
			subject.as_ref(),
			payload,
			timeout,
			may_dispatch,
			None,
		)
		.await
	}

	/// As [`Self::request_with_timeout_guarded`], on the ordered lane `ticket` was taken from:
	/// the subscriber receives this request after every request with an earlier ticket on the
	/// lane that was sent. The ticket must come from [`PubSub::order_ticket`] for the same worker
	/// and subject.
	#[allow(clippy::too_many_arguments)]
	pub async fn request_ordered(
		&self,
		class: RequestClass,
		worker: &WorkerId,
		subject: impl AsRef<[u8]>,
		payload: &[u8],
		timeout: Duration,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
		ticket: OrderTicket,
	) -> Result<Bytes> {
		anyhow::ensure!(
			ticket.is_for(worker, subject.as_ref()),
			"an order ticket was used for a request to another lane"
		);
		self.guarded_request(
			class,
			worker,
			subject.as_ref(),
			payload,
			timeout,
			may_dispatch,
			Some(ticket),
		)
		.await
	}

	#[allow(clippy::too_many_arguments)]
	async fn guarded_request(
		&self,
		class: RequestClass,
		worker: &WorkerId,
		subject: &[u8],
		payload: &[u8],
		timeout: Duration,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
		ticket: Option<OrderTicket>,
	) -> Result<Bytes> {
		if !may_dispatch() {
			return Err(errors::DispatchRejected.build());
		}
		self.validate(subject, payload)?;
		let mut operation = crate::metrics::Operation::new("request");
		let timeout = timeout.min(self.inner.options.request_timeout);
		let result = if worker == self.worker_id() {
			self.local_request(class, subject, payload, timeout, may_dispatch, ticket)
				.await
		} else {
			self.remote_request(
				class,
				worker,
				subject,
				payload,
				timeout,
				may_dispatch,
				ticket,
			)
			.await
		};
		operation.finish(&result);
		if let Err(error) = &result {
			let error = rivet_error::RivetError::extract(error);
			let outcome = match error.code() {
				"timeout" => "timeout",
				"no_responders" => "no_responders",
				"unknown_worker" => "unknown_worker",
				"overloaded" => "overloaded",
				_ => "other_error",
			};
			crate::metrics::OPERATIONS
				.with_label_values(&["request_outcome", outcome])
				.inc();
		}
		result
	}
}

impl PubSub {
	/// Dispatches a request to a subscriber in this process, as a stream with one input packet.
	/// An ordered request waits for its turn, and the in-process dispatch inside `open_stream`
	/// completes before it passes the turn.
	async fn local_request(
		&self,
		class: RequestClass,
		subject: &[u8],
		payload: &[u8],
		timeout: Duration,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
		mut ticket: Option<OrderTicket>,
	) -> Result<Bytes> {
		let clock = self.env().clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), timeout)?;
		if let Some(ticket) = &ticket {
			ticket.turn(clock.as_ref(), deadline).await?;
		}
		let stream = self
			.open_stream(
				self.worker_id(),
				subject,
				StreamOptions {
					class,
					lifetime: Some(timeout),
				},
				timeout,
				Some(deadline),
				Some(payload),
				may_dispatch,
			)
			.await;
		if let Some(ticket) = &mut ticket {
			ticket.pass();
		}
		let (_, receiver) = stream?.split();
		receiver.receive_one().await
	}

	/// Sends a request to a remote worker over one of its peer channels.
	#[allow(clippy::too_many_arguments)]
	async fn remote_request(
		&self,
		class: RequestClass,
		worker: &WorkerId,
		subject: &[u8],
		payload: &[u8],
		timeout: Duration,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
		ticket: Option<OrderTicket>,
	) -> Result<Bytes> {
		let clock = self.env().clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), timeout)?;
		// The request holds its class and subject budget until it is answered, as a stream does.
		let _outgoing = self.outgoing_permit(class, subject)?;
		crate::metrics::REQUESTS
			.with_label_values(&["remote", class.as_str()])
			.inc();
		let endpoint = rivet_env::timeout_at(
			clock.as_ref(),
			deadline,
			self.inner.topology.resolve_worker(worker),
		)
		.await
		.map_err(timeout_error)?
		.map_err(endpoint_unresolved)?
		.ok_or_else(|| errors::UnknownWorker.build())?;
		check_peer_versions(&endpoint)?;
		self.channel_request(
			class,
			worker,
			&endpoint,
			subject,
			payload,
			deadline,
			may_dispatch,
			ticket,
		)
		.await
	}
}

pub struct RequestSubscriber {
	streams: StreamSubscriber,
	input_timeout: Duration,
	clock: Arc<dyn rivet_env::Clock>,
	pending: FuturesUnordered<BoxFuture<'static, Option<RequestMessage>>>,
	input_capacity: usize,
	closed: bool,
}

impl RequestSubscriber {
	/// Cancelling this wait preserves accepted inputs and their deadlines. A request whose input
	/// is already complete when it arrives, as every unary request's is, is delivered at once, so
	/// such requests leave in the order they were dispatched to this subject and an ordered lane
	/// keeps its order. Other inputs are read concurrently, so an unfinished upload cannot block
	/// ready requests.
	pub async fn next(&mut self) -> Result<Option<RequestMessage>> {
		loop {
			if self.closed && self.pending.is_empty() {
				return Ok(None);
			}
			let can_admit = !self.closed && self.pending.len() < self.input_capacity;
			rivet_env::select_biased!("deliver ready requests before accepting more inputs",
				message = rivet_env::tokio::when(!self.pending.is_empty(), self.pending.next()) => {
					if let Some(Some(message)) = message {
						return Ok(Some(message));
					}
				},
				stream = rivet_env::tokio::when(can_admit, self.streams.next()) => {
					let Some(stream) = stream? else {
						self.closed = true;
						continue;
					};
					let timeout = self.input_timeout;
					let clock = self.clock.clone();
					let mut input = async move {
						let source_worker_id = stream.peer_worker_id().clone();
						let class = stream.class();
						let (reply, input) = stream.split();
						// Unary inputs remain bounded even if the stream has no lifetime limit.
						let result = rivet_env::timeout(clock.as_ref(), timeout, input.receive_one())
							.await
							.map_err(timeout_error)
							.and_then(|result| result);

						match result {
							Ok(payload) => Some(RequestMessage { source_worker_id, class, payload, reply }),
							Err(error) => {
								let _ = reply.fail(&error).await;
								None
							}
						}
					}.boxed();
					// Polling once here delivers a complete input before anything that arrives
					// after it; an incomplete one keeps reading among the pending inputs.
					match futures_util::poll!(&mut input) {
						std::task::Poll::Ready(Some(message)) => return Ok(Some(message)),
						std::task::Poll::Ready(None) => {}
						std::task::Poll::Pending => self.pending.push(input),
					}
				},
			);
		}
	}
}

/// Holds incoming capacity until reply/drop, even after the caller times out.
pub struct RequestMessage {
	/// The immutable caller identity carried by the trusted Engine transport.
	pub source_worker_id: WorkerId,
	class: RequestClass,
	pub payload: Bytes,
	reply: PacketSender,
}

impl RequestMessage {
	/// The class the caller sent this request under.
	pub fn class(&self) -> RequestClass {
		self.class
	}

	/// Receiving-host monotonic deadline propagated by the caller. None denotes an explicitly
	/// unbounded stream; handlers requiring a finite command budget must reject or cap that case.
	pub fn deadline_boot_ms(&self) -> Option<u64> {
		self.reply.deadline_boot_ms()
	}

	pub fn is_request_expired(&self) -> Result<bool> {
		self.reply.is_closed()
	}

	/// Returns a structured transport error when the application cannot decode a request.
	pub async fn fail(self, error: &anyhow::Error) -> Result<()> {
		self.reply.fail(error).await
	}

	pub fn reply(self, payload: &[u8]) -> Result<()> {
		self.reply.reply(payload)
	}
}

pub(crate) fn timeout_error(error: rivet_env::TimeoutError) -> anyhow::Error {
	match error {
		rivet_env::TimeoutError::Elapsed => errors::Timeout.build(),
		rivet_env::TimeoutError::Clock(error) => error.into(),
	}
}

/// A request budget shared by PubSub clones: the header pool, one class and direction, or one
/// reserved subject's window. The permit and its observability guard always travel together.
pub(super) struct RequestLimit {
	capacity: Arc<Semaphore>,
	usage: Arc<Buffer>,
	kind: &'static str,
	class: Option<RequestClass>,
}

impl RequestLimit {
	pub(super) fn new(kind: &'static str, capacity: usize) -> Self {
		Self {
			capacity: Arc::new(Semaphore::new(capacity)),
			usage: Buffer::new(kind, capacity),
			kind,
			class: None,
		}
	}

	/// A budget that belongs to `class`. `class_gauges` adds it to the class in-use and capacity
	/// gauges for `direction`.
	pub(super) fn for_class(
		kind: &'static str,
		capacity: usize,
		class: RequestClass,
		direction: &'static str,
		class_gauges: bool,
	) -> Self {
		Self {
			capacity: Arc::new(Semaphore::new(capacity)),
			usage: Buffer::for_class(kind, capacity, class.as_str(), direction, class_gauges),
			kind,
			class: Some(class),
		}
	}

	pub(super) fn try_acquire(&self) -> Result<RequestPermit> {
		let permit = self.capacity.clone().try_acquire_owned().map_err(|_| {
			crate::metrics::overloaded(self.kind, self.class.map(RequestClass::as_str))
		})?;

		Ok(RequestPermit {
			_subject: None,
			_class: None,
			_usage: self.usage.acquire(1),
			_permit: permit,
		})
	}
}

/// The per-process budget of every request class.
pub(super) struct ClassLimits {
	depot_log_outgoing: RequestLimit,
	depot_volume_outgoing: RequestLimit,
	depot_kv_command_outgoing: RequestLimit,
	depot_coordinator_outgoing: RequestLimit,
	depot_stop_readiness_outgoing: RequestLimit,
	envoy_command_outgoing: RequestLimit,
	envoy_command_incoming: RequestLimit,
	gateway_http_outgoing: RequestLimit,
	gateway_http_incoming: RequestLimit,
	gateway_websocket_outgoing: RequestLimit,
	gateway_websocket_incoming: RequestLimit,
	actor_start_notice_outgoing: RequestLimit,
	actor_start_notice_incoming: RequestLimit,
}

impl ClassLimits {
	pub(super) fn new(capacities: &rivet_config::config::pubsub::ClassCapacities) -> Self {
		let outgoing = |class: RequestClass, capacity: usize| {
			RequestLimit::for_class("class_outgoing", capacity, class, "outgoing", true)
		};
		let incoming = |class: RequestClass, capacity: usize| {
			RequestLimit::for_class("class_incoming", capacity, class, "incoming", true)
		};

		Self {
			depot_log_outgoing: outgoing(RequestClass::DepotLog, capacities.depot_log_outgoing),
			depot_volume_outgoing: outgoing(
				RequestClass::DepotVolume,
				capacities.depot_volume_outgoing,
			),
			depot_kv_command_outgoing: outgoing(
				RequestClass::DepotKvCommand,
				capacities.depot_kv_command_outgoing,
			),
			depot_coordinator_outgoing: outgoing(
				RequestClass::DepotCoordinator,
				capacities.depot_coordinator_outgoing,
			),
			depot_stop_readiness_outgoing: outgoing(
				RequestClass::DepotStopReadiness,
				capacities.depot_stop_readiness_outgoing,
			),
			envoy_command_outgoing: outgoing(
				RequestClass::EnvoyCommand,
				capacities.envoy_command.outgoing,
			),
			envoy_command_incoming: incoming(
				RequestClass::EnvoyCommand,
				capacities.envoy_command.incoming,
			),
			gateway_http_outgoing: outgoing(
				RequestClass::GatewayHttp,
				capacities.gateway_http.outgoing,
			),
			gateway_http_incoming: incoming(
				RequestClass::GatewayHttp,
				capacities.gateway_http.incoming,
			),
			gateway_websocket_outgoing: outgoing(
				RequestClass::GatewayWebSocket,
				capacities.gateway_websocket.outgoing,
			),
			gateway_websocket_incoming: incoming(
				RequestClass::GatewayWebSocket,
				capacities.gateway_websocket.incoming,
			),
			actor_start_notice_outgoing: outgoing(
				RequestClass::ActorStartNotice,
				capacities.actor_start_notice.outgoing,
			),
			actor_start_notice_incoming: incoming(
				RequestClass::ActorStartNotice,
				capacities.actor_start_notice.incoming,
			),
		}
	}

	pub(super) fn outgoing(&self, class: RequestClass) -> &RequestLimit {
		match class {
			RequestClass::DepotLog => &self.depot_log_outgoing,
			RequestClass::DepotVolume => &self.depot_volume_outgoing,
			RequestClass::DepotKvCommand => &self.depot_kv_command_outgoing,
			RequestClass::DepotCoordinator => &self.depot_coordinator_outgoing,
			RequestClass::DepotStopReadiness => &self.depot_stop_readiness_outgoing,
			RequestClass::EnvoyCommand => &self.envoy_command_outgoing,
			RequestClass::GatewayHttp => &self.gateway_http_outgoing,
			RequestClass::GatewayWebSocket => &self.gateway_websocket_outgoing,
			RequestClass::ActorStartNotice => &self.actor_start_notice_outgoing,
		}
	}

	/// Subject-scoped classes have no process incoming cap; their subject window is the cap.
	pub(super) fn incoming(&self, class: RequestClass) -> Option<&RequestLimit> {
		match class {
			RequestClass::DepotLog
			| RequestClass::DepotVolume
			| RequestClass::DepotKvCommand
			| RequestClass::DepotCoordinator
			| RequestClass::DepotStopReadiness => None,
			RequestClass::EnvoyCommand => Some(&self.envoy_command_incoming),
			RequestClass::GatewayHttp => Some(&self.gateway_http_incoming),
			RequestClass::GatewayWebSocket => Some(&self.gateway_websocket_incoming),
			RequestClass::ActorStartNotice => Some(&self.actor_start_notice_incoming),
		}
	}
}

/// Drop accounting before releasing capacity, so a concurrent admission cannot
/// transiently report usage above the configured cap. An outgoing request on a reserved subject
/// holds the subject window here and its class permit in `_class`.
pub(super) struct RequestPermit {
	pub(super) _subject: Option<Arc<super::subject::SubjectLimits>>,
	pub(super) _class: Option<Box<RequestPermit>>,
	_usage: BufferUsage,
	_permit: OwnedSemaphorePermit,
}

/// A topology resolution failure, before anything was sent: a typed refusal the request's
/// edge may retry, instead of an untyped internal error.
pub(crate) fn endpoint_unresolved(error: anyhow::Error) -> anyhow::Error {
	tracing::debug!(
		?error,
		"worker endpoint resolution failed; the request was not sent"
	);
	errors::EndpointUnresolved.build()
}

/// Checks that `endpoint` decodes the UPS frame versions this binary writes. Frames go to one
/// peer, which decodes, acts on and drops them, so the pairwise range decides: each family's
/// version is `min(own maxWrite, peer maxRead)`. This binary encodes only its latest version
/// of each UPS family today, so a peer that reads only older versions gets a
/// `topology.unsupported_version` refusal before anything is sent. UPS never retries it.
/// Once a UPS family gains a second version, encode at the returned version instead.
pub(crate) fn check_peer_versions(endpoint: &rivet_topology::worker::Worker) -> anyhow::Result<()> {
	use rivet_topology::versions::{VersionFamily, for_peer};

	let families = [
		(VersionFamily::UPS_CHANNEL, rivet_data::UPS_CHANNEL_VERSION),
		(VersionFamily::UPS_PACKET, rivet_data::UPS_PACKET_VERSION),
		(VersionFamily::UPS_REQUEST, rivet_data::UPS_REQUEST_VERSION),
	];
	for (family, encoded) in families {
		let own = family
			.info()
			.map(rivet_data::families::FamilyInfo::versions);
		let version = for_peer(own.as_slice(), &endpoint.versions, family)?;
		if version != encoded {
			return Err(rivet_topology::versions::UnsupportedVersion {
				family: family.0,
				peer_min_read: rivet_data::families::advertised(&endpoint.versions, family)
					.map_or(0, |range| range.min_read),
				peer_max_read: version,
				min_write: encoded,
				max_write: encoded,
			}
			.build());
		}
	}
	Ok(())
}


