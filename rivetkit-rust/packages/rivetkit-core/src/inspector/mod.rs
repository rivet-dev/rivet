use anyhow::Result;
use futures::future::BoxFuture;
use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

pub(crate) mod protocol;

type WorkflowHistoryCallback =
	Arc<dyn Fn() -> BoxFuture<'static, Result<Option<Vec<u8>>>> + Send + Sync>;
type WorkflowReplayCallback =
	Arc<dyn Fn(Option<String>) -> BoxFuture<'static, Result<Option<Vec<u8>>>> + Send + Sync>;
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
	listeners: std::sync::RwLock<Vec<(u64, InspectorListener)>>,
	get_workflow_history: Option<WorkflowHistoryCallback>,
	replay_workflow: Option<WorkflowReplayCallback>,
}

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
			.field("get_workflow_history", &self.get_workflow_history.is_some())
			.field("replay_workflow", &self.replay_workflow.is_some())
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
			listeners: std::sync::RwLock::new(Vec::new()),
			get_workflow_history: None,
			replay_workflow: None,
		}
	}
}

impl Drop for InspectorSubscription {
	fn drop(&mut self) {
		let Some(inspector) = self.inspector.upgrade() else {
			return;
		};
		let connected_clients = {
			let mut listeners = match inspector.listeners.write() {
				Ok(listeners) => listeners,
				Err(poisoned) => poisoned.into_inner(),
			};
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

	pub fn with_workflow_callbacks(
		get_workflow_history: Option<WorkflowHistoryCallback>,
		replay_workflow: Option<WorkflowReplayCallback>,
	) -> Self {
		Self(Arc::new(InspectorInner {
			get_workflow_history,
			replay_workflow,
			..InspectorInner::default()
		}))
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

	pub fn is_workflow_enabled(&self) -> bool {
		self.0.get_workflow_history.is_some()
	}

	pub async fn get_workflow_history(&self) -> Result<Option<Vec<u8>>> {
		let Some(callback) = &self.0.get_workflow_history else {
			return Ok(None);
		};
		callback().await
	}

	pub async fn replay_workflow(&self, entry_id: Option<String>) -> Result<Option<Vec<u8>>> {
		let Some(callback) = &self.0.replay_workflow else {
			return Ok(None);
		};
		callback(entry_id).await
	}

	pub(crate) fn subscribe(&self, listener: InspectorListener) -> InspectorSubscription {
		let listener_id = self.0.next_listener_id.fetch_add(1, Ordering::SeqCst);
		let connected_clients = {
			let mut listeners = match self.0.listeners.write() {
				Ok(listeners) => listeners,
				Err(poisoned) => poisoned.into_inner(),
			};
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

	#[allow(dead_code)]
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
			let listeners = match self.0.listeners.read() {
				Ok(listeners) => listeners,
				Err(poisoned) => poisoned.into_inner(),
			};
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

#[cfg(test)]
#[path = "../../tests/modules/inspector.rs"]
mod tests;
