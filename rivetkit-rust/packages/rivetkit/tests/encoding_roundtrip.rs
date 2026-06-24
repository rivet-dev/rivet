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

/// Encode/decode round-trip for a structured virtual filesystem stat payload.
/// Phase 2 gate: every field
/// (bool, u32, u64, f64, camelCase rename) survives the framework's
/// encode -> ciborium decode -> revive_json_compat path losslessly.
#[test]
fn encode_then_decode_round_trips_virtual_stat() {
	#[derive(Serialize)]
	struct VirtualStat {
		mode: u32,
		size: u64,
		blocks: u64,
		dev: u64,
		rdev: u64,
		#[serde(rename = "isDirectory")]
		is_directory: bool,
		#[serde(rename = "isSymbolicLink")]
		is_symbolic_link: bool,
		#[serde(rename = "atimeMs")]
		atime_ms: f64,
		#[serde(rename = "mtimeMs")]
		mtime_ms: f64,
		#[serde(rename = "ctimeMs")]
		ctime_ms: f64,
		#[serde(rename = "birthtimeMs")]
		birthtime_ms: f64,
		ino: u64,
		nlink: u64,
		uid: u32,
		gid: u32,
	}
	let value = VirtualStat {
		mode: 0o100_644,
		size: 7,
		blocks: 1,
		dev: 42,
		rdev: 0,
		is_directory: false,
		is_symbolic_link: true,
		atime_ms: 1_780_000_000_000.5,
		mtime_ms: 1_780_000_001_000.25,
		ctime_ms: 1_780_000_002_000.125,
		birthtime_ms: 1_780_000_003_000.0625,
		ino: 9_876_543_210,
		nlink: 1,
		uid: 1000,
		gid: 1000,
	};

	let encoded = encode_json_compat_to_vec(&value).expect("encode");
	let intermediate = ciborium_to_json(&encoded);
	let revived = revive_json_compat(intermediate);

	// u32 / u16 / small integers come back as JSON numbers.
	assert_eq!(revived["mode"], serde_json::json!(0o100_644u32));
	assert_eq!(revived["size"], serde_json::json!(7));
	assert_eq!(revived["blocks"], serde_json::json!(1));
	assert_eq!(revived["dev"], serde_json::json!(42));
	assert_eq!(revived["rdev"], serde_json::json!(0));

	// Booleans.
	assert_eq!(revived["isDirectory"], serde_json::json!(false));
	assert_eq!(revived["isSymbolicLink"], serde_json::json!(true));

	// f64 timestamps must preserve fractional precision.
	assert_eq!(
		revived["atimeMs"].as_f64().expect("atimeMs f64"),
		1_780_000_000_000.5,
	);
	assert_eq!(
		revived["mtimeMs"].as_f64().expect("mtimeMs f64"),
		1_780_000_001_000.25,
	);
	assert_eq!(
		revived["ctimeMs"].as_f64().expect("ctimeMs f64"),
		1_780_000_002_000.125,
	);
	assert_eq!(
		revived["birthtimeMs"].as_f64().expect("birthtimeMs f64"),
		1_780_000_003_000.0625,
	);

	// Large u64 — must not silently downcast through f64.
	assert_eq!(revived["ino"].as_u64().expect("ino u64"), 9_876_543_210u64,);
	assert_eq!(revived["nlink"], serde_json::json!(1));
	assert_eq!(revived["uid"], serde_json::json!(1000));
	assert_eq!(revived["gid"], serde_json::json!(1000));

	// camelCase renames must not leak the snake_case Rust names.
	assert!(revived.get("is_directory").is_none());
	assert!(revived.get("atime_ms").is_none());
}
