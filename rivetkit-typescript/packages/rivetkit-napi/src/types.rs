use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

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
