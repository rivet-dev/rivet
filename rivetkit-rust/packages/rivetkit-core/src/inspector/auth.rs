use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use rivet_error::RivetError as RivetErrorDerive;
use serde::{Deserialize, Serialize};

use crate::ActorContext;

const INSPECTOR_TOKEN_KEY: [u8; 1] = [3];
const INSPECTOR_TOKEN_ENV: &str = "RIVET_INSPECTOR_TOKEN";
const INSPECTOR_TOKEN_BYTES: usize = 32;

#[derive(Clone, Copy, Debug, Default)]
pub struct InspectorAuth;

#[derive(RivetErrorDerive, Clone, Debug, Deserialize, Serialize)]
#[error(
	"inspector",
	"unauthorized",
	"Inspector request requires a valid bearer token"
)]
struct InspectorUnauthorized;

impl InspectorAuth {
	pub fn new() -> Self {
		Self
	}

	pub async fn verify(&self, ctx: &ActorContext, bearer_token: Option<&str>) -> Result<()> {
		let Some(bearer_token) = bearer_token.filter(|token| !token.is_empty()) else {
			return Err(InspectorUnauthorized.build());
		};

		if let Some(configured_token) = std::env::var(INSPECTOR_TOKEN_ENV)
			.ok()
			.filter(|token| !token.is_empty())
		{
			return verify_token_bytes(bearer_token.as_bytes(), configured_token.as_bytes());
		}

		let stored_token = ctx
			.kv()
			.get(&INSPECTOR_TOKEN_KEY)
			.await
			.ok()
			.flatten()
			.ok_or_else(|| InspectorUnauthorized.build())?;

		verify_token_bytes(bearer_token.as_bytes(), &stored_token)
	}
}

/// Ensures the actor has an inspector token persisted in KV at `[3]` so the
/// engine-facing KV API can serve the token to the dashboard inspector.
/// Skips the write when the token already exists. No-ops when the
/// `RIVET_INSPECTOR_TOKEN` env override is set, since that takes precedence
/// over any KV-stored token and we do not want to pin a per-actor token that
/// will never be consulted.
pub async fn init_inspector_token(ctx: &ActorContext) -> Result<()> {
	if std::env::var(INSPECTOR_TOKEN_ENV)
		.ok()
		.is_some_and(|token| !token.is_empty())
	{
		return Ok(());
	}

	let existing = ctx
		.kv()
		.get(&INSPECTOR_TOKEN_KEY)
		.await
		.context("load inspector token")?;
	if existing.is_some() {
		return Ok(());
	}

	let token = generate_inspector_token();
	ctx.kv()
		.put(&INSPECTOR_TOKEN_KEY, token.as_bytes())
		.await
		.context("persist inspector token")?;
	tracing::debug!(actor_id = %ctx.actor_id(), "generated new inspector token");
	Ok(())
}

fn generate_inspector_token() -> String {
	let mut bytes = [0u8; INSPECTOR_TOKEN_BYTES];
	rand::thread_rng().fill_bytes(&mut bytes);
	URL_SAFE_NO_PAD.encode(bytes)
}

fn verify_token_bytes(candidate: &[u8], expected: &[u8]) -> Result<()> {
	if timing_safe_equal(candidate, expected) {
		Ok(())
	} else {
		Err(InspectorUnauthorized.build())
	}
}

fn timing_safe_equal(left: &[u8], right: &[u8]) -> bool {
	let max_len = left.len().max(right.len());
	let mut diff = left.len() ^ right.len();

	for idx in 0..max_len {
		let left_byte = left.get(idx).copied().unwrap_or_default();
		let right_byte = right.get(idx).copied().unwrap_or_default();
		diff |= usize::from(left_byte ^ right_byte);
	}

	diff == 0
}
