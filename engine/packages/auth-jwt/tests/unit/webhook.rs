use super::*;

#[test]
fn namespace_rings_persist_without_sharing_authentication_keys() {
	let ns = Id::new_v1(1);
	let ring = Ring::bootstrap(ns, 1, 1, 1000).unwrap();
	let bytes = ring.encode().unwrap();
	assert_eq!(Ring::decode(ns, &bytes).unwrap(), ring);
	assert!(Ring::decode(Id::new_v1(1), &bytes).is_err());
	let other = Ring::bootstrap(Id::new_v1(1), 1, 1, 1000).unwrap();
	assert_ne!(
		ring.keys.active.key.public_key,
		other.keys.active.key.public_key
	);
	assert!(public_kid(&ring.keys.active.key.public_key).starts_with("ds_"));
	let jwks = serde_json::to_value(ring.public_keys(1000)).unwrap();
	assert_eq!(jwks[0].as_object().unwrap().len(), 6);
	assert!(!jwks.to_string().contains("seed"));
}

#[test]
fn normal_rotation_publishes_before_use_and_retires_after_full_window() {
	let ring = Ring::bootstrap(Id::new_v1(1), 1, 1, 0).unwrap();
	let old = public_kid(&ring.keys.active.key.public_key);
	let stage = policy().rotation_interval_ms - policy().publish_lead_ms;
	let pending = ring.reconcile(1, 1, stage).unwrap().unwrap();
	assert_eq!(pending.public_keys(stage).len(), 2);
	assert_eq!(public_kid(&pending.keys.active.key.public_key), old);
	assert!(pending.reconcile(1, 1, stage + 1).unwrap().is_none());
	let active = pending
		.reconcile(1, 1, policy().rotation_interval_ms)
		.unwrap()
		.unwrap();
	assert_ne!(public_kid(&active.keys.active.key.public_key), old);
	let deadline = policy().rotation_interval_ms + 365_000;
	assert_eq!(active.keys.retiring[0].verify_until_ts, deadline);
	assert!(
		active
			.public_keys(deadline - 1)
			.iter()
			.any(|key| key.kid == old)
	);
	assert!(
		!active
			.public_keys(deadline)
			.iter()
			.any(|key| key.kid == old)
	);
}

#[test]
fn emergency_receipt_retries_compare_original_external_ids_before_resolving_removed_keys() {
	let ring = Ring::bootstrap(Id::new_v1(1), 1, 1, 0).unwrap();
	let old = public_kid(&ring.keys.active.key.public_key);
	let revoked = ring.revoke([1; 16], 1, &[old.clone()], 10).unwrap();
	assert!(!revoked.public_keys(10).iter().any(|key| key.kid == old));
	let restored = Ring::decode(ring.namespace_id, &revoked.encode().unwrap()).unwrap();
	assert_eq!(restored.revoke([1; 16], 1, &[old], 20).unwrap(), revoked);
	assert!(
		restored
			.revoke(
				[1; 16],
				1,
				&[public_kid(&restored.keys.active.key.public_key)],
				20
			)
			.is_err()
	);
	assert!(restored.rotate(1, 20).is_err());
	assert!(
		restored
			.revoke([2; 16], 2, &["unknown".into()], 20)
			.is_err()
	);
}

#[test]
fn namespace_lifecycle_fences_obsolete_topology() {
	let ring = Ring::bootstrap(Id::new_v1(1), 1, 2, 0).unwrap();
	assert!(ring.reconcile(1, 1, 1).is_err());
	assert!(ring.reconcile(2, 2, 1).is_err());
	let config = ring.reconcile(2, 3, 1).unwrap().unwrap();
	let moved = config.reconcile(2, 3, 2).unwrap().unwrap();
	assert_eq!(moved.keys.leader_datacenter_id, 2);
	assert!(moved.reconcile(1, 2, 3).is_err());
}

#[test]
fn signature_covers_the_timestamp_and_exact_binary_body() {
	use ed25519_dalek::{Signature as DalekSignature, Verifier, VerifyingKey};
	let ring = Ring::bootstrap(Id::new_v1(1), 1, 1, 1_699_999_999_000).unwrap();
	let body = [0, 255, b'\r', b'\n', b'{', b'}'];
	let signature = sign(&ring, &body, 1_700_000_000_999).unwrap();
	assert_eq!(signature.timestamp, "1700000000");
	assert!(!signature.signature.contains('='));
	let decoded = URL_SAFE_NO_PAD.decode(&signature.signature).unwrap();
	assert_eq!(decoded.len(), 64);
	let signature = DalekSignature::from_slice(&decoded).unwrap();
	let key = VerifyingKey::from_bytes(&ring.keys.active.key.public_key).unwrap();
	let mut message = b"1700000000.".to_vec();
	message.extend_from_slice(&body);
	key.verify(&message, &signature).unwrap();
	message[0] = b'2';
	assert!(key.verify(&message, &signature).is_err());
	message[0] = b'1';
	message.pop();
	assert!(key.verify(&message, &signature).is_err());
	assert!(sign(&ring, b"", 1_700_000_000_000).is_ok());
	assert!(sign(&ring, b"", ring.keys.active.sign_until_ts).is_err());
}

#[test]
fn ds_signature_and_thumbprint_match_independent_node_crypto_vector() {
	let mut ring = Ring::bootstrap(Id::new_v1(1), 1, 1, 1_699_999_999_000).unwrap();
	let seed = std::array::from_fn(|i| i as u8);
	let signing_key = crate::SigningKey::from_seed(KeyId::from_bytes([9; 16]), seed);
	ring.keys.active.key.public.kid = signing_key.kid();
	ring.keys.active.key.public.public_key = signing_key.public_key();
	ring.keys.active.key.signing_key = signing_key;
	let signature = sign(&ring, &[0, 255, 13, 10, 123, 125], 1_700_000_000_000).unwrap();
	assert_eq!(
		signature.kid,
		"ds_1IG2tMH7J2wbJZnOf8LJzQitKf7LMvoAElsuDMVM54Y"
	);
	assert_eq!(
		signature.signature,
		"bSaq685LYN-Z-YQr_ek2LGPvIUveIhxOAb1Fayki66s_MV_yoMjVEQEFl542ZTRr2-BCf_6S4QhC4RmoTwWKBg"
	);
}

#[test]
fn payload_limit_accepts_4096_canonicalized_maximum_path_snapshots() {
	use ed25519_dalek::{Signature as DalekSignature, Verifier, VerifyingKey};
	let streams: Vec<_> = (0..4096).map(|i| {
        let raw = format!("{i:04}/{}", "é".repeat(1020));
        assert!(raw.len() <= 2048);
        let canonical = url::Url::parse(&format!("https://streams.gameinc.io/{raw}")).unwrap().path().trim_start_matches('/').to_owned();
        serde_json::json!({"path":canonical,"link_type":"explicit","acked_offset":"0000000000000001_0000000000000042","tail_offset":"0000000000000002_0000000000000084","has_pending":true})
    }).collect();
	let body = serde_json::to_vec(&serde_json::json!({"subscription_id":"sub-1","wake_id":"wake-1","generation":7,"streams":streams,"callback_url":"https://streams.gameinc.io/__ds/subscriptions/sub-1/callback","callback_token":"a".repeat(crate::MAX_TOKEN_BYTES)})).unwrap();
	assert!(body.len() > 24 * 1024 * 1024);
	assert!(
		body.len() < 32 * 1024 * 1024,
		"4096-stream fixture must fit the default cap"
	);
	let ring = Ring::bootstrap(Id::new_v1(1), 1, 1, 0).unwrap();
	let signature = sign(&ring, &body, 1000).unwrap();
	let mut message = b"1.".to_vec();
	message.extend_from_slice(&body);
	VerifyingKey::from_bytes(&ring.keys.active.key.public_key)
		.unwrap()
		.verify(
			&message,
			&DalekSignature::from_slice(&URL_SAFE_NO_PAD.decode(signature.signature).unwrap())
				.unwrap(),
		)
		.unwrap();
	assert!(sign(&ring, &vec![0; MAX_PAYLOAD_BYTES + 1], 1000).is_err());
}
