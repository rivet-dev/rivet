//! Network actions: `vmFetch` routes an HTTP request to a service
//! listening on a guest loopback port via [`AgentOs::fetch`].

use std::collections::BTreeMap;

use agent_os_client::AgentOs;
use anyhow::Result;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Optional request shape for `vmFetch(port, url, options?)`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchOptions {
	#[serde(default)]
	pub method: Option<String>,
	#[serde(default)]
	pub headers: Option<BTreeMap<String, String>>,
	#[serde(default)]
	pub body: Option<Vec<u8>>,
}

/// JSON-serializable response returned to the TS client. `body` is wrapped
/// via `serde_bytes` so the rivetkit `JsonCompatAdapter` re-encodes it as
/// `["$Uint8Array", base64]`, which the TS client decodes back to a
/// `Uint8Array` (the shape the example's `TextDecoder` expects).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResponseDto {
	pub status: u16,
	pub headers: BTreeMap<String, String>,
	pub body: serde_bytes::ByteBuf,
}

/// `vmFetch(port, url, options?)` — port of [`AgentOs::fetch`].
pub async fn fetch(
	vm: &AgentOs,
	port: u16,
	url: &str,
	options: FetchOptions,
) -> Result<FetchResponseDto> {
	let method = options.method.as_deref().unwrap_or("GET");
	let mut builder = http::Request::builder().method(method).uri(url);
	if let Some(headers) = &options.headers {
		for (name, value) in headers {
			builder = builder.header(name.as_str(), value.as_str());
		}
	}
	let body = Bytes::from(options.body.unwrap_or_default());
	let request = builder.body(body)?;

	let response = vm.fetch(port, request).await?;
	let status = response.status().as_u16();
	let headers = response
		.headers()
		.iter()
		.map(|(name, value)| {
			(
				name.as_str().to_owned(),
				value.to_str().unwrap_or_default().to_owned(),
			)
		})
		.collect();
	let body = serde_bytes::ByteBuf::from(response.into_body().to_vec());
	Ok(FetchResponseDto {
		status,
		headers,
		body,
	})
}
