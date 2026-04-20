use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Bound::{Excluded, Unbounded};
use std::sync::Arc;
use std::sync::{Mutex, RwLock, RwLockReadGuard, Weak};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use uuid::Uuid;

use tokio::sync::oneshot;

use crate::actor::callbacks::{ActorEvent, Reply, Request};
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
	transport_disconnect_handler: RwLock<Option<DisconnectCallback>>,
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
			transport_disconnect_handler: RwLock::new(None),
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
		if let Some(handler) = self.transport_disconnect_handler() {
			handler(reason.map(str::to_owned)).await?;
		}
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

	pub(crate) fn configure_transport_disconnect_handler(
		&self,
		disconnect_handler: Option<DisconnectCallback>,
	) {
		*self
			.0
			.transport_disconnect_handler
			.write()
			.expect("connection transport disconnect handler lock poisoned") =
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

	pub(crate) fn hibernation(&self) -> Option<HibernatableConnectionMetadata> {
		self
			.0
			.hibernation
			.read()
			.expect("connection hibernation lock poisoned")
			.clone()
	}

	pub(crate) fn set_server_message_index(
		&self,
		message_index: u16,
	) -> Option<HibernatableConnectionMetadata> {
		let mut hibernation = self
			.0
			.hibernation
			.write()
			.expect("connection hibernation lock poisoned");
		let hibernation = hibernation.as_mut()?;
		hibernation.server_message_index = message_index;
		Some(hibernation.clone())
	}

	pub(crate) fn persisted_with_state(
		&self,
		state: Vec<u8>,
	) -> Option<PersistedConnection> {
		let hibernation = self
			.0
			.hibernation
			.read()
			.expect("connection hibernation lock poisoned")
			.clone()?;

		Some(PersistedConnection {
			id: self.id().to_owned(),
			parameters: self.params(),
			state,
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

	pub(crate) fn managed_disconnect_handler(&self) -> Result<DisconnectCallback> {
		self.disconnect_handler()
	}

	pub(crate) async fn disconnect_transport_only(&self) -> Result<()> {
		let Some(handler) = self.transport_disconnect_handler() else {
			return Ok(());
		};
		handler(None).await
	}

	fn transport_disconnect_handler(&self) -> Option<DisconnectCallback> {
		self.0
			.transport_disconnect_handler
			.read()
			.expect("connection transport disconnect handler lock poisoned")
			.clone()
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
	connections: RwLock<BTreeMap<ConnId, ConnHandle>>,
	pending_hibernation_updates: RwLock<BTreeSet<ConnId>>,
	pending_hibernation_removals: RwLock<BTreeSet<ConnId>>,
	// Serialize disconnect-side connection removal with pending hibernation
	// bookkeeping so persistence snapshots never observe a half-applied state.
	disconnect_state: Mutex<()>,
	metrics: ActorMetrics,
}

#[derive(Default)]
pub(crate) struct PendingHibernationChanges {
	pub updated: BTreeSet<ConnId>,
	pub removed: BTreeSet<ConnId>,
}

/// Lock-backed iterator over live connection handles.
///
/// Do not hold this iterator across `.await`. It keeps a read lock on the
/// connection map until dropped, which blocks writers such as add/remove or
/// connection reconfiguration.
#[must_use = "connection iterators hold a read lock until dropped"]
pub struct ConnHandles<'a> {
	guard: RwLockReadGuard<'a, BTreeMap<ConnId, ConnHandle>>,
	next_after: Option<ConnId>,
}

impl<'a> ConnHandles<'a> {
	fn new(guard: RwLockReadGuard<'a, BTreeMap<ConnId, ConnHandle>>) -> Self {
		Self {
			guard,
			next_after: None,
		}
	}

	pub fn len(&self) -> usize {
		self.guard.len()
	}

	pub fn is_empty(&self) -> bool {
		self.guard.is_empty()
	}
}

impl Iterator for ConnHandles<'_> {
	type Item = ConnHandle;

	fn next(&mut self) -> Option<Self::Item> {
		let (conn_id, conn) = match self.next_after.as_ref() {
			Some(conn_id) => self.guard.range((Excluded(conn_id.clone()), Unbounded)).next()?,
			None => self.guard.iter().next()?,
		};
		self.next_after = Some(conn_id.clone());
		Some(conn.clone())
	}
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
			connections: RwLock::new(BTreeMap::new()),
			pending_hibernation_updates: RwLock::new(BTreeSet::new()),
			pending_hibernation_removals: RwLock::new(BTreeSet::new()),
			disconnect_state: Mutex::new(()),
			metrics,
		}))
	}

	pub(crate) fn configure_runtime(&self, config: ActorConfig) {
		*self
			.0
			.config
			.write()
			.expect("connection manager config lock poisoned") = config;
	}

	pub(crate) fn iter(&self) -> ConnHandles<'_> {
		ConnHandles::new(
			self.0
				.connections
				.read()
				.expect("connection manager connections lock poisoned"),
		)
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

	fn remove_existing_for_disconnect(
		&self,
		conn_id: &str,
	) -> Option<ConnHandle> {
		let _disconnect_state = self
			.0
			.disconnect_state
			.lock()
			.expect("connection disconnect state lock poisoned");
		let (removed, active_count) = {
			let mut connections = self
				.0
				.connections
				.write()
				.expect("connection manager connections lock poisoned");
			let removed = connections.remove(conn_id)?;

			if removed.is_hibernatable() {
				self
					.0
					.pending_hibernation_updates
					.write()
					.expect("pending hibernation updates lock poisoned")
					.remove(conn_id);
				self
					.0
					.pending_hibernation_removals
					.write()
					.expect("pending hibernation removals lock poisoned")
					.insert(conn_id.to_owned());
			}

			(removed, connections.len())
		};
		self.0.metrics.set_active_connections(active_count);
		Some(removed)
	}

	pub(crate) fn queue_hibernation_update(&self, conn_id: impl Into<ConnId>) {
		let _disconnect_state = self
			.0
			.disconnect_state
			.lock()
			.expect("connection disconnect state lock poisoned");
		let conn_id = conn_id.into();
		self
			.0
			.pending_hibernation_updates
			.write()
			.expect("pending hibernation updates lock poisoned")
			.insert(conn_id.clone());
		self
			.0
			.pending_hibernation_removals
			.write()
			.expect("pending hibernation removals lock poisoned")
			.remove(&conn_id);
	}

	pub(crate) fn queue_hibernation_removal(&self, conn_id: impl Into<ConnId>) {
		let _disconnect_state = self
			.0
			.disconnect_state
			.lock()
			.expect("connection disconnect state lock poisoned");
		let conn_id = conn_id.into();
		self
			.0
			.pending_hibernation_updates
			.write()
			.expect("pending hibernation updates lock poisoned")
			.remove(&conn_id);
		self
			.0
			.pending_hibernation_removals
			.write()
			.expect("pending hibernation removals lock poisoned")
			.insert(conn_id);
	}

	pub(crate) fn take_pending_hibernation_changes(
		&self,
	) -> PendingHibernationChanges {
		let _disconnect_state = self
			.0
			.disconnect_state
			.lock()
			.expect("connection disconnect state lock poisoned");
		PendingHibernationChanges {
			updated: std::mem::take(
				&mut *self
					.0
					.pending_hibernation_updates
					.write()
					.expect("pending hibernation updates lock poisoned"),
			),
			removed: std::mem::take(
				&mut *self
					.0
					.pending_hibernation_removals
					.write()
					.expect("pending hibernation removals lock poisoned"),
			),
		}
	}

	pub(crate) fn pending_hibernation_removals(&self) -> Vec<ConnId> {
		let _disconnect_state = self
			.0
			.disconnect_state
			.lock()
			.expect("connection disconnect state lock poisoned");
		self
			.0
			.pending_hibernation_removals
			.read()
			.expect("pending hibernation removals lock poisoned")
			.iter()
			.cloned()
			.collect()
	}

	pub(crate) fn has_pending_hibernation_changes(&self) -> bool {
		let _disconnect_state = self
			.0
			.disconnect_state
			.lock()
			.expect("connection disconnect state lock poisoned");
		let has_updates = !self
			.0
			.pending_hibernation_updates
			.read()
			.expect("pending hibernation updates lock poisoned")
			.is_empty();
		let has_removals = !self
			.0
			.pending_hibernation_removals
			.read()
			.expect("pending hibernation removals lock poisoned")
			.is_empty();
		has_updates || has_removals
	}

	pub(crate) fn restore_pending_hibernation_changes(
		&self,
		pending: PendingHibernationChanges,
	) {
		let _disconnect_state = self
			.0
			.disconnect_state
			.lock()
			.expect("connection disconnect state lock poisoned");
		if !pending.updated.is_empty() {
			self
				.0
				.pending_hibernation_updates
				.write()
				.expect("pending hibernation updates lock poisoned")
				.extend(pending.updated);
		}
		if !pending.removed.is_empty() {
			self
				.0
				.pending_hibernation_removals
				.write()
				.expect("pending hibernation removals lock poisoned")
				.extend(pending.removed);
		}
	}

	pub(crate) async fn connect_with_state<F>(
		&self,
		ctx: &ActorContext,
		params: Vec<u8>,
		is_hibernatable: bool,
		hibernation: Option<HibernatableConnectionMetadata>,
		request: Option<Request>,
		create_state: F,
	) -> Result<ConnHandle>
	where
		F: std::future::Future<Output = Result<Vec<u8>>> + Send,
	{
		let config = self.config();

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
			params.clone(),
			state,
			is_hibernatable,
		);
		conn.configure_hibernation(hibernation);
		self.prepare_managed_conn(ctx, &conn);
		self.insert_existing(conn.clone());

		if let Err(error) =
			self.emit_connection_open(ctx, &conn, params, request).await
		{
			self.remove_existing(conn.id());
			return Err(error);
		}
		self.0.metrics.inc_connections_total();

		Ok(conn)
	}

	pub(crate) fn encode_hibernation_delta(
		&self,
		conn_id: &str,
		bytes: Vec<u8>,
	) -> Result<Vec<u8>> {
		let conn = self
			.connection(conn_id)
			.ok_or_else(|| anyhow!("cannot persist unknown hibernatable connection `{conn_id}`"))?;
		let persisted = conn
			.persisted_with_state(bytes)
			.ok_or_else(|| anyhow!("connection `{conn_id}` is not hibernatable"))?;
		encode_persisted_connection(&persisted).context("encode persisted connection")
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

	pub(crate) fn reconnect_hibernatable(
		&self,
		ctx: &ActorContext,
		gateway_id: &[u8],
		request_id: &[u8],
	) -> Result<ConnHandle> {
		let Some(conn) = self
			.iter()
			.find(|conn| match conn.hibernation() {
				Some(hibernation) => {
					hibernation.gateway_id == gateway_id
						&& hibernation.request_id == request_id
				}
				None => false,
			})
		else {
			return Err(anyhow!(
				"cannot find hibernatable connection for restored websocket"
			));
		};

		ctx.record_connections_updated();
		ctx.notify_activity_dirty_or_reset_sleep_timer();
		Ok(conn)
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

	fn from_weak(weak: &Weak<ConnectionManagerInner>) -> Result<Self> {
		weak.upgrade()
			.map(Self)
			.ok_or_else(|| anyhow!("connection manager is no longer available"))
	}

	async fn disconnect_managed(
		&self,
		ctx: &ActorContext,
		conn_id: &str,
		reason: Option<String>,
	) -> Result<()> {
		let Some(conn) = self.remove_existing_for_disconnect(conn_id) else {
			return Ok(());
		};
		conn.clear_subscriptions();

		ctx
			.try_send_actor_event(
				ActorEvent::ConnectionClosed { conn },
				"connection_closed",
			)
			.with_context(|| disconnect_message(conn_id, reason.as_deref()))?;

		ctx.record_connections_updated();
		ctx.notify_activity_dirty_or_reset_sleep_timer();
		Ok(())
	}

	async fn emit_connection_open(
		&self,
		ctx: &ActorContext,
		conn: &ConnHandle,
		params: Vec<u8>,
		request: Option<Request>,
	) -> Result<()> {
		let config = self.config();
		let (reply_tx, reply_rx) = oneshot::channel();
		ctx.try_send_actor_event(
			ActorEvent::ConnectionOpen {
				conn: conn.clone(),
				params,
				request,
				reply: Reply::from(reply_tx),
			},
			"connection_open",
		)?;
		timeout(config.on_connect_timeout, reply_rx)
			.await
			.with_context(|| timeout_message("connection_open", config.on_connect_timeout))?
			.context("receive connection_open reply")??;
		Ok(())
	}

	pub(crate) fn connection(&self, conn_id: &str) -> Option<ConnHandle> {
		self.0
			.connections
			.read()
			.expect("connection manager connections lock poisoned")
			.get(conn_id)
			.cloned()
	}

	pub(crate) async fn disconnect_transport_only<F>(
		&self,
		ctx: &ActorContext,
		mut predicate: F,
	) -> Result<()>
	where
		F: FnMut(&ConnHandle) -> bool,
	{
		let connections: Vec<_> = self.iter().filter(|conn| predicate(conn)).collect();
		let mut disconnected_ids = Vec::new();
		let mut failures = Vec::new();

		for conn in &connections {
			match conn.disconnect_transport_only().await {
				Ok(()) => disconnected_ids.push(conn.id().to_owned()),
				Err(error) => {
					tracing::error!(
						conn_id = %conn.id(),
						?error,
						"failed transport-only connection disconnect"
					);
					failures.push((conn.id().to_owned(), format!("{error:#}")));
				}
			}
		}

		let mut removed_any = false;
		for conn_id in disconnected_ids {
			let Some(conn) = self.remove_existing_for_disconnect(&conn_id) else {
				continue;
			};
			conn.clear_subscriptions();
			removed_any = true;
		}

		if removed_any {
			ctx.record_connections_updated();
			ctx.notify_activity_dirty_or_reset_sleep_timer();
		}

		if failures.is_empty() {
			return Ok(());
		}

		Err(anyhow!(
			"disconnect transport failed for {} connection(s): {}",
			failures.len(),
			failures
				.into_iter()
				.map(|(conn_id, error)| format!("{conn_id}: {error}"))
				.collect::<Vec<_>>()
				.join("; ")
		))
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
mod tests {
	use std::collections::BTreeSet;
	use std::sync::{Arc, Mutex};
	use std::sync::atomic::{AtomicUsize, Ordering};

	use tokio::sync::{Barrier, mpsc};
	use tokio::task::yield_now;

	use super::{ConnectionManager, HibernatableConnectionMetadata};
	use crate::actor::callbacks::ActorEvent;
	use crate::actor::context::ActorContext;
	use crate::actor::metrics::ActorMetrics;
	use crate::kv::Kv;

	#[tokio::test(start_paused = true)]
	async fn concurrent_disconnects_only_emit_one_close_and_one_hibernation_removal() {
		let ctx = ActorContext::new_with_kv(
			"actor-race",
			"actor",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		);
		let manager = ConnectionManager::new(
			"actor-race",
			Kv::new_in_memory(),
			crate::actor::config::ActorConfig::default(),
			ActorMetrics::default(),
		);
		ctx.configure_connection_runtime(crate::actor::config::ActorConfig::default());
		let (events_tx, mut events_rx) = mpsc::channel(8);
		ctx.configure_actor_events(Some(events_tx));
		let closed = Arc::new(AtomicUsize::new(0));
		let observed_conn_id = Arc::new(Mutex::new(None::<String>));

		let recv = tokio::spawn({
			let closed = closed.clone();
			let observed_conn_id = observed_conn_id.clone();
			async move {
				while let Some(event) = events_rx.recv().await {
					match event {
						ActorEvent::ConnectionOpen { reply, .. } => reply.send(Ok(())),
						ActorEvent::ConnectionClosed { conn } => {
							*observed_conn_id
								.lock()
								.expect("observed connection id lock poisoned") =
								Some(conn.id().to_owned());
							closed.fetch_add(1, Ordering::SeqCst);
							break;
						}
						other => panic!("unexpected event: {other:?}"),
					}
				}
			}
		});

		let conn = manager
			.connect_with_state(
				&ctx,
				vec![1],
				true,
				Some(HibernatableConnectionMetadata {
					gateway_id: vec![1, 2, 3, 4],
					request_id: vec![5, 6, 7, 8],
					..HibernatableConnectionMetadata::default()
				}),
				None,
				async { Ok(vec![9]) },
			)
			.await
			.expect("connection should open");
		let conn_id = conn.id().to_owned();
		ctx.record_connections_updated();
		ctx.notify_activity_dirty_or_reset_sleep_timer();

		let barrier = Arc::new(Barrier::new(2));
		conn.configure_transport_disconnect_handler(Some(Arc::new({
			let barrier = barrier.clone();
			move |_reason| {
				let barrier = barrier.clone();
				Box::pin(async move {
					barrier.wait().await;
					Ok(())
				})
			}
		})));

		let first = tokio::spawn({
			let conn = conn.clone();
			async move { conn.disconnect(Some("first")).await }
		});
		let second = tokio::spawn({
			let conn = conn.clone();
			async move { conn.disconnect(Some("second")).await }
		});

		yield_now().await;
		first
			.await
			.expect("first disconnect task should join")
			.expect("first disconnect should succeed");
		second
			.await
			.expect("second disconnect task should join")
			.expect("second disconnect should succeed");
		recv.await.expect("event receiver should join");

		assert_eq!(closed.load(Ordering::SeqCst), 1);
		assert_eq!(
			observed_conn_id
				.lock()
				.expect("observed connection id lock poisoned")
				.as_deref(),
			Some(conn_id.as_str())
		);
		assert!(manager.connection(&conn_id).is_none());

		let pending = manager.take_pending_hibernation_changes();
		assert!(pending.updated.is_empty());
		assert_eq!(pending.removed, BTreeSet::from([conn_id]));
	}

	#[tokio::test(start_paused = true)]
	async fn remove_existing_for_disconnect_has_exactly_one_winner() {
		let manager = ConnectionManager::new(
			"actor-race",
			Kv::new_in_memory(),
			crate::actor::config::ActorConfig::default(),
			ActorMetrics::default(),
		);
		let conn = super::ConnHandle::new(
			"conn-race",
			vec![1],
			vec![2],
			true,
		);
		conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: vec![1, 2, 3, 4],
			request_id: vec![5, 6, 7, 8],
			..HibernatableConnectionMetadata::default()
		}));
		manager.insert_existing(conn);

		let barrier = Arc::new(Barrier::new(2));
		let first = tokio::spawn({
			let manager = manager.clone();
			let barrier = barrier.clone();
			async move {
				barrier.wait().await;
				manager
					.remove_existing_for_disconnect("conn-race")
					.map(|conn| conn.id().to_owned())
			}
		});
		let second = tokio::spawn({
			let manager = manager.clone();
			let barrier = barrier.clone();
			async move {
				barrier.wait().await;
				manager
					.remove_existing_for_disconnect("conn-race")
					.map(|conn| conn.id().to_owned())
			}
		});

		let first = first.await.expect("first task should join");
		let second = second.await.expect("second task should join");
		let winners = [first, second]
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();

		assert_eq!(winners, vec!["conn-race".to_owned()]);
		assert!(manager.connection("conn-race").is_none());

		let pending = manager.take_pending_hibernation_changes();
		assert!(pending.updated.is_empty());
		assert_eq!(
			pending.removed,
			BTreeSet::from(["conn-race".to_owned()])
		);
	}
}
