//! Byte-payload decoding parity with the rivetkit TypeScript framework.
//!
//! TS sits on the server end of every action call (when invoked through
//! `rivetkit-typescript`). Action responses that contain `Uint8Array`
//! payloads arrive wrapped as `["$Uint8Array", base64]` per the
//! convention defined at
//! `rivetkit-typescript/packages/rivetkit/src/common/encoding.ts:14`.
//!
//! This module strips that wrapper before handing the result to the
//! caller.
//!
//! **Scope-limited:** only `JSON_COMPAT_UINT8_ARRAY` is recognized.
//! Other JSON-compat tags from the TS side (`$BigInt`, `$ArrayBuffer`,
//! `$Set`, `$Undefined`, etc.) are not revived — add them when a real
//! consumer needs them.
//!
//! ## Caveat — `serde_json::Value` has no byte variant
//!
//! TS's `reviveJsonCompatValue` returns a real `Uint8Array`. The Rust
//! client surface uses `serde_json::Value`, which has no native byte
//! representation. The revival strips the `["$Uint8Array", ...]` tag
//! and leaves the **base64-encoded string** as the field's value. The
//! caller knows from action-shape context which fields are bytes and
//! can decode the base64 if raw bytes are needed.
//!
//! This is a known limitation. A future revision could change the
//! action-result type to one that carries bytes natively, but that's a
//! larger API change.

use serde_json::Value;

/// Tag string for the `Uint8Array` JSON-compat envelope. Matches the
/// TypeScript constant.
pub const JSON_COMPAT_UINT8_ARRAY: &str = "$Uint8Array";

/// Walk a `serde_json::Value` and strip `["$Uint8Array", base64]`
/// wrappers, leaving the base64 string in place.
///
/// Recurses into arrays and objects so nested byte fields get unwrapped
/// too. Non-wrapper arrays and other types pass through unchanged.
pub fn revive_json_compat(value: Value) -> Value {
	match value {
		Value::Array(items) if is_uint8_array_tag(&items) => {
			// ["$Uint8Array", "<base64>"] → "<base64>"
			// Safe: is_uint8_array_tag guarantees items[1] is a string.
			items.into_iter().nth(1).expect("tagged array has 2 items")
		}
		Value::Array(items) => {
			Value::Array(items.into_iter().map(revive_json_compat).collect())
		}
		Value::Object(map) => {
			let mut revived = serde_json::Map::with_capacity(map.len());
			for (k, v) in map {
				revived.insert(k, revive_json_compat(v));
			}
			Value::Object(revived)
		}
		other => other,
	}
}

fn is_uint8_array_tag(items: &[Value]) -> bool {
	items.len() == 2
		&& items[0].as_str() == Some(JSON_COMPAT_UINT8_ARRAY)
		&& items[1].is_string()
}
