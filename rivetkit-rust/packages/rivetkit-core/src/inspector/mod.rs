use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

use parking_lot::RwLock;

pub mod auth;
pub(crate) mod protocol;

pub use auth::{InspectorAuth, init_inspector_token};

type InspectorListener = Arc<dyn Fn(InspectorSignal) + Send + Sync>;

#[derive(Clone, Debug, Default)]
pub struct Inspector(Arc<InspectorInner>);

struct InspectorInner {
	state_revision: AtomicU64,
	connections_revision: AtomicU64,
	queue_revision: AtomicU64,
	active_connections: AtomicU32,
	queue_size: AtomicU32,
	connected_clients: AtomicUsize,
	next_listener_id: AtomicU64,
	// Forced-sync: subscriptions are created/dropped from sync paths and
	// listener callbacks are cloned before invocation.
	listeners: RwLock<Vec<(u64, InspectorListener)>>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectorSignal {
	StateUpdated,
	ConnectionsUpdated,
	QueueUpdated,
	WorkflowHistoryUpdated,
}

pub(crate) struct InspectorSubscription {
	inspector: Weak<InspectorInner>,
	listener_id: u64,
}

impl std::fmt::Debug for InspectorInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("InspectorInner")
			.field(
				"state_revision",
				&self.state_revision.load(Ordering::SeqCst),
			)
			.field(
				"connections_revision",
				&self.connections_revision.load(Ordering::SeqCst),
			)
			.field(
				"queue_revision",
				&self.queue_revision.load(Ordering::SeqCst),
			)
			.field(
				"active_connections",
				&self.active_connections.load(Ordering::SeqCst),
			)
			.field("queue_size", &self.queue_size.load(Ordering::SeqCst))
			.field(
				"connected_clients",
				&self.connected_clients.load(Ordering::SeqCst),
			)
			.finish()
	}
}

impl Default for InspectorInner {
	fn default() -> Self {
		Self {
			state_revision: AtomicU64::new(0),
			connections_revision: AtomicU64::new(0),
			queue_revision: AtomicU64::new(0),
			active_connections: AtomicU32::new(0),
			queue_size: AtomicU32::new(0),
			connected_clients: AtomicUsize::new(0),
			next_listener_id: AtomicU64::new(1),
			listeners: RwLock::new(Vec::new()),
		}
	}
}

impl Drop for InspectorSubscription {
	fn drop(&mut self) {
		let Some(inspector) = self.inspector.upgrade() else {
			return;
		};
		let connected_clients = {
			let mut listeners = inspector.listeners.write();
			listeners.retain(|(listener_id, _)| *listener_id != self.listener_id);
			listeners.len()
		};
		inspector
			.connected_clients
			.store(connected_clients, Ordering::SeqCst);
	}
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

	pub(crate) fn subscribe(&self, listener: InspectorListener) -> InspectorSubscription {
		let listener_id = self.0.next_listener_id.fetch_add(1, Ordering::SeqCst);
		let connected_clients = {
			let mut listeners = self.0.listeners.write();
			listeners.push((listener_id, listener));
			listeners.len()
		};
		self.set_connected_clients(connected_clients);

		InspectorSubscription {
			inspector: Arc::downgrade(&self.0),
			listener_id,
		}
	}

	pub(crate) fn record_state_updated(&self) {
		self.0.state_revision.fetch_add(1, Ordering::SeqCst);
		self.notify(InspectorSignal::StateUpdated);
	}

	pub(crate) fn record_connections_updated(&self, active_connections: u32) {
		self.0
			.active_connections
			.store(active_connections, Ordering::SeqCst);
		self.0.connections_revision.fetch_add(1, Ordering::SeqCst);
		self.notify(InspectorSignal::ConnectionsUpdated);
	}

	pub(crate) fn record_queue_updated(&self, queue_size: u32) {
		self.0.queue_size.store(queue_size, Ordering::SeqCst);
		self.0.queue_revision.fetch_add(1, Ordering::SeqCst);
		self.notify(InspectorSignal::QueueUpdated);
	}

	pub(crate) fn record_workflow_history_updated(&self) {
		self.notify(InspectorSignal::WorkflowHistoryUpdated);
	}

	pub(crate) fn set_connected_clients(&self, connected_clients: usize) {
		self.0
			.connected_clients
			.store(connected_clients, Ordering::SeqCst);
	}

	fn notify(&self, signal: InspectorSignal) {
		if self.0.connected_clients.load(Ordering::SeqCst) == 0 {
			return;
		}

		let listeners = {
			let listeners = self.0.listeners.read();
			listeners
				.iter()
				.map(|(_, listener)| listener.clone())
				.collect::<Vec<_>>()
		};

		for listener in listeners {
			listener(signal);
		}
	}
}

pub fn decode_request_payload(payload: &[u8], advertised_version: u16) -> anyhow::Result<Vec<u8>> {
	let message = protocol::decode_client_payload(payload, advertised_version)?;
	protocol::encode_client_payload_current(&message)
}

pub fn encode_response_payload(payload: &[u8], target_version: u16) -> anyhow::Result<Vec<u8>> {
	let message = protocol::decode_current_server_payload(payload)?;
	protocol::encode_server_payload(&message, target_version)
}

#[cfg(test)]
#[path = "../../tests/modules/inspector.rs"]
mod tests;
