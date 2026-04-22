use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Bound::{Excluded, Unbounded};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use futures::future::BoxFuture;
use parking_lot::{RwLock, RwLockReadGuard};
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use uuid::Uuid;

use tokio::sync::oneshot;

use crate::actor::config::ActorConfig;
use crate::actor::context::ActorContext;
use crate::actor::lifecycle_hooks::Reply;
use crate::actor::messages::{ActorEvent, Request};
use crate::actor::persist::{decode_with_embedded_version, encode_with_embedded_version};
use crate::actor::preload::PreloadedKv;
use crate::actor::state::RequestSaveOpts;
use crate::error::ActorRuntime;
use crate::types::ConnId;
use crate::types::ListOpts;

pub(crate) type EventSendCallback = Arc<dyn Fn(OutgoingEvent) -> Result<()> + Send + Sync>;
pub(crate) type DisconnectCallback =
	Arc<dyn Fn(Option<String>) -> BoxFuture<'static, Result<()>> + Send + Sync>;
type StateChangeCallback = Arc<dyn Fn(&ConnHandle) + Send + Sync>;

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
	pub gateway_id: [u8; 4],
	pub request_id: [u8; 4],
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
	pub gateway_id: [u8; 4],
	pub request_id: [u8; 4],
	pub server_message_index: u16,
	pub client_message_index: u16,
	pub request_path: String,
	pub request_headers: BTreeMap<String, String>,
}

#[derive(RivetError, Serialize)]
#[error(
	"actor",
	"invalid_request",
	"Invalid hibernatable websocket connection ID",
	"Hibernatable websocket {field} must be exactly 4 bytes, got {actual_len}."
)]
struct InvalidHibernatableConnectionId {
	field: String,
	actual_len: usize,
}

#[derive(RivetError, Serialize)]
#[error(
	"connection",
	"not_configured",
	"Connection callback is not configured",
	"Connection {component} is not configured."
)]
struct ConnectionNotConfigured {
	component: String,
}

#[derive(RivetError, Serialize)]
#[error(
	"connection",
	"not_found",
	"Connection was not found",
	"Connection '{conn_id}' was not found."
)]
struct ConnectionNotFound {
	conn_id: String,
}

#[derive(RivetError, Serialize)]
#[error(
	"connection",
	"not_hibernatable",
	"Connection is not hibernatable",
	"Connection '{conn_id}' is not hibernatable."
)]
struct ConnectionNotHibernatable {
	conn_id: String,
}

#[derive(RivetError, Serialize)]
#[error(
	"connection",
	"restore_not_found",
	"Hibernatable connection restore target was not found"
)]
struct ConnectionRestoreNotFound;

#[derive(RivetError, Serialize)]
#[error(
	"connection",
	"disconnect_failed",
	"Connection disconnect failed",
	"Disconnect transport failed for {count} connection(s): {details}"
)]
struct ConnectionDisconnectFailed {
	count: usize,
	details: String,
}

pub(crate) fn hibernatable_id_from_slice(field: &'static str, bytes: &[u8]) -> Result<[u8; 4]> {
	bytes.try_into().map_err(|_| {
		InvalidHibernatableConnectionId {
			field: field.to_owned(),
			actual_len: bytes.len(),
		}
		.build()
	})
}

pub(crate) fn encode_persisted_connection(connection: &PersistedConnection) -> Result<Vec<u8>> {
	encode_with_embedded_version(
		connection,
		CONNECTION_PERSIST_VERSION,
		"persisted connection",
	)
}

pub(crate) fn decode_persisted_connection(payload: &[u8]) -> Result<PersistedConnection> {
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
	// Forced-sync: connection handles expose synchronous state and callback
	// methods to foreign runtimes; callbacks are cloned before async work.
	state: RwLock<Vec<u8>>,
	is_hibernatable: bool,
	dirty: AtomicBool,
	subscriptions: RwLock<BTreeSet<String>>,
	hibernation: RwLock<Option<HibernatableConnectionMetadata>>,
	state_change_handler: RwLock<Option<StateChangeCallback>>,
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
			dirty: AtomicBool::new(false),
			subscriptions: RwLock::new(BTreeSet::new()),
			hibernation: RwLock::new(None),
			state_change_handler: RwLock::new(None),
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
		self.0.state.read().clone()
	}

	pub fn set_state(&self, state: Vec<u8>) {
		self.set_state_inner(state, true);
	}

	#[doc(hidden)]
	pub fn set_state_initial(&self, state: Vec<u8>) {
		self.set_state_inner(state, false);
	}

	fn set_state_inner(&self, state: Vec<u8>, mark_dirty: bool) {
		*self.0.state.write() = state;
		if mark_dirty {
			self.mark_hibernation_dirty();
		}
	}

	fn mark_hibernation_dirty(&self) {
		if !self.is_hibernatable() {
			return;
		}
		self.0.dirty.store(true, Ordering::SeqCst);
		let handler = self.0.state_change_handler.read().clone();
		if let Some(handler) = handler {
			handler(self);
		}
	}

	pub(crate) fn clear_hibernation_dirty(&self) {
		self.0.dirty.store(false, Ordering::SeqCst);
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

	pub(crate) fn configure_event_sender(&self, event_sender: Option<EventSendCallback>) {
		*self.0.event_sender.write() = event_sender;
	}

	pub(crate) fn configure_disconnect_handler(
		&self,
		disconnect_handler: Option<DisconnectCallback>,
	) {
		*self.0.disconnect_handler.write() = disconnect_handler;
	}

	pub(crate) fn configure_transport_disconnect_handler(
		&self,
		disconnect_handler: Option<DisconnectCallback>,
	) {
		*self.0.transport_disconnect_handler.write() = disconnect_handler;
	}

	pub(crate) fn subscribe(&self, event_name: impl Into<String>) -> bool {
		self.0.subscriptions.write().insert(event_name.into())
	}

	pub(crate) fn unsubscribe(&self, event_name: &str) -> bool {
		self.0.subscriptions.write().remove(event_name)
	}

	pub(crate) fn is_subscribed(&self, event_name: &str) -> bool {
		self.0.subscriptions.read().contains(event_name)
	}

	pub(crate) fn subscriptions(&self) -> Vec<String> {
		self.0.subscriptions.read().iter().cloned().collect()
	}

	pub(crate) fn clear_subscriptions(&self) {
		self.0.subscriptions.write().clear();
	}

	pub(crate) fn configure_hibernation(
		&self,
		hibernation: Option<HibernatableConnectionMetadata>,
	) {
		*self.0.hibernation.write() = hibernation;
	}

	pub(crate) fn hibernation(&self) -> Option<HibernatableConnectionMetadata> {
		self.0.hibernation.read().clone()
	}

	pub(crate) fn configure_state_change_handler(&self, handler: Option<StateChangeCallback>) {
		*self.0.state_change_handler.write() = handler;
	}

	pub(crate) fn set_server_message_index(
		&self,
		message_index: u16,
	) -> Option<HibernatableConnectionMetadata> {
		let mut hibernation = self.0.hibernation.write();
		let hibernation = hibernation.as_mut()?;
		hibernation.server_message_index = message_index;
		Some(hibernation.clone())
	}

	pub(crate) fn persisted_with_state(&self, state: Vec<u8>) -> Option<PersistedConnection> {
		let hibernation = self.0.hibernation.read().clone()?;

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
		conn.clear_hibernation_dirty();
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
			.clone()
			.ok_or_else(|| connection_not_configured("event sender"))
	}

	fn disconnect_handler(&self) -> Result<DisconnectCallback> {
		self.0
			.disconnect_handler
			.read()
			.clone()
			.ok_or_else(|| connection_not_configured("disconnect handler"))
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
		self.0.transport_disconnect_handler.read().clone()
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
			Some(conn_id) => self
				.guard
				.range((Excluded(conn_id.clone()), Unbounded))
				.next()?,
			None => self.guard.iter().next()?,
		};
		self.next_after = Some(conn_id.clone());
		Some(conn.clone())
	}
}

impl ActorContext {
	pub(crate) fn configure_connection_storage(&self, config: ActorConfig) {
		*self.0.connection_config.write() = config;
	}

	pub(crate) fn iter_connections(&self) -> ConnHandles<'_> {
		ConnHandles::new(self.0.connections.read())
	}

	pub(crate) fn active_connection_count(&self) -> u32 {
		self.0
			.connections
			.read()
			.len()
			.try_into()
			.unwrap_or(u32::MAX)
	}

	pub(crate) fn insert_existing(&self, conn: ConnHandle) {
		let conn_id = conn.id().to_owned();
		let is_hibernatable = conn.is_hibernatable();
		let active_count = {
			let mut connections = self.0.connections.write();
			connections.insert(conn_id.clone(), conn);
			connections.len()
		};
		self.0.metrics.set_active_connections(active_count);
		tracing::debug!(
			actor_id = %self.actor_id(),
			conn_id = %conn_id,
			is_hibernatable,
			active_count,
			"connection added"
		);
	}

	pub(crate) fn remove_existing(&self, conn_id: &str) -> Option<ConnHandle> {
		let (removed, active_count) = {
			let mut connections = self.0.connections.write();
			let removed = connections.remove(conn_id);
			(removed, connections.len())
		};
		self.0.metrics.set_active_connections(active_count);
		tracing::debug!(
			actor_id = %self.actor_id(),
			conn_id,
			removed = removed.is_some(),
			active_count,
			"connection removed"
		);
		removed
	}

	fn remove_existing_for_disconnect(&self, conn_id: &str) -> Option<ConnHandle> {
		let _disconnect_state = self.0.connection_disconnect_state.lock();
		let (removed, active_count) = {
			let mut connections = self.0.connections.write();
			let removed = connections.remove(conn_id)?;

			if removed.is_hibernatable() {
				self.0.pending_hibernation_updates.write().remove(conn_id);
				self.0
					.pending_hibernation_removals
					.write()
					.insert(conn_id.to_owned());
			}

			(removed, connections.len())
		};
		self.0.metrics.set_active_connections(active_count);
		tracing::debug!(
			actor_id = %self.actor_id(),
			conn_id,
			is_hibernatable = removed.is_hibernatable(),
			active_count,
			"connection removed for disconnect"
		);
		Some(removed)
	}

	pub(crate) fn queue_hibernation_update(&self, conn_id: impl Into<ConnId>) {
		let _disconnect_state = self.0.connection_disconnect_state.lock();
		let conn_id = conn_id.into();
		self.0
			.pending_hibernation_updates
			.write()
			.insert(conn_id.clone());
		self.0.pending_hibernation_removals.write().remove(&conn_id);
		tracing::debug!(
			actor_id = %self.actor_id(),
			conn_id = %conn_id,
			"hibernatable connection transport queued for save"
		);
	}

	pub(crate) fn dirty_hibernatable_conns_inner(&self) -> Vec<ConnHandle> {
		let _disconnect_state = self.0.connection_disconnect_state.lock();
		let update_ids: Vec<_> = self
			.0
			.pending_hibernation_updates
			.read()
			.iter()
			.cloned()
			.collect();
		let connections = self.0.connections.read();
		update_ids
			.into_iter()
			.filter_map(|conn_id| connections.get(&conn_id).cloned())
			.filter(|conn| conn.is_hibernatable() && conn.hibernation().is_some())
			.collect()
	}

	pub(crate) fn queue_hibernation_removal_inner(&self, conn_id: impl Into<ConnId>) {
		let _disconnect_state = self.0.connection_disconnect_state.lock();
		let conn_id = conn_id.into();
		self.0.pending_hibernation_updates.write().remove(&conn_id);
		self.0
			.pending_hibernation_removals
			.write()
			.insert(conn_id.clone());
		tracing::debug!(
			actor_id = %self.actor_id(),
			conn_id = %conn_id,
			"hibernatable connection transport queued for removal"
		);
	}

	pub(crate) fn take_pending_hibernation_changes_inner(&self) -> PendingHibernationChanges {
		let _disconnect_state = self.0.connection_disconnect_state.lock();
		PendingHibernationChanges {
			updated: std::mem::take(&mut *self.0.pending_hibernation_updates.write()),
			removed: std::mem::take(&mut *self.0.pending_hibernation_removals.write()),
		}
	}

	pub(crate) fn pending_hibernation_removals(&self) -> Vec<ConnId> {
		let _disconnect_state = self.0.connection_disconnect_state.lock();
		self.0
			.pending_hibernation_removals
			.read()
			.iter()
			.cloned()
			.collect()
	}

	pub(crate) fn has_pending_hibernation_changes_inner(&self) -> bool {
		let _disconnect_state = self.0.connection_disconnect_state.lock();
		let has_updates = !self.0.pending_hibernation_updates.read().is_empty();
		let has_removals = !self.0.pending_hibernation_removals.read().is_empty();
		has_updates || has_removals
	}

	pub(crate) fn restore_pending_hibernation_changes(&self, pending: PendingHibernationChanges) {
		let _disconnect_state = self.0.connection_disconnect_state.lock();
		if !pending.updated.is_empty() {
			self.0
				.pending_hibernation_updates
				.write()
				.extend(pending.updated);
		}
		if !pending.removed.is_empty() {
			self.0
				.pending_hibernation_removals
				.write()
				.extend(pending.removed);
		}
	}

	pub(crate) async fn connect_with_state<F>(
		&self,
		params: Vec<u8>,
		is_hibernatable: bool,
		hibernation: Option<HibernatableConnectionMetadata>,
		request: Option<Request>,
		create_state: F,
	) -> Result<ConnHandle>
	where
		F: std::future::Future<Output = Result<Vec<u8>>> + Send,
	{
		let config = self.connection_config();

		let state = timeout(config.create_conn_state_timeout, create_state)
			.await
			.with_context(|| {
				timeout_message("create_conn_state", config.create_conn_state_timeout)
			})??;

		let conn = ConnHandle::new(
			Uuid::new_v4().to_string(),
			params.clone(),
			state,
			is_hibernatable,
		);
		conn.configure_hibernation(hibernation);
		self.prepare_managed_conn(&conn);
		self.insert_existing(conn.clone());

		if let Err(error) = self.emit_connection_open(&conn, params, request).await {
			self.remove_existing(conn.id());
			return Err(error);
		}
		self.0.metrics.inc_connections_total();
		self.record_connections_updated();
		self.reset_sleep_timer();

		Ok(conn)
	}

	pub(crate) fn encode_hibernation_delta(
		&self,
		conn_id: &str,
		bytes: Vec<u8>,
	) -> Result<Vec<u8>> {
		let conn = self.connection(conn_id).ok_or_else(|| {
			ConnectionNotFound {
				conn_id: conn_id.to_owned(),
			}
			.build()
		})?;
		let persisted = conn.persisted_with_state(bytes).ok_or_else(|| {
			ConnectionNotHibernatable {
				conn_id: conn_id.to_owned(),
			}
			.build()
		})?;
		encode_persisted_connection(&persisted).context("encode persisted connection")
	}

	pub(crate) async fn restore_persisted(
		&self,
		preloaded_kv: Option<&PreloadedKv>,
	) -> Result<Vec<ConnHandle>> {
		let entries = if let Some(entries) =
			preloaded_kv.and_then(|kv| kv.prefix_entries(CONNECTION_KEY_PREFIX))
		{
			entries
		} else {
			self.0
				.kv
				.list_prefix(
					CONNECTION_KEY_PREFIX,
					ListOpts {
						reverse: false,
						limit: None,
					},
				)
				.await?
		};
		let mut restored = Vec::new();

		for (_key, value) in entries {
			match decode_persisted_connection(&value) {
				Ok(persisted) => {
					let conn = ConnHandle::from_persisted(persisted);
					self.prepare_managed_conn(&conn);
					self.insert_existing(conn.clone());
					tracing::debug!(
						actor_id = %self.actor_id(),
						conn_id = conn.id(),
						"hibernatable connection restored"
					);
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
		gateway_id: &[u8],
		request_id: &[u8],
	) -> Result<ConnHandle> {
		let gateway_id = hibernatable_id_from_slice("gateway_id", gateway_id)?;
		let request_id = hibernatable_id_from_slice("request_id", request_id)?;
		let Some(conn) = self
			.iter_connections()
			.find(|conn| match conn.hibernation() {
				Some(hibernation) => {
					hibernation.gateway_id == gateway_id && hibernation.request_id == request_id
				}
				None => false,
			})
		else {
			return Err(ConnectionRestoreNotFound.build());
		};

		self.record_connections_updated();
		self.reset_sleep_timer();
		tracing::debug!(
			actor_id = %self.actor_id(),
			conn_id = conn.id(),
			"hibernatable connection transport restored"
		);
		Ok(conn)
	}

	fn prepare_managed_conn(&self, conn: &ConnHandle) {
		let ctx = self.downgrade();
		let conn_id = conn.id().to_owned();

		conn.configure_state_change_handler(Some(Arc::new({
			let ctx = ctx.clone();
			move |conn| {
				let Some(ctx) = ActorContext::from_weak(&ctx) else {
					tracing::warn!(
						conn_id = conn.id(),
						"skipping hibernatable connection state save without actor context"
					);
					return;
				};
				ctx.queue_hibernation_update(conn.id().to_owned());
				ctx.request_save(RequestSaveOpts::default());
			}
		})));

		conn.configure_disconnect_handler(Some(Arc::new(move |reason| {
			let ctx = ctx.clone();
			let conn_id = conn_id.clone();
			Box::pin(async move {
				let ctx = ActorContext::from_weak(&ctx).ok_or_else(|| {
					ActorRuntime::NotConfigured {
						component: "actor context".to_owned(),
					}
					.build()
				})?;
				ctx.with_disconnect_callback(|| async {
					ctx.disconnect_managed(&conn_id, reason).await
				})
				.await
			})
		})));
	}

	fn connection_config(&self) -> ActorConfig {
		self.0.connection_config.read().clone()
	}

	#[cfg(test)]
	pub(crate) fn connection_config_for_tests(&self) -> ActorConfig {
		self.connection_config()
	}

	async fn disconnect_managed(&self, conn_id: &str, reason: Option<String>) -> Result<()> {
		let Some(conn) = self.remove_existing_for_disconnect(conn_id) else {
			tracing::debug!(
				actor_id = %self.actor_id(),
				conn_id,
				reason = ?reason.as_deref(),
				"connection disconnect skipped because connection was already removed"
			);
			return Ok(());
		};
		conn.clear_subscriptions();

		self.try_send_actor_event(ActorEvent::ConnectionClosed { conn }, "connection_closed")
			.with_context(|| disconnect_message(conn_id, reason.as_deref()))?;

		self.record_connections_updated();
		self.reset_sleep_timer();
		tracing::debug!(
			actor_id = %self.actor_id(),
			conn_id,
			reason = ?reason.as_deref(),
			"connection disconnected"
		);
		Ok(())
	}

	async fn emit_connection_open(
		&self,
		conn: &ConnHandle,
		params: Vec<u8>,
		request: Option<Request>,
	) -> Result<()> {
		let config = self.connection_config();
		let (reply_tx, reply_rx) = oneshot::channel();
		self.try_send_actor_event(
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
		self.0.connections.read().get(conn_id).cloned()
	}

	pub(crate) async fn disconnect_transport_only<F>(&self, mut predicate: F) -> Result<()>
	where
		F: FnMut(&ConnHandle) -> bool,
	{
		let connections: Vec<_> = self
			.iter_connections()
			.filter(|conn| predicate(conn))
			.collect();
		let mut disconnected_ids = Vec::new();
		let mut failures = Vec::new();

		for conn in &connections {
			match conn.disconnect_transport_only().await {
				Ok(()) => {
					tracing::debug!(
						actor_id = %self.actor_id(),
						conn_id = conn.id(),
						"connection transport disconnect completed"
					);
					disconnected_ids.push(conn.id().to_owned());
				}
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
				tracing::debug!(
					actor_id = %self.actor_id(),
					conn_id = %conn_id,
					"connection transport removal skipped because connection was already removed"
				);
				continue;
			};
			conn.clear_subscriptions();
			removed_any = true;
			tracing::debug!(
				actor_id = %self.actor_id(),
				conn_id = %conn_id,
				"connection transport removed"
			);
		}

		if removed_any {
			self.record_connections_updated();
			self.reset_sleep_timer();
		}

		if failures.is_empty() {
			return Ok(());
		}

		let count = failures.len();
		Err(ConnectionDisconnectFailed {
			count,
			details: failures
				.into_iter()
				.map(|(conn_id, error)| format!("{conn_id}: {error}"))
				.collect::<Vec<_>>()
				.join("; "),
		}
		.build())
	}
}

fn connection_not_configured(component: &str) -> anyhow::Error {
	ConnectionNotConfigured {
		component: component.to_owned(),
	}
	.build()
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
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};

	use parking_lot::Mutex;
	use tokio::sync::{Barrier, mpsc};
	use tokio::task::yield_now;

	use super::{
		HibernatableConnectionMetadata, PersistedConnection, decode_persisted_connection,
		encode_persisted_connection, hibernatable_id_from_slice, make_connection_key,
	};
	use crate::actor::context::ActorContext;
	use crate::actor::messages::ActorEvent;
	use crate::actor::preload::PreloadedKv;
	use crate::actor::task::LifecycleEvent;
	use crate::kv::Kv;

	fn next_non_activity_lifecycle_event(
		rx: &mut mpsc::Receiver<LifecycleEvent>,
	) -> Option<LifecycleEvent> {
		rx.try_recv().ok()
	}

	#[tokio::test]
	async fn restore_persisted_uses_preloaded_connection_prefix_when_present() {
		let ctx = ActorContext::new_with_kv(
			"actor-preload",
			"actor",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		);
		let persisted = PersistedConnection {
			id: "conn-preloaded".to_owned(),
			parameters: vec![1],
			state: vec![2],
			gateway_id: [1, 2, 3, 4],
			request_id: [5, 6, 7, 8],
			request_path: "/socket".to_owned(),
			..PersistedConnection::default()
		};
		let preloaded = PreloadedKv::new_with_requested_get_keys(
			[(
				make_connection_key(&persisted.id),
				encode_persisted_connection(&persisted)
					.expect("persisted connection should encode"),
			)],
			Vec::new(),
			vec![vec![2]],
		);

		let restored = ctx
			.restore_persisted(Some(&preloaded))
			.await
			.expect("restore should use preloaded entries instead of unconfigured kv");

		assert_eq!(restored.len(), 1);
		assert_eq!(restored[0].id(), "conn-preloaded");
		assert_eq!(restored[0].state(), vec![2]);
		assert!(ctx.connection("conn-preloaded").is_some());
	}

	#[test]
	fn persisted_connection_uses_ts_v4_fixed_id_wire_format() {
		let persisted = PersistedConnection {
			id: "c".to_owned(),
			parameters: vec![1, 2],
			state: vec![3],
			gateway_id: [10, 11, 12, 13],
			request_id: [20, 21, 22, 23],
			server_message_index: 9,
			client_message_index: 10,
			request_path: "/".to_owned(),
			..PersistedConnection::default()
		};

		let encoded =
			encode_persisted_connection(&persisted).expect("persisted connection should encode");

		assert_eq!(
			encoded,
			vec![
				4, 0, // embedded version
				1, b'c', // id
				2, 1, 2, // parameters
				1, 3, // state
				0, // subscriptions
				10, 11, 12, 13, // gatewayId fixed data[4]
				20, 21, 22, 23, // requestId fixed data[4]
				9, 0, // serverMessageIndex
				10, 0, // clientMessageIndex
				1, b'/', // requestPath
				0,    // requestHeaders
			]
		);

		let decoded =
			decode_persisted_connection(&encoded).expect("persisted connection should decode");
		assert_eq!(decoded.gateway_id, [10, 11, 12, 13]);
		assert_eq!(decoded.request_id, [20, 21, 22, 23]);
	}

	#[test]
	fn hibernatable_id_validation_returns_rivet_error() {
		let error = hibernatable_id_from_slice("gateway_id", &[1, 2, 3])
			.expect_err("invalid id should fail");
		let error = rivet_error::RivetError::extract(&error);

		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "invalid_request");
	}

	#[tokio::test(start_paused = true)]
	async fn concurrent_disconnects_only_emit_one_close_and_one_hibernation_removal() {
		let ctx = ActorContext::new_with_kv(
			"actor-race",
			"actor",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		);
		ctx.configure_connection_runtime(crate::actor::config::ActorConfig::default());
		let (events_tx, mut events_rx) = mpsc::unbounded_channel();
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
							*observed_conn_id.lock() = Some(conn.id().to_owned());
							closed.fetch_add(1, Ordering::SeqCst);
							break;
						}
						other => panic!("unexpected event: {other:?}"),
					}
				}
			}
		});

		let conn = ctx
			.connect_with_state(
				vec![1],
				true,
				Some(HibernatableConnectionMetadata {
					gateway_id: [1, 2, 3, 4],
					request_id: [5, 6, 7, 8],
					..HibernatableConnectionMetadata::default()
				}),
				None,
				async { Ok(vec![9]) },
			)
			.await
			.expect("connection should open");
		let conn_id = conn.id().to_owned();
		ctx.record_connections_updated();
		ctx.reset_sleep_timer();

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
		assert_eq!(observed_conn_id.lock().as_deref(), Some(conn_id.as_str()));
		assert!(ctx.connection(&conn_id).is_none());

		let pending = ctx.take_pending_hibernation_changes_inner();
		assert!(pending.updated.is_empty());
		assert_eq!(pending.removed, BTreeSet::from([conn_id]));
	}

	#[tokio::test]
	async fn hibernatable_set_state_queues_save_and_non_hibernatable_stays_memory_only() {
		let ctx = ActorContext::new_with_kv(
			"actor-state-dirty",
			"actor",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		);
		let (actor_events_tx, mut actor_events_rx) = mpsc::unbounded_channel();
		let (lifecycle_events_tx, mut lifecycle_events_rx) = mpsc::channel(4);
		ctx.configure_actor_events(Some(actor_events_tx));
		ctx.configure_lifecycle_events(Some(lifecycle_events_tx));

		let open_replies = tokio::spawn(async move {
			for _ in 0..2 {
				match actor_events_rx
					.recv()
					.await
					.expect("open event should arrive")
				{
					ActorEvent::ConnectionOpen { reply, .. } => reply.send(Ok(())),
					other => panic!("unexpected actor event: {other:?}"),
				}
			}
		});

		let non_hibernatable = ctx
			.connect_with_state(vec![1], false, None, None, async { Ok(vec![2]) })
			.await
			.expect("non-hibernatable connection should open");
		non_hibernatable.set_state(vec![3]);
		assert_eq!(non_hibernatable.state(), vec![3]);
		assert!(
			ctx.dirty_hibernatable_conns_inner().is_empty(),
			"non-hibernatable state changes should not queue persistence"
		);
		assert!(
			next_non_activity_lifecycle_event(&mut lifecycle_events_rx).is_none(),
			"non-hibernatable state changes should not request actor save"
		);

		let hibernatable = ctx
			.connect_with_state(
				vec![4],
				true,
				Some(HibernatableConnectionMetadata {
					gateway_id: [1, 2, 3, 4],
					request_id: [5, 6, 7, 8],
					..HibernatableConnectionMetadata::default()
				}),
				None,
				async { Ok(vec![5]) },
			)
			.await
			.expect("hibernatable connection should open");
		hibernatable.set_state(vec![6]);

		assert_eq!(
			ctx.dirty_hibernatable_conns_inner()
				.into_iter()
				.map(|conn| conn.id().to_owned())
				.collect::<Vec<_>>(),
			vec![hibernatable.id().to_owned()]
		);
		assert_eq!(
			next_non_activity_lifecycle_event(&mut lifecycle_events_rx)
				.expect("hibernatable state change should request save"),
			LifecycleEvent::SaveRequested { immediate: false }
		);

		open_replies
			.await
			.expect("open reply task should join cleanly");
	}

	#[tokio::test(start_paused = true)]
	async fn remove_existing_for_disconnect_has_exactly_one_winner() {
		let ctx = ActorContext::new_with_kv(
			"actor-race",
			"actor",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		);
		let conn = super::ConnHandle::new("conn-race", vec![1], vec![2], true);
		conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: [1, 2, 3, 4],
			request_id: [5, 6, 7, 8],
			..HibernatableConnectionMetadata::default()
		}));
		ctx.insert_existing(conn);

		let barrier = Arc::new(Barrier::new(2));
		let first = tokio::spawn({
			let ctx = ctx.clone();
			let barrier = barrier.clone();
			async move {
				barrier.wait().await;
				ctx.remove_existing_for_disconnect("conn-race")
					.map(|conn| conn.id().to_owned())
			}
		});
		let second = tokio::spawn({
			let ctx = ctx.clone();
			let barrier = barrier.clone();
			async move {
				barrier.wait().await;
				ctx.remove_existing_for_disconnect("conn-race")
					.map(|conn| conn.id().to_owned())
			}
		});

		let first = first.await.expect("first task should join");
		let second = second.await.expect("second task should join");
		let winners = [first, second].into_iter().flatten().collect::<Vec<_>>();

		assert_eq!(winners, vec!["conn-race".to_owned()]);
		assert!(ctx.connection("conn-race").is_none());

		let pending = ctx.take_pending_hibernation_changes_inner();
		assert!(pending.updated.is_empty());
		assert_eq!(pending.removed, BTreeSet::from(["conn-race".to_owned()]));
	}
}
