//! Encode-side tests for the `JSON_COMPAT_UINT8_ARRAY` byte-payload
//! wrapping convention. Mirrors what
//! `rivetkit-typescript/.../common/encoding.ts::encodeJsonCompatValue`
//! does for `Uint8Array` inputs.

use std::io::Cursor;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use rivetkit::encoding::encode_json_compat_to_vec;
use serde::Serialize;

fn decode_intermediate(encoded: &[u8]) -> serde_json::Value {
	ciborium::from_reader(Cursor::new(encoded)).expect("decode CBOR as JSON value")
}

#[test]
fn byte_buf_wraps_as_json_compat_uint8_array() {
	let bytes = serde_bytes::ByteBuf::from(b"hello".to_vec());
	let encoded = encode_json_compat_to_vec(&bytes).expect("encode");
	let intermediate = decode_intermediate(&encoded);

	assert!(intermediate.is_array(), "expected array, got {intermediate:?}");
	let arr = intermediate.as_array().unwrap();
	assert_eq!(arr.len(), 2);
	assert_eq!(arr[0], "$Uint8Array");
	assert_eq!(arr[1], BASE64_STANDARD.encode(b"hello"));
}

#[test]
fn nested_byte_field_in_struct_wraps() {
	#[derive(Serialize)]
	struct Reply {
		status: u16,
		body: serde_bytes::ByteBuf,
	}
	let value = Reply {
		status: 200,
		body: serde_bytes::ByteBuf::from(b"ok".to_vec()),
	};
	let encoded = encode_json_compat_to_vec(&value).expect("encode");
	let intermediate = decode_intermediate(&encoded);

	assert_eq!(intermediate["status"], 200);
	let body = &intermediate["body"];
	assert!(body.is_array(), "expected body to be wrapped array, got {body:?}");
	assert_eq!(body[0], "$Uint8Array");
	assert_eq!(body[1], BASE64_STANDARD.encode(b"ok"));
}

#[test]
fn plain_vec_u8_stays_as_array() {
	// Without #[serde(with = "serde_bytes")], Vec<u8> serializes via
	// `serialize_seq` (one integer per element), not `serialize_bytes`.
	// Matches TS's distinction between Uint8Array and other typed arrays.
	let value: Vec<u8> = vec![1, 2, 3];
	let encoded = encode_json_compat_to_vec(&value).expect("encode");
	let intermediate = decode_intermediate(&encoded);

	assert!(intermediate.is_array());
	let arr = intermediate.as_array().unwrap();
	assert_eq!(arr.len(), 3);
	assert_eq!(arr[0], 1);
	assert_eq!(arr[1], 2);
	assert_eq!(arr[2], 3);
}

#[test]
fn non_byte_types_pass_through_unchanged() {
	#[derive(Serialize)]
	struct Reply {
		msg: String,
		count: u32,
		enabled: bool,
		ratio: f64,
	}
	let value = Reply {
		msg: "hi".into(),
		count: 7,
		enabled: true,
		ratio: 1.5,
	};
	let encoded_compat = encode_json_compat_to_vec(&value).expect("encode via compat");
	let encoded_raw = {
		let mut buf = Vec::new();
		ciborium::into_writer(&value, &mut buf).expect("encode via ciborium");
		buf
	};
	// Compat path should be identical to raw ciborium when there are no
	// byte payloads to wrap.
	assert_eq!(
		encoded_compat, encoded_raw,
		"non-byte types should round-trip identically"
	);
}

#[test]
fn nested_byte_field_inside_optional_wraps() {
	#[derive(Serialize)]
	struct Reply {
		maybe_body: Option<serde_bytes::ByteBuf>,
	}
	let value = Reply {
		maybe_body: Some(serde_bytes::ByteBuf::from(b"present".to_vec())),
	};
	let encoded = encode_json_compat_to_vec(&value).expect("encode");
	let intermediate = decode_intermediate(&encoded);

	let body = &intermediate["maybe_body"];
	assert!(
		body.is_array(),
		"expected Some(byte_buf) to wrap, got {body:?}"
	);
	assert_eq!(body[0], "$Uint8Array");
	assert_eq!(body[1], BASE64_STANDARD.encode(b"present"));
}

#[test]
fn byte_field_inside_seq_wraps_each_element() {
	let values: Vec<serde_bytes::ByteBuf> = vec![
		serde_bytes::ByteBuf::from(b"a".to_vec()),
		serde_bytes::ByteBuf::from(b"bc".to_vec()),
	];
	let encoded = encode_json_compat_to_vec(&values).expect("encode");
	let intermediate = decode_intermediate(&encoded);

	let arr = intermediate.as_array().expect("outer should be array");
	assert_eq!(arr.len(), 2);
	for (i, expected) in [b"a".as_ref(), b"bc".as_ref()].into_iter().enumerate() {
		let item = &arr[i];
		assert!(item.is_array(), "item {i} should be wrapped array");
		assert_eq!(item[0], "$Uint8Array");
		assert_eq!(item[1], BASE64_STANDARD.encode(expected));
	}
}
