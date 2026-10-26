mod packet;

use std::{
	sync::{
		Arc, OnceLock, Weak,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

use anyhow::Result;
use bytes::Bytes;
use rivet_topology::worker::WorkerRole;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::{Inner, PubSub, WorkerId, lifecycle::ActiveRequest, request::RequestPermit};
use crate::{
	errors,
	metrics::{Buffer, BufferUsage},
};
pub(crate) use packet::HttpBody;
pub use packet::{PacketReceiver, PacketSender};
pub(super) use packet::{Reader, seal_frame};

/// Opening is bounded by the configured request timeout. Once open, streams have
/// no lifetime limit unless the requester supplies one. A timeout never undoes work.
#[derive(Clone, Copy, Debug)]
pub struct StreamOptions {
	/// The capacity class this stream is admitted under.
	pub class: super::class::RequestClass,
	pub lifetime: Option<Duration>,
}

impl StreamOptions {
	/// A stream of `class` with no lifetime limit.
	pub fn new(class: super::class::RequestClass) -> Self {
		Self {
			class,
			lifetime: None,
		}
	}
}

struct StreamDeadlines {
	deadline: Option<u64>,
	open_deadline: u64,
}

impl StreamDeadlines {
	fn new(
		clock: &dyn rivet_env::Clock,
		options: StreamOptions,
		open_timeout: Duration,
		absolute_deadline: Option<u64>,
	) -> Result<Self> {
		let lifetime_deadline = options
			.lifetime
			.map(|lifetime| {
				anyhow::ensure!(!lifetime.is_zero(), "stream lifetime must be positive");
				rivet_env::deadline_after(clock, lifetime).map_err(anyhow::Error::from)
			})
			.transpose()?;
		let deadline = absolute_deadline.or(lifetime_deadline);
		let open_deadline = rivet_env::deadline_after(clock, open_timeout)?;
		let open_deadline = deadline.map_or(open_deadline, |deadline| deadline.min(open_deadline));

		Ok(Self {
			deadline,
			open_deadline,
		})
	}
}

pub struct DuplexStream {
	peer_worker_id: WorkerId,
	class: super::class::RequestClass,
	sender: PacketSender,
	receiver: PacketReceiver,
}

impl DuplexStream {
	/// The class the stream was opened under, carried in its open header.
	pub fn class(&self) -> super::class::RequestClass {
		self.class
	}

	/// Peer identity supplied by the trusted Engine transport, not an authentication assertion.
	pub fn peer_worker_id(&self) -> &WorkerId {
		&self.peer_worker_id
	}

	pub fn split(self) -> (PacketSender, PacketReceiver) {
		(self.sender, self.receiver)
	}

	pub async fn send(&mut self, packet: impl Into<Bytes>) -> Result<()> {
		self.sender.send(packet).await
	}

	pub async fn recv(&mut self) -> Result<Option<Bytes>> {
		self.receiver.recv().await
	}
}

/// Send input packets, then finish the input and require exactly one response.
pub struct RequestInputStream {
	stream: DuplexStream,
}

impl RequestInputStream {
	pub async fn send(&mut self, packet: impl Into<Bytes>) -> Result<()> {
		self.stream.send(packet).await
	}

	pub async fn finish(self) -> Result<Bytes> {
		let (sender, receiver) = self.stream.split();
		// A handler may reply early and close its input; preserve that response.
		let _ = sender.finish().await;
		receiver.receive_one().await
	}
}

pub(in crate::pubsub) struct StreamControl {
	endpoint: OnceLock<rivet_topology::worker::Worker>,
	topology: Arc<rivet_topology::topology::Topology>,
	wait: packet::Wait,
	max_packet_bytes: usize,
	failed: AtomicBool,
	/// The active-stream gauge and duration timer, or `None` for the in-process side of a
	/// request a peer channel delivered, whose stream the served side already counts.
	metrics: Option<(
		rivet_metrics::IntGaugeGuard,
		rivet_metrics::prometheus::HistogramTimer,
	)>,
	_incoming: Option<Arc<ActiveRequest>>,
	_outgoing: Option<RequestPermit>,
}

impl StreamControl {
	fn new(
		ups: &PubSub,
		deadline: Option<u64>,
		incoming: Option<Arc<ActiveRequest>>,
		outgoing: Option<RequestPermit>,
	) -> Arc<Self> {
		use rivet_metrics::GaugeGuardExt;

		let metrics = (
			crate::metrics::STREAM_ACTIVE.inc_guard(),
			crate::metrics::STREAM_DURATION.start_timer(),
		);
		Self::build(ups, deadline, incoming, outgoing, Some(metrics))
	}

	/// A control that adds nothing to the stream metrics.
	fn uncounted(ups: &PubSub, deadline: Option<u64>) -> Arc<Self> {
		Self::build(ups, deadline, None, None, None)
	}

	fn build(
		ups: &PubSub,
		deadline: Option<u64>,
		incoming: Option<Arc<ActiveRequest>>,
		outgoing: Option<RequestPermit>,
		metrics: Option<(
			rivet_metrics::IntGaugeGuard,
			rivet_metrics::prometheus::HistogramTimer,
		)>,
	) -> Arc<Self> {
		Arc::new(Self {
			endpoint: OnceLock::new(),
			topology: ups.inner.topology.clone(),
			wait: packet::Wait {
				deadline,
				clock: ups.env().clock(),
				stopped: ups.inner.lifecycle.stopped.clone(),
			},
			max_packet_bytes: ups.inner.options.max_message_bytes,
			failed: AtomicBool::new(false),
			metrics,
			_incoming: incoming,
			_outgoing: outgoing,
		})
	}

	async fn handle_opening_error(&self, error: &anyhow::Error) {
		// Only opening timeouts suggest a stale route. Stream lifetime and handler
		// timeouts after headers must not turn healthy traffic into S3 lookups. Even an
		// opening timeout is often overload, so topology keeps a recently confirmed route
		// and the report takes no lookup admission.
		//
		// A storage endpoint is never invalidated on a timeout. Its point read returns the
		// same heartbeat that periodic discovery lists, so a timeout there means the worker
		// is slow, and hiding its route would only force every caller through point reads.
		// A restarted storage worker still surfaces as `wrong_worker` or `transport`, and
		// discovery replaces a moved address on its next refresh.
		if rivet_error::RivetError::extract(error).code() == "timeout"
			&& let Some(endpoint) = self.endpoint.get()
			&& endpoint.role == WorkerRole::Stateless
		{
			self.topology.report_endpoint_timeout(endpoint);
		}

		self.observe_error(Some(error)).await;
	}

	async fn observe_error(&self, error: Option<&anyhow::Error>) {
		if let Some(error) = error {
			self.record_error(error);
		}
		if let (Some(endpoint), Some(error)) = (self.endpoint.get(), error)
			&& matches!(
				rivet_error::RivetError::extract(error).code(),
				"transport" | "wrong_worker"
			) {
			if let Err(error) = self.topology.invalidate_endpoint(endpoint).await {
				tracing::warn!(%error, "failed to invalidate worker endpoint");
			}
		}
	}

	fn record<T>(&self, result: &Result<T>) {
		if let Err(error) = result {
			self.record_error(error);
		}
	}

	fn record_error(&self, error: &anyhow::Error) {
		if self.metrics.is_none() || self.failed.swap(true, Ordering::Relaxed) {
			return;
		}

		let error = rivet_error::RivetError::extract(error);
		let code = match error.code() {
			"timeout" => "timeout",
			"consumer_closed" => "consumer_closed",
			"unexpected_packet" => "unexpected_packet",
			"invalid_message" => "invalid_message",
			_ => "transport_or_protocol",
		};
		crate::metrics::STREAM_ERRORS
			.with_label_values(&[code])
			.inc();
	}
}

impl PubSub {
	pub async fn open_bidirectional_stream(
		&self,
		worker: &WorkerId,
		subject: impl AsRef<[u8]>,
		options: StreamOptions,
	) -> Result<DuplexStream> {
		self.open_stream(
			worker,
			subject.as_ref(),
			options,
			self.inner.options.request_timeout,
			None,
			None,
			&|| true,
		)
		.await
	}

	pub async fn request_with_response_stream(
		&self,
		worker: &WorkerId,
		subject: impl AsRef<[u8]>,
		packet: &[u8],
		options: StreamOptions,
	) -> Result<PacketReceiver> {
		let stream = self
			.open_stream(
				worker,
				subject.as_ref(),
				options,
				self.inner.options.request_timeout,
				None,
				Some(packet),
				&|| true,
			)
			.await?;
		Ok(stream.receiver)
	}

	pub async fn request_with_input_stream(
		&self,
		worker: &WorkerId,
		subject: impl AsRef<[u8]>,
		options: StreamOptions,
	) -> Result<RequestInputStream> {
		Ok(RequestInputStream {
			stream: self
				.open_bidirectional_stream(worker, subject, options)
				.await?,
		})
	}

	pub(super) async fn open_stream(
		&self,
		worker: &WorkerId,
		subject: &[u8],
		options: StreamOptions,
		open_timeout: Duration,
		absolute_deadline: Option<u64>,
		single_packet_input: Option<&[u8]>,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
	) -> Result<DuplexStream> {
		self.validate(subject, single_packet_input.unwrap_or_default())?;
		let clock = self.env().clock();
		let StreamDeadlines {
			deadline,
			open_deadline,
		} = StreamDeadlines::new(clock.as_ref(), options, open_timeout, absolute_deadline)?;

		let outgoing = self.outgoing_permit(options.class, subject)?;
		let control = StreamControl::new(self, deadline, None, Some(outgoing));
		let (mut sender, body) =
			packet::channel(control.clone(), self.inner.options.stream_buffer_capacity);
		if let Some(packet) = single_packet_input {
			// Queue Packet + End together before awaiting response headers. Sending
			// them with the opening header avoids an extra RTT for single-input calls
			// and works even when the server reads the full upload before replying.
			sender.queue_packet_and_end(packet)?;
		}

		let local = worker == self.worker_id();
		crate::metrics::REQUESTS
			.with_label_values(&[
				if local { "local" } else { "remote" },
				options.class.as_str(),
			])
			.inc();
		let response = rivet_env::timeout_at(clock.as_ref(), open_deadline, async {
			if local {
				if !may_dispatch() {
					return Err(errors::DispatchRejected.build());
				}
				let active = self.accept_request(Some((options.class, subject)))?;
				return self.dispatch_stream(
					options.class,
					self.worker_id().clone(),
					subject,
					body,
					deadline,
					Arc::new(active),
				);
			}

			let endpoint = self
				.inner
				.topology
				.resolve_worker(worker)
				.await
				.map_err(super::request::endpoint_unresolved)?
				.ok_or_else(|| errors::UnknownWorker.build())?;
			super::request::check_peer_versions(&endpoint)?;
			let _ = control.endpoint.set(endpoint.clone());
			let now = clock.observe()?;
			let open_remaining_ms = super::peer_deadline::remaining_ms(open_deadline, now);
			let lifetime_remaining_ms =
				deadline.map(|deadline| super::peer_deadline::remaining_ms(deadline, now));
			self.send_stream(
				options.class,
				worker,
				&endpoint,
				subject.to_vec(),
				body,
				open_remaining_ms,
				lifetime_remaining_ms,
				may_dispatch,
			)
			.await
		})
		.await
		.map_err(super::request::timeout_error)
		.and_then(|result| result);

		if let Err(error) = &response {
			control.handle_opening_error(error).await;
		}
		let response = response?;

		Ok(DuplexStream {
			peer_worker_id: worker.clone(),
			class: options.class,
			sender,
			receiver: PacketReceiver::new(response, control),
		})
	}

	pub fn subscribe_stream(&self, subject: impl AsRef<[u8]>) -> Result<StreamSubscriber> {
		let subject = subject.as_ref().to_vec();
		self.validate(&subject, &[])?;
		let mut subscriptions = self.inner.subscriptions.lock();
		if self.inner.lifecycle.draining.load(Ordering::Acquire) {
			return Err(errors::ConsumerClosed.build());
		}
		if subscriptions
			.get(&subject)
			.is_some_and(|registration| !registration.sender.is_closed())
		{
			return Err(errors::SubscriptionExists.build());
		}

		let capacity = self.subject_queue_capacity(&subject);
		let (tx, rx) = mpsc::channel(capacity);
		let id = self.env().rng("ups.subscription_id").uuid_v4();
		let usage = Buffer::request_queue(id, capacity);
		subscriptions.insert(
			subject.clone(),
			StreamRegistration {
				id,
				sender: tx,
				usage,
			},
		);
		Ok(StreamSubscriber {
			rx,
			subject,
			id,
			inner: Arc::downgrade(&self.inner),
		})
	}

	/// `subject` is the decoded class and subject, or `None` before the header is read.
	pub(super) fn accept_request(
		&self,
		subject: Option<(super::class::RequestClass, &[u8])>,
	) -> Result<ActiveRequest> {
		let _subscriptions = self.inner.subscriptions.lock();
		if self.inner.lifecycle.draining.load(Ordering::Acquire) {
			return Err(errors::ConsumerClosed.build());
		}
		let permit = match subject {
			Some((class, subject)) => self.incoming_permit(class, subject)?,
			None => self.header_permit()?,
		};
		Ok(self.inner.lifecycle.accept(permit))
	}

	/// Dispatches one unary request that arrived on a peer channel, exactly as the in-process
	/// path dispatches one: a single input packet, a single reply. The request is in its
	/// subscriber's queue when this returns; the returned receiver yields the reply. `active`
	/// holds its admission; the caller keeps its own reference until the reply is queued.
	pub(in crate::pubsub) fn dispatch_unary(
		&self,
		class: super::class::RequestClass,
		source_worker_id: WorkerId,
		subject: &[u8],
		payload: &[u8],
		deadline: u64,
		active: Arc<ActiveRequest>,
	) -> Result<PacketReceiver> {
		// The served stream that `dispatch_stream` opens counts this request once.
		let control = StreamControl::uncounted(self, Some(deadline));
		let (mut sender, body) =
			packet::channel(control.clone(), self.inner.options.stream_buffer_capacity);
		sender.queue_packet_and_end(payload)?;
		let response = self.dispatch_stream(
			class,
			source_worker_id,
			subject,
			body,
			Some(deadline),
			active,
		)?;
		Ok(PacketReceiver::new(response, control))
	}

	pub(super) fn dispatch_stream(
		&self,
		class: super::class::RequestClass,
		source_worker_id: WorkerId,
		subject: &[u8],
		body: HttpBody,
		deadline: Option<u64>,
		active: Arc<ActiveRequest>,
	) -> Result<HttpBody> {
		let subscriptions = self.inner.subscriptions.lock();
		if self.inner.lifecycle.draining.load(Ordering::Acquire) {
			return Err(errors::ConsumerClosed.build());
		}
		let registration = subscriptions
			.get(subject)
			.ok_or_else(|| errors::NoResponders.build())?;
		let permit = registration
			.sender
			.try_reserve()
			.map_err(|error| match error {
				mpsc::error::TrySendError::Full(_) => {
					crate::metrics::REQUEST_QUEUE_REJECTED_TOTAL.inc();
					crate::metrics::overloaded("request_pending", Some(class.as_str()))
				}
				mpsc::error::TrySendError::Closed(_) => errors::NoResponders.build(),
			})?;
		let control = StreamControl::new(self, deadline, Some(active), None);
		let (sender, response) =
			packet::channel(control.clone(), self.inner.options.stream_buffer_capacity);
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let queued_at = std::time::Instant::now();
		permit.send(QueuedStream {
			stream: DuplexStream {
				peer_worker_id: source_worker_id,
				class,
				sender,
				receiver: PacketReceiver::new(body, control),
			},
			queued_at,
			_usage: registration.usage.acquire(1),
		});
		Ok(response)
	}
}

pub(super) struct StreamRegistration {
	id: Uuid,
	sender: mpsc::Sender<QueuedStream>,
	usage: Arc<Buffer>,
}

struct QueuedStream {
	stream: DuplexStream,
	queued_at: std::time::Instant,
	_usage: BufferUsage,
}

pub struct StreamSubscriber {
	rx: mpsc::Receiver<QueuedStream>,
	subject: Vec<u8>,
	id: Uuid,
	inner: Weak<Inner>,
}

impl StreamSubscriber {
	/// Withdraws this exact subscription while retaining streams already accepted.
	/// Dispatch reserves and queues under this same lock, so no accepted stream
	/// can arrive after the receiver has drained. A replacement is unaffected.
	pub fn close(&mut self) {
		if let Some(inner) = self.inner.upgrade() {
			let mut subscriptions = inner.subscriptions.lock();
			if subscriptions
				.get(&self.subject)
				.is_some_and(|r| r.id == self.id)
			{
				subscriptions.remove(&self.subject);
			}
			self.rx.close();
		} else {
			self.rx.close();
		}
	}

	pub async fn next(&mut self) -> Result<Option<DuplexStream>> {
		while let Some(queued) = self.rx.recv().await {
			crate::metrics::REQUEST_QUEUE_WAIT.observe(queued.queued_at.elapsed().as_secs_f64());
			if queued.stream.sender.is_closed()? {
				crate::metrics::REQUEST_EXPIRED.inc();
				continue;
			}
			return Ok(Some(queued.stream));
		}
		Ok(None)
	}
}

impl Drop for StreamSubscriber {
	fn drop(&mut self) {
		if let Some(inner) = self.inner.upgrade() {
			let mut subscriptions = inner.subscriptions.lock();
			if subscriptions
				.get(&self.subject)
				.is_some_and(|registration| registration.id == self.id)
			{
				subscriptions.remove(&self.subject);
			}
		}
	}
}
