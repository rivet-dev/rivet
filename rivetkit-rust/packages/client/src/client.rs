use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use serde_json::Value as JsonValue;

use crate::{
	common::{ActorKey, EncodingKind, TransportKind},
	handle::ActorHandle,
	protocol::query::*,
	remote_manager::{DEFAULT_CONNECT_TIMEOUT, DEFAULT_CONTROL_TIMEOUT, RemoteManager, Timeouts},
};

#[derive(Default)]
pub struct GetWithIdOptions {
	pub params: Option<JsonValue>,
}

#[derive(Default)]
pub struct GetOptions {
	pub params: Option<JsonValue>,
}

#[derive(Default)]
pub struct GetOrCreateOptions {
	pub params: Option<JsonValue>,
	pub create_in_region: Option<String>,
	pub create_with_input: Option<JsonValue>,
	/// Overrides the client's configured pool name for this actor if it is created.
	pub pool_name: Option<String>,
}

#[derive(Default)]
pub struct CreateOptions {
	pub params: Option<JsonValue>,
	pub region: Option<String>,
	pub input: Option<JsonValue>,
	/// Overrides the client's configured pool name for this actor.
	pub pool_name: Option<String>,
}

pub struct ClientConfig {
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: Option<String>,
	pub pool_name: Option<String>,
	pub encoding: EncodingKind,
	pub transport: TransportKind,
	pub headers: Option<HashMap<String, String>>,
	pub max_input_size: Option<usize>,
	pub disable_metadata_lookup: bool,
	/// Bounds TCP connect for HTTP requests and the full WebSocket handshake.
	/// Defaults to 10 seconds.
	///
	/// It does not bound waiting for an HTTP response once connected. See
	/// `control_timeout` and `request_timeout` for that.
	pub connect_timeout: Duration,
	/// Bounds each control request end to end, from send until the response body
	/// has been read. Defaults to 30 seconds.
	///
	/// Control requests are the short Engine calls the client makes on its own
	/// behalf: the `/metadata` lookup and the actor get, get-or-create, create,
	/// and list calls. They back `resolve`, `resolve_handle`, and
	/// `resolve_optional`, and they run before every action, queue send, `fetch`,
	/// and WebSocket call. Because the bound includes the response, a peer that
	/// accepts the connection and then never answers makes the call fail instead
	/// of hang.
	///
	/// It never applies to user requests (see `request_timeout`) or to
	/// established WebSocket connections.
	pub control_timeout: Duration,
	/// Bounds each user request end to end, from send until the response body has
	/// been read.
	///
	/// User requests are HTTP actions, queue `send` and `send_and_wait`, `fetch`,
	/// and `reload`. Defaults to `None` (no limit) on purpose, because a queue
	/// wait or a long-running action can legitimately take minutes. Actions and
	/// queue waits are therefore not bounded unless this is set. Set it when you
	/// want a hard upper bound for your workload.
	///
	/// It does not apply to control requests (see `control_timeout`) or to
	/// established WebSocket connections.
	pub request_timeout: Option<Duration>,
}

impl ClientConfig {
	pub fn new(endpoint: impl Into<String>) -> Self {
		Self {
			endpoint: endpoint.into(),
			token: None,
			namespace: None,
			pool_name: None,
			encoding: EncodingKind::Bare,
			transport: TransportKind::WebSocket,
			headers: None,
			max_input_size: None,
			disable_metadata_lookup: false,
			connect_timeout: DEFAULT_CONNECT_TIMEOUT,
			control_timeout: DEFAULT_CONTROL_TIMEOUT,
			request_timeout: None,
		}
	}

	pub fn token(mut self, token: impl Into<String>) -> Self {
		self.token = Some(token.into());
		self
	}

	pub fn token_opt(mut self, token: Option<String>) -> Self {
		self.token = token;
		self
	}

	pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
		self.namespace = Some(namespace.into());
		self
	}

	pub fn pool_name(mut self, pool_name: impl Into<String>) -> Self {
		self.pool_name = Some(pool_name.into());
		self
	}

	pub fn encoding(mut self, encoding: EncodingKind) -> Self {
		self.encoding = encoding;
		self
	}

	pub fn transport(mut self, transport: TransportKind) -> Self {
		self.transport = transport;
		self
	}

	pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.headers
			.get_or_insert_with(HashMap::new)
			.insert(key.into(), value.into());
		self
	}

	pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
		self.headers = Some(headers);
		self
	}

	pub fn max_input_size(mut self, max_input_size: usize) -> Self {
		self.max_input_size = Some(max_input_size);
		self
	}

	pub fn disable_metadata_lookup(mut self, disable: bool) -> Self {
		self.disable_metadata_lookup = disable;
		self
	}

	/// Sets how long to wait for a TCP connection or WebSocket handshake to
	/// complete. Defaults to 10 seconds.
	pub fn connect_timeout(mut self, timeout: Duration) -> Self {
		self.connect_timeout = timeout;
		self
	}

	/// Sets how long a control request may take, including its response.
	/// Defaults to 30 seconds. See [`ClientConfig::control_timeout`].
	pub fn control_timeout(mut self, timeout: Duration) -> Self {
		self.control_timeout = timeout;
		self
	}

	/// Sets an end-to-end limit for each user request such as an action, queue
	/// send, or `fetch`. Unset by default, so long-running actions and queue
	/// waits are not cut off. See [`ClientConfig::request_timeout`].
	pub fn request_timeout(mut self, timeout: Duration) -> Self {
		self.request_timeout = Some(timeout);
		self
	}
}

pub struct Client {
	remote_manager: RemoteManager,
	encoding_kind: EncodingKind,
	transport_kind: TransportKind,
	shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
}

impl Clone for Client {
	fn clone(&self) -> Self {
		Self {
			remote_manager: self.remote_manager.clone(),
			encoding_kind: self.encoding_kind,
			transport_kind: self.transport_kind,
			shutdown_tx: self.shutdown_tx.clone(),
		}
	}
}

impl std::fmt::Debug for Client {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Client")
			.field("encoding_kind", &self.encoding_kind)
			.field("transport_kind", &self.transport_kind)
			.finish_non_exhaustive()
	}
}

impl Client {
	pub fn new(config: ClientConfig) -> Self {
		let remote_manager = RemoteManager::from_config(
			config.endpoint,
			config.token,
			config.namespace,
			config.pool_name,
			config.headers,
			config.max_input_size,
			config.disable_metadata_lookup,
			Timeouts {
				connect: config.connect_timeout,
				control: config.control_timeout,
				request: config.request_timeout,
			},
		);

		Self {
			remote_manager,
			encoding_kind: config.encoding,
			transport_kind: config.transport,
			shutdown_tx: Arc::new(tokio::sync::broadcast::channel(1).0),
		}
	}

	pub fn from_endpoint(endpoint: impl Into<String>) -> Self {
		Self::new(ClientConfig::new(endpoint))
	}

	fn create_handle(&self, params: Option<JsonValue>, query: ActorQuery) -> ActorHandle {
		let handle = ActorHandle::new(
			self.remote_manager.clone(),
			params,
			query,
			self.shutdown_tx.clone(),
			self.transport_kind,
			self.encoding_kind,
		);

		handle
	}

	pub fn get(&self, name: &str, key: ActorKey, opts: GetOptions) -> Result<ActorHandle> {
		let actor_query = ActorQuery::GetForKey {
			get_for_key: GetForKeyRequest {
				name: name.to_string(),
				key,
			},
		};

		let handle = self.create_handle(opts.params, actor_query);

		Ok(handle)
	}

	pub fn get_for_id(&self, name: &str, actor_id: &str, opts: GetOptions) -> Result<ActorHandle> {
		let actor_query = ActorQuery::GetForId {
			get_for_id: GetForIdRequest {
				name: name.to_string(),
				actor_id: actor_id.to_string(),
			},
		};

		let handle = self.create_handle(opts.params, actor_query);

		Ok(handle)
	}

	pub fn get_or_create(
		&self,
		name: &str,
		key: ActorKey,
		opts: GetOrCreateOptions,
	) -> Result<ActorHandle> {
		let input = opts.create_with_input;
		let region = opts.create_in_region;

		let actor_query = ActorQuery::GetOrCreateForKey {
			get_or_create_for_key: GetOrCreateRequest {
				name: name.to_string(),
				key: key,
				input,
				region,
				pool_name: opts.pool_name,
			},
		};

		let handle = self.create_handle(opts.params, actor_query);

		Ok(handle)
	}

	pub async fn create(
		&self,
		name: &str,
		key: ActorKey,
		opts: CreateOptions,
	) -> Result<ActorHandle> {
		let input = opts.input;
		let _region = opts.region;

		let actor_id = self
			.remote_manager
			.create_actor(name, &key, input, opts.pool_name)
			.await?;

		let get_query = ActorQuery::GetForId {
			get_for_id: GetForIdRequest {
				name: name.to_string(),
				actor_id,
			},
		};

		let handle = self.create_handle(opts.params, get_query);

		Ok(handle)
	}

	pub fn disconnect(self) {
		drop(self)
	}

	pub fn dispose(self) {
		self.disconnect()
	}
}

impl Drop for Client {
	fn drop(&mut self) {
		// Notify all subscribers to shutdown
		let _ = self.shutdown_tx.send(());
	}
}
