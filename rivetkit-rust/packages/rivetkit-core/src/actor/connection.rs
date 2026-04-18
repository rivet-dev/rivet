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
use crate::actor::metrics::ActorMetrics;
use crate::actor::persist::{
	decode_with_embedded_version, encode_with_embedded_version,
};
use crate::kv::Kv;
use crate::types::ListOpts;
use crate::types::ConnId;

pub(crate) type EventSendCallback =
	Arc<dyn Fn(OutgoingEvent) -> Result<()> + Send + Sync>;
pub(crate) type DisconnectCallback =
	Arc<dyn Fn(Option<String>) -> BoxFuture<'static, Result<()>> + Send + Sync>;

const CONNECTION_KEY_PREFIX: &[u8] = &[2];
const CONNECTION_PERSIST_VERSION: u16 = 4;
const CONNECTION_PERSIST_COMPATIBLE_VERSIONS: &[u16] = &[3, 4];

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

pub(crate) fn encode_persisted_connection(
	connection: &PersistedConnection,
) -> Result<Vec<u8>> {
	encode_with_embedded_version(
		connection,
		CONNECTION_PERSIST_VERSION,
		"persisted connection",
	)
}

pub(crate) fn decode_persisted_connection(
	payload: &[u8],
) -> Result<PersistedConnection> {
	decode_with_embedded_version(
		payload,
		CONNECTION_PERSIST_COMPATIBLE_VERSIONS,
		"persisted connection",
	)
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
	metrics: ActorMetrics,
}

impl ConnectionManager {
	pub(crate) fn new(
		actor_id: impl Into<String>,
		kv: Kv,
		config: ActorConfig,
		metrics: ActorMetrics,
	) -> Self {
		Self(Arc::new(ConnectionManagerInner {
			_actor_id: actor_id.into(),
			kv,
			config: RwLock::new(config),
			callbacks: RwLock::new(Arc::new(ActorInstanceCallbacks::default())),
			connections: RwLock::new(BTreeMap::new()),
			metrics,
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

	pub(crate) fn active_count(&self) -> u32 {
		self
			.0
			.connections
			.read()
			.expect("connection manager connections lock poisoned")
			.len()
			.try_into()
			.unwrap_or(u32::MAX)
	}

	pub(crate) fn insert_existing(&self, conn: ConnHandle) {
		let active_count = {
			let mut connections = self
				.0
				.connections
				.write()
				.expect("connection manager connections lock poisoned");
			connections.insert(conn.id().to_owned(), conn);
			connections.len()
		};
		self.0.metrics.set_active_connections(active_count);
	}

	pub(crate) fn remove_existing(&self, conn_id: &str) -> Option<ConnHandle> {
		let (removed, active_count) = {
			let mut connections = self
				.0
				.connections
				.write()
				.expect("connection manager connections lock poisoned");
			let removed = connections.remove(conn_id);
			(removed, connections.len())
		};
		self.0.metrics.set_active_connections(active_count);
		removed
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
		self.0.metrics.inc_connections_total();

		Ok(conn)
	}

	pub(crate) async fn persist_hibernatable(&self) -> Result<()> {
		for conn in self.list() {
			let Some(persisted) = conn.persisted() else {
				continue;
			};

			let encoded = encode_persisted_connection(&persisted)
				.context("encode persisted connection")?;
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
			match decode_persisted_connection(&value) {
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

		ctx.record_connections_updated();
		ctx.reset_sleep_timer();
		Ok(())
	}
}

impl Default for ConnectionManager {
	fn default() -> Self {
		Self::new(
			"",
			Kv::default(),
			ActorConfig::default(),
			ActorMetrics::default(),
		)
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
#[path = "../../tests/modules/connection.rs"]
mod tests;
