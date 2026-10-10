use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use serde_json::Value as JsonValue;

use crate::{
	common::{ActorKey, EncodingKind, TransportKind},
	handle::ActorHandle,
	protocol::query::*,
	remote_manager::RemoteManager,
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
	pub reconnect_backoff: Option<crate::backoff::BackoffConfig>,
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
			reconnect_backoff: None,
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

	pub fn reconnect_backoff(mut self, mut backoff: crate::backoff::BackoffConfig) -> Self {
		backoff.normalize();
		self.reconnect_backoff = Some(backoff);
		self
	}

	pub fn reconnect_delays(
		mut self,
		initial: std::time::Duration,
		max: std::time::Duration,
	) -> Self {
		self.reconnect_backoff = Some(crate::backoff::BackoffConfig::new(initial, max));
		self
	}

	/// Disables automatic reconnection after a connection drops.
	///
	/// The initial connection attempt via `connect()` will still be made.
	/// This only prevents retries if that initial connection (or a later
	/// established connection) fails or disconnects.
	pub fn disable_reconnect(mut self) -> Self {
		self.reconnect_backoff =
			Some(crate::backoff::BackoffConfig::default().max_retries(Some(0)));
		self
	}
}

pub struct Client {
	remote_manager: RemoteManager,
	encoding_kind: EncodingKind,
	transport_kind: TransportKind,
	reconnect_backoff: crate::backoff::BackoffConfig,
	shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
}

impl Clone for Client {
	fn clone(&self) -> Self {
		Self {
			remote_manager: self.remote_manager.clone(),
			encoding_kind: self.encoding_kind,
			transport_kind: self.transport_kind,
			reconnect_backoff: self.reconnect_backoff.clone(),
			shutdown_tx: self.shutdown_tx.clone(),
		}
	}
}

impl std::fmt::Debug for Client {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Client")
			.field("encoding_kind", &self.encoding_kind)
			.field("transport_kind", &self.transport_kind)
			.field("reconnect_backoff", &self.reconnect_backoff)
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
		);

		let mut reconnect_backoff = config.reconnect_backoff.unwrap_or_default();
		reconnect_backoff.normalize();

		Self {
			remote_manager,
			encoding_kind: config.encoding,
			transport_kind: config.transport,
			reconnect_backoff,
			shutdown_tx: Arc::new(tokio::sync::broadcast::channel(1).0),
		}
	}

	pub fn from_endpoint(endpoint: impl Into<String>) -> Self {
		Self::new(ClientConfig::new(endpoint))
	}

	fn create_handle(&self, params: Option<JsonValue>, query: ActorQuery) -> ActorHandle {
		ActorHandle::new(
			self.remote_manager.clone(),
			params,
			query,
			self.shutdown_tx.clone(),
			self.transport_kind,
			self.encoding_kind,
			self.reconnect_backoff.clone(),
		)
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
