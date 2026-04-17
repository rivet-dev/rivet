use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use std::sync::{RwLock, Weak};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use uuid::Uuid;

use crate::actor::callbacks::{
	ActorInstanceCallbacks, OnBeforeConnectRequest, OnConnectRequest,
	OnDisconnectRequest,
};
use crate::actor::config::ActorConfig;
use crate::actor::context::ActorContext;
use crate::kv::Kv;
use crate::types::ListOpts;
use crate::types::ConnId;

pub(crate) type EventSendCallback =
	Arc<dyn Fn(OutgoingEvent) -> Result<()> + Send + Sync>;
pub(crate) type DisconnectCallback =
	Arc<dyn Fn(Option<String>) -> BoxFuture<'static, Result<()>> + Send + Sync>;

const CONNECTION_KEY_PREFIX: &[u8] = &[2];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutgoingEvent {
	pub name: String,
	pub args: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HibernatableConnectionMetadata {
	pub gateway_id: Vec<u8>,
	pub request_id: Vec<u8>,
	pub server_message_index: u16,
	pub client_message_index: u16,
	pub request_path: String,
	pub request_headers: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedSubscription {
	pub event_name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedConnection {
	pub id: String,
	pub parameters: Vec<u8>,
	pub state: Vec<u8>,
	pub subscriptions: Vec<PersistedSubscription>,
	pub gateway_id: Vec<u8>,
	pub request_id: Vec<u8>,
	pub server_message_index: u16,
	pub client_message_index: u16,
	pub request_path: String,
	pub request_headers: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct ConnHandle(Arc<ConnHandleInner>);

struct ConnHandleInner {
	id: ConnId,
	params: Vec<u8>,
	state: RwLock<Vec<u8>>,
	is_hibernatable: bool,
	subscriptions: RwLock<BTreeSet<String>>,
	hibernation: RwLock<Option<HibernatableConnectionMetadata>>,
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
			hibernation: RwLock::new(None),
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

	pub(crate) fn configure_hibernation(
		&self,
		hibernation: Option<HibernatableConnectionMetadata>,
	) {
		*self
			.0
			.hibernation
			.write()
			.expect("connection hibernation lock poisoned") = hibernation;
	}

	pub(crate) fn persisted(&self) -> Option<PersistedConnection> {
		let hibernation = self
			.0
			.hibernation
			.read()
			.expect("connection hibernation lock poisoned")
			.clone()?;

		Some(PersistedConnection {
			id: self.id().to_owned(),
			parameters: self.params(),
			state: self.state(),
			subscriptions: self
				.subscriptions()
				.into_iter()
				.map(|event_name| PersistedSubscription { event_name })
				.collect(),
			gateway_id: hibernation.gateway_id,
			request_id: hibernation.request_id,
			server_message_index: hibernation.server_message_index,
			client_message_index: hibernation.client_message_index,
			request_path: hibernation.request_path,
			request_headers: hibernation.request_headers,
		})
	}

	pub(crate) fn from_persisted(persisted: PersistedConnection) -> Self {
		let conn = Self::new(
			persisted.id.clone(),
			persisted.parameters,
			persisted.state,
			true,
		);
		conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: persisted.gateway_id,
			request_id: persisted.request_id,
			server_message_index: persisted.server_message_index,
			client_message_index: persisted.client_message_index,
			request_path: persisted.request_path,
			request_headers: persisted.request_headers,
		}));
		for subscription in persisted.subscriptions {
			conn.subscribe(subscription.event_name);
		}
		conn
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

#[derive(Clone, Debug)]
pub(crate) struct ConnectionManager(Arc<ConnectionManagerInner>);

#[derive(Debug)]
struct ConnectionManagerInner {
	_actor_id: String,
	kv: Kv,
	config: RwLock<ActorConfig>,
	callbacks: RwLock<Arc<ActorInstanceCallbacks>>,
	connections: RwLock<BTreeMap<ConnId, ConnHandle>>,
}

impl ConnectionManager {
	pub(crate) fn new(
		actor_id: impl Into<String>,
		kv: Kv,
		config: ActorConfig,
	) -> Self {
		Self(Arc::new(ConnectionManagerInner {
			_actor_id: actor_id.into(),
			kv,
			config: RwLock::new(config),
			callbacks: RwLock::new(Arc::new(ActorInstanceCallbacks::default())),
			connections: RwLock::new(BTreeMap::new()),
		}))
	}

	pub(crate) fn configure_runtime(
		&self,
		config: ActorConfig,
		callbacks: Arc<ActorInstanceCallbacks>,
	) {
		*self
			.0
			.config
			.write()
			.expect("connection manager config lock poisoned") = config;
		*self
			.0
			.callbacks
			.write()
			.expect("connection manager callbacks lock poisoned") = callbacks;
	}

	pub(crate) fn list(&self) -> Vec<ConnHandle> {
		self.0
			.connections
			.read()
			.expect("connection manager connections lock poisoned")
			.values()
			.cloned()
			.collect()
	}

	pub(crate) fn insert_existing(&self, conn: ConnHandle) {
		self.0
			.connections
			.write()
			.expect("connection manager connections lock poisoned")
			.insert(conn.id().to_owned(), conn);
	}

	pub(crate) fn remove_existing(&self, conn_id: &str) -> Option<ConnHandle> {
		self.0
			.connections
			.write()
			.expect("connection manager connections lock poisoned")
			.remove(conn_id)
	}

	pub(crate) async fn connect_with_state<F>(
		&self,
		ctx: &ActorContext,
		params: Vec<u8>,
		is_hibernatable: bool,
		hibernation: Option<HibernatableConnectionMetadata>,
		create_state: F,
	) -> Result<ConnHandle>
	where
		F: std::future::Future<Output = Result<Vec<u8>>> + Send,
	{
		let config = self.config();
		let callbacks = self.callbacks();

		self
			.call_on_before_connect(
				&config,
				&callbacks,
				ctx,
				params.clone(),
			)
			.await?;

		let state = timeout(config.create_conn_state_timeout, create_state)
			.await
			.with_context(|| {
				timeout_message(
					"create_conn_state",
					config.create_conn_state_timeout,
				)
			})??;

		let conn = ConnHandle::new(
			Uuid::new_v4().to_string(),
			params,
			state,
			is_hibernatable,
		);
		conn.configure_hibernation(hibernation);
		self.prepare_managed_conn(ctx, &conn);
		self.insert_existing(conn.clone());

		if let Err(error) = self.call_on_connect(&config, &callbacks, ctx, &conn).await {
			self.remove_existing(conn.id());
			return Err(error);
		}

		Ok(conn)
	}

	pub(crate) async fn persist_hibernatable(&self) -> Result<()> {
		for conn in self.list() {
			let Some(persisted) = conn.persisted() else {
				continue;
			};

			let encoded =
				serde_bare::to_vec(&persisted).context("encode persisted connection")?;
			let key = make_connection_key(conn.id());
			self.0
				.kv
				.put(&key, &encoded)
				.await
				.with_context(|| format!("persist connection `{}`", conn.id()))?;
		}

		Ok(())
	}

	pub(crate) async fn restore_persisted(
		&self,
		ctx: &ActorContext,
	) -> Result<Vec<ConnHandle>> {
		let entries = self
			.0
			.kv
			.list_prefix(
				CONNECTION_KEY_PREFIX,
				ListOpts {
					reverse: false,
					limit: None,
				},
			)
			.await?;
		let mut restored = Vec::new();

		for (_key, value) in entries {
			match serde_bare::from_slice::<PersistedConnection>(&value) {
				Ok(persisted) => {
					let conn = ConnHandle::from_persisted(persisted);
					self.prepare_managed_conn(ctx, &conn);
					self.insert_existing(conn.clone());
					restored.push(conn);
				}
				Err(error) => {
					tracing::error!(?error, "failed to decode persisted connection");
				}
			}
		}

		Ok(restored)
	}

	fn prepare_managed_conn(&self, ctx: &ActorContext, conn: &ConnHandle) {
		let manager = Arc::downgrade(&self.0);
		let ctx = ctx.downgrade();
		let conn_id = conn.id().to_owned();

		conn.configure_disconnect_handler(Some(Arc::new(move |reason| {
			let manager = manager.clone();
			let ctx = ctx.clone();
			let conn_id = conn_id.clone();
			Box::pin(async move {
				let manager = ConnectionManager::from_weak(&manager)?;
				let ctx = ActorContext::from_weak(&ctx).ok_or_else(|| {
					anyhow!("actor context is no longer available")
				})?;
				manager.disconnect_managed(&ctx, &conn_id, reason).await
			})
		})));
	}

	fn config(&self) -> ActorConfig {
		self.0
			.config
			.read()
			.expect("connection manager config lock poisoned")
			.clone()
	}

	fn callbacks(&self) -> Arc<ActorInstanceCallbacks> {
		self.0
			.callbacks
			.read()
			.expect("connection manager callbacks lock poisoned")
			.clone()
	}

	fn from_weak(weak: &Weak<ConnectionManagerInner>) -> Result<Self> {
		weak.upgrade()
			.map(Self)
			.ok_or_else(|| anyhow!("connection manager is no longer available"))
	}

	async fn call_on_before_connect(
		&self,
		config: &ActorConfig,
		callbacks: &Arc<ActorInstanceCallbacks>,
		ctx: &ActorContext,
		params: Vec<u8>,
	) -> Result<()> {
		let Some(callback) = &callbacks.on_before_connect else {
			return Ok(());
		};

		timeout(
			config.on_before_connect_timeout,
			callback(OnBeforeConnectRequest {
				ctx: ctx.clone(),
				params,
			}),
		)
		.await
		.with_context(|| {
			timeout_message(
				"on_before_connect",
				config.on_before_connect_timeout,
			)
		})??;

		Ok(())
	}

	async fn call_on_connect(
		&self,
		config: &ActorConfig,
		callbacks: &Arc<ActorInstanceCallbacks>,
		ctx: &ActorContext,
		conn: &ConnHandle,
	) -> Result<()> {
		let Some(callback) = &callbacks.on_connect else {
			return Ok(());
		};

		timeout(
			config.on_connect_timeout,
			callback(OnConnectRequest {
				ctx: ctx.clone(),
				conn: conn.clone(),
			}),
		)
		.await
		.with_context(|| timeout_message("on_connect", config.on_connect_timeout))??;

		Ok(())
	}

	async fn disconnect_managed(
		&self,
		ctx: &ActorContext,
		conn_id: &str,
		reason: Option<String>,
	) -> Result<()> {
		let Some(conn) = self.remove_existing(conn_id) else {
			return Ok(());
		};

		let callbacks = self.callbacks();
		conn.clear_subscriptions();

		if conn.is_hibernatable() {
			let key = make_connection_key(conn.id());
			self.0
				.kv
				.delete(&key)
				.await
				.with_context(|| format!("delete persisted connection `{}`", conn.id()))?;
		}

		if let Some(callback) = &callbacks.on_disconnect {
			ctx.begin_pending_disconnect();
			let result = callback(OnDisconnectRequest {
				ctx: ctx.clone(),
				conn,
			})
			.await
			.with_context(|| disconnect_message(conn_id, reason.as_deref()));
			ctx.end_pending_disconnect();
			result?;
		}

		ctx.reset_sleep_timer();
		Ok(())
	}
}

impl Default for ConnectionManager {
	fn default() -> Self {
		Self::new("", Kv::default(), ActorConfig::default())
	}
}

fn timeout_message(callback_name: &str, timeout: Duration) -> String {
	format!(
		"`{callback_name}` timed out after {} ms",
		timeout.as_millis()
	)
}

fn disconnect_message(conn_id: &str, reason: Option<&str>) -> String {
	match reason {
		Some(reason) => format!("disconnect connection `{conn_id}` with reason `{reason}`"),
		None => format!("disconnect connection `{conn_id}`"),
	}
}

pub(crate) fn make_connection_key(conn_id: &str) -> Vec<u8> {
	let mut key = Vec::with_capacity(CONNECTION_KEY_PREFIX.len() + conn_id.len());
	key.extend_from_slice(CONNECTION_KEY_PREFIX);
	key.extend_from_slice(conn_id.as_bytes());
	key
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;
	use std::sync::Arc;
	use std::sync::Mutex;
	use std::time::Duration;

	use anyhow::Result;
	use tokio::sync::oneshot;
	use tokio::time::sleep;

	use super::{
		ConnHandle, ConnectionManager, EventSendCallback,
		HibernatableConnectionMetadata, OutgoingEvent, PersistedConnection,
		make_connection_key,
	};
	use crate::actor::callbacks::ActorInstanceCallbacks;
	use crate::actor::config::ActorConfig;
	use crate::actor::context::ActorContext;

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

	#[test]
	fn persisted_connection_round_trips_with_bare() {
		let mut headers = BTreeMap::new();
		headers.insert("x-test".to_owned(), "1".to_owned());
		let persisted = PersistedConnection {
			id: "conn-1".to_owned(),
			parameters: vec![1, 2],
			state: vec![3, 4],
			subscriptions: vec![super::PersistedSubscription {
				event_name: "updated".to_owned(),
			}],
			gateway_id: vec![1, 2, 3, 4],
			request_id: vec![5, 6, 7, 8],
			server_message_index: 9,
			client_message_index: 10,
			request_path: "/ws".to_owned(),
			request_headers: headers,
		};

		let encoded =
			serde_bare::to_vec(&persisted).expect("persisted connection should encode");
		let decoded: PersistedConnection =
			serde_bare::from_slice(&encoded).expect("persisted connection should decode");

		assert_eq!(decoded, persisted);
	}

	#[test]
	fn make_connection_key_matches_typescript_layout() {
		assert_eq!(make_connection_key("conn-1"), b"\x02conn-1".to_vec());
	}

	#[tokio::test]
	async fn connect_runs_connection_lifecycle_callbacks() -> Result<()> {
		let ctx = ActorContext::default();
		let manager = ConnectionManager::default();

		let before_called = Arc::new(Mutex::new(false));
		let before_called_clone = before_called.clone();
		let connect_called = Arc::new(Mutex::new(false));
		let connect_called_clone = connect_called.clone();

		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_before_connect = Some(Box::new(move |request| {
			let before_called = before_called_clone.clone();
			Box::pin(async move {
				assert_eq!(request.params, b"params".to_vec());
				*before_called.lock().expect("before connect lock poisoned") = true;
				Ok(())
			})
		}));
		callbacks.on_connect = Some(Box::new(move |request| {
			let connect_called = connect_called_clone.clone();
			Box::pin(async move {
				assert_eq!(request.conn.params(), b"params".to_vec());
				*connect_called.lock().expect("connect lock poisoned") = true;
				Ok(())
			})
		}));

		manager.configure_runtime(ActorConfig::default(), Arc::new(callbacks));
		let conn = manager
			.connect_with_state(
				&ctx,
				b"params".to_vec(),
				false,
				None,
				async { Ok(b"state".to_vec()) },
			)
			.await?;

		assert_eq!(conn.state(), b"state".to_vec());
		assert!(*before_called.lock().expect("before connect lock poisoned"));
		assert!(*connect_called.lock().expect("connect lock poisoned"));
		assert_eq!(manager.list().len(), 1);

		Ok(())
	}

	#[tokio::test]
	async fn connect_honors_callback_and_state_timeouts() {
		let ctx = ActorContext::default();
		let manager = ConnectionManager::default();
		let mut config = ActorConfig::default();
		config.on_before_connect_timeout = Duration::from_millis(10);
		config.create_conn_state_timeout = Duration::from_millis(10);
		config.on_connect_timeout = Duration::from_millis(10);

		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_before_connect = Some(Box::new(|_| {
			Box::pin(async move {
				sleep(Duration::from_millis(50)).await;
				Ok(())
			})
		}));
		manager.configure_runtime(config.clone(), Arc::new(callbacks));

		let error = manager
			.connect_with_state(&ctx, Vec::new(), false, None, async { Ok(Vec::new()) })
			.await
			.expect_err("on_before_connect should time out");
		assert!(error.to_string().contains("`on_before_connect` timed out"));

		let manager = ConnectionManager::default();
		manager.configure_runtime(config.clone(), Arc::new(ActorInstanceCallbacks::default()));
		let error = manager
			.connect_with_state(&ctx, Vec::new(), false, None, async {
				sleep(Duration::from_millis(50)).await;
				Ok(Vec::new())
			})
			.await
			.expect_err("create_conn_state should time out");
		assert!(error.to_string().contains("`create_conn_state` timed out"));

		let manager = ConnectionManager::default();
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_connect = Some(Box::new(|_| {
			Box::pin(async move {
				sleep(Duration::from_millis(50)).await;
				Ok(())
			})
		}));
		manager.configure_runtime(config, Arc::new(callbacks));
		let error = manager
			.connect_with_state(&ctx, Vec::new(), false, None, async { Ok(Vec::new()) })
			.await
			.expect_err("on_connect should time out");
		assert!(error.to_string().contains("`on_connect` timed out"));
	}

	#[tokio::test]
	async fn managed_disconnect_removes_connection_and_clears_subscriptions() -> Result<()> {
		let ctx = ActorContext::default();
		let manager = ConnectionManager::default();
		let (tx, rx) = oneshot::channel::<ConnHandle>();
		let tx = Arc::new(Mutex::new(Some(tx)));
		let tx_clone = tx.clone();

		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_disconnect = Some(Box::new(move |request| {
			let tx = tx_clone.clone();
			Box::pin(async move {
				if let Some(tx) = tx.lock().expect("disconnect sender lock poisoned").take() {
					let _ = tx.send(request.conn.clone());
				}
				Ok(())
			})
		}));
		manager.configure_runtime(ActorConfig::default(), Arc::new(callbacks));

		let conn = manager
			.connect_with_state(
				&ctx,
				b"params".to_vec(),
				false,
				None,
				async { Ok(b"state".to_vec()) },
			)
			.await?;
		conn.subscribe("updated");
		conn.disconnect(Some("bye")).await?;

		let disconnected = rx.await.expect("disconnect callback should receive conn");
		assert!(disconnected.subscriptions().is_empty());
		assert!(manager.list().is_empty());

		Ok(())
	}

	#[test]
	fn restored_connection_keeps_hibernation_metadata() {
		let conn = ConnHandle::new("conn-1", b"params".to_vec(), b"state".to_vec(), true);
		conn.subscribe("updated");
		conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: vec![1, 2, 3, 4],
			request_id: vec![5, 6, 7, 8],
			server_message_index: 9,
			client_message_index: 10,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::from([("x-test".to_owned(), "1".to_owned())]),
		}));

		let persisted = conn.persisted().expect("connection should persist");
		let restored = ConnHandle::from_persisted(persisted.clone());

		assert_eq!(restored.persisted(), Some(persisted));
		assert!(restored.is_subscribed("updated"));
	}
}
