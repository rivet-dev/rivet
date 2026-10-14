use super::*;

#[test]
fn publishes_only_live_public_keys() {
	let key = |kid: &str, lifecycle| VerificationKeyEntry {
		kid: kid.to_owned(),
		public_key: vec![7; 32],
		lifecycle,
	};
	let keys = vec![
		key(
			"pending",
			VerificationKeyLifecycle::Pending {
				activate_after_ts: 20,
			},
		),
		key(
			"active",
			VerificationKeyLifecycle::Active {
				activated_ts: 1,
				sign_until_ts: 20,
			},
		),
		key(
			"expired",
			VerificationKeyLifecycle::Retiring {
				retired_ts: 1,
				max_token_exp_ts: 8,
				verify_until_ts: 10,
			},
		),
	];
	let jwks = JwkSet::from_verification_keys(&keys, 10);
	assert_eq!(jwks.keys.len(), 2);
	assert_eq!(jwks.keys[0].kid, "pending");
	assert_eq!(jwks.keys[1].kid, "active");
	let json = serde_json::to_value(jwks).unwrap();
	let published_key = &json["keys"][0];
	assert_eq!(published_key["use"], "sig");
	assert_eq!(published_key["kty"], "OKP");
	assert!(published_key.get("d").is_none());
	assert_eq!(published_key["x"], URL_SAFE_NO_PAD.encode([7; 32]));
}
