use parking_lot::Mutex;
pub(crate) mod channel;
pub(super) mod class;
pub(super) mod http;
mod lifecycle;
pub(super) mod order;
mod peer_deadline;
pub(super) mod request;
pub(super) mod stream;
pub(super) mod subject;

use std::{
	collections::BTreeMap,
	sync::{Arc, Weak},
};

use anyhow::Result;
use rivet_topology::topology::Topology;
use uuid::Uuid;

use self::{
	request::{ClassLimits, RequestLimit},
	stream::StreamRegistration,
};
use crate::errors;

pub use rivet_topology::worker::WorkerId;

pub use rivet_config::config::pubsub::Options;

#[derive(Clone)]
pub struct PubSub {
	pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
	pub(crate) env: Arc<dyn rivet_env::Env>,

	pub(crate) worker_id: WorkerId,
	pub(crate) instance_id: Uuid,
	pub(super) topology: Arc<Topology>,
	pub(super) client: Arc<dyn rivet_env::capabilities::http::HttpClient>,
	lifecycle: Arc<lifecycle::Lifecycle>,
	/// Header reads before a request's class is known. Released once the header decodes.
	header: RequestLimit,
	classes: ClassLimits,
	subject_usage: Arc<crate::metrics::Buffer>,
	subject_limits: Mutex<BTreeMap<Vec<u8>, Weak<subject::SubjectLimits>>>,
	pub(super) options: Options,
	subscriptions: Mutex<BTreeMap<Vec<u8>, StreamRegistration>>,
	/// Spreads remote requests over the peer connections of their lane.
	pub(super) next_route_key: std::sync::atomic::AtomicU64,
	/// Open channels to peers and the tasks serving this worker's own.
	pub(super) channels: channel::client::Connections,
	/// Turns on every ordered lane this process sends on.
	order_lanes: order::OrderLanes,
}

impl PubSub {
	pub fn env(&self) -> &Arc<dyn rivet_env::Env> {
		&self.inner.env
	}

	/// Uses shared worker topology and a shared internal HTTP client. The client must
	/// reject redirects and support full-duplex HTTP/2 without a whole-body timeout.
	pub fn new(
		env: Arc<dyn rivet_env::Env>,
		worker_id: WorkerId,
		instance_id: Uuid,
		topology: Arc<Topology>,
		client: Arc<dyn rivet_env::capabilities::http::HttpClient>,
		options: Options,
	) -> Result<Self> {
		options.validate()?;
		let channels = channel::client::Connections::new(&env, &options.channel);

		Ok(Self {
			inner: Arc::new(Inner {
				env,
				worker_id,
				instance_id,
				topology,
				client,
				header: RequestLimit::new("request_header", options.request_header_capacity),
				classes: ClassLimits::new(&options.classes),
				subject_usage: crate::metrics::Buffer::new(
					"request_subjects",
					options.max_request_subjects,
				),
				subject_limits: Mutex::new(BTreeMap::new()),
				options,
				subscriptions: Mutex::new(BTreeMap::new()),
				next_route_key: std::sync::atomic::AtomicU64::new(0),
				channels,
				order_lanes: order::OrderLanes::new(),
				lifecycle: Arc::new(lifecycle::Lifecycle::new()),
			}),
		})
	}

	/// The worker's shared topology, which also selects the versions its writers encode.
	pub fn topology(&self) -> &Arc<Topology> {
		&self.inner.topology
	}

	/// Returns the worker identity shared by this process's services.
	/// Topology generates a fresh worker ID at each process startup.
	pub fn worker_id(&self) -> &WorkerId {
		&self.inner.worker_id
	}

	/// Returns the separate process incarnation advertised by the worker heartbeat.
	pub fn instance_id(&self) -> Uuid {
		self.inner.instance_id
	}

	/// Takes the next turn on the ordered lane to `subject` at `worker`. Requests sent with
	/// tickets from one lane reach the subscriber in the order the tickets were taken, so take it
	/// synchronously at the caller's own ordering point. See [`order::OrderTicket`].
	pub fn order_ticket(&self, worker: &WorkerId, subject: impl AsRef<[u8]>) -> order::OrderTicket {
		self.inner.order_lanes.ticket(worker, subject.as_ref())
	}

	/// Maximum encoded payload accepted by a request or stream packet.
	pub fn max_message_bytes(&self) -> usize {
		self.inner.options.max_message_bytes
	}

	pub(crate) fn validate(&self, subject: &[u8], payload: &[u8]) -> Result<()> {
		if subject.is_empty()
			|| subject.len() > 1024
			|| payload.len() > self.inner.options.max_message_bytes
		{
			return Err(errors::InvalidMessage.build());
		}

		Ok(())
	}
}
