use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer as _, SigningKey as DalekSigningKey, pkcs8::EncodePrivateKey};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use rivet_auth_jwt::{
	CLAIMS_VERSION, Claims, DecodeOptions, Id, KeyId, MAX_TOKEN_BYTES, SigningKey, TOKEN_TYPE,
	TokenError, TokenId, ValidatedGrantSet, VerificationKey, decode, encode, encode_grants,
	is_reserved_token, peek_header,
};
use rivet_auth_policy::{OperationKind, OwnedGrant, ResourceKind, Scope};

const NOW: u64 = 1_788_138_000;

fn signing_key() -> SigningKey {
	SigningKey::from_seed(KeyId::from_bytes([7; 16]), [42; 32])
}

fn claims() -> Claims {
	let namespace_id = Id::nil();
	let grants = [OwnedGrant {
		namespace: Scope::Id(namespace_id),
		resource: ResourceKind::Actor,
		target: Scope::Any,
		operations: vec![OperationKind::Read],
	}];
	let grants = ValidatedGrantSet::new(namespace_id, grants).unwrap();

	Claims {
		rivet_ver: CLAIMS_VERSION,
		iss: "https://api.rivet.dev".into(),
		aud: "rivet-api".into(),
		sub: Some("user_123".into()),
		iat: NOW,
		exp: NOW + 3_600,
		jti: TokenId::from_bytes([9; 16]).to_string(),
		rivet_ns: namespace_id.to_string(),
		rivet_grants: encode_grants(&grants).unwrap(),
	}
}

fn options() -> DecodeOptions<'static> {
	DecodeOptions {
		issuer: "https://api.rivet.dev",
		audience: "rivet-api",
		now: NOW,
	}
}

fn sign_claims_json(signing_key: &SigningKey, claims_json: &str) -> String {
	let mut header = Header::new(Algorithm::EdDSA);
	header.typ = Some(TOKEN_TYPE.into());
	header.kid = Some(signing_key.kid().to_string());
	let encoded_header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
	let encoded_claims = URL_SAFE_NO_PAD.encode(claims_json);
	let message = format!("{encoded_header}.{encoded_claims}");
	let signer = DalekSigningKey::from_bytes(signing_key.seed());
	let signature = signer.sign(message.as_bytes());

	format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
}

fn sign_claims_value(signing_key: &SigningKey, claims: serde_json::Value) -> String {
	let mut header = Header::new(Algorithm::EdDSA);
	header.typ = Some(TOKEN_TYPE.into());
	header.kid = Some(signing_key.kid().to_string());
	let signer = DalekSigningKey::from_bytes(signing_key.seed());
	let document = signer.to_pkcs8_der().unwrap();

	jsonwebtoken::encode(
		&header,
		&claims,
		&EncodingKey::from_ed_der(document.as_bytes()),
	)
	.unwrap()
}

#[test]
fn signs_and_verifies_the_complete_rivet_profile() {
	let signing_key = signing_key();
	let token = encode(&signing_key, &claims()).unwrap();
	let decoded = decode(&token, &signing_key.verification_key(), options()).unwrap();

	assert_eq!(decoded.claims, claims());
	assert_eq!(decoded.namespace_id, Id::nil());
	assert_eq!(decoded.grants.len(), 1);
	assert!(is_reserved_token(&token));
}

#[test]
fn only_reserves_rivet_access_tokens() {
	assert!(!is_reserved_token("opaque-token"));
	assert!(!is_reserved_token("one.two"));
	assert!(!is_reserved_token("one.two.three"));
	assert!(!is_reserved_token("e30.payload.signature"));

	let ordinary_header = URL_SAFE_NO_PAD.encode(br#"{"typ":"JWT","alg":"EdDSA"}"#);
	assert!(!is_reserved_token(&format!(
		"{ordinary_header}.payload.signature"
	)));
}

#[test]
fn emits_only_the_allowed_jose_header_members() {
	let signing_key = signing_key();
	let token = encode(&signing_key, &claims()).unwrap();
	let encoded_header = token.split('.').next().unwrap();
	let header: serde_json::Value =
		serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded_header).unwrap()).unwrap();

	assert_eq!(header["alg"], "EdDSA");
	assert_eq!(header["typ"], TOKEN_TYPE);
	assert_eq!(header["kid"], signing_key.kid().to_string());
	assert_eq!(header.as_object().unwrap().len(), 3);
	assert_eq!(peek_header(&token).unwrap().kid, signing_key.kid());
}

#[test]
fn rejects_the_wrong_key_issuer_and_audience() {
	let signing_key = signing_key();
	let token = encode(&signing_key, &claims()).unwrap();
	let wrong_key = SigningKey::from_seed(signing_key.kid(), [43; 32]).verification_key();
	assert_eq!(
		decode(&token, &wrong_key, options()),
		Err(TokenError::InvalidSignature)
	);
	let wrong_kid = VerificationKey::new(KeyId::from_bytes([8; 16]), signing_key.public_key());
	assert_eq!(
		decode(&token, &wrong_kid, options()),
		Err(TokenError::InvalidKeyId)
	);

	let wrong_issuer = DecodeOptions {
		issuer: "https://other.example",
		..options()
	};
	assert_eq!(
		decode(&token, &signing_key.verification_key(), wrong_issuer),
		Err(TokenError::InvalidClaims)
	);
	let wrong_audience = DecodeOptions {
		audience: "other-api",
		..options()
	};
	assert_eq!(
		decode(&token, &signing_key.verification_key(), wrong_audience),
		Err(TokenError::InvalidClaims)
	);
}

#[test]
fn enforces_protocol_time_bounds() {
	let signing_key = signing_key();
	let token = encode(&signing_key, &claims()).unwrap();
	let expired = DecodeOptions {
		now: claims().exp + 30,
		..options()
	};
	assert_eq!(
		decode(&token, &signing_key.verification_key(), expired),
		Err(TokenError::Expired)
	);

	let mut future_claims = claims();
	future_claims.iat = NOW + 31;
	future_claims.exp = future_claims.iat + 60;
	let future_token = encode(&signing_key, &future_claims).unwrap();
	assert_eq!(
		decode(&future_token, &signing_key.verification_key(), options()),
		Err(TokenError::IssuedInFuture)
	);

	let mut long_claims = claims();
	long_claims.exp = long_claims.iat + 86_401;
	assert_eq!(
		encode(&signing_key, &long_claims),
		Err(TokenError::LifetimeTooLong)
	);

	let mut inverted_claims = claims();
	inverted_claims.exp = inverted_claims.iat;
	assert_eq!(
		encode(&signing_key, &inverted_claims),
		Err(TokenError::InvalidClaims)
	);

	let mut future_profile = claims();
	future_profile.rivet_ver = CLAIMS_VERSION + 1;
	assert_eq!(
		encode(&signing_key, &future_profile),
		Err(TokenError::InvalidClaims)
	);

	let mut invalid_namespace = claims();
	invalid_namespace.rivet_ns = "not-an-id".into();
	assert_eq!(
		encode(&signing_key, &invalid_namespace),
		Err(TokenError::InvalidClaims)
	);

	let mut short_multibyte_namespace = serde_json::to_value(claims()).unwrap();
	short_multibyte_namespace["rivet_ns"] = serde_json::json!("💥");
	let short_multibyte_token = sign_claims_value(&signing_key, short_multibyte_namespace);
	assert_eq!(
		decode(
			&short_multibyte_token,
			&signing_key.verification_key(),
			options()
		),
		Err(TokenError::InvalidClaims)
	);

	let mut invalid_token_id = claims();
	invalid_token_id.jti = "not-a-token-id".into();
	assert_eq!(
		encode(&signing_key, &invalid_token_id),
		Err(TokenError::InvalidClaims)
	);

	assert_ne!(
		peek_header(&"x".repeat(MAX_TOKEN_BYTES)),
		Err(TokenError::TokenTooLarge)
	);
	assert_eq!(
		peek_header(&"x".repeat(MAX_TOKEN_BYTES + 1)),
		Err(TokenError::TokenTooLarge)
	);
}

#[test]
fn reserves_duplicate_typ_headers_but_strictly_rejects_them() {
	let header = URL_SAFE_NO_PAD.encode(format!(
		r#"{{"typ":"{TOKEN_TYPE}","typ":"other","alg":"EdDSA","kid":"{}"}}"#,
		signing_key().kid()
	));
	let payload = URL_SAFE_NO_PAD.encode(b"{}");
	let signature = URL_SAFE_NO_PAD.encode([0; 64]);
	let token = format!("{header}.{payload}.{signature}");

	assert!(is_reserved_token(&token));
	assert_eq!(peek_header(&token), Err(TokenError::InvalidHeader));
}

#[test]
fn rejects_padded_or_extended_jose_headers() {
	let signing_key = signing_key();
	let token = encode(&signing_key, &claims()).unwrap();
	let mut parts = token.split('.').map(str::to_owned).collect::<Vec<_>>();
	parts[0].push('=');
	let padded = parts.join(".");
	assert!(is_reserved_token(&padded));
	assert_eq!(peek_header(&padded), Err(TokenError::InvalidEncoding));

	let extended_header = URL_SAFE_NO_PAD.encode(format!(
		r#"{{"typ":"{TOKEN_TYPE}","alg":"EdDSA","kid":"{}","jku":"https://example.com/jwks"}}"#,
		signing_key.kid()
	));
	parts[0] = extended_header;
	assert_eq!(
		peek_header(&parts.join(".")),
		Err(TokenError::InvalidHeader)
	);
}

#[test]
fn rejects_duplicate_unknown_and_fractional_claims_after_signature_verification() {
	let signing_key = signing_key();
	let serialized_claims = serde_json::to_string(&claims()).unwrap();
	let duplicate_claims = format!(
		"{{\"iss\":\"https://duplicate.example\",{}",
		&serialized_claims[1..]
	);
	let duplicate_token = sign_claims_json(&signing_key, &duplicate_claims);
	assert_eq!(
		decode(&duplicate_token, &signing_key.verification_key(), options()),
		Err(TokenError::InvalidClaims)
	);

	let mut unknown_claims = serde_json::to_value(claims()).unwrap();
	unknown_claims["nbf"] = NOW.into();
	let unknown_token = sign_claims_value(&signing_key, unknown_claims);
	assert_eq!(
		decode(&unknown_token, &signing_key.verification_key(), options()),
		Err(TokenError::InvalidClaims)
	);

	let mut fractional_claims = serde_json::to_value(claims()).unwrap();
	fractional_claims["exp"] = serde_json::json!(1788141600.5);
	let fractional_token = sign_claims_value(&signing_key, fractional_claims);
	assert_eq!(
		decode(
			&fractional_token,
			&signing_key.verification_key(),
			options()
		),
		Err(TokenError::InvalidClaims)
	);
}

#[test]
fn rejects_algorithm_substitution_before_key_use() {
	let signing_key = signing_key();
	let header = URL_SAFE_NO_PAD.encode(format!(
		r#"{{"typ":"{TOKEN_TYPE}","alg":"HS256","kid":"{}"}}"#,
		signing_key.kid()
	));
	let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims()).unwrap());
	let signature = URL_SAFE_NO_PAD.encode([0; 64]);
	let token = format!("{header}.{payload}.{signature}");

	assert!(is_reserved_token(&token));
	assert_eq!(peek_header(&token), Err(TokenError::InvalidHeader));
}

#[test]
fn reconstructs_the_same_public_key_from_the_persisted_seed() {
	let key = signing_key();
	let restored = SigningKey::from_seed(key.kid(), *key.seed());
	assert_eq!(restored.public_key(), key.public_key());
	assert_eq!(
		restored.verification_key(),
		VerificationKey::new(key.kid(), key.public_key())
	);
}
