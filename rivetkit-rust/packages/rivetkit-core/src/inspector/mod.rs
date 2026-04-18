use anyhow::Result;
use futures::future::BoxFuture;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

type WorkflowHistoryCallback =
	Arc<dyn Fn() -> BoxFuture<'static, Result<Option<Vec<u8>>>> + Send + Sync>;
type WorkflowReplayCallback = Arc<
	dyn Fn(Option<String>) -> BoxFuture<'static, Result<Option<Vec<u8>>>> + Send + Sync,
>;

#[derive(Clone, Debug, Default)]
pub struct Inspector(Arc<InspectorInner>);

#[derive(Default)]
struct InspectorInner {
	state_revision: AtomicU64,
	connections_revision: AtomicU64,
	queue_revision: AtomicU64,
	active_connections: AtomicU32,
	queue_size: AtomicU32,
	connected_clients: AtomicUsize,
	get_workflow_history: Option<WorkflowHistoryCallback>,
	replay_workflow: Option<WorkflowReplayCallback>,
}

impl std::fmt::Debug for InspectorInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("InspectorInner")
			.field("state_revision", &self.state_revision.load(Ordering::SeqCst))
			.field(
				"connections_revision",
				&self.connections_revision.load(Ordering::SeqCst),
			)
			.field("queue_revision", &self.queue_revision.load(Ordering::SeqCst))
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
