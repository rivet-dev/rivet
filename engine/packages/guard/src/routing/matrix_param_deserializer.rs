//! Serde deserializer for URI matrix parameters.
//!
//! Matrix params use the format `;key=value;key2=value2` on a path segment.
//! This deserializer converts a pre-parsed list of `(name, MatrixParamValue)`
//! entries into a typed struct via serde, supporting string, sequence (for
//! comma-separated keys), optional, and enum values.

use std::fmt;

use serde::{
	de::{self, DeserializeSeed, IntoDeserializer, MapAccess, Visitor, value},
	forward_to_deserialize_any,
};

#[derive(Debug)]
pub(crate) enum MatrixParamValue {
	String(String),
	Seq(Vec<String>),
}

pub(crate) struct MatrixParamDeserializer {
	pub(crate) entries: Vec<(String, MatrixParamValue)>,
}

struct MatrixParamMapAccess {
	entries: std::vec::IntoIter<(String, MatrixParamValue)>,
	next_value: Option<MatrixParamValue>,
}

struct MatrixParamValueDeserializer {
	value: MatrixParamValue,
}

impl<'de> serde::Deserializer<'de> for MatrixParamDeserializer {
	type Error = value::Error;

	fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_map(MatrixParamMapAccess {
			entries: self.entries.into_iter(),
			next_value: None,
		})
	}

	fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		self.deserialize_any(visitor)
	}

	fn deserialize_struct<V>(
		self,
		_name: &'static str,
		_fields: &'static [&'static str],
		visitor: V,
	) -> std::result::Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		self.deserialize_any(visitor)
	}

	forward_to_deserialize_any! {
		bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
		byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct enum
		identifier ignored_any
	}
}

impl<'de> MapAccess<'de> for MatrixParamMapAccess {
	type Error = value::Error;

	fn next_key_seed<K>(
		&mut self,
		seed: K,
	) -> std::result::Result<Option<K::Value>, Self::Error>
	where
		K: DeserializeSeed<'de>,
	{
		match self.entries.next() {
			Some((key, value)) => {
				self.next_value = Some(value);
				seed.deserialize(key.into_deserializer()).map(Some)
			}
			None => Ok(None),
		}
	}

	fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
	where
		V: DeserializeSeed<'de>,
	{
		let Some(value) = self.next_value.take() else {
			return Err(de::Error::custom("missing matrix param value"));
		};

		seed.deserialize(MatrixParamValueDeserializer { value })
	}
}

impl<'de> serde::Deserializer<'de> for MatrixParamValueDeserializer {
	type Error = value::Error;

	fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			MatrixParamValue::String(value) => visitor.visit_string(value),
			MatrixParamValue::Seq(values) => visitor.visit_seq(value::SeqDeserializer::new(
				values.into_iter().map(IntoDeserializer::into_deserializer),
			)),
		}
	}

	fn deserialize_option<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_some(self)
	}

	fn deserialize_enum<V>(
		self,
		name: &'static str,
		variants: &'static [&'static str],
		visitor: V,
	) -> std::result::Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			MatrixParamValue::String(value) => {
				value.into_deserializer().deserialize_enum(name, variants, visitor)
			}
			MatrixParamValue::Seq(_) => Err(de::Error::custom("expected string matrix param")),
		}
	}

	fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			MatrixParamValue::Seq(values) => visitor.visit_seq(value::SeqDeserializer::new(
				values.into_iter().map(IntoDeserializer::into_deserializer),
			)),
			MatrixParamValue::String(_) => Err(de::Error::custom("expected sequence matrix param")),
		}
	}

	fn deserialize_str<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			MatrixParamValue::String(value) => visitor.visit_string(value),
			MatrixParamValue::Seq(_) => Err(de::Error::custom("expected string matrix param")),
		}
	}

	fn deserialize_string<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		self.deserialize_str(visitor)
	}

	forward_to_deserialize_any! {
		bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes byte_buf map
		struct tuple tuple_struct unit unit_struct newtype_struct identifier ignored_any
	}
}

impl fmt::Debug for MatrixParamDeserializer {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("MatrixParamDeserializer")
			.field("entry_count", &self.entries.len())
			.finish()
	}
}
