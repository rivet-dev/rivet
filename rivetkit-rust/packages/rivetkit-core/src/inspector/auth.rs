use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use parking_lot::RwLock;
use rand::RngCore;
use rivet_error::RivetError as RivetErrorDerive;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::Mutex;
use std::sync::OnceLock;
use subtle::ConstantTimeEq;

use crate::ActorContext;
use crate::actor::keys::INSPECTOR_TOKEN_KEY;
use crate::actor::preload::PreloadedKv;

/// Test-only override. Not a public/production auth mechanism; production
/// inspector auth goes through the per-actor KV token.
const INSPECTOR_TOKEN_ENV: &str = "_RIVET_TEST_INSPECTOR_TOKEN";
const INSPECTOR_TOKEN_BYTES: usize = 32;

static INSPECTOR_TEST_TOKEN_OVERRIDE: OnceLock<RwLock<Option<String>>> = OnceLock::new();

#[cfg(test)]
pub(crate) fn test_inspector_env_lock() -> &'static Mutex<()> {
	static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
	LOCK.get_or_init(|| Mutex::new(()))
}

pub fn set_test_inspector_token_override(token: Option<String>) {
	*INSPECTOR_TEST_TOKEN_OVERRIDE
		.get_or_init(|| RwLock::new(None))
		.write() = token.filter(|token| !token.is_empty());
}

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

		if let Some(configured_token) = configured_test_token() {
			return verify_token_bytes(bearer_token.as_bytes(), configured_token.as_bytes());
		}

		let stored_token = ctx
			.kv_internal()
			.get(&INSPECTOR_TOKEN_KEY)
			.await
			.ok()
			.flatten()
			.ok_or_else(|| InspectorUnauthorized.build())?;

		verify_token_bytes(bearer_token.as_bytes(), &stored_token)
	}
}

/// Ensures the actor has an inspector token persisted in KV so the
/// engine-facing KV API can serve the token to the dashboard inspector.
/// Skips the write when the token already exists. No-ops when the
/// `_RIVET_TEST_INSPECTOR_TOKEN` env override is set, since that takes
/// precedence over any KV-stored token and we do not want to pin a per-actor
/// token that will never be consulted.
pub async fn init_inspector_token(ctx: &ActorContext) -> Result<()> {
	init_inspector_token_with_preload(ctx, None).await
}

pub(crate) async fn init_inspector_token_with_preload(
	ctx: &ActorContext,
	preloaded_kv: Option<&PreloadedKv>,
) -> Result<()> {
	if configured_test_token().is_some() {
		return Ok(());
	}

	let existing =
		match preloaded_kv.and_then(|preloaded| preloaded.key_entry(&INSPECTOR_TOKEN_KEY)) {
			Some(existing) => existing,
			None => ctx
				.kv_internal()
				.get(&INSPECTOR_TOKEN_KEY)
				.await
				.context("load inspector token")?,
		};
	if existing.is_some() {
		return Ok(());
	}

	let token = generate_inspector_token();
	ctx.kv_internal()
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

fn configured_test_token() -> Option<String> {
	std::env::var(INSPECTOR_TOKEN_ENV)
		.ok()
		.filter(|token| !token.is_empty())
		.or_else(|| {
			INSPECTOR_TEST_TOKEN_OVERRIDE
				.get()
				.and_then(|token| token.read().clone())
		})
}

fn verify_token_bytes(candidate: &[u8], expected: &[u8]) -> Result<()> {
	if candidate.ct_eq(expected).into() {
		Ok(())
	} else {
		Err(InspectorUnauthorized.build())
	}
}
