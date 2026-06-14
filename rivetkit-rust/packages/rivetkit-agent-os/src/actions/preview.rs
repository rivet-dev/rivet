//! Preview URL actions. These are a rivetkit-actor-layer feature, not part
//! of the core `AgentOs` API: they issue a signed, time-limited token that
//! maps an external request path to a guest loopback port. The actor's HTTP
//! event handler (`crate::run`) proxies `/preview/{token}/...` requests to
//! that port via [`agent_os_client::AgentOs::fetch`].

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use uuid::Uuid;

/// Default lifetime of a signed preview URL: one hour.
const PREVIEW_TTL_MS: i64 = 60 * 60 * 1000;

/// One issued preview token: the guest port it grants access to and the
/// epoch-millis instant after which it is no longer valid.
#[derive(Debug, Clone, Copy)]
pub struct PreviewEntry {
	pub port: u16,
	pub expires_at: i64,
}

/// Per-actor table of live preview tokens. Owned by the run loop and mutated
/// only from the single-threaded event dispatch, so no locking is required.
pub type PreviewStore = HashMap<String, PreviewEntry>;

/// `{ path, token, port, expiresAt }` returned by `createSignedPreviewUrl`.
///
/// `expires_at` is an epoch-millis timestamp serialized as `f64` so it
/// crosses the napi boundary as a JS `number` (not a `BigInt`), matching the
/// core API and the example's `new Date(expiresAt)` usage. Millisecond
/// timestamps are exactly representable in `f64` well past the year 10000.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedPreviewUrlDto {
	pub path: String,
	pub token: String,
	pub port: u16,
	pub expires_at: f64,
}

fn now_ms() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_millis() as i64)
		.unwrap_or(0)
}

/// Issue a signed preview URL for `port`, valid for [`PREVIEW_TTL_MS`].
pub fn create(store: &mut PreviewStore, port: u16) -> SignedPreviewUrlDto {
	let token = Uuid::new_v4().to_string();
	let expires_at = now_ms() + PREVIEW_TTL_MS;
	store.insert(token.clone(), PreviewEntry { port, expires_at });
	SignedPreviewUrlDto {
		path: format!("/preview/{token}"),
		token,
		port,
		expires_at: expires_at as f64,
	}
}

/// Revoke a previously issued preview token. Idempotent.
pub fn expire(store: &mut PreviewStore, token: &str) {
	store.remove(token);
}

/// Resolve `token` to its target port if it exists and has not expired.
/// Expired tokens are pruned as a side effect.
pub fn resolve(store: &mut PreviewStore, token: &str) -> Option<u16> {
	let entry = *store.get(token)?;
	if entry.expires_at <= now_ms() {
		store.remove(token);
		return None;
	}
	Some(entry.port)
}
