//! Preview URL actions. These are a rivetkit-actor-layer feature, not part
//! of the core `AgentOs` API: they issue a signed, time-limited token that
//! maps an external request path to a guest loopback port. The actor's HTTP
//! event handler (`crate::run`) proxies `/preview/{token}/...` requests to
//! that port via [`agent_os_client::AgentOs::fetch`].
//!
//! Tokens are persisted to the actor's SQLite database (`agent_os_preview_tokens`)
//! via `ctx.db_*`, so issued previews survive actor sleep/wake.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rivetkit::Ctx;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::actor::AgentOsActor;
use crate::persistence::{query_rows, run_stmt};

/// Default lifetime of a signed preview URL: one hour.
const PREVIEW_TTL_MS: i64 = 60 * 60 * 1000;

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

/// Issue a signed preview URL for `port`, valid for `ttl_seconds` (falling back
/// to [`PREVIEW_TTL_MS`] when the caller passes `0`).
pub async fn create(
	ctx: &Ctx<AgentOsActor>,
	port: u16,
	ttl_seconds: u64,
) -> Result<SignedPreviewUrlDto> {
	let token = Uuid::new_v4().to_string();
	let created_at = now_ms();
	let ttl_ms = if ttl_seconds == 0 {
		PREVIEW_TTL_MS
	} else {
		(ttl_seconds as i64).saturating_mul(1000)
	};
	let expires_at = created_at + ttl_ms;
	run_stmt(
		ctx,
		"INSERT INTO agent_os_preview_tokens (token, port, created_at, expires_at) \
		 VALUES (?, ?, ?, ?)",
		&[
			json!(token),
			json!(port),
			json!(created_at),
			json!(expires_at),
		],
	)
	.await?;
	Ok(SignedPreviewUrlDto {
		// The `/request` prefix routes through rivetkit's raw-actor-HTTP path
		// (RegistryHttpRoute::UserRawRequest); the gateway strips `/request`
		// before the actor sees it, so `proxy_preview` receives `/preview/<token>`.
		// Without this prefix the gateway classifies the path as NotFound (404).
		path: format!("/request/preview/{token}"),
		token,
		port,
		expires_at: expires_at as f64,
	})
}

/// Revoke a previously issued preview token. Idempotent.
pub async fn expire(ctx: &Ctx<AgentOsActor>, token: &str) -> Result<()> {
	run_stmt(
		ctx,
		"DELETE FROM agent_os_preview_tokens WHERE token = ?",
		&[json!(token)],
	)
	.await
}

/// Resolve `token` to its target port if it exists and has not expired.
/// Expired tokens are pruned as a side effect.
pub async fn resolve(ctx: &Ctx<AgentOsActor>, token: &str) -> Result<Option<u16>> {
	let rows = query_rows(
		ctx,
		"SELECT port, expires_at FROM agent_os_preview_tokens WHERE token = ?",
		&[json!(token)],
	)
	.await?;
	let Some(row) = rows.into_iter().next() else {
		return Ok(None);
	};
	let expires_at = row.get("expires_at").and_then(|v| v.as_i64()).unwrap_or(0);
	let port = row.get("port").and_then(|v| v.as_i64()).unwrap_or(0) as u16;
	if expires_at <= now_ms() {
		expire(ctx, token).await?;
		return Ok(None);
	}
	Ok(Some(port))
}
