use std::time::Duration;

use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Exact-subject worker requests and streams.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PubSub {
	/// Default and maximum request budget, including resolution, transit, and queueing.
	pub request_timeout_ms: Option<u64>,
	pub max_message_bytes: Option<usize>,
	/// Pending requests per subject; a full queue rejects new requests immediately.
	pub request_subscription_capacity: Option<usize>,
	/// Concurrent HTTP request header reads (default 2048). Held only until the header decodes,
	/// then the request moves to its class or subject budget.
	pub request_header_capacity: Option<usize>,
	/// Per-class request caps. Every class has a defined cap; omitted values use the defaults.
	pub classes: Option<RequestClasses>,
	/// Maximum concurrently reserved request subjects with isolated capacity.
	pub max_request_subjects: Option<usize>,
	/// Buffered packets per stream direction; full buffers apply backpressure.
	pub stream_buffer_capacity: Option<usize>,
	/// Peer connection sets. Unary requests travel on peer channels over request connections, and
	/// long-lived streams use separate stream connections, so a stalled stream cannot hold up
	/// requests.
	#[serde(default)]
	pub peer_transport: PeerTransport,
}

/// Connections and flow-control windows per peer. Omitted values use the defaults.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PeerTransport {
	/// Connections per peer for unary requests, each carrying one peer channel (default 4).
	pub request_connections: Option<usize>,
	/// Connections per peer for long-lived streams such as tunnels (default 2).
	pub stream_connections: Option<usize>,
	/// Peer channels this worker may open across all remote workers (default 4096).
	pub outgoing_channel_capacity: Option<usize>,
	/// Peer channels this worker may serve across all remote workers (default 4096).
	pub incoming_channel_capacity: Option<usize>,
	/// TCP connections the internal HTTP listener may serve at once (default 8192).
	pub server_connection_capacity: Option<usize>,
	/// HTTP/2 connection window of a request connection (default 16 MiB).
	pub request_connection_window_bytes: Option<u32>,
	/// HTTP/2 stream window this worker grants a peer channel's replies (default 4 MiB). A
	/// worker's server grants each incoming stream a fixed 4 MiB window.
	pub request_stream_window_bytes: Option<u32>,
	/// HTTP/2 connection window of a stream connection (default 32 MiB).
	pub stream_connection_window_bytes: Option<u32>,
	/// HTTP/2 stream window of a long-lived stream (default 1 MiB).
	pub stream_stream_window_bytes: Option<u32>,
	/// Frames a peer channel queues for writing; a full queue refuses the request (default 8192).
	pub channel_queue_frames: Option<usize>,
	/// Bytes of requests a peer channel queues for writing; a request that would exceed it is
	/// refused, though an empty queue always takes one (default 64 MiB).
	pub channel_queue_bytes: Option<usize>,
	/// Bytes of replies one served peer channel queues for writing. A channel at its cap refuses
	/// new requests as overloaded until its caller reads or the queued replies expire
	/// (default 64 MiB).
	pub channel_reply_queue_bytes: Option<usize>,
	/// Bytes of replies queued across every served peer channel of this worker. At the cap every
	/// channel refuses new requests as overloaded (default 1 GiB).
	pub reply_queue_bytes: Option<usize>,
	/// Requests with a larger payload use a peer's large-request channels, so they never queue
	/// ahead of small requests on one stream (default 64 KiB).
	pub channel_large_request_bytes: Option<usize>,
	/// Most bytes a peer channel writes at once from its queue (default 256 KiB).
	pub channel_max_write_bytes: Option<usize>,
}

/// Per-class request caps. storage classes travel on reserved subjects whose window is their
/// incoming cap, so they take only an outgoing cap across every target.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestClasses {
	/// storage log shard requests (default outgoing 8192).
	#[serde(rename = "storage_log")]
	pub depot_log: Option<SubjectClassCapacity>,
	/// storage volume lifecycle and filesystem requests (default outgoing 4096).
	#[serde(rename = "storage_volume")]
	pub depot_volume: Option<SubjectClassCapacity>,
	/// storage KV commands (default outgoing 8192).
	#[serde(rename = "storage_kv_command")]
	pub depot_kv_command: Option<SubjectClassCapacity>,
	/// storage partition coordinator requests (default outgoing 1024).
	#[serde(rename = "storage_coordinator")]
	pub depot_coordinator: Option<SubjectClassCapacity>,
	/// storage stop-readiness checks (default outgoing 1024).
	#[serde(rename = "storage_stop_readiness")]
	pub depot_stop_readiness: Option<SubjectClassCapacity>,
	/// Actor commands to node connections (default 4096 each way).
	#[serde(rename = "node_command")]
	pub envoy_command: Option<ProcessClassCapacity>,
	/// Gateway HTTP tunnels (default 4096 each way).
	pub gateway_http: Option<ProcessClassCapacity>,
	/// Gateway WebSocket tunnels, held for the socket's life (default 65536 each way).
	pub gateway_websocket: Option<ProcessClassCapacity>,
	/// Actor start notices to waiting gateway workers (default 4096 each way).
	pub actor_start_notice: Option<ProcessClassCapacity>,
}

/// The outgoing cap of a subject-scoped class, across every target.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubjectClassCapacity {
	pub outgoing_capacity: Option<usize>,
}

/// The process-wide caps of a class that travels on unreserved subjects.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessClassCapacity {
	pub outgoing_capacity: Option<usize>,
	pub incoming_capacity: Option<usize>,
}

/// Resolved caps of a process-scoped class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessClassLimits {
	pub outgoing: usize,
	pub incoming: usize,
}

/// Resolved per-class caps. Subject-scoped classes carry only their outgoing cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassCapacities {
	pub depot_log_outgoing: usize,
	pub depot_volume_outgoing: usize,
	pub depot_kv_command_outgoing: usize,
	pub depot_coordinator_outgoing: usize,
	pub depot_stop_readiness_outgoing: usize,
	pub envoy_command: ProcessClassLimits,
	pub gateway_http: ProcessClassLimits,
	pub gateway_websocket: ProcessClassLimits,
	pub actor_start_notice: ProcessClassLimits,
}

impl Default for ClassCapacities {
	fn default() -> Self {
		Self {
			depot_log_outgoing: 8192,
			depot_volume_outgoing: 4096,
			depot_kv_command_outgoing: 8192,
			depot_coordinator_outgoing: 1024,
			depot_stop_readiness_outgoing: 1024,
			envoy_command: ProcessClassLimits {
				outgoing: 4096,
				incoming: 4096,
			},
			gateway_http: ProcessClassLimits {
				outgoing: 4096,
				incoming: 4096,
			},
			gateway_websocket: ProcessClassLimits {
				outgoing: 65536,
				incoming: 65536,
			},
			actor_start_notice: ProcessClassLimits {
				outgoing: 4096,
				incoming: 4096,
			},
		}
	}
}

impl ClassCapacities {
	fn values(&self) -> [usize; 13] {
		[
			self.depot_log_outgoing,
			self.depot_volume_outgoing,
			self.depot_kv_command_outgoing,
			self.depot_coordinator_outgoing,
			self.depot_stop_readiness_outgoing,
			self.envoy_command.outgoing,
			self.envoy_command.incoming,
			self.gateway_http.outgoing,
			self.gateway_http.incoming,
			self.gateway_websocket.outgoing,
			self.gateway_websocket.incoming,
			self.actor_start_notice.outgoing,
			self.actor_start_notice.incoming,
		]
	}
}

/// Resolved UPS settings. Config loading and direct UPS construction validate the
/// same values, and all defaults are defined here.
#[derive(Clone, Debug)]
pub struct Options {
	/// Default and maximum request budget; a timeout cannot undo an operation.
	pub request_timeout: Duration,
	pub max_message_bytes: usize,
	pub request_subscription_capacity: usize,
	/// Shared header-reader budget, released once the header decodes. Admission never waits.
	pub request_header_capacity: usize,
	pub classes: ClassCapacities,
	pub max_request_subjects: usize,
	pub stream_buffer_capacity: usize,
	pub channel: ChannelOptions,
}

/// The peer channels that carry every unary request to a remote worker. See `PeerTransport`.
#[derive(Clone, Debug)]
pub struct ChannelOptions {
	/// Channels per peer, one on each request connection. Follows
	/// `peer_transport.request_connections`.
	pub channels: usize,
	pub outgoing_capacity: usize,
	pub incoming_capacity: usize,
	pub queue_frames: usize,
	pub queue_bytes: usize,
	pub reply_queue_bytes: usize,
	pub total_reply_queue_bytes: usize,
	pub large_request_bytes: usize,
	pub max_write_bytes: usize,
}

impl Default for ChannelOptions {
	fn default() -> Self {
		Self {
			channels: 4,
			outgoing_capacity: 4096,
			incoming_capacity: 4096,
			queue_frames: 8192,
			queue_bytes: 64 * 1024 * 1024,
			reply_queue_bytes: 64 * 1024 * 1024,
			total_reply_queue_bytes: 1024 * 1024 * 1024,
			large_request_bytes: 64 * 1024,
			max_write_bytes: 256 * 1024,
		}
	}
}

impl Default for Options {
	fn default() -> Self {
		Self {
			request_timeout: Duration::from_secs(30),
			max_message_bytes: 16 * 1024 * 1024,
			request_subscription_capacity: 256,
			request_header_capacity: 2048,
			classes: ClassCapacities::default(),
			max_request_subjects: 1024,
			stream_buffer_capacity: 16,
			channel: ChannelOptions::default(),
		}
	}
}

impl Options {
	pub fn validate(&self) -> Result<()> {
		ensure!(
			!self.request_timeout.is_zero() && self.request_timeout <= Duration::from_secs(300),
			"invalid pubsub.request_timeout_ms"
		);
		ensure!(
			self.max_message_bytes > 0 && self.max_message_bytes <= 64 * 1024 * 1024,
			"invalid pubsub.max_message_bytes"
		);
		ensure!(
			self.request_subscription_capacity > 0
				&& self.request_subscription_capacity <= tokio::sync::Semaphore::MAX_PERMITS,
			"invalid pubsub.request_subscription_capacity"
		);
		ensure!(
			self.request_header_capacity > 0
				&& self.request_header_capacity <= tokio::sync::Semaphore::MAX_PERMITS,
			"invalid pubsub.request_header_capacity"
		);
		ensure!(
			self.classes
				.values()
				.iter()
				.all(|value| *value > 0 && *value <= tokio::sync::Semaphore::MAX_PERMITS),
			"invalid pubsub.classes capacity"
		);
		ensure!(
			self.stream_buffer_capacity > 0
				&& self.stream_buffer_capacity <= tokio::sync::Semaphore::MAX_PERMITS,
			"invalid pubsub.stream_buffer_capacity"
		);
		ensure!(
			self.max_request_subjects > 0 && self.max_request_subjects <= 65_536,
			"invalid pubsub.max_request_subjects"
		);
		ensure!(
			self.channel.channels > 0
				&& self.channel.outgoing_capacity > 0
				&& self.channel.outgoing_capacity <= tokio::sync::Semaphore::MAX_PERMITS
				&& self.channel.incoming_capacity > 0
				&& self.channel.incoming_capacity <= tokio::sync::Semaphore::MAX_PERMITS
				&& self.channel.queue_frames > 0
				&& self.channel.queue_bytes > 0
				&& self.channel.reply_queue_bytes > 0
				&& self.channel.total_reply_queue_bytes > 0
				&& self.channel.large_request_bytes > 0
				&& self.channel.max_write_bytes > 0,
			"invalid pubsub.peer_transport channel bounds"
		);
		Ok(())
	}
}

impl PubSub {
	pub fn options(&self) -> Options {
		let defaults = Options::default();
		Options {
			request_timeout: self
				.request_timeout_ms
				.map(Duration::from_millis)
				.unwrap_or(defaults.request_timeout),
			max_message_bytes: self.max_message_bytes.unwrap_or(defaults.max_message_bytes),
			request_subscription_capacity: self
				.request_subscription_capacity
				.unwrap_or(defaults.request_subscription_capacity),
			request_header_capacity: self
				.request_header_capacity
				.unwrap_or(defaults.request_header_capacity),
			classes: self
				.classes
				.clone()
				.unwrap_or_default()
				.resolve(defaults.classes),
			max_request_subjects: self
				.max_request_subjects
				.unwrap_or(defaults.max_request_subjects),
			stream_buffer_capacity: self
				.stream_buffer_capacity
				.unwrap_or(defaults.stream_buffer_capacity),
			channel: ChannelOptions {
				channels: self
					.peer_transport
					.request_connections
					.unwrap_or(defaults.channel.channels),
				outgoing_capacity: self
					.peer_transport
					.outgoing_channel_capacity
					.unwrap_or(defaults.channel.outgoing_capacity),
				incoming_capacity: self
					.peer_transport
					.incoming_channel_capacity
					.unwrap_or(defaults.channel.incoming_capacity),
				queue_frames: self
					.peer_transport
					.channel_queue_frames
					.unwrap_or(defaults.channel.queue_frames),
				queue_bytes: self
					.peer_transport
					.channel_queue_bytes
					.unwrap_or(defaults.channel.queue_bytes),
				reply_queue_bytes: self
					.peer_transport
					.channel_reply_queue_bytes
					.unwrap_or(defaults.channel.reply_queue_bytes),
				total_reply_queue_bytes: self
					.peer_transport
					.reply_queue_bytes
					.unwrap_or(defaults.channel.total_reply_queue_bytes),
				large_request_bytes: self
					.peer_transport
					.channel_large_request_bytes
					.unwrap_or(defaults.channel.large_request_bytes),
				max_write_bytes: self
					.peer_transport
					.channel_max_write_bytes
					.unwrap_or(defaults.channel.max_write_bytes),
			},
		}
	}

	pub fn validate(&self) -> Result<()> {
		self.options().validate()?;
		ensure!(
			self.peer_transport
				.server_connection_capacity
				.unwrap_or(8192)
				> 0 && self
				.peer_transport
				.server_connection_capacity
				.unwrap_or(8192)
				<= tokio::sync::Semaphore::MAX_PERMITS,
			"invalid pubsub.peer_transport.server_connection_capacity"
		);
		Ok(())
	}
}

impl RequestClasses {
	fn resolve(&self, defaults: ClassCapacities) -> ClassCapacities {
		let subject = |class: &Option<SubjectClassCapacity>, default: usize| {
			class
				.as_ref()
				.and_then(|class| class.outgoing_capacity)
				.unwrap_or(default)
		};
		let process = |class: &Option<ProcessClassCapacity>, default: ProcessClassLimits| {
			ProcessClassLimits {
				outgoing: class
					.as_ref()
					.and_then(|class| class.outgoing_capacity)
					.unwrap_or(default.outgoing),
				incoming: class
					.as_ref()
					.and_then(|class| class.incoming_capacity)
					.unwrap_or(default.incoming),
			}
		};

		ClassCapacities {
			depot_log_outgoing: subject(&self.depot_log, defaults.depot_log_outgoing),
			depot_volume_outgoing: subject(&self.depot_volume, defaults.depot_volume_outgoing),
			depot_kv_command_outgoing: subject(
				&self.depot_kv_command,
				defaults.depot_kv_command_outgoing,
			),
			depot_coordinator_outgoing: subject(
				&self.depot_coordinator,
				defaults.depot_coordinator_outgoing,
			),
			depot_stop_readiness_outgoing: subject(
				&self.depot_stop_readiness,
				defaults.depot_stop_readiness_outgoing,
			),
			envoy_command: process(&self.envoy_command, defaults.envoy_command),
			gateway_http: process(&self.gateway_http, defaults.gateway_http),
			gateway_websocket: process(&self.gateway_websocket, defaults.gateway_websocket),
			actor_start_notice: process(&self.actor_start_notice, defaults.actor_start_notice),
		}
	}
}
