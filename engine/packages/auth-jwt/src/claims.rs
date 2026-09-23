use serde::{Deserialize, Serialize};

pub const CLAIMS_VERSION: u16 = 1;
pub const PROTOCOL_MAX_TTL: u64 = 86_400;
pub const PROTOCOL_CLOCK_SKEW: u64 = 30;

/// Rivet's versioned access-token claims profile.
///
/// Claims are intentionally strict and fail closed. Deploy verifiers that understand a new
/// `rivet_ver` before any issuer starts emitting it; additive claim changes use the same
/// verifier-first rollout because unknown fields are rejected.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
	pub rivet_ver: u16,
	pub iss: String,
	pub aud: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sub: Option<String>,
	pub iat: u64,
	pub exp: u64,
	pub jti: String,
	pub rivet_ns: String,
	pub rivet_grants: String,
}
