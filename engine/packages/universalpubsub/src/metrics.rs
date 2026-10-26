use std::sync::Arc;

use parking_lot::Mutex;
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard, prometheus::*};
use uuid::Uuid;

/// Shared accounting for bounded queues and byte budgets. Usage guards follow
/// items through enqueue, dequeue, cancellation, and receiver destruction.
pub(crate) struct Buffer {
	kind: &'static str,
	/// The request class this buffer belongs to, for warnings and class gauges.
	class: Option<&'static str>,
	cap: usize,
	depth: Mutex<Depth>,
	depth_gauge: IntGauge,
	/// `ups_class_in_use{class,direction}`, fed by the same accounting as the depth.
	class_gauge: Option<IntGauge>,
	_class_capacity: Option<IntGaugeGuard>,
	near_full: Option<IntGauge>,
	subscription_id: Option<Uuid>,
	_capacity: IntGaugeGuard,
	_queue: Option<IntGaugeGuard>,
}

impl Buffer {
	pub fn new(kind: &'static str, cap: usize) -> Arc<Self> {
		Arc::new(Self {
			kind,
			class: None,
			cap,
			depth: Mutex::new(Depth::default()),
			depth_gauge: crate::metrics::BUFFER_DEPTH.with_label_values(&[kind]),
			class_gauge: None,
			_class_capacity: None,
			near_full: None,
			subscription_id: None,
			_capacity: crate::metrics::BUFFER_CAPACITY
				.with_label_values(&[kind])
				.add_guard(cap as i64),
			_queue: None,
		})
	}

	/// A request budget of one class and direction. It also feeds the class in-use and capacity
	/// gauges, so for Depot classes the incoming series sum every reserved subject window.
	pub fn for_class(
		kind: &'static str,
		cap: usize,
		class: &'static str,
		direction: &'static str,
		class_gauges: bool,
	) -> Arc<Self> {
		let (class_gauge, class_capacity) = if class_gauges {
			(
				Some(crate::metrics::CLASS_IN_USE.with_label_values(&[class, direction])),
				Some(
					crate::metrics::CLASS_CAPACITY
						.with_label_values(&[class, direction])
						.add_guard(cap as i64),
				),
			)
		} else {
			(None, None)
		};
		Arc::new(Self {
			kind,
			class: Some(class),
			cap,
			depth: Mutex::new(Depth::default()),
			depth_gauge: crate::metrics::BUFFER_DEPTH.with_label_values(&[kind]),
			class_gauge,
			_class_capacity: class_capacity,
			near_full: None,
			subscription_id: None,
			_capacity: crate::metrics::BUFFER_CAPACITY
				.with_label_values(&[kind])
				.add_guard(cap as i64),
			_queue: None,
		})
	}

	pub fn request_queue(subscription_id: Uuid, cap: usize) -> Arc<Self> {
		Arc::new(Self {
			kind: "request_pending",
			class: None,
			cap,
			depth: Mutex::new(Depth::default()),
			depth_gauge: crate::metrics::REQUEST_QUEUE_DEPTH.clone(),
			class_gauge: None,
			_class_capacity: None,
			near_full: Some(crate::metrics::REQUEST_QUEUES_NEAR_FULL.clone()),
			subscription_id: Some(subscription_id),
			_capacity: crate::metrics::REQUEST_QUEUE_CAPACITY.add_guard(cap as i64),
			_queue: Some(crate::metrics::REQUEST_QUEUES.inc_guard()),
		})
	}

	fn update(&self, depth: &mut Depth, next: usize) {
		let threshold = near_full_threshold(self.cap);
		if let Some(near_full) = &self.near_full {
			near_full.add(i64::from(next >= threshold) - i64::from(depth.used >= threshold));
		}

		let (alarm, warning) = next_alarm(depth.alarm, next, self.cap);
		depth.alarm = alarm;
		if let Some(warning) = warning {
			if let Some(subscription_id) = self.subscription_id {
				if warning == Alarm::Full {
					tracing::warn!(
						%subscription_id,
						depth = next,
						capacity = self.cap,
						"ups request queue is full; consumer is not keeping up"
					);
				} else {
					tracing::warn!(
						%subscription_id,
						depth = next,
						capacity = self.cap,
						"ups request queue is near full; consumer is not keeping up"
					);
				}
			} else {
				tracing::warn!(
					kind = self.kind,
					class = self.class.unwrap_or("none"),
					depth = next,
					capacity = self.cap,
					"UPS buffer is near full or full; raise its configured cap or reduce load"
				);
			}
		}

		self.depth_gauge.add(next as i64 - depth.used as i64);
		if let Some(class_gauge) = &self.class_gauge {
			class_gauge.add(next as i64 - depth.used as i64);
		}
		depth.used = next;
	}

	pub fn acquire(self: &Arc<Self>, amount: usize) -> BufferUsage {
		let mut depth = self.depth.lock();
		let next = depth.used + amount;
		self.update(&mut depth, next);

		BufferUsage(self.clone(), amount)
	}
}

/// A buffer's usage and the pressure it last warned about.
#[derive(Default)]
struct Depth {
	used: usize,
	alarm: Alarm,
}

/// The highest pressure a buffer has warned about since it last drained to half its cap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Alarm {
	#[default]
	Clear,
	Near,
	Full,
}

/// Depth at which a buffer counts as near full: four fifths of its cap.
fn near_full_threshold(cap: usize) -> usize {
	cap - cap / 5
}

/// Returns the alarm after moving to `next` and the warning to log, if any. A buffer warns
/// once on reaching four fifths of its cap and once on filling, and warns again only after
/// draining below half its cap, so a buffer held at its threshold under steady load logs one
/// warning per episode instead of one per request.
pub(crate) fn next_alarm(alarm: Alarm, next: usize, cap: usize) -> (Alarm, Option<Alarm>) {
	if next >= cap {
		return match alarm {
			Alarm::Full => (Alarm::Full, None),
			Alarm::Clear | Alarm::Near => (Alarm::Full, Some(Alarm::Full)),
		};
	}
	if next >= near_full_threshold(cap) {
		return match alarm {
			Alarm::Clear => (Alarm::Near, Some(Alarm::Near)),
			Alarm::Near | Alarm::Full => (alarm, None),
		};
	}
	if next.saturating_mul(2) < cap {
		return (Alarm::Clear, None);
	}
	(alarm, None)
}



pub(crate) struct BufferUsage(Arc<Buffer>, usize);

impl Drop for BufferUsage {
	fn drop(&mut self) {
		let buffer = &self.0;
		let mut depth = buffer.depth.lock();
		let next = depth.used - self.1;
		buffer.update(&mut depth, next);
	}
}

pub(crate) struct Operation {
	operation: &'static str,
	outcome: &'static str,
	_timer: rivet_metrics::prometheus::HistogramTimer,
}

impl Operation {
	pub fn new(operation: &'static str) -> Self {
		Self {
			operation,
			outcome: "cancelled",
			_timer: crate::metrics::DURATION
				.with_label_values(&[operation])
				.start_timer(),
		}
	}

	pub fn finish<T, E>(&mut self, result: &std::result::Result<T, E>) {
		self.outcome = if result.is_ok() { "success" } else { "error" };
	}
}

impl Drop for Operation {
	fn drop(&mut self) {
		crate::metrics::OPERATIONS
			.with_label_values(&[self.operation, self.outcome])
			.inc();
	}
}

/// Record load shedding without logging each rejected request during an outage. `class` is
/// `None` only before a request's class is known or for capacities no class owns.
pub(crate) fn overloaded(capacity: &'static str, class: Option<&'static str>) -> anyhow::Error {
	crate::metrics::REJECTED
		.with_label_values(&[capacity, class.unwrap_or("none")])
		.inc();
	crate::errors::Overloaded.build()
}

lazy_static::lazy_static! {
	pub static ref ORDERED_TURN_WAIT: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"ups_ordered_lane_turn_wait_seconds",
			"Time an ordered request waited for the requests ahead of it on its lane to be queued or fail before it could enter the channel.",
		)
		.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_FRAMES_PER_WRITE: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"ups_channel_frames_per_write",
			"Frames one peer channel write carried; more than one means frames queued while a write was in flight.",
		)
		.buckets(rivet_metrics::PAGE_COUNT_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_PENDING: IntGauge = register_int_gauge_with_registry!(
		Opts::new("ups_channel_pending", "Requests waiting for a reply on peer channels."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_QUEUE_FULL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("ups_channel_queue_full_total", "Requests refused because a peer channel's write queue was full, by the limit reached: frames or bytes."),
		&["reason"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNELS_OPEN: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("ups_channels_open", "Peer channels this worker holds open, as the caller (client) or the server (server)."),
		&["side"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNELS_CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("ups_channels_capacity", "Configured peer channel admission capacity, as caller or server."),
		&["side"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_ENDS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("ups_channel_ends_total", "Peer channels this worker opened that ended, by reason: refused, unreachable (no answer to the open), closed (the stream ended) or protocol_error."),
		&["reason"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_QUEUE_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new("ups_channel_queue_bytes", "Request bytes waiting in the write queues of this worker's peer channels."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_QUEUE_BYTES_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new("ups_channel_queue_bytes_capacity", "Configured cap on request bytes queued in one peer channel."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_REPLY_QUEUE_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new("ups_channel_reply_queue_bytes", "Reply bytes waiting in the write queues of the peer channels this worker serves."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_REPLY_QUEUE_BYTES_CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("ups_channel_reply_queue_bytes_capacity", "Configured cap on queued reply bytes, for one served peer channel (channel) or across all of them (process)."),
		&["scope"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_ABANDONED_FRAMES: IntCounter = register_int_counter_with_registry!(
		Opts::new("ups_channel_abandoned_frames_total", "Queued request frames a peer channel dropped unwritten because their caller had stopped waiting."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_REPLIES_DROPPED: IntCounter = register_int_counter_with_registry!(
		Opts::new("ups_channel_replies_dropped_total", "Replies a served peer channel dropped unwritten because its caller did not read them before the request deadline."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CHANNEL_CANCELS: IntCounter = register_int_counter_with_registry!(
		Opts::new("ups_channel_cancels_total", "Channel requests whose handler this worker stopped because the caller cancelled."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref STREAM_OPEN_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"ups_stream_open_seconds",
			"Time from sending a remote stream to its response headers, by class.",
		)
		.buckets(rivet_metrics::BUCKETS.to_vec()),
		&["class"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUESTS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"ups_requests_total",
			"Requests and streams opened, by whether the target is this worker or a remote one.",
		),
		&["path", "class"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref PEER_CONNECTION_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"ups_peer_connection_errors_total",
			"Remote requests and streams that failed in the peer transport, by lane and cause.",
		),
		&["lane", "reason"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref STREAM_PACKETS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("ups_stream_packets_total", "Packets accepted for sending or delivered to receivers."),
		&["direction"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref STREAM_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("ups_stream_bytes_total", "Packet payload bytes, excluding framing."),
		&["direction"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref STREAM_ACTIVE: IntGauge = register_int_gauge_with_registry!(
		Opts::new("ups_streams_active", "Open incoming and outgoing stream handles."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref STREAM_DURATION: Histogram = register_histogram_with_registry!(
		HistogramOpts::new("ups_stream_duration_seconds", "Lifetime of incoming and outgoing stream handles."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref STREAM_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("ups_stream_errors_total", "Streams reporting an error, counted once per local stream."),
		&["reason"], *rivet_metrics::REGISTRY
	).unwrap();
pub static ref REJECTED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"ups_rejected_total",
			"Operations shed immediately because the named capacity was exhausted.",
		),
		&["capacity", "class"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CLASS_IN_USE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"ups_class_in_use",
			"Admitted requests and streams holding a request class budget, by direction.",
		),
		&["class", "direction"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref CLASS_CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"ups_class_capacity",
			"Configured request class budget by direction; Depot incoming sums the reserved subject windows.",
		),
		&["class", "direction"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_QUEUE_DEPTH: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"ups_request_queue_depth",
			"Pending requests across local request subscriptions, excluding running handlers.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_QUEUE_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"ups_request_queue_capacity",
			"Total configured pending-request capacity across local request subscriptions.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_QUEUES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"ups_request_queues",
			"Number of local request subscription queues.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_QUEUES_NEAR_FULL: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"ups_request_queues_near_full",
			"Local request queues at or above 80 percent of their configured capacity.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_QUEUE_REJECTED_TOTAL: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"ups_request_queue_rejected_total",
			"Requests rejected because their subscription queue was full.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_ACTIVE: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"ups_requests_active",
			"Admitted body readers and requests awaiting reply or handler release.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_EXPIRED: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"ups_requests_expired_total",
			"Expired requests discarded before delivery to handlers.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_QUEUE_WAIT: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"ups_request_queue_wait_seconds",
			"Time requests spend pending before dequeue.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref OPERATIONS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("ups_operations_total", "UPS operation outcomes."),
		&["operation", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"ups_operation_seconds",
			"UPS operation duration, including waiting and retries.",
		),
		&["operation"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref SHUTDOWN: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("ups_shutdown_total", "Shutdown outcomes."),
		&["result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref BUFFER_DEPTH: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("ups_buffer_depth", "Aggregate buffer usage by kind."),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref BUFFER_CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"ups_buffer_capacity",
			"Aggregate configured buffer caps by kind.",
		),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

}
