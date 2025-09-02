use std::pin::Pin;

use crate::FdbError;
use futures_util::Stream;

pub type FdbSlice = Vec<u8>;

#[derive(Debug, Clone)]
pub struct FdbValue(FdbKeyValue);

impl FdbValue {
	pub fn new(key: Vec<u8>, value: Vec<u8>) -> Self {
		FdbValue(FdbKeyValue::new(key, value))
	}

	pub fn from_keyvalue(kv: FdbKeyValue) -> Self {
		FdbValue(kv)
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

// FdbValues wraps a Vec<FdbKeyValue> to match FoundationDB API
#[derive(Debug, Clone)]
pub struct FdbValues {
	values: Vec<FdbKeyValue>,
	more: bool,
}

impl FdbValues {
	pub fn new(values: Vec<FdbKeyValue>) -> Self {
		FdbValues {
			values,
			more: false,
		}
	}

	pub fn with_more(values: Vec<FdbKeyValue>, more: bool) -> Self {
		FdbValues { values, more }
	}

	pub fn more(&self) -> bool {
		self.more
	}

	pub fn into_vec(self) -> Vec<FdbKeyValue> {
		self.values
	}

	pub fn len(&self) -> usize {
		self.values.len()
	}

	pub fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	pub fn iter(&self) -> std::slice::Iter<'_, FdbKeyValue> {
		self.values.iter()
	}

	pub fn into_iter(self) -> std::vec::IntoIter<FdbKeyValue> {
		self.values.into_iter()
	}
}

// impl Deref for FdbValues {
// 	type Target = [FdbKeyValue];
// 	fn deref(&self) -> &Self::Target {
// 		&self.values
// 	}
// }
// impl AsRef<[FdbKeyValue]> for FdbValues {
// 	fn as_ref(&self) -> &[FdbKeyValue] {
// 		self.deref()
// 	}
// }

// KeyValue type with key() and value() methods
#[derive(Debug, Clone)]
pub struct FdbKeyValue {
	key: Vec<u8>,
	value: Vec<u8>,
}

impl FdbKeyValue {
	pub fn new(key: Vec<u8>, value: Vec<u8>) -> Self {
		FdbKeyValue { key, value }
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

	pub fn to_value(self) -> FdbValue {
		FdbValue::from_keyvalue(self)
	}

	pub fn value_ref(&self) -> FdbValue {
		FdbValue::from_keyvalue(self.clone())
	}
}

// Stream type for range queries - generic over item type
pub type FdbStream<'a, T = FdbKeyValue> =
	Pin<Box<dyn Stream<Item = Result<T, FdbError>> + Send + 'a>>;

// UNIMPLEMENTED:
pub type FdbAddress = ();
pub type FdbAddresses = ();
pub type FdbValuesIter = ();
