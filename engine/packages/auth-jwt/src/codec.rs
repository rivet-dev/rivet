//! Encodes and decodes Rivet access tokens as compact JWTs. The JOSE header and claims remain
//! JSON; only `rivet_grants` contains base64url-encoded versioned BARE handled by `crate::grants`.

use std::{cell::Cell, str::FromStr};

use base64::{
	Engine as _,
	engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use jsonwebtoken::{Algorithm, Header, Validation, errors::ErrorKind};
use rivet_auth_policy::OwnedGrant;
use serde::{Deserialize, de};

use crate::{
	ALGORITHM, CLAIMS_VERSION, Claims, Id, KeyId, PROTOCOL_CLOCK_SKEW, PROTOCOL_MAX_TTL,
	SigningKey, TOKEN_TYPE, TokenError, TokenId, VerificationKey, decode_grants,
};

pub const MAX_TOKEN_BYTES: usize = 6_144;
const MAX_HEADER_BYTES: usize = 512;
const MAX_HEADER_SEGMENT_BYTES: usize = 683;
const MAX_PAYLOAD_BYTES: usize = 4_608;
const SIGNATURE_BYTES: usize = 64;
const MAX_SIGNATURE_SEGMENT_BYTES: usize = 86;
pub const MAX_SUBJECT_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenHeader {
	pub kid: KeyId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedToken {
	pub claims: Claims,
	pub namespace_id: Id,
	pub grants: Vec<OwnedGrant<Id, Id>>,
}

#[derive(Clone, Copy, Debug)]
pub struct DecodeOptions<'a> {
	pub issuer: &'a str,
	pub audience: &'a str,
	pub now: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictHeader {
	alg: String,
	kid: String,
	typ: String,
}

pub fn encode(key: &SigningKey, claims: &Claims) -> Result<String, TokenError> {
	validate_claims(claims, None)?;

	let mut header = Header::new(Algorithm::EdDSA);
	header.typ = Some(TOKEN_TYPE.to_owned());
	header.kid = Some(key.kid().to_string());

	let token = jsonwebtoken::encode(&header, claims, &key.encoding_key()?)
		.map_err(|_| TokenError::InvalidKey)?;
	if token.len() > MAX_TOKEN_BYTES {
		return Err(TokenError::TokenTooLarge);
	}

	Ok(token)
}

pub fn decode(
	token: &str,
	key: &VerificationKey,
	options: DecodeOptions<'_>,
) -> Result<DecodedToken, TokenError> {
	let header = peek_header(token)?;
	if header.kid != key.kid() {
		return Err(TokenError::InvalidKeyId);
	}

	let mut validation = Validation::new(Algorithm::EdDSA);
	validation.required_spec_claims.clear();
	validation.validate_exp = false;
	validation.validate_aud = false;
	validation.leeway = 0;

	let token_data = jsonwebtoken::decode::<Claims>(token, &key.decoding_key(), &validation)
		.map_err(|error| match error.kind() {
			ErrorKind::InvalidSignature => TokenError::InvalidSignature,
			ErrorKind::Json(_) | ErrorKind::InvalidClaimFormat(_) => TokenError::InvalidClaims,
			_ => TokenError::InvalidEncoding,
		})?;

	let (namespace_id, grants) = validate_claims(&token_data.claims, Some(options))?;
	Ok(DecodedToken {
		claims: token_data.claims,
		namespace_id,
		grants,
	})
}

pub fn peek_header(token: &str) -> Result<TokenHeader, TokenError> {
	let segments = decode_segments(token)?;
	let header: StrictHeader =
		serde_json::from_slice(&segments.header).map_err(|_| TokenError::InvalidHeader)?;
	if header.alg != ALGORITHM || header.typ != TOKEN_TYPE {
		return Err(TokenError::InvalidHeader);
	}

	Ok(TokenHeader {
		kid: KeyId::from_str(&header.kid)?,
	})
}

pub fn is_reserved_token(token: &str) -> bool {
	let bytes = token.as_bytes();
	let Some(first_dot) = bytes
		.iter()
		.take(MAX_HEADER_SEGMENT_BYTES + 1)
		.position(|byte| *byte == b'.')
	else {
		return false;
	};
	if first_dot == 0
		|| !bytes[first_dot + 1..]
			.iter()
			.take(MAX_TOKEN_BYTES + 1)
			.any(|byte| *byte == b'.')
	{
		return false;
	}

	let header_segment = &bytes[..first_dot];
	let unpadded_header = header_segment
		.strip_suffix(b"==")
		.or_else(|| header_segment.strip_suffix(b"="))
		.unwrap_or(header_segment);
	if unpadded_header.contains(&b'=') {
		return false;
	}
	let Ok(header) = URL_SAFE_NO_PAD
		.decode(unpadded_header)
		.or_else(|_| URL_SAFE.decode(header_segment))
	else {
		return false;
	};
	if header.len() > MAX_HEADER_BYTES {
		return false;
	}

	let reserved = Cell::new(false);
	let mut deserializer = serde_json::Deserializer::from_slice(&header);
	let _ = de::Deserializer::deserialize_map(&mut deserializer, TypVisitor(&reserved));
	reserved.get()
}

struct TypVisitor<'a>(&'a Cell<bool>);

impl<'de> de::Visitor<'de> for TypVisitor<'_> {
	type Value = ();

	fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str("a JOSE header object")
	}

	fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
	where
		M: de::MapAccess<'de>,
	{
		while let Some(key) = map.next_key::<String>()? {
			if key == "typ" {
				let value = map.next_value::<serde_json::Value>()?;
				if value.as_str() == Some(TOKEN_TYPE) {
					self.0.set(true);
				}
			} else {
				map.next_value::<de::IgnoredAny>()?;
			}
		}

		Ok(())
	}
}

struct DecodedSegments {
	header: Vec<u8>,
}

fn decode_segments(token: &str) -> Result<DecodedSegments, TokenError> {
	if token.len() > MAX_TOKEN_BYTES {
		return Err(TokenError::TokenTooLarge);
	}

	let mut segments = token.split('.');
	let (Some(header), Some(payload), Some(signature), None) = (
		segments.next(),
		segments.next(),
		segments.next(),
		segments.next(),
	) else {
		return Err(TokenError::InvalidEncoding);
	};
	if header.is_empty() || payload.is_empty() || signature.is_empty() {
		return Err(TokenError::InvalidEncoding);
	}

	let header = decode_segment(header, MAX_HEADER_SEGMENT_BYTES, MAX_HEADER_BYTES)?;
	decode_segment(payload, MAX_TOKEN_BYTES, MAX_PAYLOAD_BYTES)?;
	let signature = decode_segment(signature, MAX_SIGNATURE_SEGMENT_BYTES, SIGNATURE_BYTES)?;
	if signature.len() != SIGNATURE_BYTES {
		return Err(TokenError::InvalidEncoding);
	}

	Ok(DecodedSegments { header })
}

fn decode_segment(
	segment: &str,
	max_encoded_bytes: usize,
	max_decoded_bytes: usize,
) -> Result<Vec<u8>, TokenError> {
	if segment.len() > max_encoded_bytes || segment.contains('=') {
		return Err(TokenError::InvalidEncoding);
	}

	let decoded = URL_SAFE_NO_PAD
		.decode(segment)
		.map_err(|_| TokenError::InvalidEncoding)?;
	if decoded.len() > max_decoded_bytes || URL_SAFE_NO_PAD.encode(&decoded) != segment {
		return Err(TokenError::InvalidEncoding);
	}

	Ok(decoded)
}

fn validate_claims(
	claims: &Claims,
	options: Option<DecodeOptions<'_>>,
) -> Result<(Id, Vec<OwnedGrant<Id, Id>>), TokenError> {
	if claims.rivet_ver != CLAIMS_VERSION
		|| claims.iss.is_empty()
		|| claims.aud.is_empty()
		|| claims
			.sub
			.as_ref()
			.is_some_and(|sub| sub.len() > MAX_SUBJECT_BYTES)
		|| TokenId::from_str(&claims.jti).is_err()
	{
		return Err(TokenError::InvalidClaims);
	}

	let lifetime = claims
		.exp
		.checked_sub(claims.iat)
		.ok_or(TokenError::InvalidClaims)?;
	if lifetime == 0 {
		return Err(TokenError::InvalidClaims);
	}
	if lifetime > PROTOCOL_MAX_TTL {
		return Err(TokenError::LifetimeTooLong);
	}

	if let Some(options) = options {
		if claims.iss != options.issuer || claims.aud != options.audience {
			return Err(TokenError::InvalidClaims);
		}
		if claims.iat > options.now.saturating_add(PROTOCOL_CLOCK_SKEW) {
			return Err(TokenError::IssuedInFuture);
		}
		if options.now >= claims.exp.saturating_add(PROTOCOL_CLOCK_SKEW) {
			return Err(TokenError::Expired);
		}
	}

	let namespace_id = Id::from_str(&claims.rivet_ns).map_err(|_| TokenError::InvalidClaims)?;
	let grants = decode_grants(namespace_id, &claims.rivet_grants)?;
	Ok((namespace_id, grants))
}
