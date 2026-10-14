//! Standards-shaped, public-only JWKS representation of Rivet-issued token verification keys.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Serialize;

use crate::key_ring_cache::{VerificationKeyEntry, VerificationKeyLifecycle};

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct JwkSet {
	pub keys: Vec<Jwk>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Jwk {
	pub kty: &'static str,
	pub crv: &'static str,
	#[serde(rename = "use")]
	pub public_key_use: &'static str,
	pub alg: &'static str,
	pub kid: String,
	pub x: String,
}

impl JwkSet {
	/// Includes keys needed for verification, never private signing material. A revoked key is
	/// removed from the authoritative ring, and a retired key disappears at its verification
	/// deadline; pending keys are published before they start signing.
	pub fn from_verification_keys(keys: &[VerificationKeyEntry], now_ms: i64) -> Self {
		let keys = keys
			.iter()
			.filter(|key| match &key.lifecycle {
				VerificationKeyLifecycle::Retiring {
					verify_until_ts, ..
				} => now_ms < *verify_until_ts,
				VerificationKeyLifecycle::Pending { .. }
				| VerificationKeyLifecycle::Active { .. } => true,
			})
			.map(|key| Jwk {
				kty: "OKP",
				crv: "Ed25519",
				public_key_use: "sig",
				alg: "EdDSA",
				kid: key.kid.clone(),
				x: URL_SAFE_NO_PAD.encode(&key.public_key),
			})
			.collect();
		Self { keys }
	}
}

#[cfg(test)]
#[path = "../tests/unit/jwks.rs"]
mod tests;
