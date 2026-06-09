//! Network actions. Currently exposes `vmFetch` — a host-side fetch
//! against an HTTP server listening on a port INSIDE the VM. The body
//! travels back as raw bytes (wrapped via `serde_bytes` so the
//! rivetkit `JsonCompatAdapter` re-wraps it as `["$Uint8Array", base64]`
//! for JSON encoders).

use agent_os_client::AgentOs;
use anyhow::Result;
use http::Request;
use serde::Serialize;

/// `vmFetch(port, url)` — port of [`AgentOs::fetch`] for the
/// driver-test surface (GET against the VM-side server). The url's
/// host/authority is discarded by the agent-os-client; only the
/// path+query reaches the guest server.
pub async fn vm_fetch(vm: &AgentOs, port: u16, url: &str) -> Result<VmFetchReplyDto> {
	let request = Request::builder()
		.method(http::Method::GET)
		.uri(url)
		.body(bytes::Bytes::new())?;
	let response = vm.fetch(port, request).await?;
	let (parts, body) = response.into_parts();
	Ok(VmFetchReplyDto {
		status: parts.status.as_u16(),
		body: serde_bytes::ByteBuf::from(body.to_vec()),
	})
}

/// Reply for `vmFetch`. `status` is the HTTP status code; `body` is
/// the raw bytes the VM-side server returned, which the rivetkit
/// encoder will wrap as `["$Uint8Array", base64]` on the wire so the
/// TS client can recover a `Uint8Array`.
#[derive(Serialize)]
pub struct VmFetchReplyDto {
	pub status: u16,
	pub body: serde_bytes::ByteBuf,
}

