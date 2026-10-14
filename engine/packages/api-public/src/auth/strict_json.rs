//! Reject duplicate JSON members before converting to a typed request. Never return input text.
use serde::{
	Deserialize, Deserializer,
	de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Value};
use std::fmt;

struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
		struct UniqueVisitor;
		impl<'de> Visitor<'de> for UniqueVisitor {
			type Value = Unique;
			fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				f.write_str("JSON without duplicate fields")
			}
			fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<Unique, M::Error> {
				let mut values = Map::new();
				while let Some((key, Unique(value))) = m.next_entry::<String, Unique>()? {
					if values.insert(key, value).is_some() {
						return Err(de::Error::custom("duplicate JSON member"));
					}
				}
				Ok(Unique(Value::Object(values)))
			}
			fn visit_seq<S: SeqAccess<'de>>(self, mut s: S) -> Result<Unique, S::Error> {
				let mut values = Vec::new();
				while let Some(Unique(value)) = s.next_element()? {
					values.push(value);
				}
				Ok(Unique(Value::Array(values)))
			}
			fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
				Ok(Unique(v.into()))
			}
			fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
				Ok(Unique(v.into()))
			}
			fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
				Ok(Unique(v.into()))
			}
			fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
				Ok(Unique(Value::Number(
					serde_json::Number::from_f64(v).ok_or_else(|| E::custom("invalid number"))?,
				)))
			}
			fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
				Ok(Unique(v.into()))
			}
			fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
				Ok(Unique(v.into()))
			}
			fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
				Ok(Unique(Value::Null))
			}
		}
		d.deserialize_any(UniqueVisitor)
	}
}

pub fn parse<T: de::DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<T> {
	let unique: Unique =
		serde_json::from_slice(bytes).map_err(|_| crate::errors::Signing::InvalidInput.build())?;
	serde_json::from_value(unique.0).map_err(|_| crate::errors::Signing::InvalidInput.build())
}

#[cfg(test)]
#[path = "../../tests/unit/signing_json.rs"]
mod tests;
