//! Shared serde fixtures for codec tests: types that switch representation on
//! `is_human_readable()`, mirroring `uuid::Uuid`, to exercise both the current and legacy forms.

use serde::{Deserialize, Serialize};

/// Mimics `uuid::Uuid`: a hex string in human-readable mode, four raw bytes in binary mode.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct HrSensitive(pub(crate) [u8; 4]);

impl HrSensitive {
	pub(crate) fn hex(&self) -> String {
		self.0.iter().map(|byte| format!("{byte:02x}")).collect()
	}
}

impl Serialize for HrSensitive {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		if serializer.is_human_readable() {
			serializer.serialize_str(&self.hex())
		} else {
			serializer.serialize_bytes(&self.0)
		}
	}
}

impl<'de> Deserialize<'de> for HrSensitive {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		if deserializer.is_human_readable() {
			deserializer.deserialize_str(HrSensitiveVisitor)
		} else {
			deserializer.deserialize_bytes(HrSensitiveVisitor)
		}
	}
}

struct HrSensitiveVisitor;

impl serde::de::Visitor<'_> for HrSensitiveVisitor {
	type Value = HrSensitive;

	fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str("a hex string or four raw bytes")
	}

	fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<HrSensitive, E> {
		let bytes = (0..value.len())
			.step_by(2)
			.map(|index| u8::from_str_radix(&value[index..index + 2], 16))
			.collect::<Result<Vec<u8>, _>>()
			.map_err(E::custom)?;
		self.visit_bytes(&bytes)
	}

	fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<HrSensitive, E> {
		let bytes: [u8; 4] = value
			.try_into()
			.map_err(|_| E::custom("expected four bytes"))?;
		Ok(HrSensitive(bytes))
	}
}

/// A `(u8, u8)` tuple in binary mode, a `"a:b"` string in human-readable mode. Its binary form
/// is a CBOR array (not bytes), so only a full binary-mode retry can decode the legacy form.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct HrTuple(pub(crate) u8, pub(crate) u8);

impl Serialize for HrTuple {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		if serializer.is_human_readable() {
			serializer.serialize_str(&format!("{}:{}", self.0, self.1))
		} else {
			use serde::ser::SerializeTuple;

			let mut tuple = serializer.serialize_tuple(2)?;
			tuple.serialize_element(&self.0)?;
			tuple.serialize_element(&self.1)?;
			tuple.end()
		}
	}
}

impl<'de> Deserialize<'de> for HrTuple {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		use serde::de::Error;

		if deserializer.is_human_readable() {
			let text = String::deserialize(deserializer)?;
			let (first, second) = text
				.split_once(':')
				.ok_or_else(|| D::Error::custom("expected \"a:b\""))?;
			Ok(HrTuple(
				first.parse().map_err(D::Error::custom)?,
				second.parse().map_err(D::Error::custom)?,
			))
		} else {
			let (first, second) = <(u8, u8)>::deserialize(deserializer)?;
			Ok(HrTuple(first, second))
		}
	}
}
