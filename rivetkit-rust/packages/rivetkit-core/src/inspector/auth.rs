use anyhow::Result;
use rivet_error::RivetError as RivetErrorDerive;
use serde::{Deserialize, Serialize};

use crate::ActorContext;

const INSPECTOR_TOKEN_KEY: [u8; 1] = [3];
const INSPECTOR_TOKEN_ENV: &str = "RIVET_INSPECTOR_TOKEN";

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
