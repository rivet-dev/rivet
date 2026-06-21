//! Decode-side tests for the `JSON_COMPAT_UINT8_ARRAY` revival.
//!
//! Mirrors `rivetkit-typescript/.../common/encoding.ts::reviveJsonCompatValue`
//! for the `Uint8Array` case.

use rivetkit_client::encoding::revive_json_compat;
use serde_json::json;

#[test]
fn json_compat_uint8_array_revives_to_base64_string() {
	// Note: with `serde_json::Value`, we can't represent raw bytes
	// natively. The tag is stripped; the base64 string remains.
	// Callers know the field shape and decode as needed.
	let wrapped = json!(["$Uint8Array", "aGVsbG8="]);
	let revived = revive_json_compat(wrapped);
	assert_eq!(revived, json!("aGVsbG8="));
}

#[test]
fn nested_byte_field_revives_inside_struct() {
	let wrapped = json!({
		"status": 200,
		"body": ["$Uint8Array", "b2s="],
	});
	let revived = revive_json_compat(wrapped);
	assert_eq!(revived["status"], 200);
	assert_eq!(revived["body"], json!("b2s="));
}

#[test]
fn deeply_nested_byte_field_revives() {
	let wrapped = json!({
		"outer": {
			"middle": {
				"inner_bytes": ["$Uint8Array", "ZGVlcA=="]
			}
		}
	});
	let revived = revive_json_compat(wrapped);
	assert_eq!(revived["outer"]["middle"]["inner_bytes"], json!("ZGVlcA=="));
}

#[test]
fn non_byte_arrays_pass_through() {
	let value = json!([1, 2, 3]);
	assert_eq!(revive_json_compat(value.clone()), value);
}

#[test]
fn unrelated_tagged_arrays_pass_through() {
	// `["$BigInt", "12345"]` is a different tag; we only handle Uint8Array.
	let value = json!(["$BigInt", "12345"]);
	assert_eq!(revive_json_compat(value.clone()), value);

	// Random 2-element arrays where the first element isn't a recognized
	// tag should pass through unchanged.
	let value = json!(["hello", "world"]);
	assert_eq!(revive_json_compat(value.clone()), value);
}

#[test]
fn three_element_arrays_starting_with_tag_pass_through() {
	// Only 2-element arrays with the exact tag are recognized.
	let value = json!(["$Uint8Array", "data", "extra"]);
	assert_eq!(revive_json_compat(value.clone()), value);
}

#[test]
fn array_of_byte_payloads_each_revives() {
	let wrapped = json!([["$Uint8Array", "YQ=="], ["$Uint8Array", "YmM="],]);
	let revived = revive_json_compat(wrapped);
	assert_eq!(revived, json!(["YQ==", "YmM="]));
}

#[test]
fn primitives_pass_through() {
	assert_eq!(revive_json_compat(json!(null)), json!(null));
	assert_eq!(revive_json_compat(json!(true)), json!(true));
	assert_eq!(revive_json_compat(json!(42)), json!(42));
	assert_eq!(revive_json_compat(json!("string")), json!("string"));
	assert_eq!(revive_json_compat(json!(3.14)), json!(3.14));
}
