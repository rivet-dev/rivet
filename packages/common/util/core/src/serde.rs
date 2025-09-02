use std::{
	collections::HashMap,
	fmt,
	hash::{Hash, Hasher},
	marker::PhantomData,
	ops::{Deref, DerefMut},
};

use indexmap::IndexMap;
use serde::{
	Deserialize, Deserializer, Serialize, Serializer,
	de::{self, DeserializeOwned},
};
use serde_json::{Number, value::RawValue};
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

	pub fn with_capacity(capacity: usize) -> Self {
		HashableMap(IndexMap::with_capacity(capacity))
	}
}

impl<K: Eq + Hash, V: Hash> Default for HashableMap<K, V> {
	fn default() -> Self {
		HashableMap::new()
	}
}

impl<K: Eq + Hash, V: PartialEq + Hash> PartialEq for HashableMap<K, V> {
	fn eq(&self, other: &HashableMap<K, V>) -> bool {
		if self.len() != other.len() {
			return false;
		}

		self.0
			.iter()
			.all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
	}
}

impl<K: Eq + Hash, V: Eq + Hash> Eq for HashableMap<K, V> {}

impl<K: Eq + Hash, V: Hash> Deref for HashableMap<K, V> {
	type Target = IndexMap<K, V>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<K: Eq + Hash, V: Hash> DerefMut for HashableMap<K, V> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
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

impl<K: Eq + Clone + Hash, V: Clone + Hash> From<HashMap<K, V>> for HashableMap<K, V> {
	fn from(val: HashMap<K, V>) -> Self {
		HashableMap(val.into_iter().collect())
	}
}

impl<K: Eq + Clone + Hash, V: Clone + Hash> From<HashableMap<K, V>> for HashMap<K, V> {
	fn from(val: HashableMap<K, V>) -> Self {
		val.into_iter().collect()
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

impl<K: Eq + Hash, V: Hash> Extend<(K, V)> for HashableMap<K, V> {
	fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
		self.0.extend(iter);
	}
}

// TODO: This doesn't work
// impl<K: ToSchema + Eq + Hash, T: ToSchema + Hash> ToSchema for HashableMap<K, T> {
// 	fn schemas(
// 		schemas: &mut Vec<(
// 			String,
// 			utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
// 		)>,
// 	) {
// 		K::schemas(schemas);
// 		T::schemas(schemas);
// 	}
// }
//
// impl<K: ToSchema + Eq + Hash, T: ToSchema + Hash> PartialSchema for HashableMap<K, T> {
// 	fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
// 		utoipa::openapi::ObjectBuilder::new()
// 			.additional_properties(Some(T::schema()))
// 			.into()
// 	}
// }

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
			inner: serde_json::value::to_raw_value(t)?,
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

/// A map-like structure that serializes/deserializes as a JSON object but is backed by a Vec.
/// Preserves insertion order and allows duplicate keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeMap<K, V>(Vec<(K, V)>);

impl<K, V> FakeMap<K, V> {
	pub fn new() -> Self {
		FakeMap(Vec::new())
	}

	pub fn with_capacity(capacity: usize) -> Self {
		FakeMap(Vec::with_capacity(capacity))
	}
}

impl<K: Ord, V> FakeMap<K, V> {
	/// Sort by keys.
	pub fn sort(&mut self)
	where
		K: Ord,
	{
		self.0.sort_by(|a, b| a.0.cmp(&b.0));
	}
}

impl<K, V> Default for FakeMap<K, V> {
	fn default() -> Self {
		FakeMap::new()
	}
}

impl<K, V> Deref for FakeMap<K, V> {
	type Target = Vec<(K, V)>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<K, V> DerefMut for FakeMap<K, V> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<K: Eq + Hash, V> From<Vec<(K, V)>> for FakeMap<K, V> {
	fn from(val: Vec<(K, V)>) -> Self {
		FakeMap(val)
	}
}

impl<K: Eq + Hash, V> From<FakeMap<K, V>> for Vec<(K, V)> {
	fn from(val: FakeMap<K, V>) -> Self {
		val.0
	}
}

impl<K: Eq + Hash, V> From<HashMap<K, V>> for FakeMap<K, V> {
	fn from(val: HashMap<K, V>) -> Self {
		FakeMap(val.into_iter().collect())
	}
}

impl<K: Eq + Hash, V> From<FakeMap<K, V>> for HashMap<K, V> {
	fn from(val: FakeMap<K, V>) -> Self {
		val.into_iter().collect()
	}
}

impl<K: Serialize, V: Serialize> Serialize for FakeMap<K, V> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		use serde::ser::SerializeMap;
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (k, v) in &self.0 {
			map.serialize_entry(k, v)?;
		}
		map.end()
	}
}

impl<'de, K: Deserialize<'de>, V: Deserialize<'de>> Deserialize<'de> for FakeMap<K, V> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct FakeMapVisitor<K, V> {
			marker: PhantomData<(K, V)>,
		}

		impl<'de, K: Deserialize<'de>, V: Deserialize<'de>> de::Visitor<'de> for FakeMapVisitor<K, V> {
			type Value = FakeMap<K, V>;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("a map")
			}

			fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
			where
				M: de::MapAccess<'de>,
			{
				let mut vec = Vec::with_capacity(access.size_hint().unwrap_or(0));
				while let Some((key, value)) = access.next_entry()? {
					vec.push((key, value));
				}
				Ok(FakeMap(vec))
			}
		}

		deserializer.deserialize_map(FakeMapVisitor {
			marker: PhantomData,
		})
	}
}

impl<K, V> FromIterator<(K, V)> for FakeMap<K, V> {
	fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
		FakeMap(iter.into_iter().collect())
	}
}

impl<K, V> IntoIterator for FakeMap<K, V> {
	type Item = (K, V);
	type IntoIter = std::vec::IntoIter<(K, V)>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

impl<'a, K, V> IntoIterator for &'a FakeMap<K, V> {
	type Item = &'a (K, V);
	type IntoIter = std::slice::Iter<'a, (K, V)>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.iter()
	}
}

impl<'a, K, V> IntoIterator for &'a mut FakeMap<K, V> {
	type Item = &'a mut (K, V);
	type IntoIter = std::slice::IterMut<'a, (K, V)>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.iter_mut()
	}
}

impl<K, V> Extend<(K, V)> for FakeMap<K, V> {
	fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
		self.0.extend(iter);
	}
}
