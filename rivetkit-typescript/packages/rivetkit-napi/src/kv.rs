use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::{Kv as CoreKv, ListOpts};

use crate::napi_error;
use crate::types::{JsKvEntry, JsKvListOptions};

#[napi]
pub struct Kv {
	inner: CoreKv,
}

impl Kv {
	pub(crate) fn new(inner: CoreKv) -> Self {
		Self { inner }
	}
}

#[napi]
impl Kv {
	#[napi]
	pub async fn get(&self, key: Buffer) -> napi::Result<Option<Buffer>> {
		self.inner
			.get(key.as_ref())
			.await
			.map(|value| value.map(Buffer::from))
			.map_err(napi_error)
	}

	#[napi]
	pub async fn put(&self, key: Buffer, value: Buffer) -> napi::Result<()> {
		self.inner
			.put(key.as_ref(), value.as_ref())
			.await
			.map_err(napi_error)
	}

	#[napi]
	pub async fn delete(&self, key: Buffer) -> napi::Result<()> {
		self.inner.delete(key.as_ref()).await.map_err(napi_error)
	}

	#[napi]
	pub async fn delete_range(&self, start: Buffer, end: Buffer) -> napi::Result<()> {
		self.inner
			.delete_range(start.as_ref(), end.as_ref())
			.await
			.map_err(napi_error)
	}

	#[napi]
	pub async fn list_prefix(
		&self,
		prefix: Buffer,
		options: Option<JsKvListOptions>,
	) -> napi::Result<Vec<JsKvEntry>> {
		self.inner
			.list_prefix(prefix.as_ref(), list_opts(options)?)
			.await
			.map(|entries| {
				entries
					.into_iter()
					.map(|(key, value)| JsKvEntry {
						key: Buffer::from(key),
						value: Buffer::from(value),
					})
					.collect()
			})
			.map_err(napi_error)
	}

	#[napi]
	pub async fn list_range(
		&self,
		start: Buffer,
		end: Buffer,
		options: Option<JsKvListOptions>,
	) -> napi::Result<Vec<JsKvEntry>> {
		self.inner
			.list_range(start.as_ref(), end.as_ref(), list_opts(options)?)
			.await
			.map(|entries| {
				entries
					.into_iter()
					.map(|(key, value)| JsKvEntry {
						key: Buffer::from(key),
						value: Buffer::from(value),
					})
					.collect()
			})
			.map_err(napi_error)
	}

	#[napi]
	pub async fn batch_get(&self, keys: Vec<Buffer>) -> napi::Result<Vec<Option<Buffer>>> {
		let key_refs: Vec<&[u8]> = keys.iter().map(Buffer::as_ref).collect();
		self.inner
			.batch_get(&key_refs)
			.await
			.map(|values| values.into_iter().map(|value| value.map(Buffer::from)).collect())
			.map_err(napi_error)
	}

	#[napi]
	pub async fn batch_put(&self, entries: Vec<JsKvEntry>) -> napi::Result<()> {
		let entry_refs: Vec<(&[u8], &[u8])> = entries
			.iter()
			.map(|entry| (entry.key.as_ref(), entry.value.as_ref()))
			.collect();
		self.inner.batch_put(&entry_refs).await.map_err(napi_error)
	}

	#[napi]
	pub async fn batch_delete(&self, keys: Vec<Buffer>) -> napi::Result<()> {
		let key_refs: Vec<&[u8]> = keys.iter().map(Buffer::as_ref).collect();
		self.inner.batch_delete(&key_refs).await.map_err(napi_error)
	}
}

fn list_opts(options: Option<JsKvListOptions>) -> napi::Result<ListOpts> {
	let reverse = options
		.as_ref()
		.and_then(|options| options.reverse)
		.unwrap_or(false);
	let limit = match options.and_then(|options| options.limit) {
		Some(limit) if limit < 0 => {
			return Err(napi::Error::from_reason(
				"kv list limit must be non-negative",
			));
		}
		Some(limit) => Some(u32::try_from(limit).map_err(|_| {
			napi::Error::from_reason("kv list limit exceeds u32 range")
		})?),
		None => None,
	};

	Ok(ListOpts { reverse, limit })
}
