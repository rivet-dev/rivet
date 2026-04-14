use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

/// Configuration for starting the native envoy client.
#[napi(object)]
pub struct JsEnvoyConfig {
	pub endpoint: String,
	pub token: String,
	pub namespace: String,
	pub pool_name: String,
	pub version: u32,
	pub metadata: Option<serde_json::Value>,
	pub not_global: bool,
	/// Log level for the Rust tracing subscriber (e.g. "trace", "debug", "info", "warn", "error").
	/// Falls back to RIVET_LOG_LEVEL, then LOG_LEVEL, then RUST_LOG env vars. Defaults to "warn".
	pub log_level: Option<String>,
}

/// Options for KV list operations.
#[napi(object)]
pub struct JsKvListOptions {
	pub reverse: Option<bool>,
	pub limit: Option<i64>,
}

/// A key-value entry returned from KV list operations.
#[napi(object)]
pub struct JsKvEntry {
	pub key: Buffer,
	pub value: Buffer,
}

/// A single hibernating request entry.
#[napi(object)]
pub struct HibernatingRequestEntry {
	pub gateway_id: Buffer,
	pub request_id: Buffer,
	pub envoy_message_index: u16,
	pub rivet_message_index: u16,
	pub path: String,
	pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Encode a protocol MessageId into a 10-byte buffer.
pub fn encode_message_id(msg_id: &rivet_envoy_protocol::MessageId) -> Vec<u8> {
	let mut buf = Vec::with_capacity(10);
	buf.extend_from_slice(&msg_id.gateway_id);
	buf.extend_from_slice(&msg_id.request_id);
	buf.extend_from_slice(&msg_id.message_index.to_le_bytes());
	buf
}

/// Decode a 10-byte buffer into a protocol MessageId.
pub fn decode_message_id(buf: &[u8]) -> Option<rivet_envoy_protocol::MessageId> {
	if buf.len() < 10 {
		return None;
	}
	let mut gateway_id = [0u8; 4];
	let mut request_id = [0u8; 4];
	gateway_id.copy_from_slice(&buf[0..4]);
	request_id.copy_from_slice(&buf[4..8]);
	let message_index = u16::from_le_bytes([buf[8], buf[9]]);
	Some(rivet_envoy_protocol::MessageId {
		gateway_id,
		request_id,
		message_index,
	})
}
