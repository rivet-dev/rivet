use std::collections::HashMap;
use std::sync::Arc;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Runtime;

use rivet_envoy_protocol as protocol;

use crate::bridge_actor::{ResponseMap, SqliteSchemaVersionMap, SqliteStartupMap, WsSenderMap};
use crate::types::{self, JsKvEntry, JsKvListOptions};

fn make_ws_key(gateway_id: &[u8], request_id: &[u8]) -> [u8; 8] {
	let mut key = [0u8; 8];
	if gateway_id.len() >= 4 {
		key[..4].copy_from_slice(&gateway_id[..4]);
	}
	if request_id.len() >= 4 {
		key[4..].copy_from_slice(&request_id[..4]);
	}
	key
}

/// Native envoy handle exposed to JavaScript via N-API.
#[napi]
pub struct JsEnvoyHandle {
	pub(crate) runtime: Arc<Runtime>,
	pub(crate) handle: EnvoyHandle,
	pub(crate) response_map: ResponseMap,
	pub(crate) ws_sender_map: WsSenderMap,
	pub(crate) sqlite_startup_map: SqliteStartupMap,
	pub(crate) sqlite_schema_version_map: SqliteSchemaVersionMap,
}

impl JsEnvoyHandle {
	pub fn new(
		runtime: Arc<Runtime>,
		handle: EnvoyHandle,
		response_map: ResponseMap,
		ws_sender_map: WsSenderMap,
		sqlite_startup_map: SqliteStartupMap,
		sqlite_schema_version_map: SqliteSchemaVersionMap,
	) -> Self {
		Self {
			runtime,
			handle,
			response_map,
			ws_sender_map,
			sqlite_startup_map,
			sqlite_schema_version_map,
		}
	}

	pub async fn clone_sqlite_schema_version(&self, actor_id: &str) -> Option<u32> {
		self.sqlite_schema_version_map
			.lock()
			.await
			.get(actor_id)
			.copied()
	}

	pub async fn clone_sqlite_startup_data(
		&self,
		actor_id: &str,
	) -> Option<protocol::SqliteStartupData> {
		self.sqlite_startup_map.lock().await.get(actor_id).cloned()
	}
}

#[napi]
impl JsEnvoyHandle {
	// -- Lifecycle --

	#[napi]
	pub async fn started(&self) -> napi::Result<()> {
		let handle = self.handle.clone();
		self.runtime
			.spawn(async move { handle.started().await })
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))
	}

	#[napi]
	pub fn shutdown(&self, immediate: bool) {
		self.handle.shutdown(immediate);
	}

	#[napi(getter)]
	pub fn envoy_key(&self) -> String {
		self.handle.get_envoy_key().to_string()
	}

	// -- Actor lifecycle --

	#[napi]
	pub fn sleep_actor(&self, actor_id: String, generation: Option<u32>) {
		self.handle.sleep_actor(actor_id, generation);
	}

	#[napi]
	pub fn stop_actor(&self, actor_id: String, generation: Option<u32>, error: Option<String>) {
		self.handle.stop_actor(actor_id, generation, error);
	}

	#[napi]
	pub fn destroy_actor(&self, actor_id: String, generation: Option<u32>) {
		self.handle.destroy_actor(actor_id, generation);
	}

	#[napi]
	pub fn set_alarm(&self, actor_id: String, alarm_ts: Option<i64>, generation: Option<u32>) {
		self.handle.set_alarm(actor_id, alarm_ts, generation);
	}

	// -- KV operations --

	#[napi]
	pub async fn kv_get(
		&self,
		actor_id: String,
		keys: Vec<Buffer>,
	) -> napi::Result<Vec<Option<Buffer>>> {
		let handle = self.handle.clone();
		let keys_vec: Vec<Vec<u8>> = keys.into_iter().map(|b| b.to_vec()).collect();
		let result = self
			.runtime
			.spawn(async move { handle.kv_get(actor_id, keys_vec).await })
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))?;
		Ok(result
			.into_iter()
			.map(|opt| opt.map(Buffer::from))
			.collect())
	}

	#[napi]
	pub async fn kv_put(&self, actor_id: String, entries: Vec<JsKvEntry>) -> napi::Result<()> {
		let handle = self.handle.clone();
		let kv_entries: Vec<(Vec<u8>, Vec<u8>)> = entries
			.into_iter()
			.map(|e| (e.key.to_vec(), e.value.to_vec()))
			.collect();
		self.runtime
			.spawn(async move { handle.kv_put(actor_id, kv_entries).await })
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))
	}

	#[napi]
	pub async fn kv_delete(&self, actor_id: String, keys: Vec<Buffer>) -> napi::Result<()> {
		let handle = self.handle.clone();
		let keys_vec: Vec<Vec<u8>> = keys.into_iter().map(|b| b.to_vec()).collect();
		self.runtime
			.spawn(async move { handle.kv_delete(actor_id, keys_vec).await })
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))
	}

	#[napi]
	pub async fn kv_delete_range(
		&self,
		actor_id: String,
		start: Buffer,
		end: Buffer,
	) -> napi::Result<()> {
		let handle = self.handle.clone();
		let start_vec = start.to_vec();
		let end_vec = end.to_vec();
		self.runtime
			.spawn(async move { handle.kv_delete_range(actor_id, start_vec, end_vec).await })
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))
	}

	#[napi]
	pub async fn kv_list_all(
		&self,
		actor_id: String,
		options: Option<JsKvListOptions>,
	) -> napi::Result<Vec<JsKvEntry>> {
		let handle = self.handle.clone();
		let reverse = options.as_ref().and_then(|o| o.reverse);
		let limit = options.as_ref().and_then(|o| o.limit).map(|l| l as u64);
		let result = self
			.runtime
			.spawn(async move { handle.kv_list_all(actor_id, reverse, limit).await })
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))?;
		Ok(result
			.into_iter()
			.map(|(k, v)| JsKvEntry {
				key: Buffer::from(k),
				value: Buffer::from(v),
			})
			.collect())
	}

	#[napi]
	pub async fn kv_list_range(
		&self,
		actor_id: String,
		start: Buffer,
		end: Buffer,
		exclusive: Option<bool>,
		options: Option<JsKvListOptions>,
	) -> napi::Result<Vec<JsKvEntry>> {
		let handle = self.handle.clone();
		let start_vec = start.to_vec();
		let end_vec = end.to_vec();
		let exclusive = exclusive.unwrap_or(false);
		let reverse = options.as_ref().and_then(|o| o.reverse);
		let limit = options.as_ref().and_then(|o| o.limit).map(|l| l as u64);
		let result = self
			.runtime
			.spawn(async move {
				handle
					.kv_list_range(actor_id, start_vec, end_vec, exclusive, reverse, limit)
					.await
			})
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))?;
		Ok(result
			.into_iter()
			.map(|(k, v)| JsKvEntry {
				key: Buffer::from(k),
				value: Buffer::from(v),
			})
			.collect())
	}

	#[napi]
	pub async fn kv_list_prefix(
		&self,
		actor_id: String,
		prefix: Buffer,
		options: Option<JsKvListOptions>,
	) -> napi::Result<Vec<JsKvEntry>> {
		let handle = self.handle.clone();
		let prefix_vec = prefix.to_vec();
		let reverse = options.as_ref().and_then(|o| o.reverse);
		let limit = options.as_ref().and_then(|o| o.limit).map(|l| l as u64);
		let result = self
			.runtime
			.spawn(async move {
				handle
					.kv_list_prefix(actor_id, prefix_vec, reverse, limit)
					.await
			})
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))?;
		Ok(result
			.into_iter()
			.map(|(k, v)| JsKvEntry {
				key: Buffer::from(k),
				value: Buffer::from(v),
			})
			.collect())
	}

	#[napi]
	pub async fn kv_drop(&self, actor_id: String) -> napi::Result<()> {
		let handle = self.handle.clone();
		self.runtime
			.spawn(async move { handle.kv_drop(actor_id).await })
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))
	}

	// -- Hibernation --

	#[napi]
	pub fn restore_hibernating_requests(
		&self,
		actor_id: String,
		requests: Vec<types::HibernatingRequestEntry>,
	) {
		let meta_entries: Vec<rivet_envoy_client::tunnel::HibernatingWebSocketMetadata> = requests
			.into_iter()
			.map(|r| {
				let mut gateway_id = [0u8; 4];
				let mut request_id = [0u8; 4];
				let gw_bytes = r.gateway_id.to_vec();
				let rq_bytes = r.request_id.to_vec();
				if gw_bytes.len() >= 4 {
					gateway_id.copy_from_slice(&gw_bytes[..4]);
				}
				if rq_bytes.len() >= 4 {
					request_id.copy_from_slice(&rq_bytes[..4]);
				}
				rivet_envoy_client::tunnel::HibernatingWebSocketMetadata {
					gateway_id,
					request_id,
					envoy_message_index: 0,
					rivet_message_index: 0,
					path: String::new(),
					headers: HashMap::new(),
				}
			})
			.collect();

		self.handle
			.restore_hibernating_requests(actor_id, meta_entries);
	}

	#[napi]
	pub fn send_hibernatable_web_socket_message_ack(
		&self,
		gateway_id: Buffer,
		request_id: Buffer,
		client_message_index: u32,
	) {
		let mut gw = [0u8; 4];
		let mut rq = [0u8; 4];
		let gw_bytes = gateway_id.to_vec();
		let rq_bytes = request_id.to_vec();
		if gw_bytes.len() >= 4 {
			gw.copy_from_slice(&gw_bytes[..4]);
		}
		if rq_bytes.len() >= 4 {
			rq.copy_from_slice(&rq_bytes[..4]);
		}
		self.handle
			.send_hibernatable_ws_message_ack(gw, rq, client_message_index as u16);
	}

	// -- WebSocket send --

	/// Send a message on an open WebSocket connection identified by messageIdHex.
	#[napi]
	pub async fn send_ws_message(
		&self,
		gateway_id: Buffer,
		request_id: Buffer,
		data: Buffer,
		binary: bool,
	) -> napi::Result<()> {
		let key = make_ws_key(&gateway_id, &request_id);
		let map = self.ws_sender_map.lock().await;
		if let Some(sender) = map.get(&key) {
			sender.send(data.to_vec(), binary);
		} else {
			// The sender can disappear during shutdown after the JavaScript
			// side has already observed the socket as closed. Treat this like
			// a best-effort send on a closed socket instead of surfacing an
			// unhandled rejection back into the actor runtime.
		}
		Ok(())
	}

	/// Close an open WebSocket connection.
	#[napi]
	pub async fn close_websocket(
		&self,
		gateway_id: Buffer,
		request_id: Buffer,
		code: Option<u32>,
		reason: Option<String>,
	) {
		let key = make_ws_key(&gateway_id, &request_id);
		let mut map = self.ws_sender_map.lock().await;
		if let Some(sender) = map.remove(&key) {
			sender.close(code.map(|c| c as u16), reason);
		}
	}

	// -- Serverless --

	#[napi]
	pub async fn start_serverless(&self, payload: Buffer) -> napi::Result<()> {
		let handle = self.handle.clone();
		let payload_vec = payload.to_vec();
		self.runtime
			.spawn(async move { handle.start_serverless_actor(&payload_vec).await })
			.await
			.map_err(|e| napi::Error::from_reason(e.to_string()))?
			.map_err(|e| napi::Error::from_reason(e.to_string()))
	}

	// -- Callback responses --

	#[napi]
	pub async fn respond_callback(
		&self,
		response_id: String,
		data: serde_json::Value,
	) -> napi::Result<()> {
		let mut map = self.response_map.lock().await;
		if let Some(tx) = map.remove(&response_id) {
			let _ = tx.send(data);
		}
		Ok(())
	}
}
