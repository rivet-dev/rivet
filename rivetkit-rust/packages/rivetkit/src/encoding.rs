//! Byte-payload encoding parity with the rivetkit TypeScript framework.
//!
//! TS sits on at least one end of every action call (usually the client end).
//! The wire convention `["$Uint8Array", base64]` is what TS emits and what
//! TS expects. This module mirrors it for action-response encoding so Rust
//! actors can return byte payloads that round-trip correctly across all
//! three wire encodings (bare, cbor, json).
//!
//! **Scope-limited:** only `JSON_COMPAT_UINT8_ARRAY` is implemented. Other
//! JSON-compat tags from the TS side (`$BigInt`, `$ArrayBuffer`, `$Set`,
//! `$Undefined`, etc.) are not mirrored — add them when a real consumer
//! needs them.
//!
//! Reference: `rivetkit-typescript/packages/rivetkit/src/common/encoding.ts`
//! (`JSON_COMPAT_UINT8_ARRAY`, `encodeJsonCompatValue`).
//!
//! ## Convention
//!
//! Byte payloads (anything that goes through `serialize_bytes`, including
//! `serde_bytes::ByteBuf`, `serde_bytes::Bytes`, and `&[u8]` annotated
//! `#[serde(with = "serde_bytes")]`) are wrapped as a 2-element tagged
//! array:
//!
//! ```ignore
//! ["$Uint8Array", "<base64-encoded-bytes>"]
//! ```
//!
//! Plain `Vec<u8>` without `#[serde(with = "serde_bytes")]` is treated as
//! a CBOR array of integers — matching TS's distinction between
//! `Uint8Array` (wrapped) and other typed arrays (passed through).
//!
//! All other serde calls pass through to `ciborium` unchanged.

use std::io::Write;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde::Serialize;

/// Tag string for the `Uint8Array` JSON-compat envelope. Matches the
/// TypeScript constant in `rivetkit-typescript/.../common/encoding.ts:14`.
/// Note the capital `U`.
pub const JSON_COMPAT_UINT8_ARRAY: &str = "$Uint8Array";

/// Encode `value` as CBOR with byte payloads wrapped per the TS convention.
///
/// Use this in place of `ciborium::into_writer` at every site that
/// forwards a user value to a JS client (action replies, workflow
/// history, workflow replay).
pub fn encode_json_compat<T, W>(value: &T, writer: W) -> anyhow::Result<()>
where
	T: Serialize,
	W: Write,
{
	let wrapped = JsonCompatWrap(value);
	ciborium::into_writer(&wrapped, writer)?;
	Ok(())
}

/// Convenience wrapper that encodes to a `Vec<u8>`.
pub fn encode_json_compat_to_vec<T: Serialize>(value: &T) -> anyhow::Result<Vec<u8>> {
	let mut buf = Vec::new();
	encode_json_compat(value, &mut buf)?;
	Ok(buf)
}

/// Newtype that re-serializes any embedded `serialize_bytes` call as the
/// `["$Uint8Array", base64]` shape. Wraps any `Serialize` value so the
/// transformation applies recursively to nested fields.
struct JsonCompatWrap<'a, T: ?Sized>(&'a T);

impl<T: Serialize + ?Sized> Serialize for JsonCompatWrap<'_, T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0.serialize(JsonCompatSerializer { inner: serializer })
	}
}

/// Serializer adapter that intercepts `serialize_bytes` to emit the
/// rivetkit `Uint8Array` envelope. Every other method forwards to the
/// underlying serializer with the same `JsonCompatWrap` recursion so
/// nested byte fields get the same treatment.
struct JsonCompatSerializer<S> {
	inner: S,
}

impl<S: serde::Serializer> serde::Serializer for JsonCompatSerializer<S> {
	type Ok = S::Ok;
	type Error = S::Error;

	type SerializeSeq = JsonCompatSerializeSeq<S::SerializeSeq>;
	type SerializeTuple = JsonCompatSerializeTuple<S::SerializeTuple>;
	type SerializeTupleStruct = JsonCompatSerializeTupleStruct<S::SerializeTupleStruct>;
	type SerializeTupleVariant = JsonCompatSerializeTupleVariant<S::SerializeTupleVariant>;
	type SerializeMap = JsonCompatSerializeMap<S::SerializeMap>;
	type SerializeStruct = JsonCompatSerializeStruct<S::SerializeStruct>;
	type SerializeStructVariant = JsonCompatSerializeStructVariant<S::SerializeStructVariant>;

	fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_bool(v)
	}

	fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_i8(v)
	}

	fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_i16(v)
	}

	fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_i32(v)
	}

	fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_i64(v)
	}

	fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_i128(v)
	}

	fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_u8(v)
	}

	fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_u16(v)
	}

	fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_u32(v)
	}

	fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_u64(v)
	}

	fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_u128(v)
	}

	fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_f32(v)
	}

	fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_f64(v)
	}

	fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_char(v)
	}

	fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_str(v)
	}

	/// The load-bearing override. Byte payloads (`serde_bytes::ByteBuf`,
	/// `serde_bytes::Bytes`, `&[u8]` with `#[serde(with = "serde_bytes")]`)
	/// all funnel through here. Emit the 2-element tagged array shape.
	fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
		use serde::ser::SerializeTuple as _;
		let base64 = BASE64_STANDARD.encode(v);
		let mut tuple = self.inner.serialize_tuple(2)?;
		tuple.serialize_element(JSON_COMPAT_UINT8_ARRAY)?;
		tuple.serialize_element(&base64)?;
		tuple.end()
	}

	fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_none()
	}

	fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_some(&JsonCompatWrap(value))
	}

	fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_unit()
	}

	fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
		self.inner.serialize_unit_struct(name)
	}

	fn serialize_unit_variant(
		self,
		name: &'static str,
		variant_index: u32,
		variant: &'static str,
	) -> Result<Self::Ok, Self::Error> {
		self.inner
			.serialize_unit_variant(name, variant_index, variant)
	}

	fn serialize_newtype_struct<T: ?Sized + Serialize>(
		self,
		name: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error> {
		self.inner
			.serialize_newtype_struct(name, &JsonCompatWrap(value))
	}

	fn serialize_newtype_variant<T: ?Sized + Serialize>(
		self,
		name: &'static str,
		variant_index: u32,
		variant: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error> {
		self.inner
			.serialize_newtype_variant(name, variant_index, variant, &JsonCompatWrap(value))
	}

	fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
		Ok(JsonCompatSerializeSeq {
			inner: self.inner.serialize_seq(len)?,
		})
	}

	fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
		Ok(JsonCompatSerializeTuple {
			inner: self.inner.serialize_tuple(len)?,
		})
	}

	fn serialize_tuple_struct(
		self,
		name: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleStruct, Self::Error> {
		Ok(JsonCompatSerializeTupleStruct {
			inner: self.inner.serialize_tuple_struct(name, len)?,
		})
	}

	fn serialize_tuple_variant(
		self,
		name: &'static str,
		variant_index: u32,
		variant: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleVariant, Self::Error> {
		Ok(JsonCompatSerializeTupleVariant {
			inner: self
				.inner
				.serialize_tuple_variant(name, variant_index, variant, len)?,
		})
	}

	fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		Ok(JsonCompatSerializeMap {
			inner: self.inner.serialize_map(len)?,
		})
	}

	fn serialize_struct(
		self,
		name: &'static str,
		len: usize,
	) -> Result<Self::SerializeStruct, Self::Error> {
		Ok(JsonCompatSerializeStruct {
			inner: self.inner.serialize_struct(name, len)?,
		})
	}

	fn serialize_struct_variant(
		self,
		name: &'static str,
		variant_index: u32,
		variant: &'static str,
		len: usize,
	) -> Result<Self::SerializeStructVariant, Self::Error> {
		Ok(JsonCompatSerializeStructVariant {
			inner: self
				.inner
				.serialize_struct_variant(name, variant_index, variant, len)?,
		})
	}
}

// --- Compound serializer wrappers (each delegates element serialization
//     through `JsonCompatWrap` so nested byte fields get wrapped too) ---

struct JsonCompatSerializeSeq<S> {
	inner: S,
}

impl<S: serde::ser::SerializeSeq> serde::ser::SerializeSeq for JsonCompatSerializeSeq<S> {
	type Ok = S::Ok;
	type Error = S::Error;

	fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
		self.inner.serialize_element(&JsonCompatWrap(value))
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		self.inner.end()
	}
}

struct JsonCompatSerializeTuple<S> {
	inner: S,
}

impl<S: serde::ser::SerializeTuple> serde::ser::SerializeTuple for JsonCompatSerializeTuple<S> {
	type Ok = S::Ok;
	type Error = S::Error;

	fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
		self.inner.serialize_element(&JsonCompatWrap(value))
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		self.inner.end()
	}
}

struct JsonCompatSerializeTupleStruct<S> {
	inner: S,
}

impl<S: serde::ser::SerializeTupleStruct> serde::ser::SerializeTupleStruct
	for JsonCompatSerializeTupleStruct<S>
{
	type Ok = S::Ok;
	type Error = S::Error;

	fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
		self.inner.serialize_field(&JsonCompatWrap(value))
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		self.inner.end()
	}
}

struct JsonCompatSerializeTupleVariant<S> {
	inner: S,
}

impl<S: serde::ser::SerializeTupleVariant> serde::ser::SerializeTupleVariant
	for JsonCompatSerializeTupleVariant<S>
{
	type Ok = S::Ok;
	type Error = S::Error;

	fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
		self.inner.serialize_field(&JsonCompatWrap(value))
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		self.inner.end()
	}
}

struct JsonCompatSerializeMap<S> {
	inner: S,
}

impl<S: serde::ser::SerializeMap> serde::ser::SerializeMap for JsonCompatSerializeMap<S> {
	type Ok = S::Ok;
	type Error = S::Error;

	fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
		self.inner.serialize_key(&JsonCompatWrap(key))
	}

	fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
		self.inner.serialize_value(&JsonCompatWrap(value))
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		self.inner.end()
	}
}

struct JsonCompatSerializeStruct<S> {
	inner: S,
}

impl<S: serde::ser::SerializeStruct> serde::ser::SerializeStruct for JsonCompatSerializeStruct<S> {
	type Ok = S::Ok;
	type Error = S::Error;

	fn serialize_field<T: ?Sized + Serialize>(
		&mut self,
		key: &'static str,
		value: &T,
	) -> Result<(), Self::Error> {
		self.inner.serialize_field(key, &JsonCompatWrap(value))
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		self.inner.end()
	}
}

struct JsonCompatSerializeStructVariant<S> {
	inner: S,
}

impl<S: serde::ser::SerializeStructVariant> serde::ser::SerializeStructVariant
	for JsonCompatSerializeStructVariant<S>
{
	type Ok = S::Ok;
	type Error = S::Error;

	fn serialize_field<T: ?Sized + Serialize>(
		&mut self,
		key: &'static str,
		value: &T,
	) -> Result<(), Self::Error> {
		self.inner.serialize_field(key, &JsonCompatWrap(value))
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		self.inner.end()
	}
}
