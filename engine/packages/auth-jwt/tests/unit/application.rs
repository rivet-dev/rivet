use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;

fn claims() -> Claims {
	Claims {
        rivet_ver: 1, rivet_purpose: "durable-streams.callback".into(), iss: "https://api.rivet.dev".into(),
        aud: "durable-streams".into(), sub: None, iat: 1000, exp: 87400,
        jti: crate::TokenId::generate(&mut rand::rngs::OsRng).to_string(),
        rivet_ns: crate::Id::new_v1(1).to_string(),
        app: json!({"durable_streams":{"root":"/streams", "subscription_id":"sub-1","wake_id":"wake-1","generation":7,"operations":["callback"]}}).as_object().unwrap().clone(),
    }
}

#[test]
fn application_tokens_are_signed_but_never_engine_credentials() {
	let key = SigningKey::generate();
	let claims = claims();
	let token = encode(&key, &claims, "rivet-api").unwrap();
	assert!(crate::is_reserved_token(&token));
	assert!(crate::peek_header(&token).is_err());
	assert!(
		crate::decode(
			&token,
			&key.verification_key(),
			crate::DecodeOptions {
				issuer: &claims.iss,
				audience: &claims.aud,
				now: 1001
			}
		)
		.is_err()
	);
	let mut validation = jsonwebtoken::Validation::new(Algorithm::EdDSA);
	validation.validate_exp = false;
	validation.set_audience(&["durable-streams"]);
	validation.set_issuer(&["https://api.rivet.dev"]);
	let decoded =
		jsonwebtoken::decode::<Claims>(&token, &key.verification_key().decoding_key(), &validation)
			.unwrap();
	assert_eq!(decoded.header.typ.as_deref(), Some(TOKEN_TYPE));
	assert_eq!(decoded.claims.app, claims.app);
	assert!(
		!String::from_utf8(
			URL_SAFE_NO_PAD
				.decode(token.split('.').nth(1).unwrap())
				.unwrap()
		)
		.unwrap()
		.contains("rivet_grants")
	);
}

#[test]
fn application_profile_is_reserved_even_with_ambiguous_or_padded_headers() {
	for header in [
		r#"{"typ":"rivet-app+jwt"}"#,
		r#"{"typ":"JWT","typ":"rivet-app+jwt"}"#,
		r#"{"typ":"rivet-app+jwt","typ":"JWT"}"#,
	] {
		for encoded in [
			URL_SAFE_NO_PAD.encode(header),
			base64::engine::general_purpose::URL_SAFE.encode(header),
		] {
			assert!(crate::is_reserved_token(&format!(
				"{encoded}.e30.signature"
			)));
		}
	}
}

#[test]
fn bounds_and_reserved_fields_cannot_be_overridden() {
	let key = SigningKey::generate();
	for audience in ["rivet-api", "custom-engine"] {
		let mut c = claims();
		c.aud = audience.into();
		assert!(encode(&key, &c, "custom-engine").is_err());
	}
	for purpose in ["", "RiVeT.auth", "engine-access"] {
		let mut c = claims();
		c.rivet_purpose = purpose.into();
		assert!(encode(&key, &c, "rivet-api").is_err());
	}
	for name in ["rivet_grants", "EXP", "engine", "invalid/name"] {
		let mut c = claims();
		c.app.insert(name.into(), json!({}));
		assert!(encode(&key, &c, "rivet-api").is_err());
	}
	let mut c = claims();
	c.app
		.insert("appdata".into(), json!({"data":"a".repeat(3072)}));
	assert!(encode(&key, &c, "rivet-api").is_err());
	let mut deep = json!({});
	for _ in 0..8 {
		deep = json!({"nested":deep});
	}
	let mut c = claims();
	c.app.insert("nested".into(), deep);
	assert!(encode(&key, &c, "rivet-api").is_err());
	let mut c = claims();
	c.exp = c.iat + PROTOCOL_MAX_TTL + 1;
	assert!(encode(&key, &c, "rivet-api").is_err());
	let mut c = claims();
	c.app.insert(
		"durable_streams".into(),
		json!({"exp":0,"rivet_grants":"inert"}),
	);
	assert!(encode(&key, &c, "rivet-api").is_ok());
}
