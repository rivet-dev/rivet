//! Round-trip test: encode via `rivetkit::encoding`, decode via
//! `rivetkit_client::encoding::revive_json_compat` (through a
//! `serde_json::Value` intermediate to simulate the engine's lossy
//! decode path).

use std::io::Cursor;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use rivetkit::encoding::encode_json_compat_to_vec;
use rivetkit_client::encoding::revive_json_compat;
use serde::Serialize;

fn ciborium_to_json(encoded: &[u8]) -> serde_json::Value {
	ciborium::from_reader(Cursor::new(encoded)).expect("ciborium decode as JSON value")
}

#[test]
fn encode_then_decode_round_trips_bytes() {
	let original = b"round-trip data".to_vec();
	let value = serde_bytes::ByteBuf::from(original.clone());

	let encoded = encode_json_compat_to_vec(&value).expect("encode");
	let intermediate = ciborium_to_json(&encoded);
	let revived = revive_json_compat(intermediate);

	// After revival, the base64 string is what remains.
	let base64 = revived.as_str().expect("revived to base64 string");
	let decoded = BASE64_STANDARD.decode(base64).expect("decode base64");
	assert_eq!(decoded, original);
}

#[test]
fn encode_then_decode_round_trips_nested_struct() {
	#[derive(Serialize)]
	struct Reply {
		status: u16,
		body: serde_bytes::ByteBuf,
	}
	let value = Reply {
		status: 200,
		body: serde_bytes::ByteBuf::from(b"hello".to_vec()),
	};

	let encoded = encode_json_compat_to_vec(&value).expect("encode");
	let intermediate = ciborium_to_json(&encoded);
	let revived = revive_json_compat(intermediate);

	assert_eq!(revived["status"], 200);
	let base64 = revived["body"].as_str().expect("body revived to base64");
	let decoded = BASE64_STANDARD.decode(base64).expect("decode base64");
	assert_eq!(decoded, b"hello");
}
