//! The downstream-only JWT profile. Application data never becomes Engine authority.
use jsonwebtoken::{Algorithm, Header};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{MAX_SUBJECT_BYTES, MAX_TOKEN_BYTES, PROTOCOL_MAX_TTL, SigningKey, TokenError};

pub const TOKEN_TYPE: &str = "rivet-app+jwt";
pub const MAX_APPLICATION_CLAIMS_BYTES: usize = 3_072;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
	pub rivet_ver: u16,
	pub rivet_purpose: String,
	pub iss: String,
	pub aud: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sub: Option<String>,
	pub iat: u64,
	pub exp: u64,
	pub jti: String,
	pub rivet_ns: String,
	pub app: Map<String, Value>,
}

impl std::fmt::Debug for Claims {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("ApplicationClaims { [redacted] }")
	}
}

pub fn validate_application_data(
	audience: &str,
	purpose: &str,
	subject: Option<&str>,
	app: &Map<String, Value>,
	engine_audience: &str,
) -> Result<(), TokenError> {
	if audience.trim().is_empty()
		|| audience.len() > 256
		|| audience == "rivet-api"
		|| audience == engine_audience
		|| purpose.trim().is_empty()
		|| purpose.len() > 128
		|| reserved_prefix(purpose)
		|| subject.is_some_and(|s| s.len() > MAX_SUBJECT_BYTES)
		|| app.len() > 8
		|| app.iter().any(|(name, value)| {
			!valid_namespace(name) || !value.is_object() || !within_depth(value, 2)
		}) {
		return Err(TokenError::InvalidClaims);
	}
	if serde_json::to_vec(app)
		.map_err(|_| TokenError::InvalidClaims)?
		.len() > MAX_APPLICATION_CLAIMS_BYTES
	{
		return Err(TokenError::TokenTooLarge);
	}
	Ok(())
}

fn reserved_prefix(name: &str) -> bool {
	let lower = name.to_ascii_lowercase();
	lower.starts_with("rivet") || lower.starts_with("engine")
}

fn valid_namespace(name: &str) -> bool {
	!name.is_empty()
		&& name.len() <= 64
		&& !reserved_prefix(name)
		&& ![
			"iss", "sub", "aud", "exp", "nbf", "iat", "jti", "app", "typ", "alg", "kid",
		]
		.contains(&name.to_ascii_lowercase().as_str())
		&& name
			.bytes()
			.all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}

fn within_depth(value: &Value, depth: usize) -> bool {
	match value {
		Value::Object(values) => depth <= 8 && values.values().all(|v| within_depth(v, depth + 1)),
		Value::Array(values) => depth <= 8 && values.iter().all(|v| within_depth(v, depth + 1)),
		_ => true,
	}
}

pub fn encode(
	key: &SigningKey,
	claims: &Claims,
	engine_audience: &str,
) -> Result<String, TokenError> {
	validate_application_data(
		&claims.aud,
		&claims.rivet_purpose,
		claims.sub.as_deref(),
		&claims.app,
		engine_audience,
	)?;
	if claims.rivet_ver != 1
		|| claims.iss.is_empty()
		|| claims.iss.len() > 512
		|| claims.rivet_ns.parse::<crate::Id>().is_err()
		|| claims.jti.parse::<crate::TokenId>().is_err()
		|| claims.exp <= claims.iat
		|| claims.exp - claims.iat > PROTOCOL_MAX_TTL
	{
		return Err(TokenError::InvalidClaims);
	}
	let mut header = Header::new(Algorithm::EdDSA);
	header.typ = Some(TOKEN_TYPE.into());
	header.kid = Some(key.kid().to_string());
	let token = jsonwebtoken::encode(&header, claims, &key.encoding_key()?)
		.map_err(|_| TokenError::InvalidKey)?;
	if token.len() > MAX_TOKEN_BYTES {
		return Err(TokenError::TokenTooLarge);
	}
	Ok(token)
}

#[cfg(feature = "service")]
pub mod service {
	use super::*;
	use anyhow::{Result, ensure};
	use rand::rngs::OsRng;

	pub struct IssueRequest {
		pub namespace_id: crate::Id,
		pub audience: String,
		pub purpose: String,
		pub subject: Option<String>,
		pub app: Map<String, Value>,
		pub expires_no_later_than_ts: i64,
	}

	#[derive(Serialize)]
	pub struct IssueResponse {
		pub token: String,
		pub issued_ts: i64,
		pub expires_ts: i64,
	}

	impl crate::key_ring_cache::KeyRingCache {
		pub async fn issue_application(
			&self,
			request: &IssueRequest,
			max_duration_secs: u64,
		) -> Result<IssueResponse> {
			ensure!(
				(1..=PROTOCOL_MAX_TTL).contains(&max_duration_secs),
				"invalid application token maximum duration"
			);
			self.with_active_signer(|signer| {
				let iat = u64::try_from(signer.issued_ts / 1000)?;
				let exp = u64::try_from(request.expires_no_later_than_ts / 1000)?
					.min(iat + max_duration_secs);
				let claims = Claims {
					rivet_ver: 1,
					rivet_purpose: request.purpose.clone(),
					iss: signer.issuer.to_owned(),
					aud: request.audience.clone(),
					sub: request.subject.clone(),
					iat,
					exp,
					jti: crate::TokenId::generate(&mut OsRng).to_string(),
					rivet_ns: request.namespace_id.to_string(),
					app: request.app.clone(),
				};
				let token = encode(signer.key, &claims, signer.audience)?;
				Ok(IssueResponse {
					token,
					issued_ts: signer.issued_ts,
					expires_ts: i64::try_from(exp)? * 1000,
				})
			})
			.await
		}
	}
}

#[cfg(test)]
#[path = "../tests/unit/application.rs"]
mod tests;
