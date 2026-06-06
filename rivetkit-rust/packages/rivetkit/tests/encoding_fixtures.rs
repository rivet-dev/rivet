//! Generate cross-language parity fixtures. Writes Rust-encoded outputs
//! to JSON files that the TypeScript side reads to assert wire-format
//! parity (`tests/byte-encoding-parity.test.ts`).
//!
//! Run via `cargo test -p rivetkit --test encoding_fixtures`. The
//! fixtures land in `tests/fixtures/encoding/`.

use std::io::Cursor;
use std::path::PathBuf;

use rivetkit::encoding::encode_json_compat_to_vec;
use serde::Serialize;

fn fixture_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("tests")
		.join("fixtures")
		.join("encoding")
}

fn write_fixture(name: &str, intermediate: &serde_json::Value) {
	let dir = fixture_dir();
	std::fs::create_dir_all(&dir).expect("mkdir fixtures");
	let path = dir.join(format!("{name}.json"));
	let serialized =
		serde_json::to_string_pretty(intermediate).expect("serialize fixture as JSON");
	std::fs::write(&path, serialized).expect("write fixture");
	println!("wrote fixture: {}", path.display());
}

fn encode_and_cbor_decode<T: Serialize>(value: &T) -> serde_json::Value {
	let encoded = encode_json_compat_to_vec(value).expect("encode");
	ciborium::from_reader(Cursor::new(encoded)).expect("decode")
}

#[test]
fn fixture_uint8array_hello() {
	let bytes = serde_bytes::ByteBuf::from(b"hello".to_vec());
	let intermediate = encode_and_cbor_decode(&bytes);
	write_fixture("uint8array_hello", &intermediate);
}

#[test]
fn fixture_uint8array_1234() {
	let bytes = serde_bytes::ByteBuf::from(vec![1u8, 2, 3, 4]);
	let intermediate = encode_and_cbor_decode(&bytes);
	write_fixture("uint8array_1234", &intermediate);
}

#[test]
fn fixture_struct_with_byte_field() {
	#[derive(Serialize)]
	struct Reply {
		status: u16,
		body: serde_bytes::ByteBuf,
	}
	let value = Reply {
		status: 200,
		body: serde_bytes::ByteBuf::from(b"ok".to_vec()),
	};
	let intermediate = encode_and_cbor_decode(&value);
	write_fixture("struct_with_byte_field", &intermediate);
}

/// Structured non-byte payload modeled after `agent_os_client::VirtualStat`.
/// Exercises bool, u32, u64, f64, and camelCase `#[serde(rename)]` fields
/// to catch encoder bugs that the byte-only fixtures would miss. Phase 2
/// gate: this struct must round-trip losslessly across bare/cbor/json.
#[derive(Serialize)]
struct VirtualStatFixture {
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

#[test]
fn fixture_virtual_stat_struct() {
	let value = VirtualStatFixture {
		mode: 0o100_644,
		size: 7,
		blocks: 1,
		dev: 42,
		rdev: 0,
		is_directory: false,
		is_symbolic_link: false,
		atime_ms: 1_780_000_000_000.5,
		mtime_ms: 1_780_000_001_000.25,
		ctime_ms: 1_780_000_002_000.125,
		birthtime_ms: 1_780_000_003_000.0625,
		ino: 9_876_543_210,
		nlink: 1,
		uid: 1000,
		gid: 1000,
	};
	let intermediate = encode_and_cbor_decode(&value);
	write_fixture("virtual_stat", &intermediate);
}

#[test]
fn fixture_plain_string() {
	let value = "hello world".to_string();
	let intermediate = encode_and_cbor_decode(&value);
	write_fixture("plain_string", &intermediate);
}
