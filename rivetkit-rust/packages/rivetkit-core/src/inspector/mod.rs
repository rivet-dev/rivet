use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

#[derive(Clone, Debug, Default)]
pub struct Inspector(Arc<InspectorInner>);

#[derive(Debug, Default)]
struct InspectorInner {
	state_revision: AtomicU64,
	connections_revision: AtomicU64,
	queue_revision: AtomicU64,
	active_connections: AtomicU32,
	queue_size: AtomicU32,
	connected_clients: AtomicUsize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InspectorSnapshot {
	pub state_revision: u64,
	pub connections_revision: u64,
	pub queue_revision: u64,
	pub active_connections: u32,
	pub queue_size: u32,
	pub connected_clients: usize,
}

impl Inspector {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn snapshot(&self) -> InspectorSnapshot {
		InspectorSnapshot {
			state_revision: self.0.state_revision.load(Ordering::SeqCst),
			connections_revision: self.0.connections_revision.load(Ordering::SeqCst),
			queue_revision: self.0.queue_revision.load(Ordering::SeqCst),
			active_connections: self.0.active_connections.load(Ordering::SeqCst),
			queue_size: self.0.queue_size.load(Ordering::SeqCst),
			connected_clients: self.0.connected_clients.load(Ordering::SeqCst),
		}
	}

	pub(crate) fn record_state_updated(&self) {
		self.0.state_revision.fetch_add(1, Ordering::SeqCst);
	}

	pub(crate) fn record_connections_updated(&self, active_connections: u32) {
		self
			.0
			.active_connections
			.store(active_connections, Ordering::SeqCst);
		self
			.0
			.connections_revision
			.fetch_add(1, Ordering::SeqCst);
	}

	pub(crate) fn record_queue_updated(&self, queue_size: u32) {
		self.0.queue_size.store(queue_size, Ordering::SeqCst);
		self.0.queue_revision.fetch_add(1, Ordering::SeqCst);
	}

	#[allow(dead_code)]
	pub(crate) fn set_connected_clients(&self, connected_clients: usize) {
		self
			.0
			.connected_clients
			.store(connected_clients, Ordering::SeqCst);
	}
}

#[cfg(test)]
#[path = "../../tests/modules/inspector.rs"]
mod tests;
