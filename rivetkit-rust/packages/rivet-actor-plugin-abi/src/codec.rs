//! Generic actor wire codec (spec §4.1) — shared, in lockstep, by the RivetKit
//! host and actor plugins so action args/replies round-trip identically.
//!
//! This is rivetkit's *generic* actor encoding: the
//! `["$Uint8Array", base64]` JSON-compat byte wrapping for replies. The arg
//! decoder (`decode_positional` + the CBOR `Value` deserializer) is ported in a
//! follow-up; both live here so a change forces an ABI version bump.
//!
//! Ported verbatim from `rivetkit/src/encoding.rs`.

use std::io::Write;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde::Serialize;

/// Tag string for the `Uint8Array` JSON-compat envelope (note capital `U`).
pub const JSON_COMPAT_UINT8_ARRAY: &str = "$Uint8Array";

/// Encode `value` as CBOR with byte payloads wrapped per the TS convention.
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
/// `["$Uint8Array", base64]` shape, recursively.
struct JsonCompatWrap<'a, T: ?Sized>(&'a T);

impl<T: Serialize + ?Sized> Serialize for JsonCompatWrap<'_, T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0.serialize(JsonCompatSerializer { inner: serializer })
	}
}

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

	/// The load-bearing override: byte payloads emit the 2-element tagged array.
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

macro_rules! compat_seq {
	($name:ident, $trait:path, $method:ident) => {
		struct $name<S> {
			inner: S,
		}
		impl<S: $trait> $trait for $name<S> {
			type Ok = S::Ok;
			type Error = S::Error;
			fn $method<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
				self.inner.$method(&JsonCompatWrap(value))
			}
			fn end(self) -> Result<Self::Ok, Self::Error> {
				self.inner.end()
			}
		}
	};
}

compat_seq!(
	JsonCompatSerializeSeq,
	serde::ser::SerializeSeq,
	serialize_element
);
compat_seq!(
	JsonCompatSerializeTuple,
	serde::ser::SerializeTuple,
	serialize_element
);
compat_seq!(
	JsonCompatSerializeTupleStruct,
	serde::ser::SerializeTupleStruct,
	serialize_field
);
compat_seq!(
	JsonCompatSerializeTupleVariant,
	serde::ser::SerializeTupleVariant,
	serialize_field
);

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

// ===========================================================================
// Arg decode codec — `decode_positional`/`encode_positional` + the CBOR
// `Value` deserializer. Ported verbatim from rivetkit `action.rs` + `event.rs`.
// ===========================================================================

use std::fmt;
use std::io::Cursor;

use anyhow::Context as _;
use ciborium::Value;
use serde::Deserialize;
use serde::de::{
	self, DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess,
	Visitor,
};

pub const TUPLE_ARITY_MAX: usize = 16;

/// Encode `value` as the positional CBOR array action args use.
pub fn encode_positional<T: Serialize>(value: &T) -> anyhow::Result<Vec<u8>> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded).context("encode action args as cbor")?;
	let value: Value = ciborium::from_reader(Cursor::new(&encoded))
		.context("decode action args into cbor value")?;
	let value = positional_value(value);
	encode_value(&value)
}

/// Decode positional CBOR action args into `T`, normalizing map/null/newtype.
pub fn decode_positional<T: DeserializeOwned>(args: &[u8]) -> anyhow::Result<T> {
	let value = if args.is_empty() {
		Value::Array(Vec::new())
	} else {
		ciborium::from_reader(Cursor::new(args)).context("decode action args from cbor")?
	};
	let value = match value {
		Value::Null => Value::Array(Vec::new()),
		value => value,
	};
	match decode_value::<T>(&value) {
		Ok(value) => Ok(value),
		Err(first_error) => match &value {
			Value::Array(values) if values.is_empty() => decode_value(&Value::Null)
				.or_else(|_| Err(first_error).context("decode positional action args as unit")),
			Value::Array(values) if values.len() == 1 => decode_value(&values[0])
				.or_else(|_| Err(first_error).context("decode positional action args as newtype")),
			_ => Err(first_error).context("decode positional action args"),
		},
	}
}

fn positional_value(value: Value) -> Value {
	match value {
		Value::Map(entries) => Value::Array(entries.into_iter().map(|(_, value)| value).collect()),
		Value::Array(values) => Value::Array(values),
		Value::Null => Value::Array(Vec::new()),
		value => Value::Array(vec![value]),
	}
}

fn encode_value(value: &Value) -> anyhow::Result<Vec<u8>> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded).context("encode positional action args as cbor")?;
	Ok(encoded)
}

fn decode_value<T: DeserializeOwned>(value: &Value) -> anyhow::Result<T> {
	deserialize_cbor_value(value.clone())
		.map_err(|error| anyhow::anyhow!(error.to_string()))
		.context("decode positional action args from cbor")
}

pub fn deserialize_cbor_value<T: DeserializeOwned>(value: Value) -> Result<T, de::value::Error> {
	T::deserialize(ValueDeserializer::new(value))
}

struct ValueDeserializer {
	value: Value,
}

impl ValueDeserializer {
	fn new(value: Value) -> Self {
		Self { value }
	}
}

impl<'de> de::Deserializer<'de> for ValueDeserializer {
	type Error = de::value::Error;

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Bool(value) => visitor.visit_bool(value),
			Value::Integer(value) => {
				let value = i128::from(value);
				if value < 0 {
					if let Ok(value) = i64::try_from(value) {
						visitor.visit_i64(value)
					} else {
						visitor.visit_i128(value)
					}
				} else if let Ok(value) = u64::try_from(value) {
					visitor.visit_u64(value)
				} else {
					visitor.visit_u128(value as u128)
				}
			}
			Value::Float(value) => visitor.visit_f64(value),
			Value::Bytes(value) => visitor.visit_byte_buf(value),
			Value::Text(value) => visitor.visit_string(value),
			Value::Null => visitor.visit_unit(),
			Value::Array(values) => visitor.visit_seq(ValueSeqAccess {
				values: values.into_iter(),
			}),
			Value::Map(entries) => visitor.visit_map(ValueMapAccess {
				entries: entries.into_iter(),
				value: None,
			}),
			Value::Tag(_, _) => Err(de::Error::custom(
				"tagged action payloads are not supported",
			)),
			_ => Err(de::Error::custom("unsupported action payload value")),
		}
	}

	fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Bool(value) => visitor.visit_bool(value),
			other => Err(invalid_type(&other, "a bool")),
		}
	}

	fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_i8(expect_signed(self.value, "an i8")?)
	}
	fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_i16(expect_signed(self.value, "an i16")?)
	}
	fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_i32(expect_signed(self.value, "an i32")?)
	}
	fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_i64(expect_signed(self.value, "an i64")?)
	}
	fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_i128(expect_signed(self.value, "an i128")?)
	}
	fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_u8(expect_unsigned(self.value, "a u8")?)
	}
	fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_u16(expect_unsigned(self.value, "a u16")?)
	}
	fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_u32(expect_unsigned(self.value, "a u32")?)
	}
	fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_u64(expect_unsigned(self.value, "a u64")?)
	}
	fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_u128(expect_unsigned(self.value, "a u128")?)
	}

	fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Float(value) => visitor.visit_f32(value as f32),
			other => Err(invalid_type(&other, "an f32")),
		}
	}
	fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Float(value) => visitor.visit_f64(value),
			other => Err(invalid_type(&other, "an f64")),
		}
	}

	fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Text(value) => {
				let mut chars = value.chars();
				match (chars.next(), chars.next()) {
					(Some(ch), None) => visitor.visit_char(ch),
					_ => Err(de::Error::custom("expected a single-character string")),
				}
			}
			other => Err(invalid_type(&other, "a char")),
		}
	}

	fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Text(value) => visitor.visit_string(value),
			other => Err(invalid_type(&other, "a string")),
		}
	}
	fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		self.deserialize_str(visitor)
	}

	fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Bytes(value) => visitor.visit_byte_buf(value),
			other => Err(invalid_type(&other, "bytes")),
		}
	}
	fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		self.deserialize_bytes(visitor)
	}

	fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Null => visitor.visit_none(),
			other => visitor.visit_some(ValueDeserializer::new(other)),
		}
	}

	fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Null => visitor.visit_unit(),
			other => Err(invalid_type(&other, "null")),
		}
	}
	fn deserialize_unit_struct<V>(
		self,
		_name: &'static str,
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		self.deserialize_unit(visitor)
	}
	fn deserialize_newtype_struct<V>(
		self,
		_name: &'static str,
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_newtype_struct(self)
	}

	fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Array(values) => visitor.visit_seq(ValueSeqAccess {
				values: values.into_iter(),
			}),
			other => Err(invalid_type(&other, "an array")),
		}
	}

	fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Array(values) => {
				if values.len() != len {
					return Err(de::Error::custom(format!(
						"expected tuple action payload with {len} elements, got {}",
						values.len()
					)));
				}
				visitor.visit_seq(ValueSeqAccess {
					values: values.into_iter(),
				})
			}
			other => Err(invalid_type(&other, "an array")),
		}
	}
	fn deserialize_tuple_struct<V>(
		self,
		_name: &'static str,
		len: usize,
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		self.deserialize_tuple(len, visitor)
	}

	fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Map(entries) => visitor.visit_map(ValueMapAccess {
				entries: entries.into_iter(),
				value: None,
			}),
			other => Err(invalid_type(&other, "a map")),
		}
	}

	fn deserialize_struct<V>(
		self,
		_name: &'static str,
		_fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Map(entries) => visitor.visit_map(ValueMapAccess {
				entries: entries.into_iter(),
				value: None,
			}),
			Value::Array(values) => visitor.visit_seq(ValueSeqAccess {
				values: values.into_iter(),
			}),
			other => Err(invalid_type(&other, "a map or array")),
		}
	}

	fn deserialize_enum<V>(
		self,
		_name: &'static str,
		_variants: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			Value::Text(variant) => visitor.visit_enum(ValueEnumAccess {
				variant,
				value: None,
			}),
			Value::Map(mut entries) if entries.len() == 1 => {
				let Some((key, value)) = entries.pop() else {
					return Err(de::Error::custom(
						"expected externally tagged enum map to contain one entry",
					));
				};
				match key {
					Value::Text(variant) => visitor.visit_enum(ValueEnumAccess {
						variant,
						value: Some(value),
					}),
					other => Err(invalid_type(&other, "a string enum variant")),
				}
			}
			other => Err(invalid_type(&other, "an externally tagged enum")),
		}
	}

	fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		self.deserialize_str(visitor)
	}
	fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_unit()
	}
}

struct ValueSeqAccess {
	values: std::vec::IntoIter<Value>,
}
impl<'de> SeqAccess<'de> for ValueSeqAccess {
	type Error = de::value::Error;
	fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
	where
		T: DeserializeSeed<'de>,
	{
		self.values
			.next()
			.map(|value| seed.deserialize(ValueDeserializer::new(value)))
			.transpose()
	}
	fn size_hint(&self) -> Option<usize> {
		Some(self.values.len())
	}
}

struct ValueMapAccess {
	entries: std::vec::IntoIter<(Value, Value)>,
	value: Option<Value>,
}
impl<'de> MapAccess<'de> for ValueMapAccess {
	type Error = de::value::Error;
	fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
	where
		K: DeserializeSeed<'de>,
	{
		match self.entries.next() {
			Some((key, value)) => {
				self.value = Some(value);
				seed.deserialize(ValueDeserializer::new(key)).map(Some)
			}
			None => Ok(None),
		}
	}
	fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
	where
		V: DeserializeSeed<'de>,
	{
		let value = self
			.value
			.take()
			.ok_or_else(|| de::Error::custom("value requested before key"))?;
		seed.deserialize(ValueDeserializer::new(value))
	}
	fn size_hint(&self) -> Option<usize> {
		Some(self.entries.len())
	}
}

struct ValueEnumAccess {
	variant: String,
	value: Option<Value>,
}
impl<'de> EnumAccess<'de> for ValueEnumAccess {
	type Error = de::value::Error;
	type Variant = ValueVariantAccess;
	fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
	where
		V: DeserializeSeed<'de>,
	{
		let variant = seed.deserialize(
			serde::de::value::StringDeserializer::<de::value::Error>::new(self.variant),
		)?;
		Ok((variant, ValueVariantAccess { value: self.value }))
	}
}

struct ValueVariantAccess {
	value: Option<Value>,
}
impl<'de> VariantAccess<'de> for ValueVariantAccess {
	type Error = de::value::Error;
	fn unit_variant(self) -> Result<(), Self::Error> {
		match self.value {
			None | Some(Value::Null) => Ok(()),
			Some(other) => Err(invalid_type(&other, "null")),
		}
	}
	fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
	where
		T: DeserializeSeed<'de>,
	{
		seed.deserialize(ValueDeserializer::new(self.value.unwrap_or(Value::Null)))
	}
	fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		de::Deserializer::deserialize_tuple(
			ValueDeserializer::new(self.value.unwrap_or(Value::Null)),
			len,
			visitor,
		)
	}
	fn struct_variant<V>(
		self,
		fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		de::Deserializer::deserialize_struct(
			ValueDeserializer::new(self.value.unwrap_or(Value::Null)),
			"enum",
			fields,
			visitor,
		)
	}
}

fn expect_signed<T>(value: Value, expected: &'static str) -> Result<T, de::value::Error>
where
	T: TryFrom<i128>,
{
	match value {
		Value::Integer(value) => T::try_from(i128::from(value))
			.map_err(|_| de::Error::custom(format!("expected {expected}"))),
		other => Err(invalid_type(&other, expected)),
	}
}

fn expect_unsigned<T>(value: Value, expected: &'static str) -> Result<T, de::value::Error>
where
	T: TryFrom<u128>,
{
	match value {
		Value::Integer(value) => T::try_from(
			u128::try_from(value).map_err(|_| de::Error::custom(format!("expected {expected}")))?,
		)
		.map_err(|_| de::Error::custom(format!("expected {expected}"))),
		other => Err(invalid_type(&other, expected)),
	}
}

fn invalid_type(value: &Value, expected: &'static str) -> de::value::Error {
	de::Error::invalid_type(unexpected(value), &Expected(expected))
}

fn unexpected(value: &Value) -> de::Unexpected<'_> {
	match value {
		Value::Bool(value) => de::Unexpected::Bool(*value),
		Value::Integer(value) => {
			let signed = i128::from(*value);
			if signed < 0 {
				if let Ok(value) = i64::try_from(signed) {
					de::Unexpected::Signed(value)
				} else {
					de::Unexpected::Other("integer")
				}
			} else if let Ok(value) = u64::try_from(signed) {
				de::Unexpected::Unsigned(value)
			} else {
				de::Unexpected::Other("integer")
			}
		}
		Value::Float(value) => de::Unexpected::Float(*value),
		Value::Bytes(value) => de::Unexpected::Bytes(value),
		Value::Text(value) => de::Unexpected::Str(value),
		Value::Null => de::Unexpected::Other("null"),
		Value::Tag(_, _) => de::Unexpected::Other("tag"),
		Value::Array(_) => de::Unexpected::Seq,
		Value::Map(_) => de::Unexpected::Map,
		_ => de::Unexpected::Other("value"),
	}
}

struct Expected(&'static str);
impl de::Expected for Expected {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(self.0)
	}
}

/// `Raw` action marker: refuses serde decoding (use `decode_positional`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raw;
impl<'de> Deserialize<'de> for Raw {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let _ = de::IgnoredAny::deserialize(deserializer)?;
		Err(de::Error::custom(
			"Raw cannot be deserialized; use decode_positional instead",
		))
	}
}

#[cfg(test)]
mod decode_tests {
	use super::{decode_positional, encode_positional};
	use serde::{Deserialize, Serialize};

	#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
	struct NamedArgs {
		first: String,
		second: String,
	}
	#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
	struct TupleArgs(String, String);
	#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
	struct NewtypeArg(u32);
	#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
	struct UnitArg;

	#[test]
	fn positional_encode_has_ts_byte_parity() {
		assert_eq!(
			encode_positional(&NamedArgs {
				first: "a".into(),
				second: "b".into()
			})
			.unwrap(),
			vec![0x82, 0x61, b'a', 0x61, b'b']
		);
		assert_eq!(encode_positional(&NewtypeArg(5)).unwrap(), vec![0x81, 0x05]);
		assert_eq!(encode_positional(&UnitArg).unwrap(), vec![0x80]);
	}

	#[test]
	fn positional_round_trips_arg_shapes() {
		let named = NamedArgs {
			first: "a".into(),
			second: "b".into(),
		};
		assert_eq!(
			decode_positional::<NamedArgs>(&encode_positional(&named).unwrap()).unwrap(),
			named
		);
		let tuple = TupleArgs("a".into(), "b".into());
		assert_eq!(
			decode_positional::<TupleArgs>(&encode_positional(&tuple).unwrap()).unwrap(),
			tuple
		);
		assert_eq!(
			decode_positional::<NewtypeArg>(&encode_positional(&NewtypeArg(5)).unwrap()).unwrap(),
			NewtypeArg(5)
		);
		assert_eq!(
			decode_positional::<UnitArg>(&encode_positional(&UnitArg).unwrap()).unwrap(),
			UnitArg
		);
	}
}
