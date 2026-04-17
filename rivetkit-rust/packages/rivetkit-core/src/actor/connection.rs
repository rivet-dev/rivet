use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::{Result, anyhow};
use futures::future::BoxFuture;

use crate::types::ConnId;

pub(crate) type EventSendCallback =
	Arc<dyn Fn(OutgoingEvent) -> Result<()> + Send + Sync>;
pub(crate) type DisconnectCallback =
	Arc<dyn Fn(Option<String>) -> BoxFuture<'static, Result<()>> + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutgoingEvent {
	pub name: String,
	pub args: Vec<u8>,
}

#[derive(Clone)]
pub struct ConnHandle(Arc<ConnHandleInner>);

struct ConnHandleInner {
	id: ConnId,
	params: Vec<u8>,
	state: RwLock<Vec<u8>>,
	is_hibernatable: bool,
	subscriptions: RwLock<BTreeSet<String>>,
	event_sender: RwLock<Option<EventSendCallback>>,
	disconnect_handler: RwLock<Option<DisconnectCallback>>,
}

impl ConnHandle {
	pub fn new(
		id: impl Into<ConnId>,
		params: Vec<u8>,
		state: Vec<u8>,
		is_hibernatable: bool,
	) -> Self {
		Self(Arc::new(ConnHandleInner {
			id: id.into(),
			params,
			state: RwLock::new(state),
			is_hibernatable,
			subscriptions: RwLock::new(BTreeSet::new()),
			event_sender: RwLock::new(None),
			disconnect_handler: RwLock::new(None),
		}))
	}

	pub fn id(&self) -> &str {
		&self.0.id
	}

	pub fn params(&self) -> Vec<u8> {
		self.0.params.clone()
	}

	pub fn state(&self) -> Vec<u8> {
		self.0
			.state
			.read()
			.expect("connection state lock poisoned")
			.clone()
	}

	pub fn set_state(&self, state: Vec<u8>) {
		*self
			.0
			.state
			.write()
			.expect("connection state lock poisoned") = state;
	}

	pub fn is_hibernatable(&self) -> bool {
		self.0.is_hibernatable
	}

	pub fn send(&self, name: &str, args: &[u8]) {
		if let Err(error) = self.try_send(name, args) {
			tracing::error!(
				?error,
				conn_id = self.id(),
				event_name = name,
				"failed to send event to connection"
			);
		}
	}

	pub async fn disconnect(&self, reason: Option<&str>) -> Result<()> {
		let handler = self.disconnect_handler()?;
		handler(reason.map(str::to_owned)).await
	}

	#[allow(dead_code)]
	pub(crate) fn configure_event_sender(
		&self,
		event_sender: Option<EventSendCallback>,
	) {
		*self
			.0
			.event_sender
			.write()
			.expect("connection event sender lock poisoned") = event_sender;
	}

	#[allow(dead_code)]
	pub(crate) fn configure_disconnect_handler(
		&self,
		disconnect_handler: Option<DisconnectCallback>,
	) {
		*self
			.0
			.disconnect_handler
			.write()
			.expect("connection disconnect handler lock poisoned") =
			disconnect_handler;
	}

	#[allow(dead_code)]
	pub(crate) fn subscribe(&self, event_name: impl Into<String>) -> bool {
		self.0
			.subscriptions
			.write()
			.expect("connection subscriptions lock poisoned")
			.insert(event_name.into())
	}

	#[allow(dead_code)]
	pub(crate) fn unsubscribe(&self, event_name: &str) -> bool {
		self.0
			.subscriptions
			.write()
			.expect("connection subscriptions lock poisoned")
			.remove(event_name)
	}

	pub(crate) fn is_subscribed(&self, event_name: &str) -> bool {
		self.0
			.subscriptions
			.read()
			.expect("connection subscriptions lock poisoned")
			.contains(event_name)
	}

	pub(crate) fn subscriptions(&self) -> Vec<String> {
		self.0
			.subscriptions
			.read()
			.expect("connection subscriptions lock poisoned")
			.iter()
			.cloned()
			.collect()
	}

	#[allow(dead_code)]
	pub(crate) fn clear_subscriptions(&self) {
		self.0
			.subscriptions
			.write()
			.expect("connection subscriptions lock poisoned")
			.clear();
	}

	pub(crate) fn try_send(&self, name: &str, args: &[u8]) -> Result<()> {
		let event_sender = self.event_sender()?;
		event_sender(OutgoingEvent {
			name: name.to_owned(),
			args: args.to_vec(),
		})
	}

	fn event_sender(&self) -> Result<EventSendCallback> {
		self.0
			.event_sender
			.read()
			.expect("connection event sender lock poisoned")
			.clone()
			.ok_or_else(|| anyhow!("connection event sender is not configured"))
	}

	fn disconnect_handler(&self) -> Result<DisconnectCallback> {
		self.0
			.disconnect_handler
			.read()
			.expect("connection disconnect handler lock poisoned")
			.clone()
			.ok_or_else(|| anyhow!("connection disconnect handler is not configured"))
	}
}

impl Default for ConnHandle {
	fn default() -> Self {
		Self::new("", Vec::new(), Vec::new(), false)
	}
}

impl fmt::Debug for ConnHandle {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ConnHandle")
			.field("id", &self.0.id)
			.field("is_hibernatable", &self.0.is_hibernatable)
			.field("subscriptions", &self.subscriptions())
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;
	use std::sync::Mutex;

	use anyhow::Result;

	use super::{ConnHandle, EventSendCallback, OutgoingEvent};

	#[test]
	fn send_uses_configured_event_sender() {
		let sent = Arc::new(Mutex::new(Vec::<OutgoingEvent>::new()));
		let sent_clone = sent.clone();
		let conn = ConnHandle::new("conn-1", b"params".to_vec(), b"state".to_vec(), true);
		let sender: EventSendCallback = Arc::new(move |event| {
			sent_clone
				.lock()
				.expect("sent events lock poisoned")
				.push(event);
			Ok(())
		});

		conn.configure_event_sender(Some(sender));
		conn.send("updated", b"payload");

		assert_eq!(
			*sent.lock().expect("sent events lock poisoned"),
			vec![OutgoingEvent {
				name: "updated".to_owned(),
				args: b"payload".to_vec(),
			}]
		);
		assert_eq!(conn.params(), b"params");
		assert_eq!(conn.state(), b"state");
		assert!(conn.is_hibernatable());
	}

	#[tokio::test]
	async fn disconnect_returns_configuration_error_without_handler() {
		let conn = ConnHandle::default();
		let error = conn
			.disconnect(None)
			.await
			.expect_err("disconnect should fail without a handler");

		assert!(
			error
				.to_string()
				.contains("connection disconnect handler is not configured")
		);
	}

	#[tokio::test]
	async fn disconnect_uses_configured_handler() -> Result<()> {
		let conn = ConnHandle::new("conn-1", Vec::new(), Vec::new(), false);
		conn.configure_disconnect_handler(Some(Arc::new(|reason| {
			Box::pin(async move {
				assert_eq!(reason.as_deref(), Some("bye"));
				Ok(())
			})
		})));

		conn.disconnect(Some("bye")).await
	}
}
