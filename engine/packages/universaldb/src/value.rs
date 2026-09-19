use std::{
	ops::{Deref, DerefMut},
	pin::Pin,
};

use anyhow::Result;

#[derive(Debug, PartialEq, Eq)]
pub struct Slice(Vec<u8>);

impl Slice {
	pub fn new() -> Self {
		Slice(Vec::new())
	}
}

impl Deref for Slice {
	type Target = Vec<u8>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for Slice {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl From<Vec<u8>> for Slice {
	fn from(value: Vec<u8>) -> Self {
		Slice(value)
	}
}

impl From<Slice> for Vec<u8> {
	fn from(value: Slice) -> Self {
		value.0
	}
}

#[derive(Debug, Clone)]
pub struct Value(KeyValue);

impl Value {
	pub fn new(key: Vec<u8>, value: Vec<u8>) -> Self {
		Value(KeyValue::new(key, value))
	}

	pub fn from_keyvalue(kv: KeyValue) -> Self {
		Value(kv)
	}

	pub fn key(&self) -> &[u8] {
		self.0.key()
	}

	pub fn value(&self) -> &[u8] {
		self.0.value()
	}

	pub fn into_parts(self) -> (Vec<u8>, Vec<u8>) {
		self.0.into_parts()
	}
}

// Values wraps a Vec<KeyValue> to match FoundationDB API
#[derive(Debug, Clone)]
pub struct Values {
	values: Vec<KeyValue>,
	more: bool,
	last_db_key: Option<Vec<u8>>,
}

impl Values {
	pub fn new(values: Vec<KeyValue>) -> Self {
		Values {
			values,
			more: false,
			last_db_key: None,
		}
	}

	/// One fetch of a chunked range scan. `last_db_key` is the last key the database itself returned,
	/// which is what pagination has to advance from. It is not always the last key in `values`:
	/// read-your-writes merging can append a pending write that sits past the end of this fetch, and a
	/// pending clear can empty the fetch entirely. Advancing from the merged rows would skip the keys
	/// in between in the first case and stop the scan early in the second.
	pub fn chunk(values: Vec<KeyValue>, more: bool, last_db_key: Option<Vec<u8>>) -> Self {
		Values {
			values,
			more,
			last_db_key,
		}
	}

	pub fn more(&self) -> bool {
		self.more
	}

	pub fn last_db_key(&self) -> Option<&[u8]> {
		self.last_db_key.as_deref()
	}

	pub fn into_vec(self) -> Vec<KeyValue> {
		self.values
	}

	pub fn len(&self) -> usize {
		self.values.len()
	}

	pub fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	pub fn iter(&self) -> std::slice::Iter<'_, KeyValue> {
		self.values.iter()
	}

	pub fn into_iter(self) -> std::vec::IntoIter<KeyValue> {
		self.values.into_iter()
	}
}

// impl Deref for Values {
// 	type Target = [KeyValue];
// 	fn deref(&self) -> &Self::Target {
// 		&self.values
// 	}
// }
// impl AsRef<[KeyValue]> for Values {
// 	fn as_ref(&self) -> &[KeyValue] {
// 		self.deref()
// 	}
// }

// KeyValue type with key() and value() methods
#[derive(Debug, Clone)]
pub struct KeyValue {
	key: Vec<u8>,
	value: Vec<u8>,
}

impl KeyValue {
	pub fn new(key: Vec<u8>, value: Vec<u8>) -> Self {
		KeyValue { key, value }
	}

	pub fn key(&self) -> &[u8] {
		&self.key
	}

	pub fn value(&self) -> &[u8] {
		&self.value
	}

	pub fn into_parts(self) -> (Vec<u8>, Vec<u8>) {
		(self.key, self.value)
	}

	pub fn to_value(self) -> Value {
		Value::from_keyvalue(self)
	}

	pub fn value_ref(&self) -> Value {
		Value::from_keyvalue(self.clone())
	}
}

// Stream type for range queries - generic over item type
pub type Stream<'a, T = KeyValue> =
	Pin<Box<dyn futures_util::Stream<Item = Result<T>> + Send + 'a>>;
