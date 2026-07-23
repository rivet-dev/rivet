use crate::{
	common::{EncodingKind, RawWebSocket, TransportKind, HEADER_CONN_PARAMS, HEADER_ENCODING},
	connection::{start_connection, ActorConnection, ActorConnectionInner},
	protocol::{codec, query::*},
	remote_manager::RemoteManager,
};
use anyhow::{anyhow, Result};
use bytes::Bytes;
use reqwest::{
	header::{HeaderMap, HeaderValue},
	Method, Response,
};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::{
	ops::Deref,
	sync::{Arc, Mutex},
	time::Duration,
};

pub use crate::protocol::codec::{QueueMessageStatus, QueueSendReceipt};

#[derive(Debug, Clone, Default)]
pub struct QueueSendOptions {
	pub dedupe_key: Option<String>,
	pub delay: Option<Duration>,
}

pub struct ActorHandleStateless {
	remote_manager: RemoteManager,
	params: Option<JsonValue>,
	encoding_kind: EncodingKind,
	// Mutex (not RefCell) so the handle is `Sync` and `&handle` futures
	// remain `Send` — required to call `.action(...)` from within axum
	// middleware that needs `Send` futures.
	query: Mutex<ActorQuery>,
}

impl ActorHandleStateless {
	pub fn new(
		remote_manager: RemoteManager,
		params: Option<JsonValue>,
		encoding_kind: EncodingKind,
		query: ActorQuery,
	) -> Self {
		Self {
			remote_manager,
			params,
			encoding_kind,
			query: Mutex::new(query),
		}
	}

	pub async fn action(&self, name: &str, args: Vec<JsonValue>) -> Result<JsonValue> {
		// Resolve actor ID
		let query = self.query.lock().expect("query lock poisoned").clone();
		let actor_id = self.remote_manager.resolve_actor_id(&query).await?;

		let body = codec::encode_http_action_request(self.encoding_kind, &args)?;

		let headers = self.protocol_headers()?;

		// Send request via gateway
		let path = format!("/action/{}", urlencoding::encode(name));
		let res = self
			.remote_manager
			.send_request(
				&actor_id,
				&path,
				Method::POST,
				headers,
				Some(Bytes::from(body)),
			)
			.await?;

		if !res.status().is_success() {
			let status = res.status();
			let body = res.bytes().await?;
			if let Ok((group, code, message, metadata)) =
				codec::decode_http_error(self.encoding_kind, &body)
			{
				return Err(anyhow!(
					"action failed ({group}/{code}): {message}, metadata={metadata:?}"
				));
			}
			return Err(anyhow!("action failed: {status}"));
		}

		// Decode response
		let output = res.bytes().await?;
		codec::decode_http_action_response(self.encoding_kind, &output)
	}

	pub async fn send(
		&self,
		name: &str,
		body: impl Serialize,
		opts: QueueSendOptions,
	) -> Result<QueueSendReceipt> {
		self.send_queue(name, &body, opts).await
	}

	async fn send_queue<T: Serialize>(
		&self,
		name: &str,
		body: &T,
		opts: QueueSendOptions,
	) -> Result<QueueSendReceipt> {
		let query = self.query.lock().expect("query lock poisoned").clone();
		let actor_id = self.remote_manager.resolve_actor_id(&query).await?;
		let delay_ms = opts
			.delay
			.map(|duration| {
				let delay_ms = u64::try_from(duration.as_millis())
					.map_err(|_| anyhow!("queue delay exceeds the supported range"))?;
				if delay_ms > 9_007_199_254_740_991 {
					return Err(anyhow!(
						"queue delay must not exceed JavaScript's maximum safe integer"
					));
				}
				Ok(delay_ms)
			})
			.transpose()?;
		let request_body = codec::encode_http_queue_request(
			self.encoding_kind,
			name,
			body,
			opts.dedupe_key.as_deref(),
			delay_ms,
		)?;

		let headers = self.protocol_headers()?;

		let path = format!("/queue/{}", urlencoding::encode(name));
		let res = self
			.remote_manager
			.send_request(
				&actor_id,
				&path,
				Method::POST,
				headers,
				Some(Bytes::from(request_body)),
			)
			.await?;

		if !res.status().is_success() {
			let status = res.status();
			let body = res.bytes().await?;
			if let Ok((group, code, message, metadata)) =
				codec::decode_http_error(self.encoding_kind, &body)
			{
				return Err(anyhow!(
					"queue send failed ({group}/{code}): {message}, metadata={metadata:?}"
				));
			}
			return Err(anyhow!("queue send failed: {status}"));
		}

		let body = res.bytes().await?;
		codec::decode_http_queue_response(self.encoding_kind, &body)
	}

	pub async fn queue_status(&self, receipt_id: &str) -> Result<QueueMessageStatus> {
		let query = self.query.lock().expect("query lock poisoned").clone();
		let actor_id = self.remote_manager.resolve_actor_id(&query).await?;
		let path = format!("/queue/receipts/{}", urlencoding::encode(receipt_id));
		let res = self
			.remote_manager
			.send_request(
				&actor_id,
				&path,
				Method::GET,
				self.protocol_headers()?,
				None,
			)
			.await?;
		if !res.status().is_success() {
			let status = res.status();
			let body = res.bytes().await?;
			if let Ok((group, code, message, metadata)) =
				codec::decode_http_error(self.encoding_kind, &body)
			{
				return Err(anyhow!(
					"queue status failed ({group}/{code}): {message}, metadata={metadata:?}"
				));
			}
			return Err(anyhow!("queue status failed: {status}"));
		}
		codec::decode_http_queue_status(self.encoding_kind, &res.bytes().await?)
	}

	pub async fn fetch(
		&self,
		path: &str,
		method: Method,
		headers: HeaderMap,
		body: Option<Bytes>,
	) -> Result<Response> {
		let query = self.query.lock().expect("query lock poisoned").clone();
		let actor_id = self.remote_manager.resolve_actor_id(&query).await?;
		let path = normalize_fetch_path(path);
		self.remote_manager
			.send_request(&actor_id, &path, method, headers, body)
			.await
	}

	pub async fn web_socket(
		&self,
		path: &str,
		protocols: Option<Vec<String>>,
	) -> Result<RawWebSocket> {
		let query = self.query.lock().expect("query lock poisoned").clone();
		let actor_id = self.remote_manager.resolve_actor_id(&query).await?;
		self.remote_manager
			.open_raw_websocket(&actor_id, path, self.params.clone(), protocols)
			.await
	}

	pub fn gateway_url(&self) -> Result<String> {
		let query = self.query.lock().expect("query lock poisoned").clone();
		self.remote_manager.gateway_url(&query)
	}

	pub fn get_gateway_url(&self) -> Result<String> {
		self.gateway_url()
	}

	pub async fn reload(&self) -> Result<()> {
		let query = self.query.lock().expect("query lock poisoned").clone();
		let actor_id = self.remote_manager.resolve_actor_id(&query).await?;
		let res = self
			.remote_manager
			.send_request(
				&actor_id,
				"/dynamic/reload",
				Method::PUT,
				HeaderMap::new(),
				None,
			)
			.await?;
		if !res.status().is_success() {
			let status = res.status();
			let body = res.text().await.unwrap_or_default();
			return Err(anyhow!("reload failed with status {status}: {body}"));
		}
		Ok(())
	}

	pub async fn resolve(&self) -> Result<String> {
		let query = {
			let Ok(query) = self.query.lock() else {
				return Err(anyhow!("Failed to lock actor query"));
			};
			query.clone()
		};

		match query {
			ActorQuery::Create { .. } => Err(anyhow!("actor query cannot be create")),
			ActorQuery::GetForId { get_for_id } => Ok(get_for_id.actor_id.clone()),
			_ => {
				let actor_id = self.remote_manager.resolve_actor_id(&query).await?;

				// Get name from the original query
				let name = match &query {
					ActorQuery::GetForKey { get_for_key } => get_for_key.name.clone(),
					ActorQuery::GetOrCreateForKey {
						get_or_create_for_key,
					} => get_or_create_for_key.name.clone(),
					_ => return Err(anyhow!("unexpected query type")),
				};

				{
					let Ok(mut query_mut) = self.query.lock() else {
						return Err(anyhow!("Failed to lock actor query mutably"));
					};

					*query_mut = ActorQuery::GetForId {
						get_for_id: GetForIdRequest {
							name,
							actor_id: actor_id.clone(),
						},
					};
				}

				Ok(actor_id)
			}
		}
	}

	fn protocol_headers(&self) -> Result<HeaderMap> {
		let mut headers = HeaderMap::new();
		headers.insert(
			HEADER_ENCODING,
			HeaderValue::from_str(self.encoding_kind.as_str())?,
		);

		if let Some(params) = &self.params {
			headers.insert(
				HEADER_CONN_PARAMS,
				HeaderValue::from_str(&serde_json::to_string(params)?)?,
			);
		}

		Ok(headers)
	}
}

fn normalize_fetch_path(path: &str) -> String {
	let path = path.trim_start_matches('/');
	if path.is_empty() {
		"/request".to_string()
	} else {
		format!("/request/{path}")
	}
}

pub struct ActorHandle {
	handle: ActorHandleStateless,
	remote_manager: RemoteManager,
	params: Option<JsonValue>,
	query: ActorQuery,
	client_shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
	transport_kind: crate::TransportKind,
	encoding_kind: EncodingKind,
}

impl ActorHandle {
	pub fn new(
		remote_manager: RemoteManager,
		params: Option<JsonValue>,
		query: ActorQuery,
		client_shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
		transport_kind: TransportKind,
		encoding_kind: EncodingKind,
	) -> Self {
		let handle = ActorHandleStateless::new(
			remote_manager.clone(),
			params.clone(),
			encoding_kind,
			query.clone(),
		);

		Self {
			handle,
			remote_manager,
			params,
			query,
			client_shutdown_tx,
			transport_kind,
			encoding_kind,
		}
	}

	pub fn connect(&self) -> ActorConnection {
		let conn = ActorConnectionInner::new(
			self.remote_manager.clone(),
			self.query.clone(),
			self.transport_kind,
			self.encoding_kind,
			self.params.clone(),
		);

		let rx = self.client_shutdown_tx.subscribe();
		start_connection(&conn, rx);

		conn
	}
}

impl Deref for ActorHandle {
	type Target = ActorHandleStateless;

	fn deref(&self) -> &Self::Target {
		&self.handle
	}
}
