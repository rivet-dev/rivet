use universaldb::prelude::{FormalKey, TuplePack, TupleUnpack};

use super::*;
use crate::{RotationPolicy, SigningKeyRecord, bootstrap};

const DAY: i64 = 24 * 60 * 60 * 1_000;

fn ring() -> SigningKeyRing {
	bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&SigningKeyRecord::generate(0),
		0,
		RotationPolicy {
			rotation_interval_ms: 7 * DAY,
			publish_lead_ms: 10 * 60 * 1_000,
			max_signing_lifetime_ms: 14 * DAY,
			max_token_ttl_ms: DAY,
			clock_skew_ms: 30 * 1_000,
		},
	)
	.unwrap()
	.successor
}

#[test]
fn signing_ring_logical_key_and_value_round_trip() {
	let key = SigningKeyRingKey;
	let packed = key.pack_to_vec();
	let unpacked = SigningKeyRingKey::unpack_root(&packed).unwrap();

	let expected = ring();
	let encoded = unpacked.serialize(expected.clone()).unwrap();
	assert_eq!(unpacked.deserialize(&encoded).unwrap(), expected);
}
