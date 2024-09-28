use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Number;
use thiserror::Error;

/// Like serde_json::Value but with no nesting types (arrays, objects).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
}

#[derive(Debug, Error)]
pub enum DeserializeError {
    #[error("arrays and objects are not supported")]
    OnlySimple,
}

impl<'de> Deserialize<'de> for SimpleValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SimpleValueVisitor;

        impl<'de> de::Visitor<'de> for SimpleValueVisitor {
            type Value = SimpleValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a simple value (null, bool, number, or string)")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(SimpleValue::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(SimpleValue::Number(value.into()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(SimpleValue::Number(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(Number::from_f64(value).map_or(SimpleValue::Null, SimpleValue::Number))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(SimpleValue::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(SimpleValue::String(value))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(SimpleValue::Null)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(SimpleValue::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            fn visit_map<V>(self, _visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                Err(de::Error::custom(DeserializeError::OnlySimple))
            }

            fn visit_seq<V>(self, _visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                Err(de::Error::custom(DeserializeError::OnlySimple))
            }
        }

        deserializer.deserialize_any(SimpleValueVisitor)
    }
}

impl Serialize for SimpleValue {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        match self {
            SimpleValue::Null => serializer.serialize_unit(),
            SimpleValue::Bool(b) => serializer.serialize_bool(*b),
            SimpleValue::Number(n) => n.serialize(serializer),
            SimpleValue::String(s) => serializer.serialize_str(s),
        }
    }
}
