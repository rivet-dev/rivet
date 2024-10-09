use std::{
	collections::HashMap,
	fmt,
	hash::{Hash, Hasher},
	marker::PhantomData,
	ops::Deref,
};

use indexmap::IndexMap;
use serde::{de::{self, DeserializeOwned}, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{value::RawValue, Number};
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

/// Used in workflow activity inputs/outputs. Using this over BTreeMap is preferred because this does not
/// reorder keys, providing faster insert and lookup.
#[derive(Serialize, Deserialize)]
pub struct HashableMap<K: Eq + Hash, V: Hash>(IndexMap<K, V>);

impl<K: Eq + Hash, V: Hash> HashableMap<K, V> {
	pub fn new() -> Self {
		HashableMap(IndexMap::new())
	}
}

impl<K: Eq + Hash, V: Hash> Default for HashableMap<K, V> {
	fn default() -> Self {
		HashableMap::new()
	}
}

impl<K: Eq + Hash, V: Hash> Deref for HashableMap<K, V> {
	type Target = IndexMap<K, V>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<K: Eq + Ord + Hash, V: Hash> Hash for HashableMap<K, V> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		let mut kv = Vec::from_iter(&self.0);
		kv.sort_unstable_by(|a, b| a.0.cmp(b.0));
		kv.hash(state);
	}
}

impl<K: Eq + Hash + fmt::Debug, V: Hash + fmt::Debug> fmt::Debug for HashableMap<K, V> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_map().entries(self.iter()).finish()
	}
}

impl<K: Eq + Hash + Clone, V: Hash + Clone> Clone for HashableMap<K, V> {
	fn clone(&self) -> Self {
		HashableMap(self.0.clone())
	}

	fn clone_from(&mut self, other: &Self) {
		self.0.clone_from(&other.0);
	}
}

pub trait AsHashableExt<K: Eq + Hash, V: Hash> {
	/// Converts the iterable to a `HashableMap` via cloning.
	fn as_hashable(&self) -> HashableMap<K, V>;
}

impl<K: Eq + Clone + Hash, V: Clone + Hash> AsHashableExt<K, V> for HashMap<K, V> {
	fn as_hashable(&self) -> HashableMap<K, V> {
		HashableMap(self.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
	}
}

impl<K: Eq + Clone + Hash, V: Clone + Hash> Into<HashableMap<K, V>> for HashMap<K, V> {
	fn into(self) -> HashableMap<K, V> {
		HashableMap(self.into_iter().collect())
	}
}

impl<K: Eq + Hash, V: Hash> FromIterator<(K, V)> for HashableMap<K, V> {
	fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
		HashableMap(iter.into_iter().collect())
	}
}

impl<K: Eq + Hash, V: Hash> IntoIterator for HashableMap<K, V> {
	type Item = (K, V);
	type IntoIter = indexmap::map::IntoIter<K, V>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

impl<'a, K: Eq + Hash, V: Hash> IntoIterator for &'a HashableMap<K, V> {
	type Item = (&'a K, &'a V);
	type IntoIter = indexmap::map::Iter<'a, K, V>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.iter()
	}
}

impl<'a, K: Eq + Hash, V: Hash> IntoIterator for &'a mut HashableMap<K, V> {
	type Item = (&'a K, &'a mut V);
	type IntoIter = indexmap::map::IterMut<'a, K, V>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.iter_mut()
	}
}

/// Allows partial json ser/de.
/// Effectively a `serde_json::value::RawValue` with type information.
pub struct Raw<T> {
	_marker: PhantomData<T>,
	inner: Box<RawValue>,
}

impl<T> Raw<T> {
	pub fn from_string(s: String) -> Result<Self, serde_json::Error> {
		Ok(Raw {
			_marker: PhantomData,
			inner: serde_json::value::RawValue::from_string(s)?,
		})
	}
}

impl<T: Serialize> Raw<T> {
	pub fn new(t: &T) -> Result<Self, serde_json::Error> {
		Ok(Raw {
			_marker: PhantomData,
			inner: serde_json::value::RawValue::from_string(serde_json::to_string(&t)?)?,
		})
	}
}

impl<T: DeserializeOwned> Raw<T> {
	pub fn deserialize(&self) -> Result<T, serde_json::Error> {
		serde_json::from_str(self.inner.get())
	}
}

impl<T> Serialize for Raw<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.inner.serialize(serializer)
	}
}

impl<'de, T> Deserialize<'de> for Raw<T>
where
	T: DeserializeOwned,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let inner = Box::<RawValue>::deserialize(deserializer)?;
		Ok(Raw {
			_marker: PhantomData,
			inner,
		})
	}
}

impl<T> Clone for Raw<T> {
	fn clone(&self) -> Self {
		Raw {
			_marker: PhantomData,
			inner: self.inner.clone(),
		}
	}
}

impl<T: Hash> Hash for Raw<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.inner.get().hash(state);
	}
}

impl<T> std::ops::Deref for Raw<T> {
	type Target = RawValue;

	fn deref(&self) -> &Self::Target {
		self.inner.as_ref()
	}
}

impl<T: std::fmt::Debug> std::fmt::Debug for Raw<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.debug_tuple("Raw")
			.field(&format_args!("{}", self.inner.get()))
			.finish()
	}
}

impl<T, DB> sqlx::Type<DB> for Raw<T>
where
	DB: sqlx::Database,
	for<'a> sqlx::types::Json<&'a RawValue>: sqlx::Type<DB>,
{
	fn type_info() -> DB::TypeInfo {
		<RawValue as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &DB::TypeInfo) -> bool {
		<RawValue as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, T, DB> sqlx::Encode<'q, DB> for Raw<T>
where
	for<'a> sqlx::types::Json<&'a RawValue>: sqlx::Encode<'q, DB>,
	DB: sqlx::Database,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull {
		<sqlx::types::Json<&RawValue> as sqlx::Encode<'q, DB>>::encode(
			sqlx::types::Json(self.inner.as_ref()),
			buf,
		)
	}
}

impl<T, DB> sqlx::Decode<'_, DB> for Raw<T>
where
	DB: sqlx::Database,
	for<'a> std::string::String: sqlx::Decode<'a, DB>,
{
	fn decode(
		value: <DB as sqlx::database::HasValueRef<'_>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError> {
		Ok(Raw {
			_marker: PhantomData,
			inner: RawValue::from_string(<String as sqlx::Decode<DB>>::decode(value)?)?,
		})
	}
}

impl<T> sqlx::postgres::PgHasArrayType for Raw<T> {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
		// JSONB array
        sqlx::postgres::PgTypeInfo::with_name("_jsonb")
	}
}

impl<T> sqlx::postgres::PgHasArrayType for &Raw<T> {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
		// JSONB array
        sqlx::postgres::PgTypeInfo::with_name("_jsonb")
	}
}
